use crate::config;
use anyhow::{anyhow, bail, Context};
use libcups::{
    document::{Document, DocumentName, DocumentType},
    options::Options,
    printer::{DeviceName, JobTitle, Printer},
};
use reqwest::{
    blocking::{get, Client},
    Url,
};
use std::{
    io::{self, Read},
    time::Instant,
};

pub fn print_remote_file(
    printer: &str,
    docname: &String,
    url: &Url,
    config: &config::Print,
) -> anyhow::Result<()> {
    tokio::task::block_in_place(|| {
        let Some(printer) = Printer::find_by_name(DeviceName::new(printer).unwrap()) else {
            bail!("printer '{printer}' not found");
        };

        let document_reader = get(url.to_owned()).map_err(|err| {
            anyhow!(
                "failed to download document '{docname}': {}",
                err.without_url()
            )
        })?;

        // TODO: Support images.
        let (ty, mut reader): (_, Box<dyn io::Read>) = if docname.ends_with(".txt") {
            (DocumentType::PlainText, Box::new(document_reader))
        } else if docname.ends_with(".pdf") {
            (DocumentType::Pdf, Box::new(document_reader))
        } else if docname.ends_with(".docx") {
            let pdf = docx_to_pdf(document_reader)?;
            (DocumentType::Pdf, Box::new(io::Cursor::new(pdf)))
        } else {
            bail!("Unsupported file format");
        };

        let document = Document::new(DocumentName::new(docname).unwrap(), ty, &mut reader);

        let options = Options::default()
            .media_format(config.paper_size)
            .orientation(config.orientation)
            .sides(config.sides)
            .color_mode(config.color_mode)
            .quality(config.quality)
            .copies(config.copies);

        printer.print_documents(JobTitle::new(docname).unwrap(), options, vec![document])?;

        Ok(())
    })
}

fn docx_to_pdf(mut reader: impl io::Read) -> anyhow::Result<Vec<u8>> {
    use base64::{engine::general_purpose, read::DecoderReader, write::EncoderStringWriter};
    use iter_read::IterRead;

    let mut docx_base64_encoder = EncoderStringWriter::new(&general_purpose::STANDARD);
    io::copy(&mut reader, &mut docx_base64_encoder)
        .context("copying data from pdf to base64 encoder")?;

    let request = format!(
        r#"<?xml version="1.0"?>
           <methodCall>
             <!-- Method description here: https://github.com/unoconv/unoserver/blob/ecaea93bced40ab7e544eb4e2a89bd2d13b4788d/src/unoserver/converter.py#L142 -->
             <methodName>convert</methodName>
             <params>
               <!-- inpath  --> <param><value><nil/></value></param>
               <!-- indata  --> <param><value><base64>{document}</base64></value></param>
               <!-- outpath --> <param><value><nil/></value></param>
               <!-- format  --> <param><value><string>pdf</string></value></param>
             </params>
           </methodCall>
    "#,
        document = docx_base64_encoder.into_inner()
    );

    // TODO: Move unoserver url to the config
    // TODO: Use bytes() instead of text()
    let pdf_response = Client::new()
        .post("http://localhost:2003/")
        .body(request)
        .send()
        .context("sending request to unoserver")?
        .text()
        .context("reading response from unoserver")?;

    let base64_open_tag = "<base64>";
    let base64_close_tag = "</base64>";

    let pdf_base64_start_idx = pdf_response
        .find(base64_open_tag)
        .map(|p| p + base64_open_tag.len())
        .ok_or_else(|| {
            anyhow!("failed to find <base64> in unoserver response: '{pdf_response}'")
        })?;

    let pdf_base64_end_idx = pdf_response.rfind(base64_close_tag).ok_or_else(|| {
        anyhow!("failed to find </base64> in unoserver response: '{pdf_response}'")
    })?;

    let instant = Instant::now();

    // It seems to me that this code should not panic,
    // because the unoserver response will always only contain ascii characters?..
    let pdf_base64 = &pdf_response.as_bytes()[pdf_base64_start_idx..pdf_base64_end_idx];

    // TODO: Is it fast? Check and, maybe, write custom iter read with filter based on memchr.
    let pdf_base64_reader = IterRead::new(pdf_base64.iter().copied().filter(|&b| b != 10));

    let mut pdf = Vec::new();
    DecoderReader::new(pdf_base64_reader, &general_purpose::STANDARD)
        .read_to_end(&mut pdf)
        .context("decoding base64 encoded pdf from unoserver")?;

    log::debug!("Time spent decoding base64: {:?}", instant.elapsed());

    Ok(pdf)
}
