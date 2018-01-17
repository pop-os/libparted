use std::ffi::{CStr, CString};
use std::io::Result;
use std::marker::PhantomData;
use std::ptr;
use std::str;
use super::{cvt, get_optional, prefer_snap, snap, Alignment, Constraint, ConstraintSource, Device,
            Geometry, Partition, MOVE_DOWN, MOVE_STILL, MOVE_UP, SECT_END, SECT_START};
use libparted_sys::{ped_constraint_any, ped_disk_add_partition, ped_disk_check as check,
                    ped_disk_clobber, ped_disk_commit as commit,
                    ped_disk_commit_to_dev as commit_to_dev,
                    ped_disk_commit_to_os as commit_to_os, ped_disk_delete_all as delete_all,
                    ped_disk_delete_partition, ped_disk_destroy, ped_disk_duplicate,
                    ped_disk_extended_partition, ped_disk_get_flag,
                    ped_disk_get_last_partition_num, ped_disk_get_max_partition_geometry,
                    ped_disk_get_max_primary_partition_count,
                    ped_disk_get_max_supported_partition_count, ped_disk_get_partition,
                    ped_disk_get_partition_alignment, ped_disk_get_partition_by_sector,
                    ped_disk_get_primary_partition_count, ped_disk_is_flag_available,
                    ped_disk_max_partition_length, ped_disk_max_partition_start_sector,
                    ped_disk_maximize_partition, ped_disk_minimize_extended_partition,
                    ped_disk_new, ped_disk_new_fresh, ped_disk_next_partition, ped_disk_print,
                    ped_disk_remove_partition, ped_disk_set_flag, ped_disk_set_partition_geom,
                    ped_disk_type_check_feature, ped_disk_type_get, ped_disk_type_get_next,
                    ped_disk_type_register, ped_disk_type_unregister, PedDisk, PedDiskType,
                    PedPartition};

pub use libparted_sys::_PedDiskFlag as DiskFlag;
pub use libparted_sys::_PedDiskTypeFeature as DiskTypeFeature;

macro_rules! disk_fn_mut {
    ($(#[$attr:meta])* fn $method:tt) => {
        $(#[$attr])*
        pub fn $method(&mut self) -> Result<()> {
            cvt(unsafe { $method(self.disk) })?;
            Ok(())
        }
    }
}

pub struct Disk<'a> {
    pub(crate) disk: *mut PedDisk,
    pub(crate) phantom: PhantomData<&'a PedDisk>,
    is_droppable: bool,
}

pub struct DiskType<'a> {
    pub(crate) type_: *mut PedDiskType,
    pub(crate) phantom: PhantomData<&'a PedDiskType>,
}

impl<'a> DiskType<'a> {
    /// This function checks if a particular type of partition table supports a feature.
    pub fn check_feature(&self, feature: DiskTypeFeature) -> bool {
        unsafe { ped_disk_type_check_feature(self.type_, feature) != 0 }
    }

    /// Returns the next disk type register, if it exists.
    pub fn get_next(&'a self) -> Option<DiskType<'a>> {
        let type_ = unsafe { ped_disk_type_get_next(self.type_) };
        if type_.is_null() {
            None
        } else {
            Some(DiskType {
                type_,
                phantom: PhantomData,
            })
        }
    }

    /// Return the disk type with the given name.
    pub fn get(name: &str) -> Option<DiskType<'a>> {
        CString::new(name.as_bytes())
            .ok()
            .map(|name| unsafe { ped_disk_type_get(name.as_ptr()) })
            .and_then(|type_| {
                if type_.is_null() {
                    None
                } else {
                    Some(DiskType {
                        type_,
                        phantom: PhantomData,
                    })
                }
            })
    }

    pub fn register(&self) {
        unsafe { ped_disk_type_register(self.type_) }
    }

    pub fn unregister(&self) {
        unsafe { ped_disk_type_unregister(self.type_) }
    }
}

pub struct DiskPartIter<'a>(&'a Disk<'a>, *mut PedPartition);

