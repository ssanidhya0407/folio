//! True redaction: content is **removed from the file**, not covered up.
//!
//! Drawing a black box over text is not redaction — the text is still in the
//! content stream, and "select all → copy" or `pdftotext` recovers it. This
//! module walks the page's content streams, deletes the glyphs and images that
//! fall inside each region, and only then paints the covering box on top.

use std::collections::BTreeMap;
use std::path::Path;

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::error::{PdfError, Result};

/// Average glyph advance, as a fraction of the font size, used to locate
/// glyphs without parsing font metrics.
///
/// Real advances vary (an `i` is narrower, a `W` wider), so a glyph's computed
/// box is approximate. [`PAD_EM`] widens the hit test to compensate, biasing
/// the approximation toward removing a borderline glyph rather than keeping
/// one — the safe direction for a redaction tool.
const WIDTH_EM: f64 = 0.6;

/// Advance assumed for CID (`Type0`) fonts, where glyphs are typically full-width.
const WIDTH_EM_CID: f64 = 1.0;

/// Extra margin added around each glyph box when hit-testing, in em.
const PAD_EM: f64 = 0.15;

/// How deep to follow nested Form XObjects before giving up.
const MAX_FORM_DEPTH: usize = 8;

/// A rectangle to redact, in **normalized page coordinates**: `0.0..1.0` of
/// the page's width and height, origin at the **top-left**, `y` growing
/// downward — the coordinate space a rendered page image hands a UI directly.
/// Page rotation (`/Rotate`) is applied for you.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactRegion {
    /// 1-based page number.
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Settings for [`redact_with`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactOptions {
    /// Fill colour for the box painted over each region, as RGB in `0.0..=1.0`.
    ///
    /// White makes a redaction read as blank space rather than an obvious bar.
    /// It changes nothing about what is removed — the content underneath is
    /// deleted from the file either way — only how the result looks.
    pub color: (f32, f32, f32),
}

impl Default for RedactOptions {
    fn default() -> Self {
        RedactOptions { color: (1.0, 1.0, 1.0) }
    }
}

/// What a redaction run removed.
#[derive(Debug, Clone, Default)]
pub struct RedactStats {
    pub regions: usize,
    pub pages: usize,
    /// Glyphs deleted from the content streams.
    pub glyphs_removed: usize,
    /// Image draw operations deleted.
    pub images_removed: usize,
    /// Annotations (links, form fields, comments) deleted.
    pub annotations_removed: usize,
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// A rectangle in PDF user space (points, origin bottom-left).
#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn intersects(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    /// True when `other`'s centre lies inside `self` — the test used for
    /// annotations, which should go only if they really belong to the region.
    fn holds_centre_of(&self, other: &Rect) -> bool {
        let cx = (other.x0 + other.x1) / 2.0;
        let cy = (other.y0 + other.y1) / 2.0;
        cx >= self.x0 && cx <= self.x1 && cy >= self.y0 && cy <= self.y1
    }
}

/// A 2-D affine matrix `[a b c d e f]`, as PDF writes one.
type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m1 × m2` — apply `m1` first, then `m2`.
fn mul(m1: &Matrix, m2: &Matrix) -> Matrix {
    [
        m1[0] * m2[0] + m1[1] * m2[2],
        m1[0] * m2[1] + m1[1] * m2[3],
        m1[2] * m2[0] + m1[3] * m2[2],
        m1[2] * m2[1] + m1[3] * m2[3],
        m1[4] * m2[0] + m1[5] * m2[2] + m2[4],
        m1[4] * m2[1] + m1[5] * m2[3] + m2[5],
    ]
}

fn apply(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// Axis-aligned bounds of the box `(x0,y0)-(x1,y1)` transformed by `m`.
fn transformed_bounds(m: &Matrix, x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    let c = [
        apply(m, x0, y0),
        apply(m, x1, y0),
        apply(m, x0, y1),
        apply(m, x1, y1),
    ];
    Rect {
        x0: c.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        y0: c.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        x1: c.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
        y1: c.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Remove every piece of page content that falls inside `regions`, then paint
/// an opaque box over each one.
///
/// Text and images are deleted from the content stream, and annotations
/// (links, form fields, comments) centred in a region are dropped. Glyph
/// positions come from the text and graphics matrices; see [`WIDTH_EM`] for
/// the one approximation involved.
pub fn redact<P: AsRef<Path>>(
    input: P,
    output: &Path,
    regions: &[RedactRegion],
) -> Result<RedactStats> {
    redact_with(input, output, regions, &RedactOptions::default())
}

/// [`redact`], with control over how the covering box looks.
pub fn redact_with<P: AsRef<Path>>(
    input: P,
    output: &Path,
    regions: &[RedactRegion],
    opts: &RedactOptions,
) -> Result<RedactStats> {
    if regions.is_empty() {
        return Err(PdfError::Invalid("select at least one area to redact".into()));
    }

    let mut doc = Document::load(input.as_ref())?;
    let pages = doc.get_pages();
    let total = pages.len();

    // Group by page so each page is rewritten exactly once.
    let mut by_page: BTreeMap<usize, Vec<RedactRegion>> = BTreeMap::new();
    for r in regions {
        if r.page == 0 || r.page > total {
            return Err(PdfError::PageOutOfRange(r.page.to_string(), total));
        }
        by_page.entry(r.page).or_default().push(*r);
    }

    let mut stats = RedactStats {
        regions: regions.len(),
        pages: by_page.len(),
        ..Default::default()
    };

    for (page_no, page_regions) in by_page {
        let page_id = *pages
            .get(&(page_no as u32))
            .ok_or_else(|| PdfError::PageOutOfRange(page_no.to_string(), total))?;

        let (media, rotate) = page_geometry(&doc, page_id);
        let rects: Vec<Rect> = page_regions
            .iter()
            .map(|r| to_user_space(r, &media, rotate))
            .collect();

        rewrite_page(&mut doc, page_id, &rects, opts, &mut stats)?;
        strip_annotations(&mut doc, page_id, &rects, &mut stats);
    }

    crate::io::save_clean(&mut doc, output)?;
    Ok(stats)
}

// ---------------------------------------------------------------------------
// Page geometry
// ---------------------------------------------------------------------------

/// The page's MediaBox and `/Rotate`, following `/Parent` for inherited values.
fn page_geometry(doc: &Document, page_id: ObjectId) -> (Rect, i64) {
    let media = inherited(doc, page_id, b"MediaBox")
        .and_then(|o| rect_from(doc, &o))
        .unwrap_or(Rect { x0: 0.0, y0: 0.0, x1: 612.0, y1: 792.0 });
    let rotate = inherited(doc, page_id, b"Rotate")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0)
        .rem_euclid(360);
    (media, rotate)
}

fn rect_from(doc: &Document, obj: &Object) -> Option<Rect> {
    let arr = deref(doc, obj).as_array().ok()?;
    let v: Vec<f64> = arr
        .iter()
        .filter_map(|o| as_num(deref(doc, o)))
        .collect();
    if v.len() != 4 {
        return None;
    }
    Some(Rect {
        x0: v[0].min(v[2]),
        y0: v[1].min(v[3]),
        x1: v[0].max(v[2]),
        y1: v[1].max(v[3]),
    })
}

/// Resolve an indirect reference, falling back to the object itself.
fn deref<'a>(doc: &'a Document, obj: &'a Object) -> &'a Object {
    doc.dereference(obj).map(|(_, o)| o).unwrap_or(obj)
}

fn as_num(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

/// Look up `key` on the page, walking up `/Parent` for inheritable attributes.
fn inherited(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut id = page_id;
    for _ in 0..MAX_FORM_DEPTH * 4 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            return Some(deref(doc, v).clone());
        }
        id = dict.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

/// Convert a normalized, top-left-origin region into PDF user space, undoing
/// the rotation a viewer applies for `/Rotate`.
///
/// With the page `W × H` in user space, a viewer showing it rotated clockwise
/// by `rotate` maps display point `(dx, dy)` (origin top-left) back to page
/// point `(px, py)` (origin bottom-left) as encoded below.
fn to_user_space(r: &RedactRegion, media: &Rect, rotate: i64) -> Rect {
    let w = media.x1 - media.x0;
    let h = media.y1 - media.y0;
    // Quarter turns swap the displayed dimensions.
    let (dw, dh) = if rotate == 90 || rotate == 270 { (h, w) } else { (w, h) };

    let dx0 = r.x as f64 * dw;
    let dy0 = r.y as f64 * dh;
    let dx1 = dx0 + r.w as f64 * dw;
    let dy1 = dy0 + r.h as f64 * dh;

    let back = |dx: f64, dy: f64| -> (f64, f64) {
        match rotate {
            90 => (dy, dx),
            180 => (w - dx, dy),
            270 => (w - dy, h - dx),
            _ => (dx, h - dy),
        }
    };

    let pts = [back(dx0, dy0), back(dx1, dy1)];
    let xs = [pts[0].0, pts[1].0];
    let ys = [pts[0].1, pts[1].1];
    Rect {
        x0: media.x0 + xs.iter().cloned().fold(f64::INFINITY, f64::min),
        y0: media.y0 + ys.iter().cloned().fold(f64::INFINITY, f64::min),
        x1: media.x0 + xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        y1: media.y0 + ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    }
}

// ---------------------------------------------------------------------------
// Page rewriting
// ---------------------------------------------------------------------------

/// Replace the page's content with a redacted copy, plus a black cover box.
fn rewrite_page(
    doc: &mut Document,
    page_id: ObjectId,
    rects: &[Rect],
    opts: &RedactOptions,
    stats: &mut RedactStats,
) -> Result<()> {
    let content = doc.get_page_content(page_id)?;
    let resources = page_resources(doc, page_id);

    let cleaned = filter_content(doc, &content, &resources, IDENTITY, rects, stats, 0)?;

    // Wrap the page's own content in `q … Q` before drawing the boxes. Plenty
    // of real PDFs (anything Skia-produced, for one) leave a `cm` in force at
    // the end of the stream; without the wrapper our rectangles inherit that
    // transform and land in the wrong place, at the wrong size. The extra `Q`s
    // close any `q` the content left open inside the wrapper.
    let mut body = Vec::with_capacity(cleaned.bytes.len() + 64);
    body.extend_from_slice(b"q\n");
    body.extend_from_slice(&cleaned.bytes);
    body.extend_from_slice(b"\n");
    body.extend_from_slice(&cover_ops(rects, cleaned.unbalanced + 1, opts.color));

    let mut stream = Stream::new(Dictionary::new(), body);
    // Compression is a nicety; an uncompressed stream is still valid.
    let _ = stream.compress();
    let new_id = doc.add_object(Object::Stream(stream));

    // Any form we copied has to be pointed at from this page only. Writing the
    // resolved /Resources straight onto the page also settles any inheritance.
    let updated = repoint(&resources, &cleaned.replacements);

    let dict = doc.get_object_mut(page_id)?.as_dict_mut()?;
    dict.set("Contents", Object::Reference(new_id));
    if let Some(resources) = updated {
        dict.set("Resources", Object::Dictionary(resources));
    }
    Ok(())
}

/// A copy of `resources` whose `/XObject` entries point at the redacted form
/// copies in `replacements`. `None` when there is nothing to repoint.
fn repoint(resources: &Dictionary, replacements: &[(Vec<u8>, ObjectId)]) -> Option<Dictionary> {
    if replacements.is_empty() {
        return None;
    }
    let mut out = resources.clone();
    let mut xobjects = out
        .get(b"XObject")
        .ok()
        .and_then(|o| o.as_dict().ok().cloned())
        .unwrap_or_default();
    for (name, id) in replacements {
        xobjects.set(name.clone(), Object::Reference(*id));
    }
    out.set("XObject", Object::Dictionary(xobjects));
    Some(out)
}

/// The page's `/Resources` dictionary (following `/Parent`), fully resolved.
fn page_resources(doc: &Document, page_id: ObjectId) -> Dictionary {
    inherited(doc, page_id, b"Resources")
        .and_then(|o| o.as_dict().ok().cloned())
        .unwrap_or_default()
}

/// Look up a named entry in a resource sub-dictionary (e.g. `XObject`/`Font`).
fn resource(doc: &Document, resources: &Dictionary, kind: &[u8], name: &[u8]) -> Option<Dictionary> {
    let sub = deref(doc, resources.get(kind).ok()?).as_dict().ok()?.clone();
    let obj = deref(doc, sub.get(name).ok()?).clone();
    match obj {
        Object::Dictionary(d) => Some(d),
        Object::Stream(s) => Some(s.dict.clone()),
        _ => None,
    }
}

/// `/Contents` bytes for the boxes covering `rects`, in page user space.
///
/// `close` is how many `Q` to emit first, to unwind the graphics stack back to
/// the page's base state — otherwise the boxes are drawn under whatever
/// transform the content left in force, rather than the one the rectangles
/// were computed in.
fn cover_ops(rects: &[Rect], close: usize, color: (f32, f32, f32)) -> Vec<u8> {
    let mut ops: Vec<Operation> = (0..close).map(|_| Operation::new("Q", vec![])).collect();
    ops.extend([
        Operation::new("q", vec![]),
        Operation::new(
            "rg",
            vec![
                Object::Real(color.0.clamp(0.0, 1.0)),
                Object::Real(color.1.clamp(0.0, 1.0)),
                Object::Real(color.2.clamp(0.0, 1.0)),
            ],
        ),
    ]);
    for r in rects {
        ops.push(Operation::new(
            "re",
            vec![
                Object::Real(r.x0 as f32),
                Object::Real(r.y0 as f32),
                Object::Real((r.x1 - r.x0) as f32),
                Object::Real((r.y1 - r.y0) as f32),
            ],
        ));
    }
    ops.push(Operation::new("f", vec![]));
    ops.push(Operation::new("Q", vec![]));

    // A leading newline guards against the previous stream ending mid-token.
    let mut out = vec![b'\n'];
    out.extend(Content { operations: ops }.encode().unwrap_or_default());
    out
}

/// Drop annotations whose centre falls in a redacted region — a link or form
/// field left behind would still carry the URL or value we just removed.
fn strip_annotations(
    doc: &mut Document,
    page_id: ObjectId,
    rects: &[Rect],
    stats: &mut RedactStats,
) {
    let Ok(dict) = doc.get_dictionary(page_id) else { return };
    let Ok(annots) = dict.get(b"Annots") else { return };
    let Ok(list) = deref(doc, annots).as_array().cloned() else { return };

    let kept: Vec<Object> = list
        .into_iter()
        .filter(|a| {
            let Ok(ad) = deref(doc, a).as_dict().cloned() else { return true };
            let Some(r) = ad.get(b"Rect").ok().and_then(|o| rect_from(doc, o)) else { return true };
            !rects.iter().any(|reg| reg.holds_centre_of(&r))
        })
        .collect();

    let removed = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Annots").ok())
        .and_then(|o| deref(doc, o).as_array().ok().map(|a| a.len()))
        .unwrap_or(0usize)
        .saturating_sub(kept.len());

    if removed > 0 {
        stats.annotations_removed += removed;
        if let Ok(d) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            d.set("Annots", Object::Array(kept));
        }
    }
}

// ---------------------------------------------------------------------------
// Content-stream filtering
// ---------------------------------------------------------------------------

/// Text-state values that affect where a glyph lands on the page.
#[derive(Clone)]
struct TextState {
    /// Text matrix and text line matrix.
    tm: Matrix,
    tlm: Matrix,
    size: f64,
    /// Character spacing (`Tc`), word spacing (`Tw`), leading (`TL`).
    char_spacing: f64,
    word_spacing: f64,
    leading: f64,
    /// Horizontal scaling (`Tz`) as a multiplier, and rise (`Ts`).
    h_scale: f64,
    rise: f64,
    /// Whether the current font encodes two bytes per glyph (`Type0`).
    two_byte: bool,
}

impl Default for TextState {
    fn default() -> Self {
        TextState {
            tm: IDENTITY,
            tlm: IDENTITY,
            size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            leading: 0.0,
            h_scale: 1.0,
            rise: 0.0,
            two_byte: false,
        }
    }
}

/// Rewrite `content`, removing anything that lands inside `rects`.
///
/// `ctm` is the transform in effect where this stream is invoked (identity for
/// a page, the accumulated matrix for a Form XObject).
#[allow(clippy::too_many_arguments)]
fn filter_content(
    doc: &mut Document,
    content: &[u8],
    resources: &Dictionary,
    ctm: Matrix,
    rects: &[Rect],
    stats: &mut RedactStats,
    depth: usize,
) -> Result<Filtered> {
    let parsed = Content::decode(content)
        .map_err(|e| PdfError::Invalid(format!("could not read the page content: {e}")))?;

    let mut out: Vec<Operation> = Vec::with_capacity(parsed.operations.len());
    let mut replacements: Vec<(Vec<u8>, ObjectId)> = Vec::new();
    let mut stack: Vec<Matrix> = Vec::new();
    let mut cur = ctm;
    let mut ts = TextState::default();

    for op in parsed.operations {
        match op.operator.as_str() {
            "q" => {
                stack.push(cur);
                out.push(op);
            }
            "Q" => {
                cur = stack.pop().unwrap_or(ctm);
                out.push(op);
            }
            "cm" => {
                if let Some(m) = matrix_of(&op.operands) {
                    cur = mul(&m, &cur);
                }
                out.push(op);
            }
            "BT" => {
                ts.tm = IDENTITY;
                ts.tlm = IDENTITY;
                out.push(op);
            }
            "Tf" => {
                ts.size = op.operands.get(1).and_then(as_num).unwrap_or(ts.size);
                ts.two_byte = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .and_then(|n| resource(doc, resources, b"Font", n))
                    .and_then(|f| f.get(b"Subtype").ok().and_then(|s| s.as_name().ok()).map(|s| s == b"Type0"))
                    .unwrap_or(false);
                out.push(op);
            }
            "Td" | "TD" => {
                let tx = op.operands.first().and_then(as_num).unwrap_or(0.0);
                let ty = op.operands.get(1).and_then(as_num).unwrap_or(0.0);
                if op.operator == "TD" {
                    ts.leading = -ty;
                }
                ts.tlm = mul(&[1.0, 0.0, 0.0, 1.0, tx, ty], &ts.tlm);
                ts.tm = ts.tlm;
                out.push(op);
            }
            "Tm" => {
                if let Some(m) = matrix_of(&op.operands) {
                    ts.tlm = m;
                    ts.tm = m;
                }
                out.push(op);
            }
            "T*" => {
                next_line(&mut ts);
                out.push(op);
            }
            "TL" => {
                ts.leading = op.operands.first().and_then(as_num).unwrap_or(ts.leading);
                out.push(op);
            }
            "Tc" => {
                ts.char_spacing = op.operands.first().and_then(as_num).unwrap_or(0.0);
                out.push(op);
            }
            "Tw" => {
                ts.word_spacing = op.operands.first().and_then(as_num).unwrap_or(0.0);
                out.push(op);
            }
            "Tz" => {
                ts.h_scale = op.operands.first().and_then(as_num).unwrap_or(100.0) / 100.0;
                out.push(op);
            }
            "Ts" => {
                ts.rise = op.operands.first().and_then(as_num).unwrap_or(0.0);
                out.push(op);
            }
            "Tj" | "TJ" | "'" | "\"" => {
                emit_show(&op, &mut ts, &cur, rects, stats, &mut out);
            }
            "Do" => {
                let name = op.operands.first().and_then(|o| o.as_name().ok()).map(|n| n.to_vec());
                match name.clone().and_then(|n| classify_xobject(doc, resources, &n)) {
                    Some(XObject::Image) => {
                        // An image fills the unit square under the current CTM.
                        let b = transformed_bounds(&cur, 0.0, 0.0, 1.0, 1.0);
                        if rects.iter().any(|r| r.intersects(&b)) {
                            stats.images_removed += 1;
                        } else {
                            out.push(op);
                        }
                    }
                    Some(XObject::Form(id)) if depth < MAX_FORM_DEPTH => {
                        let copy = redact_form(doc, id, resources, cur, rects, stats, depth)?;
                        if let Some(name) = name {
                            replacements.push((name, copy));
                        }
                        out.push(op);
                    }
                    _ => out.push(op),
                }
            }
            _ => out.push(op),
        }
    }

    let bytes = Content { operations: out }
        .encode()
        .map_err(|e| PdfError::Invalid(format!("could not rebuild the page content: {e}")))?;
    Ok(Filtered {
        bytes,
        unbalanced: stack.len(),
        replacements,
    })
}

/// A rewritten content stream, plus how many `q` it left on the graphics
/// stack. A malformed page can leave the transform altered at the end, which
/// would put our black box somewhere other than where the user drew it.
struct Filtered {
    bytes: Vec<u8>,
    unbalanced: usize,
    /// Form XObjects that were copied for this stream, as `(resource name,
    /// new object id)`. The caller must repoint its `/Resources` at them.
    replacements: Vec<(Vec<u8>, ObjectId)>,
}

enum XObject {
    Image,
    Form(ObjectId),
}

fn classify_xobject(doc: &Document, resources: &Dictionary, name: &[u8]) -> Option<XObject> {
    let sub = deref(doc, resources.get(b"XObject").ok()?).as_dict().ok()?.clone();
    let id = sub.get(name).ok()?.as_reference().ok()?;
    let stream = doc.get_object(id).ok()?.as_stream().ok()?;
    match stream.dict.get(b"Subtype").ok()?.as_name().ok()? {
        b"Image" => Some(XObject::Image),
        b"Form" => Some(XObject::Form(id)),
        _ => None,
    }
}

/// Recurse into a Form XObject, returning the id of a **redacted copy**.
///
/// The copy matters: forms are how generators share a header, footer or stamp
/// across every page. Editing one in place would mean redacting a header on
/// page 1 silently blanked it on every other page too. The caller repoints its
/// own `/Resources` at the copy, so the edit applies to that page alone.
#[allow(clippy::too_many_arguments)]
fn redact_form(
    doc: &mut Document,
    id: ObjectId,
    parent_resources: &Dictionary,
    ctm: Matrix,
    rects: &[Rect],
    stats: &mut RedactStats,
    depth: usize,
) -> Result<ObjectId> {
    let Ok(stream) = doc.get_object(id).and_then(|o| o.as_stream()).cloned() else {
        return Ok(id);
    };
    // An unfiltered stream has nothing to decompress, and lopdf reports that
    // as an error — fall back to the raw bytes rather than skipping the form.
    let content = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());

    // A form's own /Matrix maps its space into the space of its caller.
    let form_ctm = stream
        .dict
        .get(b"Matrix")
        .ok()
        .and_then(|o| deref(doc, o).as_array().ok().cloned())
        .and_then(|a| matrix_of(&a))
        .map(|m| mul(&m, &ctm))
        .unwrap_or(ctm);

    // Forms may carry their own resources; otherwise they inherit the caller's.
    let resources = stream
        .dict
        .get(b"Resources")
        .ok()
        .and_then(|o| deref(doc, o).as_dict().ok().cloned())
        .unwrap_or_else(|| parent_resources.clone());

    let cleaned = filter_content(doc, &content, &resources, form_ctm, rects, stats, depth + 1)?;

    let mut copy = stream;
    copy.dict.remove(b"Filter");
    copy.dict.remove(b"DecodeParms");
    copy.set_content(cleaned.bytes);
    let _ = copy.compress();
    // A nested form was copied too, so this copy must point at that one.
    if let Some(updated) = repoint(&resources, &cleaned.replacements) {
        copy.dict.set("Resources", Object::Dictionary(updated));
    }
    Ok(doc.add_object(Object::Stream(copy)))
}

