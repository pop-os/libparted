use std::ffi::{CStr, CString, OsStr};
use std::io::{Error, ErrorKind, Result};
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::os::raw::c_void;
use std::path::Path;
use std::ptr;
use std::str;

use libparted_sys::{ped_constraint_any, ped_device_begin_external_access, ped_device_check,
                    ped_device_close, ped_device_end_external_access, ped_device_get,
                    ped_device_get_constraint, ped_device_get_minimal_aligned_constraint,
                    ped_device_get_minimum_alignment, ped_device_get_next,
                    ped_device_get_optimal_aligned_constraint, ped_device_get_optimum_alignment,
                    ped_device_is_busy, ped_device_open, ped_device_probe_all, ped_device_sync,
                    ped_device_sync_fast, ped_device_write, ped_disk_clobber, ped_disk_probe,
                    PedDevice};

pub use libparted_sys::PedDeviceType as DeviceType;
pub use libparted_sys::_PedCHSGeometry as CHSGeometry;

use super::{cvt, Alignment, Constraint, ConstraintSource, DiskType, Geometry};

pub struct Device<'a> {
    pub(crate) device: *mut PedDevice,
    pub(crate) phantom: PhantomData<&'a PedDevice>,
    pub(crate) is_droppable: bool,
}

pub struct DeviceIter<'a>(*mut PedDevice, PhantomData<&'a PedDevice>);

pub struct DeviceExternalAccess<'a, 'b: 'a>(&'a mut Device<'b>);

macro_rules! get_bool {
    ($field:tt) => {
        pub fn $field(&self) -> bool {
            unsafe { *self.device }.$field != 0
        }
    }
}

macro_rules! get_geometry {
    ($kind:tt) => {
        pub fn $kind(&self) -> CHSGeometry {
            unsafe { (*self.device).$kind }
        }
    }
}

