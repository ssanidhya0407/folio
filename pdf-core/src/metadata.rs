use std::path::Path;

use lopdf::{Dictionary, Document, Object, StringFormat};

use crate::error::Result;

/// Document information dictionary fields (`/Info`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        *self == Metadata::default()
    }
}

/// Read the `/Info` metadata dictionary from a PDF.
pub fn read_metadata<P: AsRef<Path>>(input: P) -> Result<Metadata> {
    let doc = Document::load(input.as_ref())?;
    let mut m = Metadata::default();

    if let Ok(info) = doc.trailer.get(b"Info") {
        // /Info is normally an indirect reference, occasionally inline.
        let dict = match info.as_reference() {
            Ok(id) => doc.get_object(id).ok().and_then(|o| o.as_dict().ok()),
            Err(_) => info.as_dict().ok(),
        };
        if let Some(dict) = dict {
            m.title = get_text(dict, b"Title");
            m.author = get_text(dict, b"Author");
            m.subject = get_text(dict, b"Subject");
            m.keywords = get_text(dict, b"Keywords");
            m.creator = get_text(dict, b"Creator");
            m.producer = get_text(dict, b"Producer");
            m.creation_date = get_text(dict, b"CreationDate");
            m.mod_date = get_text(dict, b"ModDate");
        }
    }

    Ok(m)
}

/// Strip all document metadata: removes the `/Info` dictionary and any XMP
/// `/Metadata` stream on the catalog. Useful for privacy/redaction workflows.
pub fn scrub_metadata<P: AsRef<Path>>(input: P, output: &Path) -> Result<()> {
    let mut doc = Document::load(input.as_ref())?;

    // Drop the classic /Info dictionary.
    doc.trailer.remove(b"Info");

    // Drop the XMP metadata stream referenced from the catalog (/Root /Metadata).
    if let Ok(root_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Ok(dict) = doc.get_object_mut(root_id).and_then(|o| o.as_dict_mut()) {
            dict.remove(b"Metadata");
        }
    }

    crate::io::save_clean(&mut doc, output)?;
    Ok(())
}

/// Write the given metadata into the document's `/Info` dictionary (replacing
/// it). Empty fields are omitted. Non-ASCII values are stored as UTF-16BE.
pub fn set_metadata<P: AsRef<Path>>(input: P, output: &Path, meta: &Metadata) -> Result<()> {
    let mut doc = Document::load(input.as_ref())?;

    let mut info = Dictionary::new();
    put(&mut info, "Title", &meta.title);
    put(&mut info, "Author", &meta.author);
    put(&mut info, "Subject", &meta.subject);
    put(&mut info, "Keywords", &meta.keywords);
    put(&mut info, "Creator", &meta.creator);
    put(&mut info, "Producer", &meta.producer);
    put(&mut info, "CreationDate", &meta.creation_date);
    put(&mut info, "ModDate", &meta.mod_date);

    let id = doc.add_object(Object::Dictionary(info));
    doc.trailer.set("Info", id);
    crate::io::save_clean(&mut doc, output)?;
    Ok(())
}

fn put(dict: &mut Dictionary, key: &str, val: &Option<String>) {
    if let Some(v) = val {
        if !v.is_empty() {
            dict.set(key, Object::String(encode_pdf_text(v), StringFormat::Literal));
        }
    }
}

/// Encode a string for a PDF text field: plain bytes if ASCII, else UTF-16BE
/// with a BOM (matches what [`read_metadata`] decodes).
fn encode_pdf_text(s: &str) -> Vec<u8> {
    if s.is_ascii() {
        s.as_bytes().to_vec()
    } else {
        let mut out = vec![0xFE, 0xFF];
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_be_bytes());
        }
        out
    }
}

fn get_text(dict: &Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key)
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(decode_pdf_text)
        .filter(|s| !s.is_empty())
}

/// Decode a PDF text string. Handles UTF-16BE (BOM `FE FF`) and falls back to
/// a lossy Latin-1/UTF-8 interpretation otherwise.
fn decode_pdf_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::decode_pdf_text;

    #[test]
    fn decodes_utf16be_with_bom() {
        let bytes = [0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]; // "Hi"
        assert_eq!(decode_pdf_text(&bytes), "Hi");
    }

    #[test]
    fn decodes_plain_ascii() {
        assert_eq!(decode_pdf_text(b"Hello"), "Hello");
    }
}
