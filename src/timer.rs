use std::marker::PhantomData;
use libparted_sys::PedTimer;

pub struct Timer<'a> {
    pub(crate) timer: *mut PedTimer,
    pub phantom: PhantomData<&'a PedTimer>,
}
