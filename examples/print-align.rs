extern crate libparted;

use std::env::args;
use std::process::exit;
use libparted::*;

fn main() {
    let args = args().collect::<Vec<String>>();
    if args.len() != 2 {
        eprintln!("a device must be specified");
        exit(1);
    }

    let device = match Device::get(&args[1]) {
        Ok(device) => device,
        Err(why) => {
            eprintln!("unable to get {} device: {}", args[1], why);
            exit(1);
        }
    };

    match device.get_minimum_alignment() {
        Some(alignment) => println!("minimum: {} {}", alignment.offset(), alignment.grain_size()),
        None => println!("minimum: - -"),
    }

    match device.get_optimum_alignment() {
        Some(alignment) => println!("optimum: {} {}", alignment.offset(), alignment.grain_size()),
        None => println!("optimum: - -"),
    }

    let disk = match Disk::new(device) {
        Ok(disk) => disk,
        Err(why) => {
            eprintln!("unable to open disk from {} device: {}", args[1], why);
            exit(1);
        }
    };

    match disk.get_partition_alignment() {
        Ok(alignment) => println!(
            "partition alignment: {} {}",
            alignment.offset(),
            alignment.grain_size()
        ),
        Err(why) => {
            eprintln!(
                "unable to get disk partition alignment from {}: {}",
                args[1], why
            );
            exit(1);
        }
    }

    drop(disk);
}
