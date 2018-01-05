extern crate libparted_sys;

use std::io;

pub use self::alignment::Alignment;
pub use self::constraint::Constraint;
pub use self::device::Device;
pub use self::disk::Disk;
pub use self::partition::Partition;

mod alignment;
mod constraint;
mod device;
mod disk;
mod partition;

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
