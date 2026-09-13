use std::path::Path;

use lopdf::Document;

use crate::error::{PdfError, Result};

/// Save a document, stripping cross-reference linkage that only makes sense for
/// the *original* file layout.
///
/// lopdf copies the loaded trailer verbatim, which can carry a `/Prev` (and
/// `/XRefStm`) pointer from an incrementally-updated source PDF. On a full
/// rewrite those offsets are stale and point into the middle of the new file,
/// so a reader following `/Prev` fails with "Invalid file trailer". Removing
/// them forces a clean, self-contained xref.
pub(crate) fn save_clean(doc: &mut Document, output: &Path) -> Result<()> {
    doc.trailer.remove(b"Prev");
    doc.trailer.remove(b"XRefStm");
    doc.save(output)
        .map_err(|e| PdfError::io(output, std::io::Error::other(e)))?;
    Ok(())
}
