use super::ffi;
use core::ffi::c_int;
use std::{
    ffi::{CStr, CString},
    fmt::Display,
    mem::ManuallyDrop,
    ptr::null_mut,
};

pub struct Options {
    inner: *mut ffi::cups_option_t,
    count: c_int,
}

impl Options {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn media_format(mut self, format: MediaFormat) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_MEDIA), format.value());
        self
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_ORIENTATION), orientation.value());
        self
    }

    pub fn sides(mut self, sides: Sides) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_SIDES), sides.value());
        self
    }

    pub fn color_mode(mut self, mode: ColorMode) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_PRINT_COLOR_MODE), mode.value());
        self
    }

    pub fn quality(mut self, quality: PrintQuality) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_PRINT_QUALITY), quality.value());
        self
    }

    pub fn copies(mut self, copies: &'static CStr) -> Self {
        self.add_option(slice_to_cstr(ffi::CUPS_COPIES), copies);
        self
    }

    fn add_option(&mut self, name: &CStr, value: &CStr) {
        self.count = unsafe {
            ffi::cupsAddOption(name.as_ptr(), value.as_ptr(), self.count, &mut self.inner)
        };
    }

    pub fn slice(&self) -> &[ffi::cups_option_t] {
        if self.count == 0 {
            &[]
        } else {
            assert!(!self.inner.is_null());
            assert!(self.count > 0);
            unsafe { core::slice::from_raw_parts(self.inner, self.count as usize) }
        }
    }

    pub fn into_raw(self) -> (*mut ffi::cups_option_t, c_int) {
        let this = ManuallyDrop::new(self);
        (this.inner, this.count)
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            inner: null_mut(),
            count: 0,
        }
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe { ffi::cupsFreeOptions(self.count, self.inner) }
    }
}

impl Display for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice = self.slice();
        if slice.is_empty() {
            return Ok(());
        }

        let last_index = slice.len() - 1;
        for (idx, option) in slice.iter().enumerate() {
            write!(
                f,
                "{name} = {value}{separator}",
                name = unsafe { CStr::from_ptr(option.name).to_string_lossy() },
                value = unsafe { CStr::from_ptr(option.value).to_string_lossy() },
                separator = if idx != last_index { ", " } else { "" },
            )?;
        }

        Ok(())
    }
}

pub enum MediaFormat {
    F3X5,
    F4X6,
    F5X7,
    F8X10,
    A3,
    A4,
    A5,
    A6,
    Env10,
    EnvDl,
    Legal,
    Letter,
    PhotoL,
    SuperBa3,
    Tabloid,
}

impl MediaFormat {
    pub fn value(self) -> &'static CStr {
        let cups_value: &[u8] = match self {
            Self::F3X5 => ffi::CUPS_MEDIA_3X5,
            Self::F4X6 => ffi::CUPS_MEDIA_4X6,
            Self::F5X7 => ffi::CUPS_MEDIA_5X7,
            Self::F8X10 => ffi::CUPS_MEDIA_8X10,
            Self::A3 => ffi::CUPS_MEDIA_A3,
            Self::A4 => ffi::CUPS_MEDIA_A4,
            Self::A5 => ffi::CUPS_MEDIA_A5,
            Self::A6 => ffi::CUPS_MEDIA_A6,
            Self::Env10 => ffi::CUPS_MEDIA_ENV10,
            Self::EnvDl => ffi::CUPS_MEDIA_ENVDL,
            Self::Legal => ffi::CUPS_MEDIA_LEGAL,
            Self::Letter => ffi::CUPS_MEDIA_LETTER,
            Self::PhotoL => ffi::CUPS_MEDIA_PHOTO_L,
            Self::SuperBa3 => ffi::CUPS_MEDIA_SUPERBA3,
            Self::Tabloid => ffi::CUPS_MEDIA_TABLOID,
        };

        slice_to_cstr(cups_value)
    }
}

pub enum Orientation {
    Portrait,
    Landscape,
}

impl Orientation {
    pub fn value(self) -> &'static CStr {
        let cups_value: &[u8] = match self {
            Self::Portrait => ffi::CUPS_ORIENTATION_PORTRAIT,
            Self::Landscape => ffi::CUPS_ORIENTATION_LANDSCAPE,
        };

        slice_to_cstr(cups_value)
    }
}

pub enum Sides {
    OneSide,
    TwoSidedPortrait,
    TwoSidedLandscape,
}

impl Sides {
    pub fn value(self) -> &'static CStr {
        let cups_value: &[u8] = match self {
            Self::OneSide => ffi::CUPS_SIDES_ONE_SIDED,
            Self::TwoSidedPortrait => ffi::CUPS_SIDES_TWO_SIDED_PORTRAIT,
            Self::TwoSidedLandscape => ffi::CUPS_SIDES_TWO_SIDED_LANDSCAPE,
        };

        slice_to_cstr(cups_value)
    }
}

pub enum ColorMode {
    Auto,
    Color,
    Monochrome,
}

impl ColorMode {
    pub fn value(self) -> &'static CStr {
        let cups_value: &[u8] = match self {
            Self::Auto => ffi::CUPS_PRINT_COLOR_MODE_AUTO,
            Self::Color => ffi::CUPS_PRINT_COLOR_MODE_COLOR,
            Self::Monochrome => ffi::CUPS_PRINT_COLOR_MODE_MONOCHROME,
        };

        slice_to_cstr(cups_value)
    }
}

pub enum PrintQuality {
    Draft,
    Normal,
    High,
}

impl PrintQuality {
    pub fn value(self) -> &'static CStr {
        let cups_value: &[u8] = match self {
            Self::Draft => ffi::CUPS_PRINT_QUALITY_DRAFT,
            Self::Normal => ffi::CUPS_PRINT_QUALITY_NORMAL,
            Self::High => ffi::CUPS_PRINT_QUALITY_HIGH,
        };

        slice_to_cstr(cups_value)
    }
}

/// Converts null-ended slice to CStr.
///
/// Bindgen doesn't support CStr, so we need to do the cast manually.
fn slice_to_cstr(slice: &[u8]) -> &CStr {
    assert!(!slice.is_empty());
    assert_eq!(slice[slice.len() - 1], 0);

    unsafe { CStr::from_ptr(slice.as_ptr().cast()) }
}
