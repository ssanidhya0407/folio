use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::GenericImageView;
use printpdf::{Image, ImageTransform, Mm, PdfDocument};

use crate::error::{PdfError, Result};

/// Target page size for image→PDF conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    /// Page is exactly the image's size (no borders).
    Fit,
    /// A4 (210×297 mm), image scaled to fit, centered, auto-oriented.
    A4,
    /// US Letter (215.9×279.4 mm), image scaled to fit, centered, auto-oriented.
    Letter,
}

impl PageSize {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "a4" => PageSize::A4,
            "letter" => PageSize::Letter,
            _ => PageSize::Fit,
        }
    }
    fn dims_mm(self) -> Option<(f32, f32)> {
        match self {
            PageSize::Fit => None,
            PageSize::A4 => Some((210.0, 297.0)),
            PageSize::Letter => Some((215.9, 279.4)),
        }
    }
}

/// Convert one or more images into a single PDF, one image per page.
///
/// `dpi` (default 300) sets the image's natural size. `page` controls page
/// dimensions: `Fit` makes each page exactly the image size; `A4`/`Letter`
/// place the image scaled-to-fit and centered, auto-orienting the page to the
/// image's aspect ratio.
pub fn images_to_pdf<P: AsRef<Path>>(
    inputs: &[P],
    output: &Path,
    dpi: f32,
    page: PageSize,
) -> Result<()> {
    if inputs.is_empty() {
        return Err(PdfError::NoInput);
    }
    let dpi = if dpi <= 0.0 { 300.0 } else { dpi };

    let mut doc = None;

    for path in inputs {
        let path = path.as_ref();
        let dynimg = image::open(path)?;
        let (w_px, h_px) = dynimg.dimensions();
        let nat_w = px_to_mm(w_px, dpi);
        let nat_h = px_to_mm(h_px, dpi);

        // Page dimensions + placement of the image within the page.
        let (page_w, page_h, tx, ty, scale) = match page.dims_mm() {
            None => (nat_w, nat_h, 0.0, 0.0, 1.0),
            Some((pw, ph)) => {
                // auto-orient page to match the image
                let (pw, ph) = if (w_px > h_px) != (pw > ph) { (ph, pw) } else { (pw, ph) };
                let s = ((pw * 0.96) / nat_w).min((ph * 0.96) / nat_h);
                let (fw, fh) = (nat_w * s, nat_h * s);
                (pw, ph, (pw - fw) / 2.0, (ph - fh) / 2.0, s)
            }
        };

        let (doc_ref, pg, layer) = match doc.take() {
            None => PdfDocument::new("folio", Mm(page_w), Mm(page_h), "image"),
            Some(d) => {
                let d: printpdf::PdfDocumentReference = d;
                let (p, l) = d.add_page(Mm(page_w), Mm(page_h), "image");
                (d, p, l)
            }
        };

        let current_layer = doc_ref.get_page(pg).get_layer(layer);
        Image::from_dynamic_image(&dynimg).add_to_layer(
            current_layer,
            ImageTransform {
                translate_x: Some(Mm(tx)),
                translate_y: Some(Mm(ty)),
                rotate: None,
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(dpi),
            },
        );

        doc = Some(doc_ref);
    }

    let doc = doc.expect("at least one input produced a document");
    let file = File::create(output).map_err(|e| PdfError::io(output, e))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)
        .map_err(|e| PdfError::Invalid(format!("failed to write PDF: {e}")))?;
    Ok(())
}

fn px_to_mm(px: u32, dpi: f32) -> f32 {
    (px as f32) / dpi * 25.4
}
