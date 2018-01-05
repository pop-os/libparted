use std::marker::PhantomData;

use libparted_sys::{
    PedConstraint,
    ped_constraint_destroy
};

pub struct Constraint<'a> {
    pub(crate) constraint: *mut PedConstraint,
    pub(crate) phantom: PhantomData<&'a PedConstraint>
}

impl<'a> Drop for Constraint<'a> {
    fn drop(&mut self) {
        unsafe { ped_constraint_destroy(self.constraint) }
    }
}