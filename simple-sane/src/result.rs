use crate::ffi;
use thiserror::Error;

pub type Result<T> = ::core::result::Result<T, SaneError>;

#[derive(Debug, Clone, Copy, Error)]
pub enum SaneError {
    #[error("unsupported")]
    Unsupported,

    #[error("cancelled")]
    Cancelled,

    #[error("device busy")]
    DeviceBusy,

    #[error("invalid value")]
    Inval,

    #[error("end of file")]
    EOF,

    #[error("document feeder is jammed")]
    Jammed,

    #[error("document feeder is out of documents")]
    NoDocs,

    #[error("cover open")]
    CoverOpen,

    #[error("error occurred while communicating with the device")]
    IO,

    #[error("not enough memory")]
    NoMem,

    #[error("access denied")]
    AccessDenied,
}

macro_rules! sane_try {
    ($x:expr) => {
        crate::result::from_status(unsafe { $x })?;
    };
}

pub(crate) use sane_try;

pub fn from_status(status: ffi::SANE_Status) -> Result<()> {
    match status {
        ffi::SANE_Status_SANE_STATUS_GOOD => Ok(()),
        ffi::SANE_Status_SANE_STATUS_UNSUPPORTED => Err(SaneError::Unsupported),
        ffi::SANE_Status_SANE_STATUS_CANCELLED => Err(SaneError::Cancelled),
        ffi::SANE_Status_SANE_STATUS_DEVICE_BUSY => Err(SaneError::DeviceBusy),
        ffi::SANE_Status_SANE_STATUS_INVAL => Err(SaneError::Inval),
        ffi::SANE_Status_SANE_STATUS_EOF => Err(SaneError::EOF),
        ffi::SANE_Status_SANE_STATUS_JAMMED => Err(SaneError::Jammed),
        ffi::SANE_Status_SANE_STATUS_NO_DOCS => Err(SaneError::NoDocs),
        ffi::SANE_Status_SANE_STATUS_COVER_OPEN => Err(SaneError::CoverOpen),
        ffi::SANE_Status_SANE_STATUS_IO_ERROR => Err(SaneError::IO),
        ffi::SANE_Status_SANE_STATUS_NO_MEM => Err(SaneError::NoMem),
        ffi::SANE_Status_SANE_STATUS_ACCESS_DENIED => Err(SaneError::AccessDenied),
        _ => panic!("invalid status value {status}"),
    }
}
