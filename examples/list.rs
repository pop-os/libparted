extern crate libparted;

use libparted::{Device, Disk};

use std::io::Result;
use std::process;
use std::str;

fn list() -> Result<()> {
    for (dev_i, device_res) in Device::devices(true).enumerate() {
        let device = device_res?;

        println!("Device {}", dev_i);
        println!("  Model: {:?}", str::from_utf8(device.model()));
        println!("  Path: {:?}", str::from_utf8(device.path()));
        println!("  Size: {} MB", device.length() * device.sector_size() / 1000000);

        let disk = Disk::new(device)?;

        for (part_i, part) in disk.parts().enumerate() {
            println!("  Part {}", part_i);
            println!("    Type: {:?}", str::from_utf8(part.type_name()));
            if let Some(name) = part.name() {
                println!("    Name: {:?}", str::from_utf8(name));
            }
            if let Some(path) = part.path() {
                println!("    Path: {:?}", str::from_utf8(path));
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = list() {
        eprintln!("list: failed to list: {}", err);
        process::exit(1);
    }
}
