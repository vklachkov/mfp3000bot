macro_rules! cstring_wrapper {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name(::std::ffi::CString);

            impl $name {
                pub fn new(name: &str) -> Option<Self> {
                    let name = ::std::ffi::CString::new(name).ok()?;
                    Some(Self(name))
                }
            }

            impl ::std::ops::Deref for $name {
                type Target = ::std::ffi::CString;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )*
    };
}

pub(crate) use cstring_wrapper;

pub fn cups_error() -> Option<String> {
    let error = unsafe { crate::ffi::cupsLastErrorString().as_ref() }?;

    let error = unsafe { core::ffi::CStr::from_ptr(error as *const i8) }
        .to_string_lossy()
        .into_owned();

    Some(error)
}
