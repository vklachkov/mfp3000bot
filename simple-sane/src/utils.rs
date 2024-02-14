use bstr::BStr;
use std::ffi::CStr;

pub unsafe fn cstr2bstr<'a>(str: *const std::ffi::c_char) -> Option<&'a BStr> {
    str.as_ref()
        .map(|cstr| CStr::from_ptr(cstr).to_bytes().into())
}