impl<'a> Disk<'a> {
    /// Read the partition table off a device (if one is found).
    ///
    /// **Warning**: May modify the supplied `device` if the partition table indicates that the
    /// existing values are incorrect.
    pub fn new(device: &'a mut Device) -> Result<Disk<'a>> {
        let is_droppable = device.is_droppable;
        let disk = cvt(unsafe { ped_disk_new(device.ped_device()) })?;
        Ok(Disk { disk, phantom: PhantomData, is_droppable })
    }

    /// Creates a new partition table on `device`.
    ///
    /// The new partition table is only created in-memory, and nothing is written to disk until
    /// `disk.commit_to_dev()` is called.
    pub fn new_fresh(device: &'a mut Device, type_: DiskType) -> Result<Disk<'a>> {
        cvt(unsafe { ped_disk_new_fresh(device.ped_device(), type_.type_) }).map(|disk| Disk {
            disk,
            phantom: PhantomData,
            is_droppable: true
        })
    }

    /// Obtains the inner device from the disk.
    pub unsafe fn get_device<'b>(&self) -> Device<'b> {
        let mut device = Device::from_ped_device((*self.disk).dev);
        device.is_droppable = false;
        device
    }

    /// Obtains the inner device from the disk, with mutable access.
    pub unsafe fn get_device_mut<'b>(&'b mut self) -> Device<'b> {
        let mut device = Device::from_ped_device((*self.disk).dev);
        device.is_droppable = false;
        device
    }

    /// Obtains the constraint of the inner device.
    pub fn constraint_any<'b>(&self) -> Option<Constraint<'b>> {
        unsafe {
            let constraint = ped_constraint_any((*self.disk).dev);
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
    }

    pub fn get_disk_type_name<'b>(&'b self) -> Option<&str> {
        unsafe {
            let type_ = (*self.disk).type_;
            let name = (*type_).name;
            if name.is_null() {
                None
            } else {
                let cstr = CStr::from_ptr(name).to_bytes();
                Some(str::from_utf8_unchecked(cstr))
            }
        }
    }

    pub fn needs_clobber(&self) -> bool {
        unsafe { (*self.disk).needs_clobber != 0 }
    }

    pub fn update_mode(&self) -> bool {
        unsafe { (*self.disk).update_mode != 0 }
    }

    /// Get the state of a set flag on a disk.
    pub fn get_flag_state(&self, flag: DiskFlag) -> bool {
        unsafe { ped_disk_get_flag(self.disk, flag) != 0 }
    }

    /// Check whether a given flag is available on a disk
    pub fn is_flag_available(&self, flag: DiskFlag) -> bool {
        unsafe { ped_disk_is_flag_available(self.disk, flag) != 0 }
    }

    /// Prints a summary of the disk's partitions. Useful for debugging.
    pub fn print(&self) {
        unsafe {
            ped_disk_print(self.disk);
        }
    }

