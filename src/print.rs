use crate::cups::options::{ColorMode, MediaFormat, Options, Orientation, PrintQuality, Sides};
use crate::cups::printer::{Document, DocumentType, Printer, PrinterError};
use cstr::cstr;
use reqwest::{blocking::get, Url};
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error)]
pub enum PrintError {
    #[error("failed to get remote file info: {0}")]
    RemoteFile(::reqwest::Error),

    #[error("no default printer")]
    NoDefaultPrinter,

    #[error("print error: {0}")]
    Printer(PrinterError),
}

pub fn print_remote_file(name: String, url: Url, result: oneshot::Sender<Result<(), PrintError>>) {
    std::thread::spawn(move || {
        _ = result.send(_print_remote_file(name, url));
    });
}

fn _print_remote_file(file_name: String, file_url: Url) -> Result<(), PrintError> {
    let printer = Printer::get_default().ok_or(PrintError::NoDefaultPrinter)?;
    let printer_name = printer.name().to_string_lossy();

    let document = Document {
        file_name: &file_name,
        ty: DocumentType::PDF,
        reader: &mut get(file_url.clone()).map_err(PrintError::RemoteFile)?,
    };

    let options = Options::new()
        .media_format(MediaFormat::A4)
        .orientation(Orientation::Portrait)
        .sides(Sides::OneSide)
        .color_mode(ColorMode::Monochrome)
        .quality(PrintQuality::Normal)
        .copies(cstr!("1"));

    log::info!(
        "Print file '{file_name}' from url '{file_url}' on printer '{printer_name}' with options {options}"
    );

    printer
        .print_documents(&file_name, options, vec![document])
        .map_err(PrintError::Printer)
}
