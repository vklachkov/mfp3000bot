use crate::scan::{Jpeg, JpegFormat};
use printpdf::*;
use std::{collections::BTreeMap, io};

pub struct PdfBuilder {
    doc: PdfDocument,
    dpi: f32,
    pages: Vec<PdfPage>,
}

impl PdfBuilder {
    pub fn new(title: &str, dpi: f32) -> Self {
        let mut doc = PdfDocument::new(title);
        let pages = Vec::with_capacity(3);

        // ICC profile bloats file:
        // https://github.com/fschutt/printpdf/issues/174#issuecomment-2000091741
        doc.metadata.info.conformance = PdfConformance::Custom(CustomPdfConformance {
            requires_icc_profile: false,
            requires_xmp_metadata: false,
            ..Default::default()
        });

        Self { doc, dpi, pages }
    }

    pub fn add_image(&mut self, jpeg: Jpeg) -> io::Result<()> {
        let width = Px(jpeg.width);
        let height = Px(jpeg.height);
        let bits_per_component = 8;
        let color_space = match jpeg.format {
            JpegFormat::Rgb => ColorSpace::Rgb.as_string(),
            JpegFormat::Gray => ColorSpace::Greyscale.as_string(),
        };

        #[rustfmt::skip]
        let xobject_stream = ExternalStream {
            dict: BTreeMap::from([
                (String::from("Type"), DictItem::Name("XObject".into())),
                (String::from("Subtype"), DictItem::Name("Image".into())),
                (String::from("Width"), DictItem::Int(jpeg.width as i64)),
                (String::from("Height"), DictItem::Int(jpeg.height as i64)),
                (String::from("BitsPerComponent"), DictItem::Int(bits_per_component)),
                (String::from("ColorSpace"), DictItem::Name(color_space.into())),
                (String::from("Interpolate"), DictItem::Bool(false)),
                (String::from("Filter"), DictItem::Name("DCTDecode".into())),
            ]),
            content: jpeg.bytes,
            compress: false,
        };

        let xobject_id = self.doc.add_xobject(&ExternalXObject {
            stream: xobject_stream,
            width: Some(width),
            height: Some(height),
            dpi: Some(self.dpi),
        });

        let page_content = vec![Op::UseXobject {
            id: xobject_id.clone(),
            transform: XObjectTransform {
                dpi: Some(self.dpi),
                ..Default::default()
            },
        }];

        let page = PdfPage::new(
            Mm::from(Px(jpeg.width).into_pt(self.dpi)),
            Mm::from(Px(jpeg.height).into_pt(self.dpi)),
            page_content,
        );

        self.pages.push(page);

        Ok(())
    }

    pub fn write_to<W: io::Write>(mut self, w: W) -> anyhow::Result<()> {
        let mut writer = io::BufWriter::with_capacity(128 * 1024, w);
        let mut warnings = Vec::new();

        let opts = PdfSaveOptions {
            image_optimization: None,
            ..Default::default()
        };

        self.doc
            .with_pages(self.pages)
            .save_writer(&mut writer, &opts, &mut warnings);

        Ok(())
    }
}
