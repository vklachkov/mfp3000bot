use bstr::BStr;

use crate::{ffi, result::sane_try, utils::cstr2bstr, Sane, SaneError};
use std::{ffi::CStr, ops::Deref, ptr::null_mut};

#[derive(Debug, Clone, Copy)]
pub struct Device<'sane> {
    sane: &'sane Sane,
    info: DeviceInfo<'sane>,
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo<'a> {
    pub name: &'a BStr,
    pub vendor: &'a BStr,
    pub model: &'a BStr,
    pub ty: &'a BStr,
}

impl From<&ffi::SANE_Device> for DeviceInfo<'_> {
    fn from(device: &ffi::SANE_Device) -> Self {
        Self {
            name: unsafe { cstr2bstr(device.name).expect("name should be not null") },
            vendor: unsafe { cstr2bstr(device.vendor).expect("vendor should be not null") },
            model: unsafe { cstr2bstr(device.model).expect("model should be not null") },
            ty: unsafe { cstr2bstr(device.type_).expect("type should be not null") },
        }
    }
}

unsafe impl Send for Device<'_> {}

unsafe impl Sync for Device<'_> {}

impl<'s> Device<'s> {
    pub fn get_first(sane: &'s Sane) -> Result<Option<Self>, SaneError> {
        let mut device_list = null_mut();

        log::trace!("Call ffi::sane_get_devices()");
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        let device = unsafe { (*device_list).as_ref::<'s>() };
        if let Some(device) = device {
            Ok(Some(Device {
                sane,
                info: device.into(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn find_by_name(sane: &'s Sane, name: &str) -> Result<Option<Self>, SaneError> {
        let mut device_list = null_mut();

        log::trace!("Call ffi::sane_get_devices()");
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        let devices = unsafe { slice_from_c_array(device_list) };
        for device in devices {
            let info: DeviceInfo = (*device).into();
            if info.name == name {
                return Ok(Some(Device { sane, info }));
            }
        }

        Ok(None)
    }
}

pub unsafe fn slice_from_c_array<'a, T>(ptr: *const *const T) -> &'a [&'a T] {
    for i in 0..usize::MAX {
        if (*ptr.add(i)).is_null() {
            let slice = core::slice::from_raw_parts::<'a, *const T>(ptr, i);
            return std::mem::transmute(slice);
        }
    }

    unreachable!()
}

impl<'sane> Deref for Device<'sane> {
    type Target = DeviceInfo<'sane>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl std::fmt::Display for Device<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{name}' (vendor '{vendor}', model '{model}', type '{ty}')",
            name = self.info.name,
            vendor = self.info.vendor,
            model = self.info.model,
            ty = self.info.ty,
        )
    }
}