fn matrix_of(operands: &[Object]) -> Option<Matrix> {
    if operands.len() < 6 {
        return None;
    }
    let mut m = [0.0f64; 6];
    for (i, slot) in m.iter_mut().enumerate() {
        *slot = as_num(&operands[i])?;
    }
    Some(m)
}

fn next_line(ts: &mut TextState) {
    ts.tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -ts.leading], &ts.tlm);
    ts.tm = ts.tlm;
}

// ---------------------------------------------------------------------------
// Text showing
// ---------------------------------------------------------------------------

/// Rewrite one text-showing operator, dropping the glyphs inside `rects`.
///
/// Removed glyphs are replaced by the equivalent `TJ` position adjustment, so
/// the surviving text stays exactly where it was on the page.
fn emit_show(
    op: &Operation,
    ts: &mut TextState,
    ctm: &Matrix,
    rects: &[Rect],
    stats: &mut RedactStats,
    out: &mut Vec<Operation>,
) {
    // `'` and `"` move to the next line first; `"` also sets the spacings.
    let mut prelude: Vec<Operation> = Vec::new();
    let elements: Vec<Object> = match op.operator.as_str() {
        "Tj" => op.operands.first().cloned().into_iter().collect(),
        "TJ" => op
            .operands
            .first()
            .and_then(|o| o.as_array().ok().cloned())
            .unwrap_or_default(),
        "'" => {
            next_line(ts);
            prelude.push(Operation::new("T*", vec![]));
            op.operands.first().cloned().into_iter().collect()
        }
        "\"" => {
            ts.word_spacing = op.operands.first().and_then(as_num).unwrap_or(0.0);
            ts.char_spacing = op.operands.get(1).and_then(as_num).unwrap_or(0.0);
            next_line(ts);
            prelude.push(Operation::new("Tw", vec![Object::Real(ts.word_spacing as f32)]));
            prelude.push(Operation::new("Tc", vec![Object::Real(ts.char_spacing as f32)]));
            prelude.push(Operation::new("T*", vec![]));
            op.operands.get(2).cloned().into_iter().collect()
        }
        _ => return,
    };

    let mut rebuilt: Vec<Object> = Vec::new();
    let mut removed_here = 0usize;

    for element in elements {
        match element {
            Object::String(bytes, fmt) => {
                let (kept, removed) = scan_string(&bytes, ts, ctm, rects, &mut rebuilt, fmt);
                removed_here += removed;
                let _ = kept;
            }
            other => {
                // A number in a TJ array shifts the pen without drawing.
                if let Some(n) = as_num(&other) {
                    let shift = -n / 1000.0 * ts.size * ts.h_scale;
                    ts.tm = mul(&[1.0, 0.0, 0.0, 1.0, shift, 0.0], &ts.tm);
                }
                rebuilt.push(other);
            }
        }
    }

    if removed_here == 0 {
        // Nothing to redact here — keep the original operator byte-for-byte.
        out.extend(prelude);
        if op.operator == "'" || op.operator == "\"" {
            out.push(Operation::new("Tj", op.operands.iter().last().cloned().into_iter().collect()));
        } else {
            out.push(op.clone());
        }
        return;
    }

    stats.glyphs_removed += removed_here;
    out.extend(prelude);
    out.push(Operation::new("TJ", vec![Object::Array(rebuilt)]));
}

