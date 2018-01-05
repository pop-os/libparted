extern crate libc;
extern crate libparted;

use libparted::{Device, Disk};

use std::io::Result;
use std::process;
use std::str;

fn list() -> Result<()> {
    for (dev_i, device_res) in Device::devices(true).enumerate() {
        let device = device_res?;
        let hw_geom = device.hw_geom();
        let bios_geom = device.bios_geom();

        println!(
            "Device {}
    Model: {:?}
    Path: {:?}
    Size: {} MB
    Type: {:?}
    Open Count: {}
    Read Only: {}
    External Mode: {}
    Dirty: {}
    Boot Dirty: {}
    Hardware Geometry:
        Cylinders: {}
        Heads: {}
        Sectors: {}
    BIOS Geometry:
        Cylinders: {}
        Heads: {}
        Sectors: {}
    Host: {}
    Did: {}",
            dev_i,
            str::from_utf8(device.model()),
            str::from_utf8(device.path()),
            device.length() * device.sector_size() / 1000000,
            device.type_(),
            device.open_count(),
            device.read_only(),
            device.external_mode(),
            device.dirty(),
            device.boot_dirty(),
            hw_geom.cylinders,
            hw_geom.heads,
            hw_geom.sectors,
            bios_geom.cylinders,
            bios_geom.heads,
            bios_geom.sectors,
            device.host(),
            device.did()
        );

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
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("list: must be run with root");
        process::exit(1);
    }
    
    if let Err(err) = list() {
        eprintln!("list: failed to list: {}", err);
        process::exit(1);
    }
}
