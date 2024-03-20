/// Generate a new type over a `CString` with specified name.
///
/// Type implements [`std::fmt::Debug`] and [`std::ops::Deref`] to `&CString`.
///
/// Syntax: pub Ident. Visibility is optional.
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

/// Generate new enum with a `value()` method that returns a c-like values.
///
/// The generated enum implements [`std::clone::Clone`], [`std::marker::Copy`],
/// [`std::fmt::Debug`], [`serde::Serialize`], [`serde::Deserialize`].
macro_rules! c_enum {
    ($visibility:vis enum $enum_name:ident { $($n:ident: $v:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
