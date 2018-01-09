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

        println!("Device {}", dev_i);
        println!("    Mode:          {:?}", str::from_utf8(device.model()));
        println!("    Path:          {:?}", device.path());
        println!(
            "    Size:          {} MB",
            device.length() * device.sector_size() / 1000000
        );
        println!("    Type:          {:?}", device.type_());
        println!("    Open Count:    {}", device.open_count());
        println!("    Read Only:     {}", device.read_only());
        println!("    External Mode: {}", device.external_mode());
        println!("    Dirty:         {}", device.dirty());
        println!("    Boot Dirty:    {}", device.boot_dirty());
        println!("    Hardware Geometry:");
        println!("        Cylinders: {}", hw_geom.cylinders);
        println!("        Heads:     {}", hw_geom.heads);
        println!("        Sectors:   {}", hw_geom.sectors);
        println!("    BIOS Geometry:");
        println!("        Cylinders: {}", bios_geom.cylinders);
        println!("        Heads:     {}", bios_geom.heads);
        println!("        Sectors:   {}", bios_geom.sectors);
        println!("    Host:          {}", device.host());
        println!("    Did:           {}", device.did());

        let disk = Disk::new(device)?;
        eprintln!(
            "    Disk Type: {:?}",
            disk.get_disk_type_name().map(str::from_utf8)
        );

        for (part_i, part) in disk.parts().enumerate() {
            println!("    Part {}", part_i);
            println!("        Name:   {:?}", part.type_get_name());
            println!("        Type:   {:?}", part.name());
            println!("        Path:   {:?}", part.get_path());
            println!("        Active: {}", part.is_active());
            println!("        Busy:   {}", part.is_busy());
            println!("        FS:     {:?}", part.fs_type_name());
            println!("        Start:  {}", part.geom_start());
            println!("        End:    {}", part.geom_end());
            println!("        Length: {}", part.geom_length());
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
