pub fn cups_error() -> Option<String> {
    let error = unsafe { libcups_sys::cupsLastErrorString().as_ref() }?;

    let error = unsafe { core::ffi::CStr::from_ptr(error as *const core::ffi::c_char) }
        .to_string_lossy()
        .into_owned();

    Some(error)
}
