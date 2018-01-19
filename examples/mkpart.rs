extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libparted;

use libparted::*;
use std::io;
use std::env;
use std::num::ParseIntError;
use std::path::Path;
use std::process::{exit, Command, Stdio};
use std::str::{self, FromStr};

enum Unit {
    Sectors(u64),
    Mebibytes(u64),
    Megabytes(u64),
}

impl Unit {
    pub fn to_sectors(self, sector_size: u64) -> u64 {
        match self {
            Unit::Sectors(sectors) => sectors,
            Unit::Megabytes(mb) => mb * 1000 * 1000 / sector_size,
            Unit::Mebibytes(mib) => mib * 1024 * 1024 / sector_size,
        }
    }
}

impl FromStr for Unit {
    type Err = ParseIntError;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if string.ends_with("MB") {
            string[..string.len() - 2]
                .parse::<u64>()
                .map(Unit::Megabytes)
        } else if string.ends_with("MiB") {
            string[..string.len() - 3]
                .parse::<u64>()
                .map(Unit::Mebibytes)
        } else if string.ends_with("M") {
            string[..string.len() - 1]
                .parse::<u64>()
                .map(Unit::Megabytes)
        } else {
            string.parse::<u64>().map(Unit::Sectors)
        }
    }
}

fn get_config<I: Iterator<Item = String>>(
    mut args: I,
) -> io::Result<(String, Unit, Unit, Option<String>)> {
    fn config_err(msg: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg)
    }

    let device = args.next().ok_or_else(|| config_err("no device provided"))?;
    let start_str = args.next().ok_or_else(|| config_err("no start provided"))?;
    let length_str = args.next().ok_or_else(|| config_err("no length provided"))?;
    let start = start_str
        .parse::<Unit>()
        .map_err(|_| config_err("invalid start value"))?;
    let length = length_str
        .parse::<Unit>()
        .map_err(|_| config_err("invalid sector length"))?;

    Ok((device, start, length, args.next()))
}

#[derive(Debug, Fail)]
pub enum PartedError {
    #[fail(display = "unable to open device: {}", why)] OpenDevice { why: io::Error },
    #[fail(display = "unable to create new geometry: {}", why)] CreateGeometry { why: io::Error },
    #[fail(display = "unable to create new disk: {}", why)] CreateDisk { why: io::Error },
    #[fail(display = "unable to create new partition: {}", why)] CreatePartition { why: io::Error },
    #[fail(display = "unable to get exact constraint from geometry")] ExactConstraint,
    #[fail(display = "unable to add partition to disk: {}", why)] AddPartition { why: io::Error },
    #[fail(display = "unable to commit changes to disk: {}", why)] CommitChanges { why: io::Error },
    #[fail(display = "invalid file system type")] InvalidFileSystemType,
    #[fail(display = "unable to sync device: {}", why)] SyncErr { why: io::Error },
    #[fail(display = "unable to set disk flag")] DiskFlagErr,
    #[fail(display = "unable to get constraint: {}", why)] GetConstraint { why: io::Error },
    #[fail(display = "unable to intersect constraints")] ConstraintIntersect,
    #[fail(display = "unable to ind newly-created partition")] FindPartition,
    #[fail(display = "unable to format partition: {}", why)] FormatPartition { why: io::Error },
}

fn mkfs(device: &str, fs: &str) -> io::Result<()> {
    let (command, args): (&str, &[&str]) = match fs {
        "fat16" => ("mkfs.fat", &["-F", "16"]),
        "fat32" => ("mkfs.fat", &["-F", "32"]),
        "ext2" => ("mkfs.ext2", &["-F", "-q"]),
        "ext4" => ("mkfs.ext4", &["-F", "-q"]),
        "btrfs" => ("mkfs.btrfs", &["-f"]),
        "ntfs" => ("mkfs.ntfs", &["-F"]),
        "xfs" => ("mkfs.xfs", &["-f"]),
        "swap" => ("mkswap", &["-f"]),
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported fs")),
    };

    let status = Command::new(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .arg(device)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("mkfs for {} failed with {}", fs, status),
        ))
    }
}

