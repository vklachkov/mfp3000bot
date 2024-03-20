macro_rules! cstring_wrapper {
    ($visibility:vis $name:ident) => {
        #[derive(Clone)]
        $visibility struct $name(::std::ffi::CString);

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
    };
}

pub(crate) use cstring_wrapper;

macro_rules! c_enum {
    ($visibility:vis enum $enum_name:ident { $($n:ident: $v:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug)]
        $visibility enum $enum_name {
            $($n),*
        }

        impl $enum_name {
            pub fn value(self) -> &'static ::std::ffi::CStr {
                match self {
                    $( Self::$n => $v ),*
                }
            }
        }
    };
}

pub(crate) use c_enum;
