use std::ffi::CStr;

use libparted_sys::{
    PedPartition,
    ped_partition_is_active,
    ped_partition_is_busy,
    ped_partition_get_name,
    ped_partition_get_path,
    ped_partition_type_get_name,
};

pub struct Partition(*mut PedPartition);

impl Partition {
    pub unsafe fn from_ped_partition(partition: *mut PedPartition) -> Self {
        Partition(partition)
    }

    pub fn is_active(&self) -> bool {
        unsafe {
            ped_partition_is_active(self.0) != 0
        }
    }

    pub fn is_busy(&self) -> bool {
        unsafe {
            ped_partition_is_busy(self.0) != 0
        }
    }

    pub fn name(&self) -> Option<&[u8]> {
        if self.is_active() {
            unsafe {
                Some(CStr::from_ptr(ped_partition_get_name(self.0)).to_bytes())
            }
        } else {
            None
        }
    }

    pub fn path(&self) -> Option<&[u8]> {
        if self.is_active() {
            unsafe {
                Some(CStr::from_ptr(ped_partition_get_path(self.0)).to_bytes())
            }
        } else {
            None
        }
    }

    pub fn type_name(&self) -> &[u8] {
        unsafe {
            CStr::from_ptr(ped_partition_type_get_name((*self.0).type_)).to_bytes()
        }
    }
}
