use libparted_sys::{ped_alignment_align_down, ped_alignment_align_nearest, ped_alignment_align_up,
                    ped_alignment_destroy, ped_alignment_duplicate, ped_alignment_init,
                    ped_alignment_intersect, ped_alignment_is_aligned, ped_alignment_new,
                    PedAlignment};

use std::io;
use std::marker::PhantomData;
use super::{cvt, get_optional, Geometry};

pub struct Alignment<'a> {
    pub(crate) alignment: *mut PedAlignment,
    pub(crate) phantom: PhantomData<&'a PedAlignment>,
}

impl<'a> Alignment<'a> {
    fn new_(alignment: *mut PedAlignment) -> Alignment<'a> {
        Alignment {
            alignment,
            phantom: PhantomData,
        }
    }

    /// Return an alignment object representing all sectors that are of the form
    /// `offset + X * grain_size`.
    pub fn new(offset: i64, grain_size: i64) -> io::Result<Alignment<'a>> {
        cvt(unsafe { ped_alignment_new(offset, grain_size) }).map(Alignment::new_)
    }

    /// Initializes a preallocated piece of memory for an alignment object.
    ///
    /// The object will represent all sectors for which the equation
    /// `S = offset + x * grain_size` holds.
    pub fn init(&mut self, offset: i64, grain_size: i64) -> io::Result<()> {
        cvt(unsafe { ped_alignment_init(self.alignment, offset, grain_size) })?;
        Ok(())
    }

    /// Returns the sector that is closest to `sector`, satifies the `align` constraint, and lies
    /// lies inside `geom`.
    pub fn align_down(&self, geom: &Geometry, sector: i64) -> Option<u64> {
        match unsafe { ped_alignment_align_down(self.alignment, geom.geometry, sector) } {
            -1 => None,
            sector => Some(sector as u64),
        }
    }

    /// Returns the sector that is closest to `sector`, satisfies the `align` constraint, and
    /// lies inside of `geom`.
    pub fn align_nearest(&self, geom: &Geometry, sector: i64) -> Option<u64> {
        match unsafe { ped_alignment_align_nearest(self.alignment, geom.geometry, sector) } {
            -1 => None,
            sector => Some(sector as u64),
        }
    }

    /// Returns the sector that is closest to `sector`, satifies the `align` constraint, and lies
    /// lies inside `geom`.
    pub fn align_up(&self, geom: &Geometry, sector: i64) -> Option<u64> {
        match unsafe { ped_alignment_align_up(self.alignment, geom.geometry, sector) } {
            -1 => None,
            sector => Some(sector as u64),
        }
    }

    /// Clones and returns a duplicate of the alignment, if possible.
    pub fn duplicate(&self) -> io::Result<Alignment> {
        cvt(unsafe { ped_alignment_duplicate(self.alignment) }).map(|alignment| Alignment {
            alignment,
            phantom: PhantomData,
        })
    }

    /// Returns a new **Alignment** object if an intersection can between
    /// itself and a given `other` **Alignment**.
    pub fn intersect(&self, other: &Alignment) -> Option<Alignment<'a>> {
        get_optional(unsafe { ped_alignment_intersect(self.alignment, other.alignment) })
            .map(Alignment::new_)
    }

    /// Returns the sector that is closest to `sector`, satifies the `align` constraint, and lies
    /// lies inside `geom`.
    pub fn is_aligned(&self, geom: &Geometry, sector: i64) -> bool {
        unsafe { ped_alignment_is_aligned(self.alignment, geom.geometry, sector) == 1 }
    }
}
impl<'a> Drop for Alignment<'a> {
    fn drop(&mut self) {
        unsafe { ped_alignment_destroy(self.alignment) }
    }
}
