use bstr::BStr;
use std::ffi::{c_char, CStr};

pub unsafe fn cstr2bstr<'a>(str: *const c_char) -> Option<&'a BStr> {
    str.as_ref()
        .map(|cstr| CStr::from_ptr(cstr).to_bytes().into())
}

pub unsafe fn slice_from_c_array<'a, T>(ptr: *const *const T) -> &'a [&'a T]
where
    T: Sized + 'static,
{
    assert!(ptr.is_null() == false);

    for i in 0..usize::MAX {
        let cursor = ptr.add(i);
        if (*cursor).is_null() {
            let slice: &'a [*const T] = core::slice::from_raw_parts(ptr, i);
            let slice: &'a [&'a T] = std::mem::transmute(slice);
            return slice;
        }
    }

    unreachable!()
}
