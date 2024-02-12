use crate::{
    ffi,
    options::{ScannerOption, ScannerOptions},
    result::{from_status, sane_try},
    Device, Parameters, SaneError,
};
use std::{
    ffi::{c_void, CStr},
    io,
    ops::RangeInclusive,
    ptr::null_mut,
};
use thiserror::Error;

pub struct Scanner<'sane> {
    device: Device<'sane>,
    device_handle: *mut c_void,
}

impl<'sane> Scanner<'sane> {
    pub fn new(device: Device<'sane>) -> Result<Self, SaneError> {
        let mut device_handle = null_mut();

        log::trace!("Call ffi::sane_open()");
        sane_try!(ffi::sane_open(device.name, &mut device_handle));

        Ok(Self {
            device,
            device_handle,
        })
    }

    pub fn options(&self) -> ScannerOptions {
        ScannerOptions::new(self)
    }

    pub fn start<'scanner>(&'scanner mut self) -> Result<PageReader<'sane, 'scanner>, SaneError> {
        log::trace!("Call ffi::sane_start()");
        sane_try!(ffi::sane_start(self.device_handle));

        Ok(PageReader(self))
    }

    pub fn get_device(&self) -> &Device {
        &self.device
    }

    pub unsafe fn get_device_handle(&self) -> *mut c_void {
        self.device_handle
    }
}

impl Drop for Scanner<'_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_close()");
        unsafe { ffi::sane_close(self.device_handle) };
    }
}

pub struct PageReader<'sane, 'scanner>(&'scanner mut Scanner<'sane>);

impl<'sane, 'scanner> PageReader<'sane, 'scanner> {
    #[rustfmt::skip]
    pub fn get_parameters(&mut self) -> Result<Parameters, SaneError> {
        let mut params = unsafe { core::mem::zeroed() };

        log::trace!("Call ffi::sane_get_parameters()");
        sane_try!(ffi::sane_get_parameters(self.0.device_handle, &mut params));

        Ok(Parameters {
            format: params.format.into(),
            last_frame: {
                assert!([0, 1].contains(&params.last_frame));
                params.last_frame == 1
            },
            bytes_per_line:  {
                assert!(params.bytes_per_line > 0, "bytes_per_line should be greater than 0");
                params.bytes_per_line as usize
            },
            pixels_per_line:  {
                assert!(params.pixels_per_line > 0, "pixels_per_line should be greater than 0");
                params.pixels_per_line as usize
            },
            lines: {
                assert!(params.lines > 0, "lines should be greater than 0");
                params.lines as usize
            },
            depth: {
                assert!(params.depth > 0, "depth should be greater than 0");
                params.depth as usize
            },
        })
    }
}

impl io::Read for PageReader<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut count = 0;

        log::trace!("Call ffi::sane_read()");
        let read_status = unsafe {
            ffi::sane_read(
                self.0.device_handle,
                buf.as_mut_ptr(),
                buf.len().try_into().unwrap_or(i32::MAX),
                &mut count,
            )
        };

        match from_status(read_status) {
            Ok(()) => Ok(count as usize),
            Err(SaneError::EOF) => Ok(0),
            Err(SaneError::IO) => Err(io::ErrorKind::BrokenPipe.into()),
            Err(SaneError::NoMem) => Err(io::ErrorKind::OutOfMemory.into()),
            Err(SaneError::AccessDenied) => Err(io::ErrorKind::PermissionDenied.into()),
            Err(err) => Err(io::Error::other(err)),
        }
    }
}

impl Drop for PageReader<'_, '_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_cancel()");
        unsafe { ffi::sane_cancel(self.0.device_handle) };
    }
}