// TODO: Figure out how to create an 'Unformatted' partition.
fn create_partition(
    device: &str,
    start: Unit,
    length: Unit,
    fs: Option<String>,
) -> Result<(), PartedError> {
    // Get and open the device; then use that to get the geometry and disk from the device.
    let mut dev = Device::new(&device).map_err(|why| PartedError::OpenDevice { why })?;

    // Get the sector start / length of the new partition.
    let sector_size = dev.sector_size();
    let start = start.to_sectors(sector_size);
    let length = length.to_sectors(sector_size);

    let geometry = Geometry::new(&dev, start as i64, length as i64)
        .map_err(|why| PartedError::CreateGeometry { why })?;

    // Create a new partition with the following file system type.
    let fs = fs.unwrap_or("ext2".into());
    let fs_type = match FileSystemType::get(&fs) {
        Some(fs) => fs,
        None => {
            eprintln!("invalid fs provided: {}", fs);
            exit(1);
        }
    };

    {
        let mut disk = Disk::new(&mut dev).map_err(|why| PartedError::CreateDisk { why })?;

        let part_type = PartitionType::PED_PARTITION_NORMAL;

        // Create a new partition from the disk, geometry, and the type.
        let mut partition = Partition::new(
            &mut disk,
            part_type,
            Some(&fs_type),
            geometry.start(),
            geometry.start() + geometry.length(),
        ).map_err(|why| PartedError::CreatePartition { why })?;

        let constraint = geometry.exact().unwrap();

        // Add the partition to the disk, and set the corresponding partition flag.
        if let Err(why) = disk.add_partition(&mut partition, &constraint) {
            return Err(PartedError::AddPartition { why });
        }

        // Commit changes to the disk, and exit the function, which will clean up
        // the constructed objects from libparted automatically.
        if let Err(why) = disk.commit() {
            return Err(PartedError::CommitChanges { why });
        }
    }

    if let Err(why) = dev.sync() {
        return Err(PartedError::SyncErr { why });
    }

    let device_path = dev.path().to_path_buf();

    {
        let disk = Disk::new(&mut dev).map_err(|why| PartedError::CreateDisk { why })?;

        {
            let new_part = disk.get_partition_by_sector(start as i64)
                .ok_or(PartedError::FindPartition)?;

            let device_path = format!("{}{}", device_path.display(), new_part.num());
            eprintln!("mkpart: formatting '{}' with '{}'", device_path, fs);
            mkfs(&device_path, &fs).map_err(|why| PartedError::FormatPartition { why })?;
        }
    }

    // Drop and re-open the device to obtain updated partition information.
    drop(dev);
    let mut dev = Device::get(&device).map_err(|why| PartedError::OpenDevice { why })?;
    let disk = Disk::new(&mut dev).map_err(|why| PartedError::CreateDisk { why })?;

    // Displays the new partition layout to the user.
    println!("New Partition Scheme:");
    for (part_i, part) in disk.parts().enumerate() {
        let name = part.type_get_name();
        if name == "metadata" || name == "free" {
            continue;
        }
        println!("Part: {}", part_i);
        println!("    Path:   {:?}", part.get_path());
        println!("    FS:     {:?}", part.fs_type_name());
        println!("    Start:  {}", part.geom_start());
        println!("    End:    {}", part.geom_end());
        println!("    Length: {}", part.geom_length());
    }

    Ok(())
}

fn main() {
    let (device, start, length, fs) = match get_config(env::args().skip(1)) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("mkpart error: {}", why);
            eprintln!("\tUsage: mkpart <device_path> <start_sector> <length_in_sectors> [<fs>]");
            eprintln!(
                "\t       mkpart <device_path< <start_sector> <length_in_units>[M | MB] [<fs>]"
            );
            exit(1);
        }
    };

    match create_partition(&device, start, length, fs) {
        Ok(()) => (),
        Err(why) => {
            eprintln!("mkpart: {} errored: {}", device, why);
            exit(1);
        }
    }
}
