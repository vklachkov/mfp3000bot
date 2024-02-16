use crate::{ffi, result::sane_try, utils::cstr2bstr, Backend, SaneError};
use bstr::BStr;
use std::{ffi::CStr, ops::Deref, ptr::null_mut};

#[derive(Debug, Clone, Copy)]
pub struct Device<'b> {
    backend: &'b Backend,

    pub name: &'b BStr,
    pub vendor: &'b BStr,
    pub model: &'b BStr,
    pub ty: &'b BStr,
}

impl<'s> Device<'s> {
    pub(crate) fn new(backend: &'s Backend, device: &'s ffi::SANE_Device) -> Self {
        Device {
            backend,
            name: unsafe { cstr2bstr(device.name).expect("name should be not null") },
            vendor: unsafe { cstr2bstr(device.vendor).expect("vendor should be not null") },
            model: unsafe { cstr2bstr(device.model).expect("model should be not null") },
            ty: unsafe { cstr2bstr(device.type_).expect("type should be not null") },
        }
    }
}

impl std::fmt::Display for Device<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{name}' (vendor '{vendor}', model '{model}', type '{ty}')",
            name = self.name,
            vendor = self.vendor,
            model = self.model,
            ty = self.ty,
        )
    }
}
