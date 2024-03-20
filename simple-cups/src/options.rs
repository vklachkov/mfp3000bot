use crate::{job::Job, result::cups_error, utils::c_enum};
use libcups_sys::*;
use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    io,
    mem::ManuallyDrop,
    ptr::{self, null_mut},
};

/// Print options.
pub struct Options {
    ptr: *mut cups_option_t,
    count: i32,
    values: OptionsValues,
}

/// Stores string representations of options.
///
/// This is necessary because all CUPS option are stored in strings
/// and must lived until job is completed.
pub struct OptionsValues {
    copies: Option<CString>,
}

impl Options {
    /// Set paper size for print.
    pub fn media_format(mut self, format: MediaFormat) -> Self {
        self.add_option(CUPS_MEDIA, format.value());
        self
    }

    /// Set print orientation.
    ///
    /// Option doesn't work for PDF documents.
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.add_option(CUPS_ORIENTATION, orientation.value());
        self
    }

    /// Set one-sided or two-sided printing mode.
    ///
    /// If printer doesn't support two sided printing, this option does nothing.
    pub fn sides(mut self, sides: Sides) -> Self {
        self.add_option(CUPS_SIDES, sides.value());
        self
    }

    /// Set printing color mode.
    ///
    /// If printer doesn't support color printing, this option does nothing.
    pub fn color_mode(mut self, mode: ColorMode) -> Self {
        self.add_option(CUPS_PRINT_COLOR_MODE, mode.value());
        self
    }

    /// Set print quality.
    pub fn quality(mut self, quality: PrintQuality) -> Self {
        self.add_option(CUPS_PRINT_QUALITY, quality.value());
        self
    }

    fn add_option(&mut self, name: &CStr, value: &CStr) {
        self.count =
            unsafe { cupsAddOption(name.as_ptr(), value.as_ptr(), self.count, &mut self.ptr) };
    }

    /// Set number of copies.
    pub fn copies(mut self, copies: usize) -> Self {
        let copies = self.values.copies.insert(unsafe {
            CString::from_vec_with_nul_unchecked(format!("{copies}\0").into_bytes())
        });

        self.count = unsafe {
            cupsAddOption(
                CUPS_COPIES.as_ptr().cast(),
                copies.as_ptr(),
                self.count,
                &mut self.ptr,
            )
        };

        self
    }

    /// Create a new job on CUPS server.
    ///
    /// # Error
    ///
    /// Return error if [`cupsCreateJob`] returns zero.
    pub fn create_job(self, device_name: &CStr, title: &CStr) -> io::Result<Job> {
        let (inner, count, values) = unsafe {
            let this = ManuallyDrop::new(self);
            (this.ptr, this.count, ptr::read(&this.values))
        };

        let id = unsafe {
            cupsCreateJob(
                null_mut(),
                device_name.as_ptr(),
                title.as_ptr(),
                count,
                inner,
            )
        };

        if id == 0 {
            return Err(io::Error::other(cups_error().unwrap()));
        }

        Ok(Job {
            id,
            options: values,
        })
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            ptr: null_mut(),
            count: 0,
            values: OptionsValues { copies: None },
        }
    }
}

impl Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();

        for i in 0..self.count {
            let option = unsafe { self.ptr.add(i as usize) };

            let name = unsafe { CStr::from_ptr((*option).name) };
            let value = unsafe { CStr::from_ptr((*option).value) };

            debug_map.entry(&name, &value);
        }

        debug_map.finish()
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe { cupsFreeOptions(self.count, self.ptr) }
    }
}

c_enum! {
    pub enum MediaFormat {
        F3X5: CUPS_MEDIA_3X5,
        F4X6: CUPS_MEDIA_4X6,
        F5X7: CUPS_MEDIA_5X7,
        F8X10: CUPS_MEDIA_8X10,
        A3: CUPS_MEDIA_A3,
        A4: CUPS_MEDIA_A4,
        A5: CUPS_MEDIA_A5,
        A6: CUPS_MEDIA_A6,
        Env10: CUPS_MEDIA_ENV10,
        EnvDl: CUPS_MEDIA_ENVDL,
        Legal: CUPS_MEDIA_LEGAL,
        Letter: CUPS_MEDIA_LETTER,
        PhotoL: CUPS_MEDIA_PHOTO_L,
        SuperBa3: CUPS_MEDIA_SUPERBA3,
        Tabloid: CUPS_MEDIA_TABLOID,
    }
}

c_enum! {
    pub enum Orientation {
        Portrait: CUPS_ORIENTATION_PORTRAIT,
        Landscape: CUPS_ORIENTATION_LANDSCAPE,
    }
}

c_enum! {
    pub enum Sides {
        OneSide: CUPS_SIDES_ONE_SIDED,
        TwoSidedPortrait: CUPS_SIDES_TWO_SIDED_PORTRAIT,
        TwoSidedLandscape: CUPS_SIDES_TWO_SIDED_LANDSCAPE,
    }
}

c_enum! {
    pub enum ColorMode {
        Auto: CUPS_PRINT_COLOR_MODE_AUTO,
        Color: CUPS_PRINT_COLOR_MODE_COLOR,
        Monochrome: CUPS_PRINT_COLOR_MODE_MONOCHROME,
    }
}

c_enum! {
    pub enum PrintQuality {
        Draft: CUPS_PRINT_QUALITY_DRAFT,
        Normal: CUPS_PRINT_QUALITY_NORMAL,
        High: CUPS_PRINT_QUALITY_HIGH,
    }
}
