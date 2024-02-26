use anyhow::{anyhow, bail};
use cstr::cstr;
use reqwest::{blocking::get, Url};
use simple_cups::{
    options::{ColorMode, MediaFormat, Options, Orientation, PrintQuality, Sides},
    printer::{Document, DocumentType, Printer},
};

pub fn print_remote_file(printer: &str, docname: &String, url: &Url) -> anyhow::Result<()> {
    tokio::task::block_in_place(|| {
        let Some(printer) = Printer::find_by_name(printer) else {
            bail!("printer '{printer}' not found");
        };

        let mut document_reader = get(url.to_owned()).map_err(|err| {
            anyhow!(
                "failed to download document '{docname}': {}",
                err.without_url()
            )
        })?;

        // TODO: Support plaintext, images and docx.
        let document = Document {
            file_name: docname,
            ty: DocumentType::PDF,
            reader: &mut document_reader,
        };

        // TODO: Move options to the config.
        let options = Options::new()
            .media_format(MediaFormat::A4)
            .orientation(Orientation::Portrait)
            .sides(Sides::OneSide)
            .color_mode(ColorMode::Monochrome)
            .quality(PrintQuality::Normal)
            .copies(cstr!("1"));

        printer.print_documents(&docname, options, vec![document])?;

        Ok(())
    })
}
