use std::collections::BTreeMap;
use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use crate::error::{PdfError, Result};

/// Merge `inputs` (in the given order) into a single PDF written to `output`.
///
/// Page order follows the order of `inputs`, and within each file the source
/// page order is preserved.
pub fn merge<P: AsRef<Path>>(inputs: &[P], output: &Path) -> Result<()> {
    if inputs.is_empty() {
        return Err(PdfError::NoInput);
    }

    let mut max_id = 1u32;
    let mut pagenum = 1;
    // Object-id -> object for every page across all inputs, in insertion order.
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for path in inputs {
        let path = path.as_ref();
        let mut doc = Document::load(path)?;

        // Shift this doc's ids so they don't collide with already-loaded ids.
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        documents_pages.extend(
            pages
                .into_values()
                .map(|object_id| {
                    pagenum += 1;
                    (
                        object_id,
                        doc.get_object(object_id)
                            .map(|o| o.to_owned())
                            .unwrap_or(Object::Null),
                    )
                })
                .collect::<BTreeMap<ObjectId, Object>>(),
        );
        documents_objects.extend(doc.objects);
    }
    let _ = pagenum;

    // Locate (or synthesize) the single Catalog and Pages tree for the output.
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                // Keep the first catalog id; later catalogs are discarded.
                let id = catalog_object.as_ref().map(|(id, _)| *id).unwrap_or(*object_id);
                catalog_object = Some((id, object.clone()));
            }
            "Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref prev)) = pages_object {
                        if let Ok(prev_dict) = prev.as_dict() {
                            dictionary.extend(prev_dict);
                        }
                    }
                    let id = pages_object.as_ref().map(|(id, _)| *id).unwrap_or(*object_id);
                    pages_object = Some((id, Object::Dictionary(dictionary)));
                }
            }
            "Page" => {} // collected via documents_pages
            "Outlines" | "Outline" => {} // drop bookmarks for now
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    let pages_object = pages_object
        .ok_or_else(|| PdfError::Invalid("no Pages object found in inputs".into()))?;
    let catalog_object = catalog_object
        .ok_or_else(|| PdfError::Invalid("no Catalog object found in inputs".into()))?;

    // Re-parent every page to the merged Pages object and register it.
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.0);
            document.objects.insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // Rebuild the Pages dictionary: Count + Kids pointing at every page.
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as i64);
        dictionary.set(
            "Kids",
            documents_pages
                .keys()
                .map(|id| Object::Reference(*id))
                .collect::<Vec<_>>(),
        );
        document
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    // Rebuild the Catalog so it points at our Pages object.
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines");
        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document.compress();

    crate::io::save_clean(&mut document, output)?;
    Ok(())
}
