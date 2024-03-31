extern crate libc;
extern crate libparted_sys;

use std::io;

pub use self::alignment::Alignment;
pub use self::constraint::Constraint;
pub use self::device::{CHSGeometry, Device, DeviceExternalAccess, DeviceIter, DeviceType};
pub use self::disk::{Disk, DiskFlag, DiskPartIter, DiskType, DiskTypeFeature, PartitionTableType};
pub use self::file_system::{
    FileSystem, FileSystemAlias, FileSystemAliasIter, FileSystemType, FileSystemTypeIter,
};
pub use self::geometry::Geometry;
pub use self::misc::{round_down_to, round_to_nearest, round_up_to};
pub use self::partition::{Partition, PartitionFlag, PartitionType};
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

// pub(crate) const MOVE_NO: u8 = 0;
pub(crate) const MOVE_STILL: u8 = 1;
pub(crate) const MOVE_UP: u8 = 2;
pub(crate) const MOVE_DOWN: u8 = 4;

pub(crate) const SECT_START: i32 = 0;
pub(crate) const SECT_END: i32 = -1;

pub fn snap(sector: &mut i64, new_sector: i64, range: &Geometry) -> bool {
    debug_assert!(range.test_sector_inside(*sector));
    if !range.test_sector_inside(new_sector) {
        return false;
    }
    *sector = new_sector;
    true
}

pub fn prefer_snap(
    s: i64,
    what: i32,
    range: &Geometry,
    allow: &mut u8,
    part: &Partition,
    dist: &mut i64,
) -> u8 {
    let (mut up_dist, mut down_dist) = (-1i64, -1i64);
    let mut moves;

    debug_assert!(what == SECT_START || what == SECT_END);

    if *allow & (MOVE_UP | MOVE_DOWN) == 0 {
        *dist = 0;
        return MOVE_STILL;
    }

    if *allow & MOVE_UP != 0 {
        let new_sect = part.geom_end() + 1 + what as i64;
        if range.test_sector_inside(new_sect) {
            up_dist = new_sect - s;
        } else {
            *allow &= !MOVE_UP;
        }
    }

    if *allow & MOVE_DOWN != 0 {
        let new_sect = part.geom_start() + what as i64;
        if range.test_sector_inside(new_sect) {
            down_dist = s - new_sect;
        } else {
            *allow &= !MOVE_DOWN;
        }
    }

    moves = MOVE_STILL;
    if *allow & MOVE_UP != 0 && *allow & MOVE_DOWN != 0 {
        if down_dist < up_dist || (down_dist == up_dist && what == SECT_START) {
            moves = MOVE_DOWN;
        } else if up_dist < down_dist || (down_dist == up_dist && what == SECT_END) {
            moves = MOVE_UP;
        } else {
            unreachable!();
        }
    } else if *allow & MOVE_UP != 0 {
        moves = MOVE_UP;
    } else if *allow & MOVE_DOWN != 0 {
        moves = MOVE_DOWN;
    }

    *dist = if moves == MOVE_DOWN {
        down_dist
    } else if moves == MOVE_UP {
        up_dist
    } else {
        0
    };

    moves
}

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
