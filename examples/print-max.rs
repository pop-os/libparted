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

    let disk = match Device::get(&args[1]).and_then(Disk::new) {
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