#![allow(unused)]

mod ffi;
mod options;
mod printer;
mod result;

use cstr::cstr;
use options::{ColorMode, MediaFormat, Options, Orientation, PrintQuality, Sides};
use printer::{Document, DocumentType, Printer, PrinterError};
use std::ffi::OsStr;
use std::path::Path;
use std::{fs, io};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrintError {
    #[error("no default printer")]
    NoDefaultPrinter,

    #[error("failed to read file: {0}")]
    File(io::Error),

    #[error("print error: {0}")]
    Printer(PrinterError),
}

pub fn print_pdf<P: AsRef<Path>>(path: P) -> Result<(), PrintError> {
    let path = path.as_ref();
    let file_name = path.file_name().and_then(OsStr::to_str).unwrap();

    let Some(printer) = Printer::get_default() else {
        return Err(PrintError::NoDefaultPrinter);
    };

    println!("Use printer '{}'", printer.name().to_string_lossy());

    let options = Options::new()
        .media_format(MediaFormat::A4)
        .orientation(Orientation::Portrait)
        .sides(Sides::OneSide)
        .color_mode(ColorMode::Monochrome)
        .quality(PrintQuality::Normal)
        .copies(cstr!("1"));

    println!("Print with options:\n{options}");

    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(PrintError::File)?;

    printer.print_documents(
        "Job Title",
        options,
        vec![Document {
            file_name,
            ty: DocumentType::PDF,
            reader: &mut file,
        }],
    );

    println!("File '{path}' printed successfully!", path = path.display());

    Ok(())
}