impl<'a> Device<'a> {
    fn new_(device: *mut PedDevice) -> Device<'a> {
        Device {
            device,
            phantom: PhantomData,
            is_droppable: true,
        }
    }

    /// Returns the first bad sector if a bad sector was found.
    ///
    /// # Binding Note
    ///
    /// Not 100% sure if this is what this method does, as libparted's source
    /// code did not document the behavior of the function. Am basing this
    /// off the `check()` method that was documented for **Geometry**.
    pub fn check(&self, start: i64, count: i64) -> Option<u64> {
        let mut buffer: Vec<u8> = Vec::with_capacity(8192);
        let buffer_ptr = buffer.as_mut_slice().as_mut_ptr() as *mut c_void;
        match unsafe { ped_device_check(self.device, buffer_ptr, start, count) } {
            -1 => None,
            bad_sector => Some(bad_sector as u64),
        }
    }

    /// Return the type of partition table detected on `dev`
    pub fn probe(&self) -> Option<DiskType> {
        let disk_type = unsafe { ped_disk_probe(self.device) };
        if disk_type.is_null() {
            None
        } else {
            Some(DiskType {
                type_: disk_type,
                phantom: PhantomData,
            })
        }
    }

    /// Attempts to detect all devices, constructing an **Iterator** which will
    /// contain a list of all of the devices. If you want to use a device that isn't
    /// on the list, use the `new()` method, or an OS-specific constructor such as
    /// `new_from_store()`.
    pub fn devices<'b>(probe: bool) -> DeviceIter<'b> {
        if probe {
            unsafe { ped_device_probe_all() }
        }

        DeviceIter(ptr::null_mut(), PhantomData)
    }

    /// Obtains a handle to the device, but does not open it.
    pub fn get<P: AsRef<Path>>(path: P) -> Result<Device<'a>> {
        // Convert the supplied path into a C-compatible string.
        let os_str = path.as_ref().as_os_str();
        let cstr = CString::new(os_str.as_bytes())
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("Inavlid data: {}", err)))?;

        // Then attempt to get the device.
        let mut device = Device::new_(
            cvt(unsafe { ped_device_get(cstr.as_ptr()) })?
        );
        device.is_droppable = false;
        Ok(device)
    }

    /// Attempts to open the device.
    pub fn open(&mut self) -> Result<()> {
        cvt(unsafe { ped_device_open(self.device) })?;
        self.is_droppable = true;
        Ok(())
    }

    /// Attempts to get the device of the given `path`, then attempts to open that device.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Device<'a>> {
        let mut device = Device::get(path)?;
        device.open()?;
        Ok(device)
    }

    pub unsafe fn from_ped_device(device: *mut PedDevice) -> Device<'a> {
        Device::new_(device)
    }

    pub unsafe fn ped_device(&self) -> *mut PedDevice {
        self.device
    }

    /// Begins external access mode.
    ///
    /// External access mode allows you to safely do I/O on the device. If a device is open,
    /// then you should not do any I/O on that device, such as by calling an external program
    /// like e2fsck, unless you put it in external access mode. You should not use any libparted
    /// commands that do I/O to a device while a device is in external access mode.
    ///
    /// # Note:
    ///
    /// You should not close a device while it is in external access mode.
    pub fn external_access<'b>(&'b mut self) -> Result<DeviceExternalAccess<'a, 'b>> {
        cvt(unsafe { ped_device_begin_external_access(self.device) })?;

        Ok(DeviceExternalAccess(self))
    }

    /// Flushes all write-behind caches that might be holding up writes.
    ///
    /// It is slow because it guarantees cache coherency among all relevant caches.
    pub fn sync(&mut self) -> Result<()> {
        cvt(unsafe { ped_device_sync(self.device) })?;
        Ok(())
    }

    /// Flushes all write-behind caches that might be holding writes.
    ///
    /// It does not ensure cache coherency with other caches.
    pub fn sync_fast(&mut self) -> Result<()> {
        cvt(unsafe { ped_device_sync_fast(self.device) })?;
        Ok(())
    }

    /// Indicates whether the device is busy.
    pub fn is_busy(&self) -> bool {
        unsafe { ped_device_is_busy(self.device) != 0 }
    }

    /// Attempts to write the data within the buffer to the device, starting
    /// at the **start_sector**, and spanning across **sectors**.
    pub fn write_to_sectors(
        &mut self,
        buffer: &[u8],
        start_sector: i64,
        sectors: i64,
    ) -> Result<()> {
        let total_size = self.sector_size() as usize * sectors as usize;

        // Ensure that the data will fit within the region of sectors.
        debug_assert!(buffer.len() <= total_size);

        // Write as much data as needed to fill the entire sector, writing
        // zeros in the unused space, and obtaining a pointer to the buffer.
        let mut sector_buffer: Vec<u8> = Vec::with_capacity(total_size);
        sector_buffer.extend_from_slice(buffer);
        sector_buffer.extend((buffer.len()..total_size).map(|_| b'0'));
        let sector_ptr = sector_buffer.as_slice().as_ptr() as *const c_void;

        // Then attempt to write the data to the device.
        cvt(unsafe { ped_device_write(self.device, sector_ptr, start_sector, sectors) })?;
        Ok(())
    }

    /// Get a constraint that represents hardware requirements on geometry.
    ///
    /// This function will return a constraint representing the limits imposed by the size
    /// of the disk. It will not provide any alignment constraints.
    ///
    /// Alignment constraint may be desirable when using media that has a physical
    /// sector size that is a multiple of the logical sector size, as in this case proper
    /// partition alignment can benefit disk performance significantly.
    ///
    /// # Note:
    ///
    /// When you want a constraint with alignment info, use the following methods:
    /// - `Device::get_minimal_aligned_constraint()`
    /// - `Device::get_optimal_aligned_constraint()`
    pub fn get_constraint<'b>(&self) -> Result<Constraint<'b>> {
        Ok(Constraint {
            constraint: cvt(unsafe { ped_device_get_constraint(self.device) })?,
            source: ConstraintSource::New,
            phantom: PhantomData,
        })
    }

    /// Return a constraint that any region on the given device will satisfy.
    pub fn constraint_any<'b>(&self) -> Option<Constraint<'b>> {
        let constraint = unsafe { ped_constraint_any(self.device) };
        if constraint.is_null() {
            None
        } else {
            Some(Constraint {
                constraint,
                source: ConstraintSource::New,
                phantom: PhantomData,
            })
        }
    }

    pub fn constraint_from_start_end<'b>(
        &self,
        range_start: &Geometry,
        range_end: &Geometry,
    ) -> Result<Constraint<'b>> {
        let alignment_any = Alignment::new(0, 1).unwrap();
        Constraint::new(
            &alignment_any,
            &alignment_any,
            range_start,
            range_end,
            1,
            self.length() as i64,
        )
    }

    /// Get a constraint that represents hardware requirements on geometry and alignment.
    ///
    /// This function will return a constraint representing the limits imposed by the size of
    /// the disk and the minimal alignment requirements for proper performance of the disk.
    pub fn get_minimal_aligned_constraint<'b>(&self) -> Result<Constraint<'b>> {
        Ok(Constraint {
            constraint: cvt(unsafe { ped_device_get_minimal_aligned_constraint(self.device) })?,
            source: ConstraintSource::New,
            phantom: PhantomData,
        })
    }

    /// Get a constraint that represents hardware requirements on geometry and alignment.
    ///
    /// This function will return a constraint representing the limits imposed by the size of
    /// the disk and the alignment requirements for optimal performance of the disk.
    pub fn get_optimal_aligned_constraint<'b>(&self) -> Result<Constraint<'b>> {
        Ok(Constraint {
            constraint: cvt(unsafe { ped_device_get_optimal_aligned_constraint(self.device) })?,
            source: ConstraintSource::New,
            phantom: PhantomData,
        })
    }

    /// Get an alignment that represents minimum hardware requirements on alignment.
    ///
    /// When using media that has a physical sector size that is a multiple of the logical sector
    /// size, it is desirable to have disk accesses (and thus partitions) properly aligned. Having
    /// partitions not aligned to the minimum hardware requirements may lead to a performance
    /// penalty.
    ///
    /// The returned alignment describes the alignment for the start sector of the partition.
    /// The end sector should be aligned too. To get the end sector alignment, decrease the
    /// returned alignment's offset by 1.
    pub fn get_minimum_alignment<'b>(&self) -> Option<Alignment<'b>> {
        let alignment = unsafe { ped_device_get_minimum_alignment(self.device) };
        if alignment.is_null() {
            None
        } else {
            Some(Alignment {
                alignment,
                phantom: PhantomData,
            })
        }
    }

    /// Get an alignment that represents the hardware requirements for optimal performance.
    ///
    /// The returned alignment describes the alignment for the start sector of the partition.
    /// The end sector should be aligned too. To get the end alignment, decrease the returned
    /// alignment's offset by 1.
    pub fn get_optimum_alignment<'b>(&self) -> Option<Alignment<'b>> {
        let alignment = unsafe { ped_device_get_optimum_alignment(self.device) };
        if alignment.is_null() {
            None
        } else {
            Some(Alignment {
                alignment,
                phantom: PhantomData,
            })
        }
    }

    /// Remove all identifying signatures of a partition table.
    pub fn clobber(&mut self) -> Result<()> {
        cvt(unsafe { ped_disk_clobber(self.device) })?;
        Ok(())
    }

    pub fn model(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr((*self.device).model).to_bytes()) }
    }

    pub fn path(&self) -> &Path {
        let cstr = unsafe { CStr::from_ptr((*self.device).path) };
        let os_str = OsStr::from_bytes(cstr.to_bytes());
        &Path::new(os_str)
    }

    pub fn type_(&self) -> DeviceType {
        unsafe { (*self.device).type_ as DeviceType }
    }

    pub fn sector_size(&self) -> u64 {
        unsafe { (*self.device).sector_size as u64 }
    }

    pub fn phys_sector_size(&self) -> u64 {
        unsafe { (*self.device).phys_sector_size as u64 }
    }

    pub fn length(&self) -> u64 {
        unsafe { (*self.device).length as u64 }
    }

    pub fn open_count(&self) -> isize {
        unsafe { (*self.device).open_count as isize }
    }

    get_bool!(read_only);
    get_bool!(external_mode);
    get_bool!(dirty);
    get_bool!(boot_dirty);
    get_geometry!(hw_geom);
    get_geometry!(bios_geom);

    pub fn host(&self) -> i16 {
        unsafe { (*self.device).host as i16 }
    }

    pub fn did(&self) -> i16 {
        unsafe { (*self.device).did as i16 }
    }

    // TODO: arch_specific
}

impl<'a> Iterator for DeviceIter<'a> {
    type Item = Device<'a>;
    fn next(&mut self) -> Option<Device<'a>> {
        let device = unsafe { ped_device_get_next(self.0) };
        if device.is_null() {
            None
        } else {
            self.0 = device;
            let mut device = unsafe { Device::from_ped_device(device) };
            device.is_droppable = false;
            Some(device)
        }
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        unsafe {
            if self.open_count() > 0 && self.is_droppable {
                ped_device_close(self.device);
            }
        }
    }
}

impl<'a, 'b> Drop for DeviceExternalAccess<'a, 'b> {
    fn drop(&mut self) {
        unsafe {
            ped_device_end_external_access((self.0).device);
        }
    }
}
