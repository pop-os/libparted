extern crate libparted;

use libparted::*;
use std::env;
use std::process::exit;

fn create_and_append<'a>(
    disk: &mut Disk,
    ptype: PartitionType,
    ftype: Option<&FileSystemType>,
    constraint: &Constraint,
    start: i64,
    end: i64,
) -> Partition<'a> {
    let mut partition = match Partition::new(disk, ptype, ftype, start, end) {
        Ok(partition) => partition,
        Err(why) => {
            eprintln!("unable to create partition: {}", why);
            exit(1);
        }
    };

    if let Err(why) = disk.add_partition(&mut partition, &constraint) {
        eprintln!("unable to add partition to disk: {}", why);
        exit(1);
    }

    partition
}

fn main() {
    let device = if let Some(device) = env::args().skip(1).next() {
        device
    } else {
        exit(1);
    };

    let mut dev = match Device::new(&device) {
        Ok(dev) => dev,
        Err(why) => {
            eprintln!("unable to create device: {}", why);
            exit(1);
        }
    };

    // Creates a new partition table on the device while opening the disk.
    let mut disk = match Disk::new_fresh(&mut dev, DiskType::get("msdos").unwrap()) {
        Ok(disk) => disk,
        Err(why) => {
            eprintln!("unable to create partiton table on device: {}", why);
            exit(1);
        }
    };

    let constraint = disk.constraint_any().unwrap();

    let _first_part = create_and_append(
        &mut disk,
        PartitionType::PED_PARTITION_EXTENDED,
        None,
        &constraint,
        32,
        29311,
    );

    let _second_part = create_and_append(
        &mut disk,
        PartitionType::PED_PARTITION_LOGICAL,
        Some(&FileSystemType::get("ext2").unwrap()),
        &constraint,
        19584,
        29311,
    );

    let _third_part = create_and_append(
        &mut disk,
        PartitionType::PED_PARTITION_LOGICAL,
        Some(&FileSystemType::get("ext2").unwrap()),
        &constraint,
        2048,
        9727,
    );

    if let Err(why) = disk.commit() {
        eprintln!("unable to commit to disk: {}", why);
        exit(1);
    }

    let disk_dup = disk.duplicate().unwrap();

    // Checks if both partitions match
    for (src_part, dup_part) in disk.parts().zip(disk_dup.parts()) {
        let failed = src_part.geom_start() != dup_part.geom_start()
            || src_part.geom_end() != dup_part.geom_end();
        if failed {
            eprintln!("duplicated partition doesn't match");
            exit(1);
        }
    }
}
