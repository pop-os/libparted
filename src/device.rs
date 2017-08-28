use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind, Result};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use libparted_sys::{
    PedDevice,
    ped_device_probe_all,
    ped_device_get,
    ped_device_get_next,
    ped_device_open,
    ped_device_close,
    ped_device_begin_external_access,
    ped_device_end_external_access,
};

use super::cvt;

pub struct Device(*mut PedDevice);

pub struct DeviceIter(*mut PedDevice);

pub struct DeviceExternalAccess<'a>(&'a mut Device);

impl Device {
    pub fn devices(probe: bool) -> DeviceIter {
        if probe {
            unsafe {
                ped_device_probe_all()
            }
        }
        DeviceIter(ptr::null_mut())
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Device> {
        let os_str = path.as_ref().as_os_str();
        let cstr = CString::new(os_str.as_bytes()).map_err(|err| {
            Error::new(ErrorKind::InvalidData, format!("Inavlid data: {}", err))
        })?;
        let device = cvt(unsafe {
            ped_device_get(cstr.as_ptr())
        })?;
        cvt(unsafe {
            ped_device_open(device)
        })?;
        Ok(Device(device))
    }

    pub unsafe fn from_ped_device(device: *mut PedDevice) -> Device {
        Device(device)
    }

    pub unsafe fn ped_device(&self) -> *mut PedDevice {
        self.0
    }

    pub fn external_access<'a>(&'a mut self) -> Result<DeviceExternalAccess<'a>> {
        cvt(unsafe {
            ped_device_begin_external_access(self.0)
        })?;
        Ok(DeviceExternalAccess(self))
    }

    pub fn model(&self) -> &[u8] {
        unsafe {
            CStr::from_ptr((*self.0).model).to_bytes()
        }
    }

    pub fn path(&self) -> &[u8] {
        unsafe {
            CStr::from_ptr((*self.0).path).to_bytes()
        }
    }

    // TODO pub fn type

    pub fn sector_size(&self) -> u64 {
        unsafe {
            (*self.0).sector_size as u64
        }
    }

    pub fn phys_sector_size(&self) -> u64 {
        unsafe {
            (*self.0).phys_sector_size as u64
        }
    }

    pub fn length(&self) -> u64 {
        unsafe {
            (*self.0).length as u64
        }
    }

    pub fn open_count(&self) -> isize {
        unsafe {
            (*self.0).open_count as isize
        }
    }

    pub fn read_only(&self) -> bool {
        unsafe {
            (*self.0).read_only != 0
        }
    }

    //TODO: add more params
}

impl Iterator for DeviceIter {
    type Item = Result<Device>;
    fn next(&mut self) -> Option<Result<Device>> {
        let device = unsafe {
            ped_device_get_next(self.0)
        };
        if device.is_null() {
            None
        } else {
            self.0 = device;
            Some(
                cvt(unsafe {
                    ped_device_open(device)
                }).and(Ok(unsafe {
                    Device::from_ped_device(device)
                }))
            )
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            ped_device_close(self.0);
        }
    }
}

impl<'a> Drop for DeviceExternalAccess<'a> {
    fn drop(&mut self) {
        unsafe {
            ped_device_end_external_access((self.0).0);
        }
    }
}
