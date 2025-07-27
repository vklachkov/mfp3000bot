use crate::{
    options::{ScannerOption, ScannerOptions},
    result::{from_status, sane_try},
    Device, Parameters, SaneError,
};
use bstr::BStr;
use libsane_sys::*;
use std::{
    ffi::{c_void, CStr, CString}, io, marker::PhantomData, ops::RangeInclusive, ptr::null_mut
};

#[derive(Debug)]
pub struct Scanner {
    handle: SANE_Handle,
}

impl Scanner {
    pub fn new(name: impl AsRef<[u8]>) -> Result<Self, SaneError> {
        let name = CString::new(name.as_ref()).map_err(|_| SaneError::Inval)?;
        let mut handle = null_mut();

        log::trace!("Call sane_open('{}', {:p})", name.to_string_lossy(), &mut handle);
        sane_try!(sane_open(name.as_ptr(), &mut handle));

        Ok(Self { handle })
    }

    pub fn from_device(device: Device) -> Result<Self, SaneError> {
        Self::new(device.name)
    }

    pub fn options(&self) -> ScannerOptions {
        ScannerOptions::new(self)
    }

    pub fn start(&mut self) -> Result<PageReader<'_>, SaneError> {
        log::trace!("Call sane_start({:p})", self.handle);
        sane_try!(sane_start(self.handle));

        Ok(PageReader(self))
    }

    pub fn get_device_handle(&self) -> *mut c_void {
        self.handle
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        log::trace!("Call sane_close({:p})", self.handle);
        unsafe { sane_close(self.handle) };
    }
}

pub struct PageReader<'s>(&'s mut Scanner);

impl PageReader<'_> {
    #[rustfmt::skip]
    pub fn get_parameters(&mut self) -> Result<Parameters, SaneError> {
        let mut params = unsafe { core::mem::zeroed() };

        log::trace!("Call sane_get_parameters({:p}, {:p})", self.0.handle, &mut params);
        sane_try!(sane_get_parameters(self.0.handle, &mut params));

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

impl io::Read for PageReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let mut count = 0;

        log::trace!(
            "Call sane_read({:p}, {:p}, {}, {:p})",
            self.0.handle,
            buf.as_mut_ptr(),
            buf.len().try_into().unwrap_or(i32::MAX),
            &mut count,
        );

        let read_status = unsafe {
            sane_read(
                self.0.handle,
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

impl Drop for PageReader<'_> {
    fn drop(&mut self) {
        log::trace!("Call sane_cancel({:p})", self.0.handle);
        unsafe { sane_cancel(self.0.handle) };
    }
}
