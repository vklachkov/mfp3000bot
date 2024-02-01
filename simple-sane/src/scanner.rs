use std::{
    ffi::{c_void, CStr}, fmt::Display, io, marker::PhantomData, mem::ManuallyDrop, num::NonZeroUsize, ops::Deref, ptr::{null, null_mut}
};

use thiserror::Error;

use crate::ffi;

/////////////////////////////////

macro_rules! sane_try {
    ($x:expr) => {
        sane_status_to_result(unsafe { $x })?;
    };
}

#[derive(Debug, Clone, Copy)]
pub enum SaneError {
    Unsupported,
    Cancelled,
    DeviceBusy,
    Inval,
    EOF,
    Jammed,
    NoDocs,
    CoverOpen,
    IoError,
    NoMem,
    AccessDenied,
}

impl Display for SaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaneError::Unsupported => write!(f, "unsupported"),
            SaneError::Cancelled => write!(f, "cancelled"),
            SaneError::DeviceBusy => write!(f, "device busy"),
            SaneError::Inval => write!(f, "invalid value"),
            SaneError::EOF => write!(f, "end of file"),
            SaneError::Jammed => write!(f, "jammed"),
            SaneError::NoDocs => write!(f, "no docs"),
            SaneError::CoverOpen => write!(f, "cover ppen"),
            SaneError::IoError => write!(f, "io error"),
            SaneError::NoMem => write!(f, "no memory"),
            SaneError::AccessDenied => write!(f, "access denied"),
        }
    }
}

fn sane_status_to_result(status: ffi::SANE_Status) -> Result<(), SaneError> {
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
        ffi::SANE_Status_SANE_STATUS_IO_ERROR => Err(SaneError::IoError),
        ffi::SANE_Status_SANE_STATUS_NO_MEM => Err(SaneError::NoMem),
        ffi::SANE_Status_SANE_STATUS_ACCESS_DENIED => Err(SaneError::AccessDenied),
        _ => panic!("Invalid status value {status}"),
    }
}

struct Sane(PhantomData<()>);

impl Sane {
    pub fn new() -> Result<Self, SaneError> {
        log::trace!("Call ffi::sane_init()");
        sane_try!(ffi::sane_init(null_mut(), None));

        Ok(Self(PhantomData))
    }
}

impl Drop for Sane {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_exit()");
        unsafe { ffi::sane_exit() };
    }
}

struct Device<'sane> {
    sane: &'sane Sane,
    device: &'sane ffi::SANE_Device,
}

impl<'s> Device<'s> {
    fn get_first(sane: &'s Sane) -> Result<Option<Self>, SaneError> {
        let mut device_list = null_mut();

        log::trace!("Call ffi::sane_get_devices()");
        sane_try!(ffi::sane_get_devices(&mut device_list, 0));

        let device = unsafe { (*device_list).as_ref::<'s>() };
        if let Some(device) = device {
            Ok(Some(Device { sane, device }))
        } else {
            Ok(None)
        }
    }
}

impl Display for Device<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "'{name}' (vendor '{vendor}', model '{model}')",
                name = CStr::from_ptr(self.device.name).to_string_lossy(),
                model = CStr::from_ptr(self.device.model).to_string_lossy(),
                vendor = CStr::from_ptr(self.device.vendor).to_string_lossy(),
            )
        }
    }
}

impl Deref for Device<'_> {
    type Target = ffi::SANE_Device;

    fn deref(&self) -> &Self::Target {
        self.device
    }
}

struct Scanner<'sane> {
    device: Device<'sane>,
    device_handle: *mut c_void,
}

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("failed to write scanned: {0}")]
    Write(io::Error),

    #[error("sane error: {0}")]
    Sane(SaneError),
}

impl From<SaneError> for ScannerError {
    fn from(err: SaneError) -> Self {
        Self::Sane(err)
    }
}

impl<'sane> Scanner<'sane> {
    pub fn new(device: Device<'sane>) -> Result<Self, ScannerError> {
        let mut device_handle = null_mut();

        log::trace!("Call ffi::sane_open()");
        sane_try!(ffi::sane_open(device.name, &mut device_handle));

        Ok(Self {
            device,
            device_handle,
        })
    }

    pub fn start<'scanner>(&'scanner mut self) -> Result<ActiveScanner<'sane, 'scanner>, ScannerError> {
        Ok( ActiveScanner::new(self))
    }
}


impl Drop for Scanner<'_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_close()");
        unsafe { ffi::sane_close(self.device_handle) };
    }
}


struct ActiveScanner<'sane, 'scanner> {
    scanner: &'scanner mut Scanner<'sane>,
    started: bool,
}

impl<'sane, 'scanner> ActiveScanner<'sane, 'scanner> {
    fn new(scanner: &'scanner mut Scanner<'sane>) -> Self {
        Self {
            scanner,
            started: false,
        }
    } 

