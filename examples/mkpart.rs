extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libparted;

use libparted::*;
use std::io;
use std::env;
use std::num::ParseIntError;
use std::process::exit;
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
            Unit::Mebibytes(m) => m * 1000 * 1000 / sector_size,
            Unit::Megabytes(mb) => mb * 1024 * 1024 / sector_size,
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
        } else if string.ends_with("M") {
            string[..string.len() - 1]
                .parse::<u64>()
                .map(Unit::Mebibytes)
        } else {
            string.parse::<u64>().map(Unit::Sectors)
        }
    }
}

fn get_config<I: Iterator<Item = String>>(mut args: I) -> io::Result<(String, Unit, Unit)> {
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

    Ok((device, start, length))
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
}

// TODO: Figure out how to create an 'Unformatted' partition.
fn create_partition(device: &str, start: Unit, length: Unit) -> Result<(), PartedError> {
    // Get and open the device; then use that to get the geometry and disk from the device.
    let mut dev = Device::new(&device).map_err(|why| PartedError::OpenDevice { why })?;

    // Get the sector start / length of the new partition.
    let sector_size = dev.sector_size();
    let start = start.to_sectors(sector_size);
    let length = length.to_sectors(sector_size);

    let geometry = Geometry::new(&dev, start as i64, length as i64)
        .map_err(|why| PartedError::CreateGeometry { why })?;
    let mut disk = Disk::new(&mut dev).map_err(|why| PartedError::CreateDisk { why })?;

    // Create an unformatted file system type.
    let fs_type = None;
    let part_type = PartitionType::PED_PARTITION_NORMAL;

    // Create a new partition from the disk, geometry, and the type.
    let mut partition = Partition::new(
        &mut disk,
        part_type,
        fs_type.as_ref(),
        geometry.start(),
        geometry.start() + geometry.length(),
    ).map_err(|why| PartedError::CreatePartition { why })?;

    if partition.is_flag_available(PartitionFlag::PED_PARTITION_LBA) {
        let _ = partition.set_flag(PartitionFlag::PED_PARTITION_LBA, true);
    }

    // Also get the exact constraints of the geometry.
    let constraint = geometry.exact().ok_or(PartedError::ExactConstraint)?;

    // Add the partition to the disk, and set the corresponding partition flag.
    if let Err(why) = disk.add_partition(&mut partition, &constraint) {
        return Err(PartedError::AddPartition { why });
    }

    // Commit changes to the disk, and exit the function, which will clean up
    // the constructed objects from libparted automatically.
    if let Err(why) = disk.commit() {
        return Err(PartedError::CommitChanges { why });
    }

    Ok(())
}

fn main() {
    let (device, start, length) = match get_config(env::args().skip(1)) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("mkpart error: {}", why);
            eprintln!("\tUsage: mkpart <device_path> <start_sector> <length_in_sectors>");
            eprintln!("\t       mkpart <device_path< <start_sector> <length_in_units>[M | MB]");
            exit(1);
        }
    };

    match create_partition(&device, start, length) {
        Ok(()) => (),
        Err(why) => {
            eprintln!("mkpart: {} errored: {}", device, why);
            exit(1);
        }
    }
}
