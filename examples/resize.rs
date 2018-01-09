extern crate libparted;

use std::env::args;
use std::process::exit;
use libparted::*;

fn main() {
    let args = args().collect::<Vec<String>>();
    if !(args.len() == 2 || args.len() == 4) {
        eprintln!(
            "usage: {0} <device>\n       {0} <device> <start> <length>",
            args[0]
        );
        exit(1);
    }

    let dev = match Device::new(&args[1]) {
        Ok(dev) => dev,
        Err(why) => {
            eprintln!("cannot create/open device {}: {}", args[1], why);
            exit(1);
        }
    };

    let geom = match Geometry::new(&dev, 0, dev.length() as i64) {
        Ok(geom) => geom,
        Err(why) => {
            eprintln!("cannot create geometry: {}", why);
            exit(1);
        }
    };

    let mut fs = match FileSystem::open(&geom) {
        Some(fs) => fs,
        None => {
            eprintln!("cannot read file system");
            exit(1);
        }
    };

    match fs.resize(&geom, None) {
        Ok(()) => println!("filesystem resized"),
        Err(why) => {
            eprintln!("cannot resize file system: {}", why);
            exit(1);
        }
    }
}