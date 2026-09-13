use std::collections::HashSet;
use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use crate::error::{PdfError, Result};

/// A page plus the inheritable attributes resolved for it — `/Resources`,
/// `/MediaBox` and friends, copied down from its ancestors.
type ResolvedPage = (ObjectId, Vec<(Vec<u8>, Object)>);

/// Page-tree attributes a child may inherit from an ancestor `Pages` node.
/// We resolve these onto each page before flattening the tree so a page never
/// loses its size/resources/rotation when its old parent is dropped.
const INHERITABLE: &[&[u8]] = &[b"MediaBox", b"CropBox", b"Resources", b"Rotate"];

/// Rewrite `input` so its pages appear in the order given by `order` (a list of
/// 1-based page numbers from the original document). Pages omitted from `order`
/// are removed; a page number may be repeated to duplicate that page.
///
/// This is the engine behind the visual "reorder / delete pages" tool: the UI
/// shows page thumbnails, the user drags/removes them, and the resulting order
/// is handed here.
pub fn reorder<P: AsRef<Path>>(input: P, output: &Path, order: &[usize]) -> Result<()> {
    let mut doc = Document::load(input.as_ref())?;
    let pages = doc.get_pages();
    let total = pages.len();

    if order.is_empty() {
        return Err(PdfError::Invalid(
            "the new page order is empty — the result would have no pages".into(),
        ));
    }
    if let Some(&bad) = order.iter().find(|&&n| n < 1 || n > total) {
        return Err(PdfError::PageOutOfRange(bad.to_string(), total));
    }

    // Object id for each requested page, in the requested order.
    let ordered_ids: Vec<ObjectId> = order
        .iter()
        .map(|&n| pages[&(n as u32)])
        .collect();

    // Push inherited attributes down onto each kept page *before* we re-parent,
    // otherwise pages under a nested tree would lose them once flattened.
    let kept: HashSet<ObjectId> = ordered_ids.iter().copied().collect();
    let resolved: Vec<ResolvedPage> = kept
        .iter()
        .map(|&id| (id, resolve_inherited(&doc, id)))
        .collect();
    for (page_id, attrs) in resolved {
        if let Ok(dict) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
            for (key, value) in attrs {
                dict.set(key, value);
            }
        }
    }

    // Drop the pages that aren't kept (also prunes them from the tree).
    let to_delete: Vec<u32> = (1..=total)
        .map(|n| n as u32)
        .filter(|n| !kept.contains(&pages[n]))
        .collect();
    if !to_delete.is_empty() {
        doc.delete_pages(&to_delete);
    }

    // Flatten the retained pages directly under the root page-tree node in the
    // requested order.
    let pages_id = root_pages_id(&doc)?;
    for &page_id in &ordered_ids {
        if let Ok(dict) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }
    if let Ok(root) = doc.get_object_mut(pages_id).and_then(Object::as_dict_mut) {
        let kids: Vec<Object> = ordered_ids.iter().map(|&id| Object::Reference(id)).collect();
        root.set("Count", ordered_ids.len() as i64);
        root.set("Kids", kids);
    }

    doc.renumber_objects();
    doc.compress();
    crate::io::save_clean(&mut doc, output)?;
    Ok(())
}

/// Find the object id of the root `Pages` node via the catalog (`/Root /Pages`).
fn root_pages_id(doc: &Document) -> Result<ObjectId> {
    let root_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| PdfError::Invalid("PDF has no document catalog (/Root)".into()))?;
    doc.get_object(root_id)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Pages").ok())
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| PdfError::Invalid("PDF catalog has no page tree (/Pages)".into()))
}

/// Collect the inheritable attributes that apply to `page_id`, walking up the
/// `/Parent` chain. The page's own values win; otherwise the nearest ancestor's
/// value is used. Returns only keys the page does not already define itself.
fn resolve_inherited(doc: &Document, page_id: ObjectId) -> Vec<(Vec<u8>, Object)> {
    let mut found: Vec<(Vec<u8>, Object)> = Vec::new();
    let mut seen: HashSet<&[u8]> = HashSet::new();

    // The page's own keys are already present — mark them so we don't override.
    if let Ok(dict) = doc.get_object(page_id).and_then(|o| o.as_dict()) {
        for &key in INHERITABLE {
            if dict.has(key) {
                seen.insert(key);
            }
        }
    }

    let mut current = doc
        .get_object(page_id)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Parent").ok())
        .and_then(|o| o.as_reference().ok());

    while let Some(id) = current {
        let Ok(dict) = doc.get_object(id).and_then(|o| o.as_dict()) else {
            break;
        };
        for &key in INHERITABLE {
            if !seen.contains(key) {
                if let Ok(value) = dict.get(key) {
                    found.push((key.to_vec(), value.clone()));
                    seen.insert(key);
                }
            }
        }
        current = dict.get(b"Parent").ok().and_then(|o| o.as_reference().ok());
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};

    /// Build an `n`-page PDF where page `i` has a MediaBox width of `i * 10`, so
    /// the page order is recoverable after a round-trip.
    fn make_doc(n: usize) -> Document {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let kids: Vec<Object> = (1..=n)
            .map(|i| {
                doc.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), ((i * 10) as i64).into(), 100.into()],
                })
                .into()
            })
            .collect();
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => n as i64,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    /// The MediaBox widths of each page, in document order.
    fn widths(path: &Path) -> Vec<i64> {
        let doc = Document::load(path).unwrap();
        doc.get_pages()
            .values()
            .map(|&id| {
                let mb = doc.get_object(id).unwrap().as_dict().unwrap().get(b"MediaBox").unwrap();
                mb.as_array().unwrap()[2].as_i64().unwrap()
            })
            .collect()
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pdfcore_pages_{}_{name}.pdf", std::process::id()))
    }

    #[test]
    fn reorders_pages() {
        let src = tmp("src_reorder");
        let out = tmp("out_reorder");
        make_doc(3).save(&src).unwrap();

        reorder(&src, &out, &[3, 1, 2]).unwrap();
        assert_eq!(widths(&out), vec![30, 10, 20]);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn deletes_omitted_pages() {
        let src = tmp("src_delete");
        let out = tmp("out_delete");
        make_doc(4).save(&src).unwrap();

        // Keep only pages 4 and 1, in that order.
        reorder(&src, &out, &[4, 1]).unwrap();
        assert_eq!(widths(&out), vec![40, 10]);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn rejects_empty_and_out_of_range() {
        let src = tmp("src_invalid");
        make_doc(2).save(&src).unwrap();
        let out = tmp("out_invalid");

        assert!(reorder(&src, &out, &[]).is_err());
        assert!(reorder(&src, &out, &[3]).is_err());

        let _ = std::fs::remove_file(&src);
    }
}
