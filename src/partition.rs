use std::ffi::{CStr, CString, OsStr};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;
use std::str;
use super::{cvt, Device, Disk, FileSystemType};

use libparted_sys::{ped_partition_destroy, ped_partition_get_flag, ped_partition_get_name,
                    ped_partition_get_path, ped_partition_is_active, ped_partition_is_busy,
                    ped_partition_is_flag_available, ped_partition_new, ped_partition_set_flag,
                    ped_partition_set_name, ped_partition_set_system, ped_partition_type_get_name,
                    PedFileSystemType, PedPartition};

pub use libparted_sys::PedPartitionFlag as PartitionFlag;
pub use libparted_sys::PedPartitionType as PartitionType;

pub struct Partition<'a> {
    pub(crate) part: *mut PedPartition,
    pub(crate) phantom: PhantomData<&'a PedPartition>,
}

impl<'a> Partition<'a> {
    pub unsafe fn from_ped_partition(part: *mut PedPartition) -> Self {
        Partition {
            part,
            phantom: PhantomData,
        }
    }

    /// Create a new **Partition** on `disk`.
    ///
    /// # Note:
    ///
    /// The constructed partition is not added to `disk`'s partition table. Use
    /// `Disk::add_partition()` to do this.
    ///
    /// # Throws:
    ///
    /// Throws `PED_EXCEPTION_ERROR` if `type` is `EXTENDED` or `LOGICAL` but the label
    /// does not support this concept.
    pub fn new(
        disk: &Disk,
        type_: PartitionType,
        fs_type: Option<&FileSystemType>,
        start: i64,
        end: i64,
    ) -> io::Result<Partition<'a>> {
        let fs_type = fs_type.map_or(ptr::null_mut() as *mut PedFileSystemType, |f| f.fs);
        cvt(unsafe { ped_partition_new(disk.disk, type_, fs_type, start, end) })
            .map(|partition| unsafe { Partition::from_ped_partition(partition) })
    }

    pub fn num(&'a self) -> i32 {
        unsafe { (*self.part).num }
    }

    pub fn fs_type_name(&'a self) -> Option<&str> {
        unsafe {
            let fs_type = (*self.part).fs_type;
            if fs_type.is_null() {
                None
            } else {
                let fs_name = (*fs_type).name;
                if fs_name.is_null() {
                    None
                } else {
                    Some(str::from_utf8_unchecked(CStr::from_ptr(fs_name).to_bytes()))
                }
            }
        }
    }

    pub fn get_device(&'a self) -> Device<'a> {
        unsafe { Device::from_ped_device((*self.part).geom.dev) }
    }

    pub fn get_device_mut(&'a mut self) -> Device<'a> {
        unsafe { Device::from_ped_device((*self.part).geom.dev) }
    }

    pub fn geom_start(&'a self) -> i64 {
        unsafe { (*self.part).geom.start }
    }

    pub fn geom_length(&'a self) -> i64 {
        unsafe { (*self.part).geom.length }
    }

    pub fn geom_end(&'a self) -> i64 {
        unsafe { (*self.part).geom.end }
    }

    /// Get the state of a flag on the disk.
    pub fn get_flag(&self, flag: PartitionFlag) -> bool {
        unsafe { ped_partition_get_flag(self.part, flag) == 1 }
    }

    /// Return a path that can be used to address the partition in the operating system.
    pub fn get_path(&self) -> Option<&Path> {
        if self.is_active() {
            let cstr_ptr = unsafe { ped_partition_get_path(self.part) };
            let cstr = unsafe { CStr::from_ptr(cstr_ptr) };
            let os_str = OsStr::from_bytes(cstr.to_bytes());
            Some(&Path::new(os_str))
        } else {
            None
        }
    }

    /// Returns whether or not the partition is _active_.
    ///
    /// A partition is active if the type is neither `PED_PARTITION_METADATA` nor
    /// `PED_PARTITION_FREE`.
    pub fn is_active(&self) -> bool {
        unsafe { ped_partition_is_active(self.part) != 0 }
    }

    /// Check whether a partition is mounted or busy in some other way.
    ///
    /// # Note:
    /// An extended partition is busy if any logical partitions are mounted.
    pub fn is_busy(&self) -> bool {
        unsafe { ped_partition_is_busy(self.part) != 0 }
    }

    /// Check whether a given flag is available on a disk.
    pub fn is_flag_available(&self, flag: PartitionFlag) -> bool {
        unsafe { ped_partition_is_flag_available(self.part, flag) == 1 }
    }

    /// Returns the name of a partition `part`. This will only work if the disk label supports it.
    pub fn name(&self) -> Option<&[u8]> {
        if self.is_active() {
            unsafe {
                let name = ped_partition_get_name(self.part);
                if name.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(name).to_bytes())
                }
            }
        } else {
            None
        }
    }

    /// Set the state of a flag on a partition.
    ///
    /// Flags are disk label specific, although they have a global _namespace_: the flag
    /// `PED_PARTITION_BOOT`, for example, roughly means "this partition is bootable". But this
    /// means different things on different disk labels (and may not be defined on some disk
    /// labels). For example, on MS-DOS disk labels, there can only be one boot partition,
    /// and this refers to the partition that will be booted from on startup. On PC98 disk labels,
    /// the user can choose from any bootable partition on startup.
    ///
    /// # Note:
    ///
    /// It is an error to call this on an unavailable flag -- use `Partition::is_flag_available()`
    /// to determine which flags are available for a given disk label.
    ///
    /// # Throws:
    ///
    /// Throws `PED_EXCEPTION_ERROR` if the requested flag is not available for this label.
    pub fn set_flag(&mut self, flag: PartitionFlag, state: bool) -> io::Result<()> {
        let state = if state { 1 } else { 0 };
        cvt(unsafe { ped_partition_set_flag(self.part, flag, state) }).map(|_| ())
    }

    /// Sets the name of a partition.
    ///
    /// # Note:
    ///
    /// This will only work if the disk label supports it.
    ///
    /// You can use
    ///
    /// ```
    /// DiskType::check_feature(DiskTypeFeature::PED_DISK_TYPE_PARTITION_NAME);
    /// ```
    ///
    /// to check whether this feature is enabled for a label.
    ///
    /// # Additional Note:
    ///
    /// `name` will not be modified by libparted. It can be freed by the caller immediately
    /// after `Partition::set_name()` is called.
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        let name_cstring = CString::new(name).map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Inavlid data: {}", err))
        })?;
        let name_ptr = name_cstring.as_ptr();
        cvt(unsafe { ped_partition_set_name(self.part, name_ptr) }).map(|_| ())
    }

    /// Sets the system type on the partition to `fs_type`.
    ///
    /// # Note:
    ///
    /// The file system may be opened, to get more information about the file system, such as
    /// to determine if it is FAT16 or FAT32.
    pub fn set_system(&mut self, fs_type: &FileSystemType) -> io::Result<()> {
        cvt(unsafe { ped_partition_set_system(self.part, fs_type.fs) }).map(|_| ())
    }

    /// Returns a name that seems mildly appropriate for a partition type `type`.
    pub fn type_get_name(&self) -> &str {
        unsafe {
            let cstr = CStr::from_ptr(ped_partition_type_get_name((*self.part).type_));
            str::from_utf8_unchecked(cstr.to_bytes())
        }
    }
}

impl<'a> Drop for Partition<'a> {
    fn drop(&mut self) {
        unsafe { ped_partition_destroy(self.part) }
    }
}
