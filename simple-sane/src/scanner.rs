use crate::{
    ffi,
    result::{from_status, sane_try},
    Device, Parameters, SaneError,
};
use cstr::cstr;
use std::{
    ffi::{c_void, CStr},
    io,
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

        unsafe {
            //
            let mut descs = Vec::new();

            let mut n = 0;
            loop {
                let Some(desc) = ffi::sane_get_option_descriptor(device_handle, n).as_ref() else {
                    break;
                };

                descs.push(desc);
                n += 1;
            }

            for ffi::SANE_Option_Descriptor {
                name,
                title,
                desc,
                type_,
                unit,
                size,
                cap,
                constraint_type,
                constraint,
            } in descs
            {
                log::info!(
                    "Name '{name}', title '{title}', desc '{desc}', type '{ty}', unit '{unit}', size '{size}', caps '{caps}', constraint type '{constraint_type}', constraint '{constraint}'",
                    name = if name.is_null() { cstr!("unknown").to_string_lossy() } else { CStr::from_ptr(*name).to_string_lossy() },
                    title = if title.is_null() { cstr!("unknown").to_string_lossy() } else { CStr::from_ptr(*title).to_string_lossy() },
                    desc = if desc.is_null() { cstr!("unknown").to_string_lossy() } else { CStr::from_ptr(*desc).to_string_lossy() },
                    ty = match *type_ {
                        ffi::SANE_Value_Type_SANE_TYPE_BOOL => "bool",
                        ffi::SANE_Value_Type_SANE_TYPE_INT => "int",
                        ffi::SANE_Value_Type_SANE_TYPE_FIXED => "fixed",
                        ffi::SANE_Value_Type_SANE_TYPE_STRING => "string",
                        ffi::SANE_Value_Type_SANE_TYPE_BUTTON => "button",
                        ffi::SANE_Value_Type_SANE_TYPE_GROUP => "group",
                        _ => "unknown"
                    },
                    unit = match *unit {
                        ffi::SANE_Unit_SANE_UNIT_NONE => "none",
                        ffi::SANE_Unit_SANE_UNIT_PIXEL => "pixel",
                        ffi::SANE_Unit_SANE_UNIT_BIT => "bit",
                        ffi::SANE_Unit_SANE_UNIT_MM => "mm",
                        ffi::SANE_Unit_SANE_UNIT_DPI => "dpi",
                        ffi::SANE_Unit_SANE_UNIT_PERCENT => "percent",
                        ffi::SANE_Unit_SANE_UNIT_MICROSECOND => "microsecond",
                        _ => "unknown"
                    },
                    caps = {
                        let mut caps = Vec::new();

                        let cap = *cap as u32;
                        if cap & ffi::SANE_CAP_SOFT_SELECT > 0 {
                            caps.push("SOFT_SELECT");
                        }
                        if cap & ffi::SANE_CAP_HARD_SELECT > 0 {
                            caps.push("HARD_SELECT");
                        }
                        if cap & ffi::SANE_CAP_SOFT_DETECT > 0 {
                            caps.push("SOFT_DETECT");
                        }
                        if cap & ffi::SANE_CAP_EMULATED > 0 {
                            caps.push("EMULATED");
                        }
                        if cap & ffi::SANE_CAP_AUTOMATIC > 0 {
                            caps.push("AUTOMATIC");
                        }
                        if cap & ffi::SANE_CAP_INACTIVE > 0 {
                            caps.push("INACTIVE");
                        }
                        if cap & ffi::SANE_CAP_ADVANCED > 0 {
                            caps.push("ADVANCED");
                        }

                        if caps.is_empty() {
                            "no capabilities".to_owned()
                        } else {
                            caps.join(" | ")
                        }
                    },
                    constraint_type = match *constraint_type {
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_NONE => "none",
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_RANGE => "range",
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_WORD_LIST => "wordlist",
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_STRING_LIST => "stringlist",
                        _ => "unknown",
                    },
                    constraint = match *constraint_type {
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_NONE => "none".to_owned(),
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_RANGE => {
                            let c = *constraint.range;
                            format!("min {}, max {}, quant {}", c.min, c.max, c.quant)
                        },
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_WORD_LIST => {
                            "unsupported".to_owned()
                        },
                        ffi::SANE_Constraint_Type_SANE_CONSTRAINT_STRING_LIST => {
                            let mut strings = Vec::new();

                            let mut s = constraint.string_list;
                            loop {
                                if (*s).is_null() {
                                    break;
                                }

                                strings.push(CStr::from_ptr(*s).to_string_lossy());

                                s = s.add(1);
                            }

                            strings.join(", ")
                        },
                        _ => "unknown".to_owned(),
                    },
                );
            }

            //
        }

        Ok(Self {
            device,
            device_handle,
        })
    }

    pub fn start<'scanner>(&'scanner mut self) -> Result<PageReader<'sane, 'scanner>, SaneError> {
        log::trace!("Call ffi::sane_start()");
        sane_try!(ffi::sane_start(self.device_handle));

        Ok(PageReader(self))
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
