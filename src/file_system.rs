use std::ffi::{CStr, CString};
use std::io;
use std::marker::PhantomData;
use std::ptr;
use super::{cvt, get_optional, Geometry, Timer};
use libparted_sys::{ped_file_system_alias_get_next, ped_file_system_alias_register,
                    ped_file_system_alias_unregister, ped_file_system_resize,
                    ped_file_system_type_get, ped_file_system_type_get_next,
                    ped_file_system_type_register, ped_file_system_type_unregister, PedFileSystem,
                    PedFileSystemAlias, PedFileSystemType};

pub struct FileSystem<'a> {
    pub(crate) fs: *mut PedFileSystem,
    pub(crate) phantom: PhantomData<&'a mut PedFileSystem>,
}

impl<'a> FileSystem<'a> {
    pub fn from_raw(fs: *mut PedFileSystem) -> FileSystem<'a> {
        FileSystem {
            fs,
            phantom: PhantomData,
        }
    }

    pub fn checked(&self) -> bool {
        unsafe { (*self.fs).checked != 0 }
    }

    pub fn geom<'b>(&'b mut self) -> Geometry<'b> {
        Geometry::from_raw(unsafe { (*self.fs).geom })
    }

    pub fn type_<'b>(&'b mut self) -> FileSystemType<'b> {
        FileSystemType::from_raw(unsafe { (*self.fs).type_ })
    }

    /// Opens the file system stored on `geom`, if it can find one.
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
    ///
    /// # Note
    ///
    /// This actually calls `Geometry::open_fs`
    pub fn open(geom: &'a Geometry) -> Option<FileSystem<'a>> {
        geom.open_fs()
    }

    /// Attempt to detect a file system in region `geom`.
    ///
    /// This function tries to be clever at dealing with ambiguous situations, such as
    /// when one file system was not completely erased before a new file system was created on
    /// on top of it.
    ///
    /// # Note
    ///
    /// This actually calls `geom.probe_fs()`.
    pub fn probe(geom: &'a Geometry) -> io::Result<FileSystemType<'a>> {
        geom.probe_fs()
    }

    /// Attempt to find a file system and return the region it occupies.
    ///
    /// # Note
    ///
    /// This actually calls `geom.probe_specific_fs(fs_type)`
    pub fn probe_specific(geom: &'a Geometry, fs_type: &'a FileSystemType) -> Option<Geometry<'a>> {
        geom.probe_specific_fs(fs_type)
    }

    /// Resize the file system to a new geometry.
    ///
    /// # Note
    ///
    /// `geom` should satisfy the `FileSystem::get_resize_constraint()`. This isn't
    /// asserted, so it's not a bug not to, but is is likely to fail.
    ///
    /// If `timer` is not `None`, it will be used as the progress meter.
    ///
    /// # Throws
    ///
    /// Throws `PED_EXCEPTION_NO_FEATURE` if resizing of the file system is not implemented yet.
    pub fn resize(&mut self, geom: &Geometry, timer: Option<&mut Timer>) -> io::Result<()> {
        let timer = timer.map_or(ptr::null_mut(), |t| t.timer);
        cvt(unsafe { ped_file_system_resize(self.fs, geom.geometry, timer) }).map(|_| ())
    }
}

pub struct FileSystemAlias<'a> {
    pub(crate) fs: *mut PedFileSystemAlias,
    pub(crate) phantom: PhantomData<&'a mut PedFileSystemAlias>,
}

impl<'a> FileSystemAlias<'a> {
    pub fn from_raw(fs: *mut PedFileSystemAlias) -> FileSystemAlias<'a> {
        FileSystemAlias {
            fs,
            phantom: PhantomData,
        }
    }

    pub fn iter(&self) -> FileSystemAliasIter {
        FileSystemAliasIter(self, ptr::null_mut())
    }

    pub fn fs_type(&'a self) -> FileSystemType<'a> {
        unsafe { FileSystemType::from_raw((*self.fs).fs_type) }
    }

    pub fn fs_type_mut(&'a mut self) -> FileSystemType<'a> {
        unsafe { FileSystemType::from_raw((*self.fs).fs_type) }
    }

    pub fn alias(&'a self) -> &'a [u8] {
        unsafe { CStr::from_ptr((*self.fs).alias).to_bytes() }
    }

    pub fn deprecated(&self) -> bool {
        unsafe { (*self.fs).deprecated != 0 }
    }
}

pub struct FileSystemType<'a> {
    pub(crate) fs: *mut PedFileSystemType,
    pub(crate) phantom: PhantomData<&'a mut PedFileSystemType>,
}

impl<'a> FileSystemType<'a> {
    pub fn from_raw(fs: *mut PedFileSystemType) -> FileSystemType<'a> {
        FileSystemType {
            fs,
            phantom: PhantomData,
        }
    }

    pub fn iter(&self) -> FileSystemTypeIter {
        FileSystemTypeIter(self, ptr::null_mut())
    }

    pub fn name(&self) -> &[u8] {
        unsafe { CStr::from_ptr((*self.fs).name).to_bytes() }
    }

    // TODO: fn ops()

    /// Get a **FileSystemType** by its `name`.
    pub fn get(name: &str) -> Option<FileSystemType<'a>> {
        CString::new(name.as_bytes()).ok().and_then(|name_cstr| {
            let name_ptr = name_cstr.as_ptr();
            get_optional(unsafe { ped_file_system_type_get(name_ptr) })
                .map(FileSystemType::from_raw)
        })
    }

    pub fn register(&mut self) {
        unsafe { ped_file_system_type_register(self.fs) }
    }

    pub fn unregister(&mut self) {
        unsafe { ped_file_system_type_unregister(self.fs) }
    }

    pub fn register_alias(&mut self, alias: &str, deprecated: bool) {
        let _ = CString::new(alias.as_bytes()).map(|cstr| {
            let alias_ptr = cstr.as_ptr();
            let deprecated = if deprecated { 1 } else { 0 };
            unsafe { ped_file_system_alias_register(self.fs, alias_ptr, deprecated) }
        });
    }

    pub fn unregister_alias(&mut self, alias: &str) {
        let _ = CString::new(alias.as_bytes()).map(|cstr| {
            let alias_ptr = cstr.as_ptr();
            unsafe { ped_file_system_alias_unregister(self.fs, alias_ptr) }
        });
    }
}

pub struct FileSystemAliasIter<'a>(&'a FileSystemAlias<'a>, *mut PedFileSystemAlias);

impl<'a> Iterator for FileSystemAliasIter<'a> {
    type Item = FileSystemAlias<'a>;
    fn next(&mut self) -> Option<FileSystemAlias<'a>> {
        let fs = unsafe { ped_file_system_alias_get_next((self.0).fs) };
        if fs.is_null() {
            None
        } else {
            self.1 = fs;
            Some(FileSystemAlias::from_raw(fs))
        }
    }
}

pub struct FileSystemTypeIter<'a>(&'a FileSystemType<'a>, *mut PedFileSystemType);

impl<'a> Iterator for FileSystemTypeIter<'a> {
    type Item = FileSystemType<'a>;
    fn next(&mut self) -> Option<FileSystemType<'a>> {
        let fs = unsafe { ped_file_system_type_get_next((self.0).fs) };
        if fs.is_null() {
            None
        } else {
            self.1 = fs;
            Some(FileSystemType::from_raw(fs))
        }
    }
}
