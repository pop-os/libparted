use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind, Result};
use std::os::unix::ffi::OsStrExt;
use std::os::raw::c_void;
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
    ped_device_sync,
    ped_device_sync_fast,
    ped_device_write,
    ped_device_is_busy,
    ped_device_get_constraint,
    ped_device_get_minimal_aligned_constraint,
    ped_device_get_minimum_alignment,
    ped_device_get_optimal_aligned_constraint,
    ped_device_get_optimum_alignment
};

use super::{cvt, Alignment, Constraint};

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

    /// Flushes all write-behind caches that might be holding up writes.
    /// 
    /// It is slow because it guarantees cache coherency among all relevant caches.
    pub fn sync(&mut self) -> Result<()> {
        cvt(unsafe { ped_device_sync(self.0) })?;
        Ok(())
    }

    /// Flushes all write-behind caches that might be holding writes.
    /// 
    /// It does not ensure cache coherency with other caches.
    pub fn sync_fast(&mut self) -> Result<()> {
        cvt(unsafe { ped_device_sync_fast(self.0) })?;
        Ok(())
    }

    /// Indicates whether the device is busy.
    pub fn is_busy(&self) -> bool {
        unsafe { ped_device_is_busy(self.0) != 0 }
    }

    /// Attempts to write the data within the buffer to the device, starting
    /// at the **start_sector**, and spanning across **sectors**.
    pub fn write_to_sectors(
        &mut self,
        buffer: &[u8],
        start_sector: i64,
        sectors: i64
    ) -> Result<()> {
        let total_size = self.sector_size() as usize * sectors as usize;

        // Ensure that the data will fit within the region of sectors.
        assert!(buffer.len() <= total_size);
        
        // Write as much data as needed to fill the entire sector, writing
        // zeros in the unused space, and obtaining a pointer to the buffer.
        let mut sector_buffer: Vec<u8> = Vec::with_capacity(total_size);
        sector_buffer.copy_from_slice(buffer);
        for index in buffer.len()..total_size {
            sector_buffer[index] = b'0';
        }
        let sector_ptr = sector_buffer.as_slice().as_ptr() as *const c_void;

        // Then attempt to write the data to the device.
        cvt(unsafe { ped_device_write(self.0, sector_ptr, start_sector, sectors) })?;
        Ok(())
    }

    pub fn get_constraint(&self) -> Constraint {
        unsafe { Constraint(ped_device_get_constraint(self.0)) }
    }

    pub fn get_minimal_aligned_constraint(&self) -> Constraint {
        unsafe { Constraint(ped_device_get_minimal_aligned_constraint(self.0)) }
    }

    pub fn get_optimal_aligned_constraint(&self) -> Constraint {
        unsafe { Constraint(ped_device_get_optimal_aligned_constraint(self.0)) }
    }

    pub fn get_minimum_alignment(&self) -> Alignment {
        unsafe { Alignment(ped_device_get_minimum_alignment(self.0)) }
    }

    pub fn get_optimal_alignment(&self) -> Alignment {
        unsafe { Alignment(ped_device_get_optimum_alignment(self.0)) }
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
