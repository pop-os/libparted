use std::marker::PhantomData;
use std::os::raw::c_void;
use std::io;
use super::{cvt, get_optional, Constraint, ConstraintSource, Device, FileSystem, FileSystemType,
            Timer};
use libparted_sys::{ped_constraint_exact, ped_file_system_open, ped_file_system_probe,
                    ped_file_system_probe_specific, ped_geometry_check, ped_geometry_destroy,
                    ped_geometry_duplicate, ped_geometry_init, ped_geometry_intersect,
                    ped_geometry_map, ped_geometry_new, ped_geometry_read, ped_geometry_set,
                    ped_geometry_set_end, ped_geometry_set_start, ped_geometry_sync,
                    ped_geometry_sync_fast, ped_geometry_test_equal, ped_geometry_test_inside,
                    ped_geometry_write, PedGeometry};

pub struct Geometry<'a> {
    pub(crate) geometry: *mut PedGeometry,
    pub(crate) phantom: PhantomData<&'a PedGeometry>,
    pub(crate) is_droppable: bool,
}

impl<'a> Geometry<'a> {
    pub fn from_raw(geometry: *mut PedGeometry) -> Geometry<'a> {
        Geometry {
            geometry,
            phantom: PhantomData,
            is_droppable: true,
        }
    }

    /// Return a constraint that only the given region will satisfy.
    pub fn exact(&self) -> Option<Constraint> {
        get_optional(unsafe { ped_constraint_exact(self.geometry) }).map(|constraint| Constraint {
            constraint,
            source: ConstraintSource::New,
            phantom: PhantomData,
        })
    }

    /// Checks for physical disk errors.
    ///
    /// Checks a region for physical defects on `geom`. The region checked starts at `offset`
    /// sectors inside the region represented by `geom`, and is `count` sectors long.
    /// `granularity` specifies how sectors should be grouped together.
    ///
    /// The first bad sector to be returned will always be in the form:
    ///
    /// ```
    ///     offset + n * granularity
    /// ```
    ///
    /// Returns the first bad sector if a bad sector was found.
    pub fn check(
        &self,
        offset: i64,
        granularity: i64,
        count: i64,
        timer: &Timer,
    ) -> Result<(), u64> {
        let mut buffer: Vec<u8> = Vec::with_capacity(8192);
        let buffer_ptr = buffer.as_mut_slice().as_mut_ptr();
        let result = unsafe {
            ped_geometry_check(
                self.geometry,
                buffer_ptr as *mut c_void,
                buffer.len() as i64,
                offset,
                granularity,
                count,
                timer.timer,
            )
        };

        match result {
            0 => Ok(()),
            bad_sector => Err(bad_sector as u64),
        }
    }

    pub fn dev<'b>(&'b self) -> Device<'b> {
        unsafe { Device::from_ped_device((*self.geometry).dev) }
    }

    pub fn dev_mut<'b>(&'b mut self) -> Device<'b> {
        unsafe { Device::from_ped_device((*self.geometry).dev) }
    }

    /// Duplicate a `Geometry` object.
    pub fn duplicate<'b>(&self) -> io::Result<Geometry<'b>> {
        cvt(unsafe { ped_geometry_duplicate(self.geometry) }).map(Geometry::from_raw)
    }

    pub fn end(&self) -> i64 {
        unsafe { (*self.geometry).end }
    }

    /// Initializes a pre-allocated **Geometry**.
    pub fn init(&mut self, device: &Device, start: i64, length: i64) -> io::Result<()> {
        cvt(unsafe { ped_geometry_init(self.geometry, device.ped_device(), start, length) })
            .map(|_| ())
    }

    /// Return a **Geometry** object that refers to the intersection between itself and another
    /// **Geometry**.
    pub fn intersect(&self, other: &Geometry) -> Option<Geometry<'a>> {
        get_optional(unsafe { ped_geometry_intersect(self.geometry, other.geometry) })
            .map(Geometry::from_raw)
    }

    pub fn length(&self) -> i64 {
        unsafe { (*self.geometry).length }
    }

    /// Takes a `sector` inside the region described by `src` and returns that sector's address
    /// inside of our own **Geometry** marked as `self`. This means that the following
    /// code examples are equivalent:
    ///
    /// ```
    /// geometry.read(buf, geometry.map(src, sector), 1);
    /// ```
    ///
    /// ```
    /// geometry.read(buf, sector, 1);
    /// ```
    ///
    /// Clearly, this will only work if `self` and `src` overlap.
    pub fn map(&self, src: &Geometry, sector: i64) -> Option<u64> {
        let result = unsafe { ped_geometry_map(self.geometry, src.geometry, sector) };
        if result == -1 {
            None
        } else {
            Some(result as u64)
        }
    }

    /// Create a new **Geometry** object on `disk`, starting at `start`
    /// with a size of `length` sectors.
    pub fn new(device: &Device, start: i64, length: i64) -> io::Result<Geometry<'a>> {
        cvt(unsafe { ped_geometry_new(device.ped_device(), start, length) }).map(Geometry::from_raw)
    }

    /// Reads data from the region within our `Geometry`. `offset` is the location from within
    /// the region, not from the start of the disk. `count` sectors are read into `buffer`. An
    /// exception is thrown when attempting to read sectors outside of the partition.
    ///
    /// # Note:
    ///
    /// The supplied vector will be reallocated to the correct size automatically.
    ///
    /// # Throws:
    ///
    /// Throws `PED_EXCEPTION_ERROR` when attempting to read sectors outside of partition.
    pub fn read(&self, buffer: &mut Vec<u8>, offset: i64, count: i64) -> io::Result<()> {
        // Ensure that the buffer is the correct length to hold the data
        let sector_size = unsafe { (*(*self.geometry).dev).sector_size as usize };
        buffer.reserve(count as usize * sector_size + 1);

        // Then fire away with reading using a pointer to the buffer.
        let buffer_ptr = buffer.as_mut_slice().as_mut_ptr() as *mut c_void;
        cvt(unsafe { ped_geometry_read(self.geometry, buffer_ptr, offset, count) }).map(|_| ())
    }

    /// Assign a new `start` and `length`, where `end` will also be set implicitly from those
    /// values.
    pub fn set(&mut self, start: i64, length: i64) -> io::Result<()> {
        cvt(unsafe { ped_geometry_set(self.geometry, start, length) }).map(|_| ())
    }

    /// Assign a new end to `self` without changing `self->start` field.
    ///
    /// `self->length` will be updated accordingly.
    pub fn set_end(&mut self, end: i64) -> io::Result<()> {
        cvt(unsafe { ped_geometry_set_end(self.geometry, end) }).map(|_| ())
    }

    /// Assign a new start to `self` witout changing `self->end`.
    ///
    /// `self->length` will be updated accordingly.
    pub fn set_start(&mut self, start: i64) -> io::Result<()> {
        cvt(unsafe { ped_geometry_set_start(self.geometry, start) }).map(|_| ())
    }

    pub fn start(&self) -> i64 {
        unsafe { (*self.geometry).start }
    }

    /// Flushes the cache on `self`.
    ///
    /// This function flushses all write-behind caches that might be holding writes made by
    /// `Geometry::write()` to `self`. It is slow because it guarantees cache coherency among all
    /// relevant caches.
    pub fn sync(&mut self) -> io::Result<()> {
        cvt(unsafe { ped_geometry_sync(self.geometry) }).map(|_| ())
    }

    /// Flushes the cache on `self`.
    ///
    /// This function flushses all write-behind caches that might be holding writes made by
    /// `Geometry::write()` to `self`. It does not ensure cache coherency with other caches that
    /// cache data in the region described by `self`.
    ///
    /// If you need cache coherency, use `Geometry::sync()` instead.
    pub fn sync_fast(&mut self) -> io::Result<()> {
        cvt(unsafe { ped_geometry_sync_fast(self.geometry) }).map(|_| ())
    }

    /// Tests if the `other` **Geometry** refers to the same physical region as `self`.
    pub fn test_equal(&self, other: &Geometry) -> bool {
        unsafe { ped_geometry_test_equal(self.geometry, other.geometry) == 1 }
    }

    /// Tests if the `other` **Geometry** is inside `self`.
    pub fn test_inside(&self, other: &Geometry) -> bool {
        unsafe { ped_geometry_test_inside(self.geometry, other.geometry) == 1 }
    }

    /// Tests if `sector` is inside the geometry.
    pub fn test_sector_inside(&self, sector: i64) -> bool {
        debug_assert!(!self.geometry.is_null());
        sector >= self.start() && sector <= self.end()
    }

    /// Writes data into the region represented by `self`. The `offset` is the location
    /// from within the region, not from the start of the disk. `count` sectors are to be written.
    pub fn write_to_sectors(&mut self, buffer: &[u8], offset: i64, count: i64) -> io::Result<()> {
        let sector_size = unsafe { (*(*self.geometry).dev).sector_size as usize };
        let total_size = sector_size * count as usize;
        if buffer.len() != total_size {
            let mut new_buffer = Vec::with_capacity(total_size);
            new_buffer.extend_from_slice(buffer);
            new_buffer.extend((buffer.len()..total_size).map(|_| b'0'));
            let buffer_ptr = new_buffer.as_slice().as_ptr() as *const c_void;
            cvt(unsafe { ped_geometry_write(self.geometry, buffer_ptr, offset, count) }).map(|_| ())
        } else {
            let buffer_ptr = buffer.as_ptr() as *const c_void;
            cvt(unsafe { ped_geometry_write(self.geometry, buffer_ptr, offset, count) }).map(|_| ())
        }
    }

    /// Opens the file system stored in the given **Geometry**.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut fs = FileSystem::open(&mut geometry);
    /// ```
    ///
    /// ```rust
    /// let mut fs = geometry.open_fs();
    /// ```
    ///
    /// # Throws
    ///
    /// - `PED_EXCEPTION_ERROR` if the file system could not be detected.
    /// - `PED_EXCEPTION_ERROR` if the file system is bigger than its volume.
    /// - `PED_EXCEPTION_NO_FEATURE` if opening of a file system stored on `geom` is
    ///     not implemented.
    pub fn open_fs<'b>(&'b self) -> Option<FileSystem<'b>> {
        get_optional(unsafe { ped_file_system_open(self.geometry) }).map(FileSystem::from_raw)
    }

    /// Attempt to detect a file system in the given **Geometry**.
    ///
    /// This function tries to be clever at dealing with ambiguous situations, such as
    /// when one file system was not completely erased before a new file system was created on
    /// on top of it.
    pub fn probe_fs<'b>(&'b self) -> io::Result<FileSystemType<'b>> {
        cvt(unsafe { ped_file_system_probe(self.geometry) }).map(FileSystemType::from_raw)
    }

    /// Attempt to find a file system and return the region it occupies.
    pub fn probe_specific_fs<'b>(&'b self, fs_type: &'b FileSystemType) -> Option<Geometry<'b>> {
        get_optional(unsafe { ped_file_system_probe_specific(fs_type.fs, self.geometry) })
            .map(Geometry::from_raw)
    }
}

impl<'a> Drop for Geometry<'a> {
    fn drop(&mut self) {
        if self.is_droppable {
            unsafe { ped_geometry_destroy(self.geometry) }
        }
    }
}
