extern crate libparted;

use libparted::*;
use std::env::args;
use std::process::exit;

fn main() {
    let args = args().collect::<Vec<String>>();
    if args.len() != 2 {
        eprintln!("a device must be specified");
        exit(1);
    }

    let mut dev = match Device::get(&args[1]) {
        Ok(dev) => dev,
        Err(why) => {
            eprintln!("unable to get {} device: {}", args[1], why);
            exit(1);
        }
    };

    let disk = match Disk::new(&mut dev) {
        Ok(disk) => disk,
        Err(why) => {
            eprintln!("unable to get {} disk: {}", args[1], why);
            exit(1);
        }
    };

    println!(
        "max len: {}\nmax start sector: {}",
        disk.max_partition_length(),
        disk.max_partition_start_sector()
    );
}
