extern crate libparted;
use libparted::*;
use std::io;
use std::env;
use std::process::exit;

fn get_config<I: Iterator<Item = String>>(mut args: I) -> io::Result<(String, u64, u64)> {
    fn config_err(msg: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg)
    }

    let device = args.next().ok_or_else(|| config_err("no device provided"))?;
    let start_str = args.next().ok_or_else(|| config_err("no start provided"))?;
    let length_str = args.next().ok_or_else(|| config_err("no length provided"))?;
    let start = start_str.parse::<u64>().or_else(|_| Err(config_err("invalid start value")))?;
    let length = length_str.parse::<u64>().or_else(|_| Err(config_err("invalid start value")))?;

    Ok((device, start, length))
}

// TODO: Figure out how to create an 'Unformatted' partition.
fn create_partition(device: &str, start: u64, length: u64) -> Result<(), ()> {
    let dev = match Device::new(&device) {
        Ok(device) => device,
        Err(why) => {
            eprintln!("mkpart: unable to open {} device: {}", device, why);
            return Err(())
        }
    };

    let geometry = match Geometry::new(&dev, start as i64, length as i64) {
        Ok(geometry) => geometry,
        Err(why) => {
            eprintln!("unable to create new geometry: {}", why);
            return Err(());
        }
    };

    let mut disk = match Disk::new(dev) {
        Ok(disk) => disk,
        Err(why) => {
            eprintln!("mkpart: unable to open {} disk: {}", device, why);
            return Err(())
        }
    };

    use std::ptr;
    let fs_type = FileSystemType::from_raw(ptr::null_mut());
    let part_type = PartitionType::PED_PARTITION_NORMAL;

    let mut partition = match Partition::new(&mut disk, part_type, &fs_type, geometry.start(), geometry.length()) {
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

    if let Err(why) = disk.add_partition(&mut partition, &constraint) {
        eprintln!("unable to add partition to disk: {}", why);
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
    let (device, start, length) = match get_config(env::args().skip(1)) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("mkpart error: {}", why);
            eprintln!("\tUsage:\n\t\tmkpart <device> <start> <length>");
            exit(1);
        }
    };

    exit(create_partition(&device, start, length).ok().map_or(1, |_| 0));
}