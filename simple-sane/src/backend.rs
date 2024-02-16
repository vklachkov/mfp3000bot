use crate::{
    ffi,
    result::{sane_try, Result, SaneError},
    utils::slice_from_c_array,
    Device,
};
use bstr::BStr;
use std::{fmt::Debug, marker::PhantomData, ptr::null_mut};

pub struct Backend {
    __private_field: (),
}

impl Backend {
    pub fn new() -> Result<Self> {
        log::trace!("Call ffi::sane_init(0x0, 0x0)");
        sane_try!(ffi::sane_init(null_mut(), None));

        Ok(Self {
            __private_field: (),
        })
    }

    pub fn get_all_devices<'b>(&'b self) -> Result<Vec<Device<'b>>> {
        let devices = self.get_devices()?;

        let devices = unsafe { slice_from_c_array::<'b>(devices) }
            .into_iter()
            .map(|device| Device::new(self, device))
            .collect();

        Ok(devices)
    }

    pub fn find_device_by_name<'b, N>(&'b self, name: N) -> Result<Option<Device<'b>>>
    where
        N: AsRef<[u8]>,
    {
        let devices = self.get_devices()?;

        let device = unsafe { slice_from_c_array::<'b>(devices) }
            .into_iter()
            .map(|device| Device::new(self, device))
            .find(|device| device.name == BStr::new(&name));

        Ok(device)
    }

    fn get_devices(&self) -> Result<*const *const ffi::SANE_Device> {
        let mut device_list = null_mut();

        log::trace!("Call ffi::sane_get_devices({:p}, {})", &mut device_list, 0);
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        Ok(device_list)
    }
}

impl Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish()
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_exit()");
        unsafe { ffi::sane_exit() };
    }
}
