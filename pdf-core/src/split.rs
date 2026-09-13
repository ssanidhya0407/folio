use std::collections::HashSet;
use std::path::{Path, PathBuf};

use lopdf::Document;

use crate::error::{PdfError, Result};
use crate::range::PageRange;

/// Extract the pages named by `spec` (e.g. `"1-3,5"`) from `input` into a
/// single new PDF at `output`.
pub fn extract<P: AsRef<Path>>(input: P, output: &Path, spec: &str) -> Result<()> {
    let input = input.as_ref();
    let mut doc = Document::load(input)?;
    let total = doc.get_pages().len();

    let range = PageRange::parse(spec, total)?;
    // delete everything that is *not* selected
    let to_delete: Vec<u32> = range.complement(total).into_iter().map(|p| p as u32).collect();
    doc.delete_pages(&to_delete);

    doc.renumber_objects();
    doc.compress();
    crate::io::save_clean(&mut doc, output)?;
    Ok(())
}

/// Split `input` into one single-page PDF per page, written into `output_dir`
/// as `<stem>_<n>.pdf` (1-based). Returns the paths created, in order.
pub fn split_each_page<P: AsRef<Path>>(input: P, output_dir: &Path) -> Result<Vec<PathBuf>> {
    let input = input.as_ref();
    let total = Document::load(input)?.get_pages().len();
    if total == 0 {
        return Err(PdfError::Invalid("input PDF has no pages".into()));
    }

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page")
        .to_string();

    std::fs::create_dir_all(output_dir).map_err(|e| PdfError::io(output_dir, e))?;

    let mut outputs = Vec::with_capacity(total);
    for page in 1..=total {
        // Reload fresh each time, then strip everything but `page`.
        let mut doc = Document::load(input)?;
        let to_delete: Vec<u32> = (1..=total)
            .filter(|p| *p != page)
            .map(|p| p as u32)
            .collect();
        doc.delete_pages(&to_delete);
        doc.renumber_objects();
        doc.compress();

        let out = output_dir.join(format!("{stem}_{page}.pdf"));
        crate::io::save_clean(&mut doc, &out)?;
        outputs.push(out);
    }
    Ok(outputs)
}

/// Split into consecutive chunks of `n` pages each.
pub fn split_every<P: AsRef<Path>>(input: P, output_dir: &Path, n: usize) -> Result<Vec<PathBuf>> {
    if n == 0 {
        return Err(PdfError::Invalid("pages per file must be at least 1".into()));
    }
    let total = Document::load(input.as_ref())?.get_pages().len();
    let mut bounds = Vec::new();
    let mut start = 1;
    while start <= total {
        let end = (start + n - 1).min(total);
        bounds.push((start, end));
        start = end + 1;
    }
    split_ranges(input.as_ref(), output_dir, &bounds)
}

/// Split *before* each 1-based page in `points` (e.g. `[4, 8]` → 1-3, 4-7, 8-end).
pub fn split_at<P: AsRef<Path>>(input: P, output_dir: &Path, points: &[usize]) -> Result<Vec<PathBuf>> {
    let total = Document::load(input.as_ref())?.get_pages().len();
    let mut pts: Vec<usize> = points.iter().copied().filter(|&p| p > 1 && p <= total).collect();
    pts.sort_unstable();
    pts.dedup();
    if pts.is_empty() {
        return Err(PdfError::Invalid("no valid split points within the document".into()));
    }
    let mut bounds = Vec::new();
    let mut start = 1;
    for &p in &pts {
        bounds.push((start, p - 1));
        start = p;
    }
    bounds.push((start, total));
    split_ranges(input.as_ref(), output_dir, &bounds)
}

/// Write one PDF per (start, end) inclusive 1-based page range.
fn split_ranges(input: &Path, output_dir: &Path, bounds: &[(usize, usize)]) -> Result<Vec<PathBuf>> {
    let total = Document::load(input)?.get_pages().len();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("part")
        .to_string();
    std::fs::create_dir_all(output_dir).map_err(|e| PdfError::io(output_dir, e))?;

    let mut outputs = Vec::with_capacity(bounds.len());
    for (i, &(lo, hi)) in bounds.iter().enumerate() {
        let mut doc = Document::load(input)?;
        let keep: HashSet<u32> = (lo..=hi).map(|p| p as u32).collect();
        let to_delete: Vec<u32> = (1..=total).map(|p| p as u32).filter(|p| !keep.contains(p)).collect();
        doc.delete_pages(&to_delete);
        doc.renumber_objects();
        doc.compress();
        let out = output_dir.join(format!("{stem}_{}.pdf", i + 1));
        crate::io::save_clean(&mut doc, &out)?;
        outputs.push(out);
    }
    Ok(outputs)
}
