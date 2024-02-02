use std::{io, ffi::c_void, ptr::null_mut};
use thiserror::Error;
use crate::{Device, ffi, SaneError, result::{sane_try, from_status}, Parameters};


#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("failed to write scanned: {0}")]
    Write(io::Error),

    #[error("sane error: {0}")]
    Sane(SaneError),
}

impl From<SaneError> for ScannerError {
    fn from(err: SaneError) -> Self {
        Self::Sane(err)
    }
}

pub struct Scanner<'sane> {
    device: Device<'sane>,
    device_handle: *mut c_void,
}


impl<'sane> Scanner<'sane> {
    pub fn new(device: Device<'sane>) -> Result<Self, ScannerError> {
        let mut device_handle = null_mut();

        log::trace!("Call ffi::sane_open()");
        sane_try!(ffi::sane_open(device.name, &mut device_handle));

        Ok(Self {
            device,
            device_handle,
        })
    }

    pub fn start<'scanner>(&'scanner mut self) -> Result<ActiveScanner<'sane, 'scanner>, ScannerError> {
        Ok( ActiveScanner::new(self))
    }
}


impl Drop for Scanner<'_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_close()");
        unsafe { ffi::sane_close(self.device_handle) };
    }
}


pub struct ActiveScanner<'sane, 'scanner> {
    scanner: &'scanner mut Scanner<'sane>,
    started: bool,
}

impl<'sane, 'scanner> ActiveScanner<'sane, 'scanner> {
    fn new(scanner: &'scanner mut Scanner<'sane>) -> Self {
        Self {
            scanner,
            started: false,
        }
    } 

    #[rustfmt::skip]
    pub fn get_parameters(&mut self) -> Result<Parameters, ScannerError> {
        let mut params = unsafe { core::mem::zeroed() };    

        self.start_scan()?;

        log::trace!("Call ffi::sane_get_parameters()");
        sane_try!(ffi::sane_get_parameters(self.scanner.device_handle, &mut params));

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

    pub fn scan<W>(&mut self, mut writer: W, buffer_size: usize) -> Result<usize, ScannerError>
    where
        W: io::Write,
    {
        self.start_scan()?;        

        let mut total = 0;
        let mut buffer = vec![0u8; buffer_size];

        loop {
            let read_result = unsafe {
                let mut count = 0;

                log::trace!("Call ffi::sane_read()");
                from_status(ffi::sane_read(
                    self.scanner.device_handle,
                    buffer.as_mut_ptr(),
                    buffer.len().try_into().unwrap_or(i32::MAX),
                    &mut count,
                )).map(|()| count)
            };

            match read_result {
                Ok(count) => {
                    total += count as usize;
                },
                Err(SaneError::EOF) => {
                    self.started = false;
                    break;
                },
                Err(err) => {
                    return Err(ScannerError::Sane(err));
                },
            }

            writer.write_all(&buffer).map_err(ScannerError::Write)?;
        }

        Ok(total)
    }

    fn start_scan(&mut self) -> Result<(), SaneError> {
        if self.started {
            return Ok(());
        }

        log::trace!("Call ffi::sane_start()");
        sane_try!(ffi::sane_start(self.scanner.device_handle));
        
        self.started = true;
        
        Ok(())
    }
}

impl Drop for ActiveScanner<'_, '_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_cancel()");
        unsafe { ffi::sane_cancel(self.scanner.device_handle) };
    }
}