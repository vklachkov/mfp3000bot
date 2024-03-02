use crate::scan::Jpeg;
use ::image::{codecs::jpeg::JpegDecoder, ColorType, ImageDecoder};
use printpdf::*;
use std::io;

pub struct PdfBuilder {
    doc: PdfDocumentReference,
    dpi: f32,
}

impl PdfBuilder {
    pub fn new(title: &str, dpi: f32) -> Self {
        Self {
            doc: PdfDocument::empty(title),
            dpi,
        }
    }

    pub fn add_image(&self, image: Jpeg) -> io::Result<()> {
        let jpeg_decoder = JpegDecoder::new(io::Cursor::new(&image.0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

        let dimensions = jpeg_decoder.dimensions();
        let width = Px(dimensions.0 as usize);
        let height = Px(dimensions.1 as usize);

        let (color_space, bits_per_component) = match jpeg_decoder.color_type() {
            ColorType::Rgb8 => (ColorSpace::Rgb, ColorBits::Bit8),
            ColorType::Rgba8 => (ColorSpace::Rgba, ColorBits::Bit8),
            ColorType::Rgb16 => (ColorSpace::Rgb, ColorBits::Bit16),
            ColorType::Rgba16 => (ColorSpace::Rgba, ColorBits::Bit16),
            _ => return Err(io::ErrorKind::Unsupported.into()),
        };

        let (page, layer) = self.doc.add_page(
            Mm::from(width.into_pt(self.dpi)),
            Mm::from(height.into_pt(self.dpi)),
            "Image Layer",
        );

        Image::from(ImageXObject {
            width,
            height,
            color_space,
            bits_per_component,
            interpolate: false,
            image_data: image.0,
            image_filter: Some(ImageFilter::DCT),
            smask: None,
            clipping_bbox: None,
        })
        .add_to_layer(
            self.doc.get_page(page).get_layer(layer),
            ImageTransform::default(),
        );

        Ok(())
    }

    pub fn write_to<W: io::Write>(self, w: W) -> anyhow::Result<()> {
        let mut writer = io::BufWriter::with_capacity(128 * 1024, w);
        self.doc.save(&mut writer)?;
        Ok(())
    }
}
