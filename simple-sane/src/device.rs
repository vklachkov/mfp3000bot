use crate::{ffi, result::sane_try, Sane, SaneError};
use std::{ffi::CStr, ptr::null_mut};

pub struct Device<'sane> {
    sane: &'sane Sane,
    device: &'sane ffi::SANE_Device,
}

impl<'s> Device<'s> {
    pub fn get_first(sane: &'s Sane) -> Result<Option<Self>, SaneError> {
        let mut device_list = null_mut();

        log::trace!("Call ffi::sane_get_devices()");
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        let device = unsafe { (*device_list).as_ref::<'s>() };
        if let Some(device) = device {
            Ok(Some(Device { sane, device }))
        } else {
            Ok(None)
        }
    }
}

impl std::fmt::Display for Device<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "'{name}' (vendor '{vendor}', model '{model}', type '{ty}')",
                name = CStr::from_ptr(self.device.name).to_string_lossy(),
                model = CStr::from_ptr(self.device.model).to_string_lossy(),
                vendor = CStr::from_ptr(self.device.vendor).to_string_lossy(),
                ty = CStr::from_ptr(self.type_).to_string_lossy(),
            )
        }
    }
}

impl std::ops::Deref for Device<'_> {
    type Target = ffi::SANE_Device;

    fn deref(&self) -> &Self::Target {
        self.device
    }
}
