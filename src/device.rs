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

pub use libparted_sys::PedDeviceType as DeviceType;

use super::cvt;

pub struct Device(*mut PedDevice);

pub struct DeviceIter(*mut PedDevice);

pub struct DeviceExternalAccess<'a>(&'a mut Device);

pub struct Geometry {
    pub cylinders: i32,
    pub heads: i32,
    pub sectors: i32
}

macro_rules! get_bool {
    ($field:tt) => {
        pub fn $field(&self) -> bool {
            unsafe { *self.0 }.$field != 0
        }
    }
}

macro_rules! get_geometry {
    ($kind:tt) => {
        pub fn $kind(&self) -> Geometry {
            unsafe {
                let raw = (*self.0).$kind;
                Geometry {
                    cylinders: raw.cylinders as i32,
                    heads: raw.heads as i32,
                    sectors: raw.sectors as i32
                }
            }
        }
    }
}

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

    pub fn type_(&self) -> DeviceType {
        unsafe {
            (*self.0).type_ as DeviceType
        }
    }

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

    get_bool!(read_only);
    get_bool!(external_mode);
    get_bool!(dirty);
    get_bool!(boot_dirty);
    get_geometry!(hw_geom);
    get_geometry!(bios_geom);

    pub fn host(&self) -> i16 {
        unsafe {
            (*self.0).host as i16
        }
    }

    pub fn did(&self) -> i16 {
        unsafe {
            (*self.0).did as i16
        }
    }

    // TODO: arch_specific
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
