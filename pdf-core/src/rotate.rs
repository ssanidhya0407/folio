use std::path::Path;

use lopdf::Document;

use crate::error::{PdfError, Result};
use crate::range::PageRange;

/// Rotate pages by `degrees` (must be a multiple of 90; may be negative).
///
/// Rotation is **additive**: it adds to each page's existing `/Rotate` value,
/// so `rotate(.., 90, ..)` turns the page 90° clockwise from where it is.
/// `spec` selects pages (e.g. `"1-3,5"`); `None` rotates every page.
pub fn rotate<P: AsRef<Path>>(
    input: P,
    output: &Path,
    degrees: i64,
    spec: Option<&str>,
) -> Result<()> {
    if degrees % 90 != 0 {
        return Err(PdfError::Invalid(
            "rotation must be a multiple of 90 degrees".into(),
        ));
    }

    let mut doc = Document::load(input.as_ref())?;
    let pages = doc.get_pages();
    let total = pages.len();

    let selected: Vec<usize> = match spec {
        Some(s) => PageRange::parse(s, total)?.pages().to_vec(),
        None => (1..=total).collect(),
    };

    for page_no in selected {
        if let Some(&object_id) = pages.get(&(page_no as u32)) {
            let current = doc
                .get_object(object_id)
                .ok()
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Rotate").ok())
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0);

            let next = normalize(current + degrees);

            if let Ok(dict) = doc
                .get_object_mut(object_id)
                .and_then(|o| o.as_dict_mut())
            {
                dict.set("Rotate", next);
            }
        }
    }

    crate::io::save_clean(&mut doc, output)?;
    Ok(())
}

/// Normalize an arbitrary degree value to one of 0, 90, 180, 270.
fn normalize(deg: i64) -> i64 {
    deg.rem_euclid(360)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalizes_negative_and_overflow() {
        assert_eq!(normalize(-90), 270);
        assert_eq!(normalize(450), 90);
        assert_eq!(normalize(360), 0);
    }
}
