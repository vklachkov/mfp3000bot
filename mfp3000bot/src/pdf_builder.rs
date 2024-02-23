use ::image::DynamicImage;
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

    pub fn add_image(&self, image: DynamicImage) -> io::Result<()> {
        let width = Px(image.width() as usize);
        let height = Px(image.height() as usize);

        let (page, layer) = self.doc.add_page(
            Mm::from(width.into_pt(self.dpi)),
            Mm::from(height.into_pt(self.dpi)),
            "Image Layer",
        );

        let (color_space, bits_per_component) = match image {
            DynamicImage::ImageRgb8(_) => (ColorSpace::Rgb, ColorBits::Bit8),
            DynamicImage::ImageRgba8(_) => (ColorSpace::Rgba, ColorBits::Bit8),
            DynamicImage::ImageRgb16(_) => (ColorSpace::Rgb, ColorBits::Bit16),
            DynamicImage::ImageRgba16(_) => (ColorSpace::Rgba, ColorBits::Bit16),
            _ => return Err(io::ErrorKind::Unsupported.into()),
        };

        Image::from(ImageXObject {
            width,
            height,
            color_space,
            bits_per_component,
            interpolate: false,
            image_data: image.into_bytes(),
            image_filter: None,
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
