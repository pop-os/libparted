extern crate libparted;

use libparted::{Device, Disk};

use std::io::Result;
use std::process;
use std::str;

fn list() -> Result<()> {
    for device_res in Device::devices(true) {
        let device = device_res?;

        println!("Device {:?}: {:?}", str::from_utf8(device.path()), str::from_utf8(device.model()));

        let disk = Disk::new(device)?;

        for (i, part) in disk.parts().enumerate() {
            println!("  Part {}", i);
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