    #[rustfmt::skip]
    fn get_parameters(&mut self) -> Result<Parameters, ScannerError> {
        let mut params = unsafe { core::mem::zeroed() };    

        self.start_scan()?;

        log::trace!("Call ffi::sane_get_parameters()");
        sane_try!(ffi::sane_get_parameters(self.scanner.device_handle, &mut params));

        Ok(Parameters {
            format: params.format.into(),
            last_frame: {
                assert!([0, 1].contains(&params.last_frame));
                params.last_frame == 1
            },
            bytes_per_line:  {
                assert!(params.bytes_per_line > 0, "bytes_per_line should be greater than 0");
                params.bytes_per_line as usize
            },
            pixels_per_line:  {
                assert!(params.pixels_per_line > 0, "pixels_per_line should be greater than 0");
                params.pixels_per_line as usize
            },
            lines: {
                assert!(params.lines > 0, "lines should be greater than 0");
                params.lines as usize
            },
            depth: {
                assert!(params.depth > 0, "depth should be greater than 0");
                params.depth as usize
            },
        })
    }

    fn scan<W>(&mut self, mut writer: W, buffer_size: usize) -> Result<usize, ScannerError>
    where
        W: io::Write,
    {
        self.start_scan()?;        

        let mut total = 0;
        let mut buffer = vec![0u8; buffer_size];

        loop {
            let mut count = 0;
            let read_result = unsafe {
                log::trace!("Call ffi::sane_read()");
                sane_status_to_result(ffi::sane_read(
                    self.scanner.device_handle,
                    buffer.as_mut_ptr(),
                    buffer.len().try_into().unwrap_or(i32::MAX),
                    &mut count,
                ))
            };

            match read_result {
                Ok(()) => total += count as usize,
                Err(SaneError::EOF) => break,
                Err(err) => return Err(ScannerError::Sane(err)),
            }

            writer.write_all(&buffer).map_err(ScannerError::Write)?;
        }

        self.started = false;

        Ok(total)
    }

    fn start_scan(&mut self) -> Result<(), SaneError> {
        if self.started {
            return Ok(());
        }

        log::trace!("Call ffi::sane_start()");
        sane_try!(ffi::sane_start(self.scanner.device_handle));
        
        self.started = true;
        
        Ok(())
    }
}

impl Drop for ActiveScanner<'_, '_> {
    fn drop(&mut self) {
        log::trace!("Call ffi::sane_cancel()");
        unsafe { ffi::sane_cancel(self.scanner.device_handle) };
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    format: FrameFormat,
    last_frame: bool,
    bytes_per_line: usize,
    pixels_per_line: usize,
    lines: usize,
    depth: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameFormat {
    Gray,
    RGB,
    Red,
    Green,
    Blue,
}

impl Display for FrameFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameFormat::Gray => write!(f, "gray"),
            FrameFormat::RGB => write!(f, "RGB"),
            FrameFormat::Red => write!(f, "red"),
            FrameFormat::Green => write!(f, "green"),
            FrameFormat::Blue => write!(f, "blue"),
        }
    }
}

impl From<ffi::SANE_Frame> for FrameFormat {
    fn from(value: ffi::SANE_Frame) -> Self {
        match value {
            ffi::SANE_Frame_SANE_FRAME_GRAY => Self::Gray,
            ffi::SANE_Frame_SANE_FRAME_RGB => Self::RGB,
            ffi::SANE_Frame_SANE_FRAME_RED => Self::Red,
            ffi::SANE_Frame_SANE_FRAME_GREEN => Self::Green,
            ffi::SANE_Frame_SANE_FRAME_BLUE => Self::Blue,
            _ => panic!("invalid sane frame format {value}"),
        }
    }
}

impl Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "resolution {width}x{height}, depth {depth}, format '{format}' ({bytes} per pixel)",
            width = self.pixels_per_line,
            height = self.lines,
            depth = self.depth,
            format = self.format,
            bytes = self.bytes_per_line / self.pixels_per_line,
        )
    }
}

/////////////////////////////////

pub fn experiments() -> Result<(), ScannerError> {
    let sane = Sane::new()?;

    let Some(device) = Device::get_first(&sane)? else {
        panic!("No scanners!");
    };
    
    // println!("Use device {device}");

    let mut scaaaaanner = Scanner::new(device)?;

    let test_file_path = "test.bin";

    // println!("Opening file '{test_file_path}'...");
    let file1 = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&test_file_path)
        .unwrap();

        let file2 = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&test_file_path)
        .unwrap();

    // println!("Scan page into file '{test_file_path}'...");

    let mut scanner = scaaaaanner.start()?;

    let parameters = scanner.get_parameters()?;
    println!("Use parameters {parameters}");

    scanner.scan(file1, 128 * 1024)?;
    scanner.scan(file2, 128 * 1024)?;

    drop(scanner);

    let mut scanner = scaaaaanner.start()?;

    drop(scanner);

    let mut scanner = scaaaaanner.start()?;

    let file = std::fs::OpenOptions::new()
    .create(true)
    .write(true)
    .open(&test_file_path)
    .unwrap();

    scanner.scan(file, 128 * 1024)?;

    // println!("Successfully scan page and save it into file '{test_file_path}'!");

    Ok(())
}
