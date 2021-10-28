use libparted_sys::PedTimer;
use std::marker::PhantomData;

pub struct Timer<'a> {
    pub(crate) timer: *mut PedTimer,
    pub phantom: PhantomData<&'a PedTimer>,
}