    pub fn parts<'b>(&'b self) -> DiskPartIter<'b> {
        DiskPartIter(self, ptr::null_mut())
    }

    /// Adds the supplied `part` **Partition** to the disk.
    ///
    /// **Warning**: The partition's geometry may be changed, subject to `constraint`. You could
    /// set `constraint` to `constraint_exact(&part.geom)`, but many partition table schemes have
    /// special requirements on the start and end of partitions. Therefore, having an overly
    /// strict constraint will probably mean that this function will fail (in which case `part`
    /// will be left unmodified) `part` is assigned a number (`part.num`) in this process.
    pub fn add_partition(&mut self, part: &mut Partition, constraint: &Constraint) -> Result<()> {
        cvt(unsafe { ped_disk_add_partition(self.disk, part.part, constraint.constraint) })?;
        Ok(())
    }

    /// Get the highest available partition number on the disk.
    pub fn get_last_partition_num(&self) -> Option<u32> {
        match unsafe { ped_disk_get_last_partition_num(self.disk) } {
            -1 => None,
            num => Some(num.abs() as u32),
        }
    }

    /// Get the highest supported partition number on the disk.
    pub fn get_max_supported_partition_count(&self) -> Option<u32> {
        let mut supported = 0i32;
        if unsafe { ped_disk_get_max_supported_partition_count(self.disk, &mut supported) } {
            if supported < 0 {
                None
            } else {
                Some(supported.abs() as u32)
            }
        } else {
            None
        }
    }

    /// Get the maximum number of (primary) partitions that the disk label supports.
    pub fn get_max_primary_partition_count(&self) -> u32 {
        unsafe { ped_disk_get_max_primary_partition_count(self.disk) as u32 }
    }

    /// Get the maximum geometry `part` can be grown to, subject to `constraint`.
    pub fn get_max_partition_geometry(
        &'a self,
        part: &Partition,
        constraint: &Constraint,
    ) -> Result<Geometry<'a>> {
        cvt(unsafe {
            ped_disk_get_max_partition_geometry(self.disk, part.part, constraint.constraint)
        }).map(Geometry::from_raw)
    }

    disk_fn_mut!(
        /// Perform a sanity check on a partition table
        /// 
        /// **NOTE**: The check performed is generic (ie: it does not depend on the label type
        /// of the disk).
        fn check
    );

    /// Remove all identifying signatures of a partition table.
    pub fn clobber(&mut self) -> Result<()> {
        cvt(unsafe { ped_disk_clobber((*self.disk).dev) })?;
        Ok(())
    }

    disk_fn_mut!(
        /// Writes the in-memory changes to a partition table to disk and informs
        /// the operating system of the changes.
        /// 
        /// NOTE: Equivalent to calling `disk.commit_to_dev()`, followed by `disk.commit_to_os()`.
        fn commit
    );

    disk_fn_mut!(
        /// Write the changes made to the in-memory description of a partition table to the device.
        fn commit_to_dev
    );

    disk_fn_mut!(
        /// Tell the operating system kernel about the partition table layout of `disk`.
        fn commit_to_os
    );

    disk_fn_mut!(
        /// Removes and destroys all partitions on `disk`.
        fn delete_all
    );

    /// Removes `part` from disk, and destroys `part`.
    pub fn delete_partition(&mut self, num: u32) -> Result<()> {
        cvt(unsafe { ped_disk_get_partition(self.disk, num as i32) })
            .and_then(|part|
                cvt(unsafe { ped_disk_delete_partition(self.disk, part) })
            ).map(|_| ())
    }

    // Clones the disk object, returning a deep copy if it suceeds.
    pub fn duplicate<'b>(&mut self) -> Result<Disk<'b>> {
        cvt(unsafe { ped_disk_duplicate(self.disk) }).map(|disk| Disk {
            disk,
            phantom: PhantomData,
            is_droppable: true
        })
    }

    // Obtains the extended partition from the disk, if it exists.
    pub fn extended_partition<'b>(&'b self) -> Option<Partition<'b>> {
        get_optional(unsafe { ped_disk_extended_partition(self.disk) }).map(|part| Partition {
            part,
            phantom: PhantomData,
        })
    }

    /// Get the alignment needed for partition boundaries on this disk.
    ///
    /// The returned alignment describes the alignment for the start sector of the
    /// partition, for all disklabel types which require alignment, except Sun disklables, the
    /// end sector must be aligned too. To get the end sector alignment, decrease the Alignment
    /// offset by 1.
    pub fn get_partition_alignment(&'a self) -> Result<Alignment<'a>> {
        cvt(unsafe { ped_disk_get_partition_alignment(self.disk) }).map(|alignment| Alignment {
            alignment,
            phantom: PhantomData,
        })
    }

    /// Returns the partition that contains `sector`. If `sector` lies within a logical
    /// partition, then the logical partition is returned (not the extended partition).
    pub fn get_partition_by_sector(&'a self, sector: i64) -> Option<Partition<'a>> {
        let part = unsafe { ped_disk_get_partition_by_sector(self.disk, sector) };
        if part.is_null() {
            None
        } else {
            Some(Partition {
                part,
                phantom: PhantomData,
            })
        }
    }

    /// Returns the partition numbered `num`.
    pub fn get_partition(&'a self, num: u32) -> Option<Partition<'a>> {
        get_optional(unsafe { ped_disk_get_partition(self.disk, num as i32) }).map(|part| {
            Partition {
                part,
                phantom: PhantomData,
            }
        })
    }

    /// Get the number of primary partitions.
    pub fn get_primary_partition_count(&self) -> u32 {
        unsafe { ped_disk_get_primary_partition_count(self.disk) as u32 }
    }

    /// Return the maximum representable length (in sectors) of a partition on the disk.
    pub fn max_partition_length(&self) -> i64 {
        unsafe { ped_disk_max_partition_length(self.disk) }
    }

    /// Return the maximum representable start sector of a partition on the disk.
    pub fn max_partition_start_sector(&self) -> i64 {
        unsafe { ped_disk_max_partition_start_sector(self.disk) }
    }

    /// Grow the supplied `part` to the maximimum size possible, subject to `constraint`.
    /// The new geometry will be a superset of the old geometry.
    pub fn maximize_partition(
        &mut self,
        part: &mut Partition,
        constraint: &Constraint,
    ) -> Result<()> {
        cvt(unsafe { ped_disk_maximize_partition(self.disk, part.part, constraint.constraint) })
            .map(|_| ())
    }

    /// Reduce the size of the extended partition to a minimum while still wrapping its
    /// logical partitions. If there are no logical partitions, remove the extended partition.
    pub fn minimize_extended_partition(&mut self) -> Result<()> {
        cvt(unsafe { ped_disk_minimize_extended_partition(self.disk) }).map(|_| ())
    }

    /// Removes the `part` **Partition** from the disk.
    ///
    /// If `part` is an extended partition, it must not contain any logical partitions.
    /// Note that `part` will not be destroyed when passed into this function.
    pub fn remove_partition(&mut self, num: u32) -> Result<()> {
        cvt(unsafe { ped_disk_get_partition(self.disk, num as i32) })
            .and_then(|part|
                cvt(unsafe { ped_disk_remove_partition(self.disk, part) })
            ).map(|_| ())
    }

    /// Set the state of a flag on a disk.
    ///
    /// # Note
    ///
    /// It is an error to call tis on an unavailable flag. Use `disk.is_flag_available()`
    /// to determine whhich flags are available for a given disk label.
    ///
    /// # Throws
    ///
    /// Throws `PED_EXCEPTION_ERROR` if the requested flag is not available for this label.
    pub fn set_flag(&mut self, flag: DiskFlag, state: bool) -> bool {
        let state = if state { 1 } else { 0 };
        unsafe { ped_disk_set_flag(self.disk, flag, state) != 0 }
    }

    /// Sets the geometry of `part` (IE: change a partition's location).
    ///
    /// This can fail for many reasons, such as overlapping with other partitions.
    /// If it does fail, `part` will remain unchanged.
    pub fn set_partition_geometry(
        &mut self,
        part: &mut Partition,
        constraint: &Constraint,
        start: i64,
        end: i64,
    ) -> Result<()> {
        cvt(unsafe {
            ped_disk_set_partition_geom(self.disk, part.part, constraint.constraint, start, end)
        }).map(|_| ())
    }

    pub fn snap_to_boundaries(
        &self,
        new_geom: &mut Geometry,
        old_geom: Option<&Geometry>,
        start_range: &Geometry,
        end_range: &Geometry,
    ) {
        let (mut start_dist, mut end_dist) = (-1, -1);
        let mut start = new_geom.start();
        let mut end = new_geom.end();

        let start_part = match self.get_partition_by_sector(start) {
            Some(part) => part,
            None => unsafe { Partition::from_ped_partition(ptr::null_mut()) },
        };

        let end_part = match self.get_partition_by_sector(end) {
            Some(part) => part,
            None => unsafe { Partition::from_ped_partition(ptr::null_mut()) },
        };

        let adjacent = start_part.geom_end() + 1 == end_part.geom_start();
        let mut start_allow = MOVE_STILL | MOVE_UP | MOVE_DOWN;
        let mut end_allow = start_allow;

        if let Some(old_geom) = old_geom {
            if snap(&mut start, old_geom.start(), start_range) {
                start_allow = MOVE_STILL;
            }

            if snap(&mut end, old_geom.end(), end_range) {
                end_allow = MOVE_STILL;
            }
        }

        if start_part == end_part {
            start_allow &= !MOVE_UP;
            end_allow &= !MOVE_DOWN;
        }

        let mut start_want = prefer_snap(
            start,
            SECT_START,
            start_range,
            &mut start_allow,
            &start_part,
            &mut start_dist,
        );

        let mut end_want = prefer_snap(
            end,
            SECT_END,
            end_range,
            &mut end_allow,
            &end_part,
            &mut end_dist,
        );

        debug_assert!(start_dist >= 0 && end_dist >= 0);

        if adjacent && start_want == MOVE_UP && end_want == MOVE_DOWN {
            if end_dist < start_dist {
                start_allow &= !MOVE_UP;
                start_want = prefer_snap(
                    start,
                    SECT_START,
                    start_range,
                    &mut start_allow,
                    &start_part,
                    &mut start_dist,
                );
                debug_assert!(start_dist >= 0);
            } else {
                end_allow &= !MOVE_DOWN;
                end_want = prefer_snap(
                    start,
                    SECT_END,
                    end_range,
                    &mut end_allow,
                    &end_part,
                    &mut end_dist,
                );
                debug_assert!(end_dist >= 0);
            }
        }

        start = match start_want {
            MOVE_DOWN => start_part.geom_start(),
            MOVE_UP => start_part.geom_end() + 1,
            _ => start,
        };

        end = match end_want {
            MOVE_DOWN => end_part.geom_start() - 1,
            MOVE_UP => end_part.geom_end(),
            _ => end,
        };

        debug_assert!(start_range.test_sector_inside(start));
        debug_assert!(end_range.test_sector_inside(end));
        debug_assert!(start <= end);
        let _ = new_geom.set(start, end - start + 1);
    }
}

impl<'a> Iterator for DiskPartIter<'a> {
    type Item = Partition<'a>;
    fn next(&mut self) -> Option<Partition<'a>> {
        let partition = unsafe { ped_disk_next_partition((self.0).disk, self.1) };
        if partition.is_null() {
            None
        } else {
            self.1 = partition;
            unsafe { Some(Partition::from_ped_partition(partition)) }
        }
    }
}

impl<'a> Drop for Disk<'a> {
    fn drop(&mut self) {
        if self.is_droppable {
            unsafe { ped_disk_destroy(self.disk); }
        }
    }
}
