extern crate libparted;

use libparted::*;
use std::env;
use std::process::exit;

fn main() {
    // Open the device so that we may open the disk.
    let mut args = env::args().skip(1);
    let mut device = match args.next() {
        Some(path) => match Device::new(&path) {
            Ok(device) => device,
            Err(why) => {
                eprintln!("rmpart: unable to open {}: {}", path, why);
                exit(1);
            }
        },
        None => {
            eprintln!("rmpart: no device path specified");
            eprintln!("    USAGE: rmpart DEVICE PARTITION_NUM...");
            exit(1);
        }
    };

    {
        // Open the disk to make our destructive changes.
        let mut disk = match Disk::new(&mut device) {
            Ok(disk) => disk,
            Err(why) => {
                eprintln!("rmpart: unable to open disk: {}", why);
                exit(1);
            }
        };

        // Remove every partition ID specified.
        for arg in args {
            match arg.parse::<u32>().ok() {
                Some(partition_id) => {
                    if let Err(why) = disk.remove_partition_by_number(partition_id) {
                        eprintln!("rmpart: unable to add partition to removal queue: {}", why);
                        continue;
                    }
                }
                None => eprintln!("rmpart: invalid partition id: {}", arg),
            }
        }

        if let Err(why) = disk.commit() {
            eprintln!("rmpart: unable to commit changes to disk: {}", why);
            exit(1);
        }
    }

    if let Err(why) = device.sync() {
        eprintln!("rmpart: unable to sync device changes with the OS: {}", why);
        exit(1);
    }
}
