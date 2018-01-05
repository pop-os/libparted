use libparted_sys::PedConstraint;

pub struct Constraint(pub(crate) *mut PedConstraint);
