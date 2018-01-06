extern crate libparted_sys;

use std::io;

pub use self::alignment::Alignment;
pub use self::constraint::Constraint;
pub use self::device::{CHSGeometry, Device, DeviceExternalAccess, DeviceIter};
pub use self::disk::{Disk, DiskFlag, DiskPartIter, DiskType, DiskTypeFeature};
pub use self::file_system::{FileSystem, FileSystemAlias, FileSystemAliasIter, FileSystemType,
                            FileSystemTypeIter};
pub use self::geometry::Geometry;
pub use self::partition::{Partition, PartitionFlag, PartitionType};
pub use self::misc::{round_down_to, round_to_nearest, round_up_to};
pub use self::timer::Timer;

pub(crate) use self::constraint::ConstraintSource;

mod alignment;
mod constraint;
mod device;
mod disk;
mod file_system;
mod geometry;
mod misc;
mod partition;
mod timer;

pub(crate) fn get_optional<T>(data: *mut T) -> Option<*mut T> {
    if data.is_null() {
        None
    } else {
        Some(data)
    }
}

#[doc(hidden)]
pub trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($($t:ident)*) => ($(impl IsZero for $t {
        fn is_zero(&self) -> bool {
            *self == 0
        }
    })*)
}

impl_is_zero! { i8 i16 i32 i64 isize u8 u16 u32 u64 usize }

impl<T> IsZero for *const T {
    fn is_zero(&self) -> bool {
        self.is_null()
    }
}

impl<T> IsZero for *mut T {
    fn is_zero(&self) -> bool {
        self.is_null()
    }
}

fn cvt<T: IsZero>(t: T) -> io::Result<T> {
    if t.is_zero() {
        Err(io::Error::last_os_error())
    } else {
        Ok(t)
    }
}
