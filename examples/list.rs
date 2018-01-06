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
            device.path(),
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
            println!(
                "  Part {}
    Type:   {:?}
    Name:   {:?}
    Path:   {:?}
    Active: {}
    Busy:   {}
    FS:     {:?}
    Start:  {},
    End:    {},
    Length: {}",
                part_i,
                str::from_utf8(part.type_get_name()),
                part.name(),
                part.get_path(),
                part.is_active(),
                part.is_busy(),
                part.fs_type_name().map(str::from_utf8),
                part.geom_start(),
                part.geom_end(),
                part.geom_length()
            );
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
