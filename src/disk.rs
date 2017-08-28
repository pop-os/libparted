use std::io::Result;
use std::ptr;

use libparted_sys::{
    PedDisk,
    ped_disk_new,
    ped_disk_destroy,
    ped_disk_print,
    ped_disk_next_partition,
    PedPartition,
};

use super::{cvt, Device, Partition};

pub struct Disk(*mut PedDisk);

pub struct DiskPartIter<'a>(&'a Disk, *mut PedPartition);

impl Disk {
    pub fn new(device: Device) -> Result<Disk> {
        let disk = cvt(unsafe {
            ped_disk_new(device.ped_device())
        })?;
        Ok(Disk(disk))
    }

    pub fn print(&self) {
        unsafe {
            ped_disk_print(self.0);
        }
    }

    pub fn parts<'a>(&'a self) -> DiskPartIter<'a> {
        DiskPartIter(self, ptr::null_mut())
    }
}

impl<'a> Iterator for DiskPartIter<'a> {
    type Item = Partition;
    fn next(&mut self) -> Option<Partition> {
        let partition = unsafe {
            ped_disk_next_partition((self.0).0, self.1)
        };
        if partition.is_null() {
            None
        } else {
            self.1 = partition;
            unsafe {
                Some(Partition::from_ped_partition(partition))
            }
        }
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        unsafe {
            ped_disk_destroy(self.0);
        }
    }
}
