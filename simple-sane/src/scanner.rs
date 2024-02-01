use std::{
    ffi::CStr,
    fmt::Display,
    ptr::{null, null_mut},
};

use crate::ffi;

macro_rules! sane_try {
    ($x:expr) => {{
        let status = $x;
        if status != ffi::SANE_Status_SANE_STATUS_GOOD {
            return status;
        }
    }};
}

pub struct Scanner {}

impl Scanner {
    pub unsafe fn experiments() -> ffi::SANE_Status {
        // Setup SANE
        sane_try!(ffi::sane_init(null_mut(), None));

        // Get device
        let mut device_list = null_mut();
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        if device_list.is_null() {
            println!("No devices!");
            return ffi::SANE_Status_SANE_STATUS_GOOD;
        }

        Self::print_devices(device_list);

        // Open first device. Just for tests
        let device = *device_list;
        let mut device_handle = null_mut();

        sane_try!(ffi::sane_open((*device).name, &mut device_handle));

        // TODO: Setup scan options

        let mut parameters = core::mem::zeroed();
        sane_try!(ffi::sane_get_parameters(device_handle, &mut parameters));

        println!("Parameters:");
        println!("format: {}", parameters.format);
        println!("last_frame: {}", parameters.last_frame);
        println!("bytes_per_line: {}", parameters.bytes_per_line);
        println!("pixels_per_line: {}", parameters.pixels_per_line);
        println!("lines: {}", parameters.lines);
        println!("depth: {}", parameters.depth);

        // Start scan
        sane_try!(ffi::sane_start(device_handle));

        let mut image = Vec::with_capacity(10 * 1024);
        loop {
            let mut buffer = [0; 1024];
            let mut read = 0;

            // Read until EOF
            let status = ffi::sane_read(
                device_handle,
                buffer.as_mut_ptr(),
                buffer.len().try_into().unwrap_or(i32::MAX),
                &mut read,
            );

            if status == ffi::SANE_Status_SANE_STATUS_EOF {
                break;
            }

            if status != ffi::SANE_Status_SANE_STATUS_GOOD {
                return status;
            }

            image.extend(buffer);
        }

        std::fs::write("test.bin", image).unwrap();

        // Cleanup, stop SANE
        ffi::sane_exit();

        ffi::SANE_Status_SANE_STATUS_GOOD
    }

    unsafe fn print_devices(device_list: *mut *const ffi::SANE_Device) {
        let mut device = device_list;
        loop {
            if (*device).is_null() {
                break;
            }

            println!(
                "Scanner name '{name}', model '{model}', vendor '{vendor}', type '{ty}'",
                name = CStr::from_ptr((**device).name).to_string_lossy(),
                model = CStr::from_ptr((**device).model).to_string_lossy(),
                vendor = CStr::from_ptr((**device).vendor).to_string_lossy(),
                ty = CStr::from_ptr((**device).type_).to_string_lossy(),
            );

            device = device.add(1);
        }
    }
}
