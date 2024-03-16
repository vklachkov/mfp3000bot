use crate::scan::{Jpeg, JpegFormat};
use printpdf::*;
use std::io;

pub struct PdfBuilder {
    doc: PdfDocumentReference,
    dpi: f32,
}

impl PdfBuilder {
    pub fn new(title: &str, dpi: f32) -> Self {
        Self {
            doc: PdfDocument::empty(title).with_conformance(PdfConformance::Custom(
                CustomPdfConformance {
                    // ICC profile bloats file:
                    // https://github.com/fschutt/printpdf/issues/174#issuecomment-2000091741
                    requires_icc_profile: false,
                    requires_xmp_metadata: false,
                    ..Default::default()
                },
            )),
            dpi,
        }
    }

    pub fn add_page(&self, jpeg: Jpeg) -> io::Result<()> {
        let width = Px(jpeg.width);
        let height = Px(jpeg.height);

        let bits_per_component = ColorBits::Bit8;
        let color_space = match jpeg.format {
            JpegFormat::Rgb => ColorSpace::Rgb,
            JpegFormat::Gray => ColorSpace::Greyscale,
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
            image_data: jpeg.bytes,
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
