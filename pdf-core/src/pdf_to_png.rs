use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use crate::error::{PdfError, Result};

/// Output image format for [`pdf_to_pngs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImgFormat {
    Png,
    Jpeg,
}

impl ImgFormat {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "jpeg" | "jpg" => ImgFormat::Jpeg,
            _ => ImgFormat::Png,
        }
    }
    fn ext(self) -> &'static str {
        match self {
            ImgFormat::Png => "png",
            ImgFormat::Jpeg => "jpg",
        }
    }
}

/// Render pages of `input` to images in `output_dir`, named `<stem>_<n>.<ext>`
/// (1-based). `dpi` controls resolution (72 ≈ original size; default 150).
/// `range` optionally limits which pages (e.g. `"1-3,5"`); `None` = all pages.
///
/// Requires the PDFium library (`pdfium.dll` on Windows) next to the executable
/// or on the system path — it is bundled with the app.
pub fn pdf_to_pngs<P: AsRef<Path>>(
    input: P,
    output_dir: &Path,
    dpi: f32,
    format: ImgFormat,
    range: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let input = input.as_ref();
    let dpi = if dpi <= 0.0 { 150.0 } else { dpi };

    let pdfium = Pdfium::new(load_bindings()?);
    let doc = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|e| PdfError::Invalid(format!("could not open PDF: {e}")))?;

    let total = doc.pages().len() as usize;
    let selected: std::collections::HashSet<usize> = match range {
        Some(s) if !s.trim().is_empty() => {
            crate::range::PageRange::parse(s, total)?.pages().iter().copied().collect()
        }
        _ => (1..=total).collect(),
    };

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page")
        .to_string();

    std::fs::create_dir_all(output_dir).map_err(|e| PdfError::io(output_dir, e))?;

    let config = PdfRenderConfig::new().scale_page_by_factor(dpi / 72.0);

    let mut outputs = Vec::new();
    for (index, page) in doc.pages().iter().enumerate() {
        let pageno = index + 1;
        if !selected.contains(&pageno) {
            continue;
        }
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| PdfError::Invalid(format!("render failed on page {pageno}: {e}")))?;

        let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
        let buffer = image::RgbaImage::from_raw(w, h, bitmap.as_rgba_bytes())
            .ok_or_else(|| PdfError::Invalid(format!("unexpected bitmap size on page {pageno}")))?;

        let out = output_dir.join(format!("{stem}_{pageno}.{}", format.ext()));
        match format {
            ImgFormat::Png => buffer.save(&out)?,
            ImgFormat::Jpeg => image::DynamicImage::ImageRgba8(buffer).to_rgb8().save(&out)?,
        }
        outputs.push(out);
    }

    Ok(outputs)
}

/// Render up to `max_pages` pages to in-memory PNG bytes at roughly
/// `target_width` pixels wide — for GUI page thumbnails/preview.
pub fn render_thumbnails<P: AsRef<Path>>(
    input: P,
    max_pages: usize,
    target_width: u32,
) -> Result<Vec<Vec<u8>>> {
    let pdfium = Pdfium::new(load_bindings()?);
    let doc = pdfium
        .load_pdf_from_file(input.as_ref(), None)
        .map_err(|e| PdfError::Invalid(format!("could not open PDF: {e}")))?;

    let config = PdfRenderConfig::new().set_target_width(target_width as i32);

    let mut thumbs = Vec::new();
    for (index, page) in doc.pages().iter().enumerate() {
        if index >= max_pages {
            break;
        }
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| PdfError::Invalid(format!("thumbnail render failed: {e}")))?;
        let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
        let buffer = image::RgbaImage::from_raw(w, h, bitmap.as_rgba_bytes())
            .ok_or_else(|| PdfError::Invalid("unexpected bitmap size".into()))?;

        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
        thumbs.push(png);
    }
    Ok(thumbs)
}

/// Number of pages in a PDF (fast; no rendering).
pub fn page_count<P: AsRef<Path>>(input: P) -> Result<usize> {
    let doc = lopdf::Document::load(input.as_ref())?;
    Ok(doc.get_pages().len())
}

/// Locate the PDFium library: next to the executable, then the working
/// directory, then the system library path.
pub(crate) fn load_bindings() -> Result<Box<dyn PdfiumLibraryBindings>> {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        if let Ok(bindings) =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir))
        {
            return Ok(bindings);
        }
    }
    if let Ok(bindings) =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
    {
        return Ok(bindings);
    }
    Pdfium::bind_to_system_library().map_err(|e| {
        PdfError::Invalid(format!(
            "PDFium library not found (expected pdfium next to the app): {e}"
        ))
    })
}
