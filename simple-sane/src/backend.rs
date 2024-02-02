use crate::{
    ffi,
    result::{sane_try, SaneError},
};
use std::{marker::PhantomData, ptr::null_mut};

pub struct Sane {
    __private_field: (),
}

impl Sane {
    pub fn new() -> Result<Self, SaneError> {
        log::trace!("Call ffi::sane_init()");
        sane_try!(ffi::sane_init(null_mut(), None));

        Ok(Self {
            __private_field: (),
        })
    }
}

impl Drop for Sane {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_exit()");
        unsafe { ffi::sane_exit() };
    }
}
