use libparted_sys::{
    PedAlignment,
    ped_alignment_destroy,
    ped_alignment_duplicate,
    ped_alignment_init,
    ped_alignment_intersect,
    ped_alignment_new,
};

use std::io;
use std::marker::PhantomData;
use super::cvt;

pub struct Alignment<'a> {
    pub(crate) alignment: *mut PedAlignment,
    pub(crate) phantom: PhantomData<&'a PedAlignment>
}

impl<'a> Alignment<'a> {
    /// Return an alignment object representing all sectors that are of the form
    /// `offset + X * grain_size`.
    pub fn new(offset: i64, grain_size: i64) -> io::Result<Alignment<'a>> {
        let alignment = unsafe { ped_alignment_new(offset, grain_size) };
        if alignment.is_null() {
            Err(io::Error::new(io::ErrorKind::Other, "failed to allocate allignment"))
        } else {
            Ok(Alignment { alignment, phantom: PhantomData })
        }
    }

    /// Initializes a preallocated piece of memory for an alignment object.
    pub fn init(&mut self, offset: i64, grain_size: i64) -> io::Result<()> {
        cvt(unsafe { ped_alignment_init(self.alignment, offset, grain_size) })?;
        Ok(())
    }

    /// Returns a new **Alignment** object if an intersection can between
    /// itself and a given `other` **Alignment**.
    pub fn intersect(&self, other: &Alignment) -> Option<Alignment<'a>> {
        let alignment = unsafe {
            ped_alignment_intersect(self.alignment, other.alignment)
        };
        if alignment.is_null() {
            None
        } else {
            Some(Alignment { alignment, phantom: PhantomData })
        }
    }
}

impl<'a> Clone for Alignment<'a> {
    fn clone(&self) -> Self {
        Alignment {
            alignment: unsafe { ped_alignment_duplicate(self.alignment) },
            phantom: PhantomData
        }
    }
}

impl<'a> Drop for Alignment<'a> {
    fn drop(&mut self) {
        unsafe { ped_alignment_destroy(self.alignment) }
    }
}