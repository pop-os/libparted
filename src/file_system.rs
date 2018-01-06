use std::marker::PhantomData;
use libparted_sys::PedFileSystemType;

pub struct FileSystemType<'a> {
    pub(crate) fs: *mut PedFileSystemType,
    pub(crate) phantom: PhantomData<&'a PedFileSystemType>,
}
