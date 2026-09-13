use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use crate::error::{PdfError, Result};

/// How much extractable text a document needs before we consider it "already
/// searchable" and refuse to re-OCR it (average characters per page).
const ALREADY_SEARCHABLE_CHARS_PER_PAGE: usize = 40;

/// Settings for [`ocr`].
#[derive(Debug, Clone)]
pub struct OcrOptions {
    /// Tesseract language code, or `+`-joined codes (`"eng"`, `"eng+deu"`).
    pub lang: String,
    /// Rasterization resolution. 300 is the sweet spot for recognition
    /// accuracy; below ~200 accuracy drops sharply.
    pub dpi: f32,
    /// Run even when the document already contains selectable text.
    pub force: bool,
}

impl Default for OcrOptions {
    fn default() -> Self {
        OcrOptions {
            lang: "eng".to_string(),
            dpi: 300.0,
            force: false,
        }
    }
}

/// What an OCR run produced.
#[derive(Debug, Clone)]
pub struct OcrStats {
    pub pages: usize,
    /// Characters of text recognized across the whole document.
    pub chars: usize,
}

/// Make a scanned PDF searchable: rasterize every page, recognize the text
/// with Tesseract, and write a PDF with an invisible text layer over the
/// original page image.
///
/// **This rewrites pages as images.** That is what makes a scan searchable,
/// but it also means a born-digital PDF would lose its vector text and grow
/// considerably — so a document that already has selectable text is rejected
/// unless [`OcrOptions::force`] is set.
///
/// Requires the bundled Tesseract engine and a `tessdata` language file; both
/// ship next to the executable (see README).
pub fn ocr<P: AsRef<Path>>(input: P, output: &Path, opts: &OcrOptions) -> Result<OcrStats> {
    ocr_with_progress(input, output, opts, |_, _| {})
}

/// [`ocr`], reporting progress as `(pages_done, pages_total)` while pages are
/// rasterized — the slow, measurable half of the work.
pub fn ocr_with_progress<P: AsRef<Path>, F: Fn(usize, usize)>(
    input: P,
    output: &Path,
    opts: &OcrOptions,
    progress: F,
) -> Result<OcrStats> {
    let input = input.as_ref();
    let dpi = if opts.dpi <= 0.0 { 300.0 } else { opts.dpi.clamp(72.0, 600.0) };
    let lang = if opts.lang.trim().is_empty() { "eng" } else { opts.lang.trim() };

    let workdir = tempfile::Builder::new()
        .prefix("folio-ocr-")
        .tempdir()
        .map_err(|e| PdfError::io(input, e))?;

    let images = rasterize(input, workdir.path(), dpi, opts.force, &progress)?;
    if images.is_empty() {
        return Err(PdfError::Invalid("the document has no pages".into()));
    }

    // Tesseract reads a newline-separated list of image paths and emits one
    // multi-page searchable PDF, which keeps this to a single engine call.
    let list = workdir.path().join("pages.txt");
    let listing: Vec<String> = images.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    std::fs::write(&list, listing.join("\n")).map_err(|e| PdfError::io(&list, e))?;

    let out_base = workdir.path().join("searchable");
    let mut args: Vec<String> = vec![
        list.to_string_lossy().into_owned(),
        out_base.to_string_lossy().into_owned(),
        "-l".into(),
        lang.to_string(),
        "--dpi".into(),
        format!("{dpi:.0}"),
    ];
    if let Some(dir) = crate::tesseract::tessdata_dir() {
        args.push("--tessdata-dir".into());
        args.push(dir.to_string_lossy().into_owned());
    }
    // Trailing config name: emit PDF rather than plain text.
    args.push("pdf".into());

    crate::tesseract::run(&args)?;

    // Tesseract appends the extension to the output base it was given.
    let produced = out_base.with_extension("pdf");
    if !produced.exists() {
        return Err(PdfError::Invalid(
            "the OCR engine did not produce a PDF".into(),
        ));
    }
    std::fs::copy(&produced, output).map_err(|e| PdfError::io(output, e))?;

    let chars = text_len(output).unwrap_or(0);
    Ok(OcrStats {
        pages: images.len(),
        chars,
    })
}

/// Render every page to a PNG in `dir`, returning the image paths in page
/// order. Refuses a document that already has selectable text unless `force`.
fn rasterize<F: Fn(usize, usize)>(
    input: &Path,
    dir: &Path,
    dpi: f32,
    force: bool,
    progress: &F,
) -> Result<Vec<PathBuf>> {
    let pdfium = Pdfium::new(crate::pdf_to_png::load_bindings()?);
    let doc = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|e| PdfError::Invalid(format!("could not open PDF: {e}")))?;

    let total = doc.pages().len() as usize;
    if !force {
        let chars: usize = doc
            .pages()
            .iter()
            .map(|p| p.text().map(|t| t.all().trim().chars().count()).unwrap_or(0))
            .sum();
        if total > 0 && chars / total >= ALREADY_SEARCHABLE_CHARS_PER_PAGE {
            return Err(PdfError::Invalid(
                "this PDF already has searchable text — OCR would replace the \
                 pages with images and make the file larger. Tick “OCR anyway” \
                 if that is what you want."
                    .into(),
            ));
        }
    }

    let config = PdfRenderConfig::new().scale_page_by_factor(dpi / 72.0);
    let mut images = Vec::with_capacity(total);

    for (index, page) in doc.pages().iter().enumerate() {
        let pageno = index + 1;
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| PdfError::Invalid(format!("render failed on page {pageno}: {e}")))?;

        let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
        let buffer = image::RgbaImage::from_raw(w, h, bitmap.as_rgba_bytes())
            .ok_or_else(|| PdfError::Invalid(format!("unexpected bitmap size on page {pageno}")))?;

        // Zero-padded so the lexical order tesseract sees matches page order.
        let out = dir.join(format!("page_{pageno:05}.png"));
        image::DynamicImage::ImageRgba8(buffer).to_rgb8().save(&out)?;
        images.push(out);
        progress(pageno, total);
    }

    Ok(images)
}

/// True when `input` already contains selectable text worth keeping.
pub fn has_text<P: AsRef<Path>>(input: P) -> Result<bool> {
    let input = input.as_ref();
    let pdfium = Pdfium::new(crate::pdf_to_png::load_bindings()?);
    let doc = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|e| PdfError::Invalid(format!("could not open PDF: {e}")))?;
    let total = doc.pages().len() as usize;
    if total == 0 {
        return Ok(false);
    }
    let chars: usize = doc
        .pages()
        .iter()
        .map(|p| p.text().map(|t| t.all().trim().chars().count()).unwrap_or(0))
        .sum();
    Ok(chars / total >= ALREADY_SEARCHABLE_CHARS_PER_PAGE)
}

/// Total characters of extractable text, used to report what OCR recovered.
fn text_len(path: &Path) -> Result<usize> {
    let pdfium = Pdfium::new(crate::pdf_to_png::load_bindings()?);
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| PdfError::Invalid(format!("could not open PDF: {e}")))?;
    Ok(doc
        .pages()
        .iter()
        .map(|p| p.text().map(|t| t.all().trim().chars().count()).unwrap_or(0))
        .sum())
}