/// Walk a shown string glyph by glyph, appending the surviving runs (and
/// position adjustments standing in for removed glyphs) to `rebuilt`.
///
/// Returns `(glyphs_kept, glyphs_removed)` and advances `ts.tm` exactly as the
/// original operator would have.
fn scan_string(
    bytes: &[u8],
    ts: &mut TextState,
    ctm: &Matrix,
    rects: &[Rect],
    rebuilt: &mut Vec<Object>,
    fmt: lopdf::StringFormat,
) -> (usize, usize) {
    let step = if ts.two_byte { 2 } else { 1 };
    let width_em = if ts.two_byte { WIDTH_EM_CID } else { WIDTH_EM };

    let mut run: Vec<u8> = Vec::new();
    let mut pending_shift = 0.0f64;
    let mut kept = 0usize;
    let mut removed = 0usize;

    let flush_run = |run: &mut Vec<u8>, rebuilt: &mut Vec<Object>| {
        if !run.is_empty() {
            rebuilt.push(Object::String(std::mem::take(run), fmt));
        }
    };

    let mut i = 0;
    while i < bytes.len() {
        let end = (i + step).min(bytes.len());
        let glyph = &bytes[i..end];
        let is_space = step == 1 && glyph == [b' '];

        // Glyph box in em units, padded to absorb the width approximation.
        let trm = mul(
            &[ts.size * ts.h_scale, 0.0, 0.0, ts.size, 0.0, ts.rise],
            &mul(&ts.tm, ctm),
        );
        let bounds = transformed_bounds(
            &trm,
            -PAD_EM,
            -0.25 - PAD_EM,
            width_em + PAD_EM,
            0.80 + PAD_EM,
        );
        let hit = rects.iter().any(|r| r.intersects(&bounds));

        // Advance, in unscaled text-space units.
        let advance = (width_em * ts.size
            + ts.char_spacing
            + if is_space { ts.word_spacing } else { 0.0 })
            * ts.h_scale;

        if hit {
            removed += 1;
            flush_run(&mut run, rebuilt);
            pending_shift += advance;
        } else {
            if pending_shift != 0.0 {
                // TJ numbers are thousandths of a text-space unit, scaled by
                // the font size and horizontal scaling.
                let denom = ts.size * ts.h_scale;
                if denom.abs() > f64::EPSILON {
                    rebuilt.push(Object::Real((-pending_shift * 1000.0 / denom) as f32));
                }
                pending_shift = 0.0;
            }
            run.extend_from_slice(glyph);
            kept += 1;
        }

        ts.tm = mul(&[1.0, 0.0, 0.0, 1.0, advance, 0.0], &ts.tm);
        i = end;
    }

    flush_run(&mut run, rebuilt);
    if pending_shift != 0.0 {
        let denom = ts.size * ts.h_scale;
        if denom.abs() > f64::EPSILON {
            rebuilt.push(Object::Real((-pending_shift * 1000.0 / denom) as f32));
        }
    }

    (kept, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER: Rect = Rect { x0: 0.0, y0: 0.0, x1: 612.0, y1: 792.0 };

    fn region(x: f32, y: f32, w: f32, h: f32) -> RedactRegion {
        RedactRegion { page: 1, x, y, w, h }
    }

    #[test]
    fn unrotated_region_flips_the_y_axis() {
        // Top-left eighth of the page.
        let r = to_user_space(&region(0.0, 0.0, 0.25, 0.25), &LETTER, 0);
        assert!((r.x0 - 0.0).abs() < 0.01);
        assert!((r.x1 - 153.0).abs() < 0.01);
        // Top of the page is the *high* y in user space.
        assert!((r.y1 - 792.0).abs() < 0.01);
        assert!((r.y0 - 594.0).abs() < 0.01);
    }

    #[test]
    fn quarter_turns_swap_the_axes() {
        // On a 90°-rotated page the displayed width is the page height.
        let r = to_user_space(&region(0.0, 0.0, 0.5, 1.0), &LETTER, 90);
        assert!((r.x0 - 0.0).abs() < 0.01);
        assert!((r.x1 - 612.0).abs() < 0.01);
        assert!((r.y0 - 0.0).abs() < 0.01);
        assert!((r.y1 - 396.0).abs() < 0.01);
    }

    #[test]
    fn matrix_multiply_matches_pdf_order() {
        let scale = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let shift = [1.0, 0.0, 0.0, 1.0, 10.0, 5.0];
        // Scale then translate: the translation is not scaled.
        let m = mul(&scale, &shift);
        assert_eq!(apply(&m, 1.0, 1.0), (12.0, 7.0));
    }

    #[test]
    fn rects_intersect_only_when_they_overlap() {
        let a = Rect { x0: 0.0, y0: 0.0, x1: 10.0, y1: 10.0 };
        let b = Rect { x0: 5.0, y0: 5.0, x1: 15.0, y1: 15.0 };
        let c = Rect { x0: 20.0, y0: 20.0, x1: 30.0, y1: 30.0 };
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }
}
