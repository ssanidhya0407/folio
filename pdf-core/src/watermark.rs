use std::path::Path;

use lopdf::{dictionary, Dictionary, Document, Object, Stream};

use crate::error::{PdfError, Result};
use crate::range::PageRange;

/// Text watermark options.
#[derive(Debug, Clone)]
pub struct WatermarkOptions {
    pub text: String,
    pub font_size: f32,
    /// 0.0 (invisible) .. 1.0 (opaque).
    pub opacity: f32,
    /// RGB, each 0.0 .. 1.0.
    pub color: (f32, f32, f32),
    /// Degrees, counter-clockwise.
    pub rotation: f32,
    /// Repeat the text across the whole page.
    pub tiled: bool,
    /// Page range (e.g. "1-3,5"); `None` = all pages.
    pub pages: Option<String>,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        WatermarkOptions {
            text: String::new(),
            font_size: 48.0,
            opacity: 0.3,
            color: (0.5, 0.5, 0.5),
            rotation: 45.0,
            tiled: false,
            pages: None,
        }
    }
}

/// Stamp a text watermark onto `input`, writing `output`. Draws above existing
/// content with real transparency (via an ExtGState alpha) using the standard
/// Helvetica font (no embedding).
pub fn watermark(input: &Path, output: &Path, o: &WatermarkOptions) -> Result<()> {
    if o.text.trim().is_empty() {
        return Err(PdfError::Invalid("watermark text is empty".into()));
    }
    let mut doc = Document::load(input)?;
    let pages = doc.get_pages();
    let total = pages.len();
    let selected: Vec<usize> = match &o.pages {
        Some(s) if !s.trim().is_empty() => PageRange::parse(s, total)?.pages().to_vec(),
        _ => (1..=total).collect(),
    };

    let font_id = doc.add_object(dictionary! {
        "Type" => Object::Name(b"Font".to_vec()),
        "Subtype" => Object::Name(b"Type1".to_vec()),
        "BaseFont" => Object::Name(b"Helvetica".to_vec()),
    });

    for pno in selected {
        let Some(&page_id) = pages.get(&(pno as u32)) else { continue };
        let (w, h) = media_box(&doc, page_id);
        let content = build_content(o, w, h);
        let wm_id = doc.add_object(Stream::new(Dictionary::new(), content));
        append_contents(&mut doc, page_id, wm_id);
        add_resources(&mut doc, page_id, font_id, o.opacity);
    }

    crate::io::save_clean(&mut doc, output)
}

fn build_content(o: &WatermarkOptions, w: f32, h: f32) -> Vec<u8> {
    let rad = o.rotation.to_radians();
    let (c, s) = (rad.cos(), rad.sin());
    let (r, g, b) = o.color;
    let txt = escape_pdf(&o.text);

    let mut out = String::new();
    out.push_str("q\n/GS1 gs\n");
    out.push_str(&format!("{r:.3} {g:.3} {b:.3} rg\n"));
    out.push_str(&format!("BT\n/F1 {:.2} Tf\n", o.font_size));

    if o.tiled {
        let step_x = (o.text.chars().count() as f32 * o.font_size * 0.6).max(160.0);
        let step_y = (o.font_size * 3.5).max(140.0);
        let mut y = -h * 0.2;
        while y < h * 1.2 {
            let mut x = -w * 0.2;
            while x < w * 1.2 {
                out.push_str(&format!("{c:.4} {s:.4} {:.4} {c:.4} {x:.2} {y:.2} Tm\n({txt}) Tj\n", -s));
                x += step_x;
            }
            y += step_y;
        }
    } else {
        // approximate centering: Helvetica avg glyph ~0.5em wide
        let tw = o.text.chars().count() as f32 * o.font_size * 0.5;
        let (cx, cy) = (w / 2.0, h / 2.0);
        let sx = cx - (tw / 2.0) * c;
        let sy = cy - (tw / 2.0) * s;
        out.push_str(&format!("{c:.4} {s:.4} {:.4} {c:.4} {sx:.2} {sy:.2} Tm\n({txt}) Tj\n", -s));
    }

    out.push_str("ET\nQ\n");
    out.into_bytes()
}

fn escape_pdf(s: &str) -> String {
    s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

/// Resolve a page's MediaBox (walking the Parent chain); defaults to A4.
fn media_box(doc: &Document, page_id: lopdf::ObjectId) -> (f32, f32) {
    let mut cur = Some(page_id);
    while let Some(pid) = cur {
        let Ok(d) = doc.get_object(pid).and_then(Object::as_dict) else { break };
        if let Ok(mb) = d.get(b"MediaBox").and_then(Object::as_array) {
            if mb.len() == 4 {
                let n = |i: usize| {
                    mb.get(i)
                        .map(|o| o.as_float().or_else(|_| o.as_i64().map(|v| v as f32)).unwrap_or(0.0))
                        .unwrap_or(0.0)
                };
                return ((n(2) - n(0)).abs(), (n(3) - n(1)).abs());
            }
        }
        cur = d.get(b"Parent").and_then(Object::as_reference).ok();
    }
    (595.0, 842.0)
}

fn append_contents(doc: &mut Document, page_id: lopdf::ObjectId, wm_id: lopdf::ObjectId) {
    if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
        let new_contents = match page.get(b"Contents") {
            Ok(Object::Reference(id)) => Object::Array(vec![Object::Reference(*id), Object::Reference(wm_id)]),
            Ok(Object::Array(arr)) => {
                let mut a = arr.clone();
                a.push(Object::Reference(wm_id));
                Object::Array(a)
            }
            _ => Object::Reference(wm_id),
        };
        page.set("Contents", new_contents);
    }
}

fn add_resources(doc: &mut Document, page_id: lopdf::ObjectId, font_id: lopdf::ObjectId, alpha: f32) {
    let mut res = resources(doc, page_id);

    let mut fonts = subdict(doc, &res, b"Font");
    fonts.set("F1", Object::Reference(font_id));
    res.set("Font", fonts);

    let mut egs = subdict(doc, &res, b"ExtGState");
    egs.set(
        "GS1",
        dictionary! {
            "Type" => Object::Name(b"ExtGState".to_vec()),
            "ca" => Object::Real(alpha),
            "CA" => Object::Real(alpha),
        },
    );
    res.set("ExtGState", egs);

    if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
        page.set("Resources", res);
    }
}

/// Get a page's effective Resources dict (resolving refs / inheritance).
fn resources(doc: &Document, page_id: lopdf::ObjectId) -> Dictionary {
    let mut cur = Some(page_id);
    while let Some(pid) = cur {
        let Ok(d) = doc.get_object(pid).and_then(Object::as_dict) else { break };
        match d.get(b"Resources") {
            Ok(Object::Dictionary(rd)) => return rd.clone(),
            Ok(Object::Reference(id)) => {
                return doc.get_object(*id).and_then(Object::as_dict).cloned().unwrap_or_default();
            }
            _ => cur = d.get(b"Parent").and_then(Object::as_reference).ok(),
        }
    }
    Dictionary::new()
}

/// Clone a sub-dictionary (e.g. Font, ExtGState) from a resources dict.
fn subdict(doc: &Document, res: &Dictionary, key: &[u8]) -> Dictionary {
    match res.get(key) {
        Ok(Object::Dictionary(d)) => d.clone(),
        Ok(Object::Reference(id)) => doc.get_object(*id).and_then(Object::as_dict).cloned().unwrap_or_default(),
        _ => Dictionary::new(),
    }
}
