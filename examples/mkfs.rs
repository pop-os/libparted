extern crate libparted;
use libparted::*;
use std::io;
use std::env;
use std::process::exit;

fn get_config<I: Iterator<Item = String>>(mut args: I) -> io::Result<(String, String, u64, u64)> {
    fn config_err(msg: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg)
    }

    let device = args.next().ok_or_else(|| config_err("no device provided"))?;
    let fs = args.next().ok_or_else(|| config_err("no fs provided"))?;
    let start_str = args.next().ok_or_else(|| config_err("no start provided"))?;
    let length_str = args.next().ok_or_else(|| config_err("no length provided"))?;
    let start = start_str
        .parse::<u64>()
        .or_else(|_| Err(config_err("invalid start value")))?;
    let length = length_str
        .parse::<u64>()
        .or_else(|_| Err(config_err("invalid start value")))?;

    Ok((device, fs, start, length))
}

// TODO: Figure out how to get the file system type to be properly set.
fn create_partition_with_filesystem(
    device: &str,
    fs: &str,
    start: u64,
    length: u64,
) -> Result<(), ()> {
    let mut dev = match Device::new(&device) {
        Ok(device) => device,
        Err(why) => {
            eprintln!("mkfs: unable to open {} device: {}", device, why);
            return Err(());
        }
    };

    let geometry = match Geometry::new(&dev, start as i64, length as i64) {
        Ok(geometry) => geometry,
        Err(why) => {
            eprintln!("unable to create new geometry: {}", why);
            return Err(());
        }
    };

    let mut disk = match Disk::new(&mut dev) {
        Ok(disk) => disk,
        Err(why) => {
            eprintln!("mkfs: unable to open {} disk: {}", device, why);
            return Err(());
        }
    };

    let fs_type = match FileSystemType::get(&fs) {
        Some(fs_type) => fs_type,
        None => {
            eprintln!("unable to get {} file system type", fs);
            return Err(());
        }
    };

    let part_type = PartitionType::PED_PARTITION_NORMAL;

    let mut partition = match Partition::new(
        &mut disk,
        part_type,
        &fs_type,
        geometry.start(),
        geometry.length(),
    ) {
        Ok(partition) => partition,
        Err(why) => {
            eprintln!("unable to create partition: {}", why);
            return Err(());
        }
    };

    let constraint = match geometry.exact() {
        Some(constraint) => constraint,
        None => {
            eprintln!("unable to get exact constraint from geometry");
            return Err(());
        }
    };

    // Set as a boot partition.
    // if partition.is_flag_available(PartitionFlag::PED_PARTITION_BOOT) {
    //     let _ = partition.set_flag(PartitionFlag::PED_PARTITION_BOOT, true);
    // }

    if let Err(why) = disk.add_partition(&mut partition, &constraint) {
        eprintln!("unable to add partition to disk: {}", why);
        return Err(());
    }

    if let Err(why) = partition.set_system(&fs_type) {
        eprintln!(
            "unable to set the system type of the partition to the file system type: {}",
            why
        );
        return Err(());
    }

    if partition.is_flag_available(PartitionFlag::PED_PARTITION_LBA) {
        let _ = partition.set_flag(PartitionFlag::PED_PARTITION_LBA, true);
    }

    if let Err(why) = disk.commit() {
        eprintln!("unable to commit changes to disk: {}", why);
        return Err(());
    }
    Ok(())
}

fn main() {
    let (device, fs, start, length) = match get_config(env::args().skip(1)) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("mkfs error: {}", why);
            eprintln!("\tUsage:\n\t\tmkfs <device> <filesystem> <start> <length>");
            exit(1);
        }
    };

    exit(
        create_partition_with_filesystem(&device, &fs, start, length)
            .ok()
            .map_or(1, |_| 0),
    );
}
