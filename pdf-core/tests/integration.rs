use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use lopdf::Document;
use printpdf::{Mm, PdfDocument};

/// Make a blank `pages`-page A4 PDF at `path` for use as test input.
fn make_pdf(path: &Path, pages: usize) {
    let (doc, _p, _l) = PdfDocument::new("test", Mm(210.0), Mm(297.0), "layer");
    for _ in 1..pages {
        doc.add_page(Mm(210.0), Mm(297.0), "layer");
    }
    let file = File::create(path).unwrap();
    doc.save(&mut BufWriter::new(file)).unwrap();
}

fn page_count(path: &Path) -> usize {
    Document::load(path).unwrap().get_pages().len()
}

#[test]
fn merge_concatenates_page_counts() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    let b = dir.path().join("b.pdf");
    let out = dir.path().join("merged.pdf");
    make_pdf(&a, 2);
    make_pdf(&b, 3);

    pdf_core::merge(&[&a, &b], &out).unwrap();

    assert_eq!(page_count(&out), 5);
}

#[test]
fn extract_selects_range() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");
    make_pdf(&src, 10);

    pdf_core::extract(&src, &out, "2-4,7").unwrap();

    assert_eq!(page_count(&out), 4);
}

#[test]
fn split_each_page_bursts() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 4);

    let outputs = pdf_core::split_each_page(&src, dir.path()).unwrap();

    assert_eq!(outputs.len(), 4);
    for o in &outputs {
        assert_eq!(page_count(o), 1);
    }
}

#[test]
fn rotate_sets_page_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");
    make_pdf(&src, 3);

    pdf_core::rotate(&src, &out, 90, Some("1,2")).unwrap();

    let doc = Document::load(&out).unwrap();
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    let rot = |id| {
        doc.get_object(id)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
    };
    assert_eq!(rot(pages[0]), Some(90));
    assert_eq!(rot(pages[1]), Some(90));
    // page 3 was not selected, so it keeps its original rotation (0)
    assert_eq!(rot(pages[2]).unwrap_or(0), 0);
}

#[test]
fn scrub_removes_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");
    make_pdf(&src, 1);

    // printpdf writes a producer/creator, so there is something to scrub.
    let before = pdf_core::read_metadata(&src).unwrap();
    assert!(!before.is_empty());

    pdf_core::scrub_metadata(&src, &out).unwrap();

    let after = pdf_core::read_metadata(&out).unwrap();
    assert!(after.is_empty(), "metadata should be gone: {after:?}");
}

#[test]
fn split_every_chunks_pages() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 5);

    let outputs = pdf_core::split_every(&src, dir.path(), 2).unwrap();

    assert_eq!(outputs.len(), 3); // 2 + 2 + 1
    assert_eq!(page_count(&outputs[0]), 2);
    assert_eq!(page_count(&outputs[2]), 1);
}

#[test]
fn split_at_points_makes_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 6);

    // split before pages 3 and 5 -> 1-2, 3-4, 5-6
    let outputs = pdf_core::split_at(&src, dir.path(), &[3, 5]).unwrap();

    assert_eq!(outputs.len(), 3);
    for o in &outputs {
        assert_eq!(page_count(o), 2);
    }
}

#[test]
fn encrypt_then_decrypt_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let enc = dir.path().join("enc.pdf");
    let dec = dir.path().join("dec.pdf");
    make_pdf(&src, 3);

    let opts = pdf_core::EncryptOptions { user_password: "secret".into(), ..Default::default() };
    pdf_core::encrypt(&src, &enc, &opts).unwrap();

    // wrong password fails
    assert!(pdf_core::decrypt(&enc, &dec, "wrong").is_err());
    // correct password restores readable pages
    pdf_core::decrypt(&enc, &dec, "secret").unwrap();
    assert_eq!(page_count(&dec), 3);
}

#[test]
fn watermark_keeps_pages_and_loads() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("wm.pdf");
    make_pdf(&src, 3);

    let opts = pdf_core::WatermarkOptions { text: "CONFIDENTIAL".into(), ..Default::default() };
    pdf_core::watermark(&src, &out, &opts).unwrap();

    // output still loads and keeps all pages
    assert_eq!(page_count(&out), 3);
    // and it grew (watermark content was added)
    let before = std::fs::metadata(&src).unwrap().len();
    let after = std::fs::metadata(&out).unwrap().len();
    assert!(after > before, "watermarked file should be larger");
}

#[test]
fn batch_isolates_failures() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.pdf");
    make_pdf(&good, 1);
    let bad = dir.path().join("does-not-exist.pdf");

    let report = pdf_core::run_batch(vec![good.clone(), bad.clone()], |p| {
        let out = p.with_extension("out.pdf");
        pdf_core::extract(p, &out, "1")?;
        Ok(vec![out])
    });

    assert_eq!(report.succeeded(), 1);
    assert_eq!(report.failed(), 1);
}

/// Build a one-page A4 PDF with `lines` of text, each `(text, y_from_bottom_mm)`.
fn make_text_pdf(path: &Path, lines: &[(&str, f32)]) {
    let (doc, page, layer) = PdfDocument::new("test", Mm(210.0), Mm(297.0), "layer");
    let font = doc.add_builtin_font(printpdf::BuiltinFont::Helvetica).unwrap();
    let current = doc.get_page(page).get_layer(layer);
    for (text, y) in lines {
        current.use_text(*text, 14.0, Mm(20.0), Mm(*y), &font);
    }
    let file = File::create(path).unwrap();
    doc.save(&mut BufWriter::new(file)).unwrap();
}

/// Every string actually drawn by the document's content streams. Reading the
/// decoded operands (rather than the raw bytes) keeps the assertion honest
/// whichever way a writer encodes its strings — literal or hex.
fn drawn_text(path: &Path) -> Vec<u8> {
    let doc = Document::load(path).unwrap();
    let mut out = Vec::new();
    for page_id in doc.get_pages().values() {
        let content = doc.get_page_content(*page_id).unwrap();
        for op in lopdf::content::Content::decode(&content).unwrap().operations {
            for operand in &op.operands {
                match operand {
                    lopdf::Object::String(bytes, _) => out.extend_from_slice(bytes),
                    lopdf::Object::Array(items) => {
                        for item in items {
                            if let lopdf::Object::String(bytes, _) = item {
                                out.extend_from_slice(bytes);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

#[test]
fn redact_removes_text_from_the_file_not_just_the_view() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");
    // A4 is 297 mm tall: 280 mm up is near the top, 20 mm up is near the bottom.
    make_text_pdf(&src, &[("SECRETVALUE", 280.0), ("KEEPTHISLINE", 20.0)]);
    assert!(contains(&drawn_text(&src), b"SECRETVALUE"));

    // Redact the top 15% of the page.
    let region = pdf_core::RedactRegion { page: 1, x: 0.0, y: 0.0, w: 1.0, h: 0.15 };
    let stats = pdf_core::redact(&src, &out, &[region]).unwrap();

    let content = drawn_text(&out);
    assert!(!contains(&content, b"SECRETVALUE"), "redacted text is still in the file");
    assert!(contains(&content, b"KEEPTHISLINE"), "text outside the region was destroyed");
    assert!(stats.glyphs_removed >= "SECRETVALUE".len());
}

#[test]
fn redact_rejects_an_out_of_range_page() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");
    make_pdf(&src, 2);

    let region = pdf_core::RedactRegion { page: 5, x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    assert!(pdf_core::redact(&src, &out, &[region]).is_err());
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn redact_follows_page_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let rotated = dir.path().join("rotated.pdf");
    let out = dir.path().join("out.pdf");
    make_text_pdf(&src, &[("SECRETVALUE", 280.0), ("KEEPTHISLINE", 20.0)]);

    // Mark the page as displayed a quarter turn clockwise. A viewer now shows
    // an A4 page on its side, so the region the user drags is in a different
    // coordinate space than the one the text lives in.
    {
        let mut doc = Document::load(&src).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        doc.get_object_mut(page_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Rotate", 90i64);
        doc.save(&rotated).unwrap();
    }

    // On the rotated page the text sits near the right edge, a little down
    // from the top — nowhere near the top-left corner it occupies unrotated.
    let region = pdf_core::RedactRegion { page: 1, x: 0.88, y: 0.0, w: 0.12, h: 0.32 };
    let stats = pdf_core::redact(&rotated, &out, &[region]).unwrap();

    let content = drawn_text(&out);
    assert!(
        !contains(&content, b"SECRETVALUE"),
        "rotation was not accounted for: the targeted text survived"
    );
    // The negative control: without this, a redaction that simply wiped the
    // whole page would pass the assertion above.
    assert!(
        contains(&content, b"KEEPTHISLINE"),
        "rotation was mapped to the wrong area: untargeted text was destroyed"
    );
    assert!(stats.glyphs_removed >= "SECRETVALUE".len());
}

/// Build a two-page PDF whose pages both draw the *same* Form XObject — the
/// shape a generator produces for a shared header or stamp.
fn make_shared_form_pdf(path: &Path) {
    use lopdf::{dictionary, Object, Stream};

    let mut doc = Document::with_version("1.5");
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let font_res = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });

    // The form paints one line of text near the top of the page.
    let form = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => font_res,
        },
        b"BT /F1 14 Tf 60 750 Td (SHAREDHEADER) Tj ET".to_vec(),
    ));

    let resources = doc.add_object(dictionary! { "XObject" => dictionary! { "Fm0" => form } });
    let pages_id = doc.new_object_id();

    let mut page_ids = Vec::new();
    for _ in 0..2 {
        let content = doc.add_object(Stream::new(dictionary! {}, b"/Fm0 Do".to_vec()));
        page_ids.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content,
            "Resources" => resources,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        })));
    }

    let count = page_ids.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages", "Count" => count, "Kids" => page_ids,
    }));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.save(path).unwrap();
}

/// Text drawn by one specific page, following the Form XObjects it invokes.
fn drawn_text_on_page(path: &Path, page_no: usize) -> Vec<u8> {
    let doc = Document::load(path).unwrap();
    let page_id = doc.get_pages()[&(page_no as u32)];
    let mut out = doc.get_page_content(page_id).unwrap();

    // Follow this page's form references and append what they draw.
    let resources = doc
        .get_dictionary(page_id)
        .unwrap()
        .get(b"Resources")
        .map(|o| doc.dereference(o).unwrap().1.as_dict().unwrap().clone())
        .unwrap();
    if let Ok(xobjects) = resources.get(b"XObject").map(|o| doc.dereference(o).unwrap().1.as_dict().unwrap().clone()) {
        for (_, value) in xobjects.iter() {
            let id = value.as_reference().unwrap();
            let stream = doc.get_object(id).unwrap().as_stream().unwrap();
            out.extend(stream.decompressed_content().unwrap_or_else(|_| stream.content.clone()));
        }
    }
    out
}

#[test]
fn redacting_a_shared_form_leaves_other_pages_alone() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("shared.pdf");
    let out = dir.path().join("out.pdf");
    make_shared_form_pdf(&src);
    assert!(contains(&drawn_text_on_page(&src, 1), b"SHAREDHEADER"));
    assert!(contains(&drawn_text_on_page(&src, 2), b"SHAREDHEADER"));

    // Redact the header on page 1 only.
    let region = pdf_core::RedactRegion { page: 1, x: 0.0, y: 0.0, w: 1.0, h: 0.12 };
    pdf_core::redact(&src, &out, &[region]).unwrap();

    assert!(
        !contains(&drawn_text_on_page(&out, 1), b"SHAREDHEADER"),
        "the redacted page kept its header"
    );
    assert!(
        contains(&drawn_text_on_page(&out, 2), b"SHAREDHEADER"),
        "editing the shared form blanked the header on an untouched page"
    );
}

#[test]
fn cover_box_ignores_a_transform_the_page_left_in_force() {
    use lopdf::{dictionary, Object, Stream};

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.pdf");
    let out = dir.path().join("out.pdf");

    // A `cm` with no matching `q`/`Q` — exactly what Skia-produced PDFs (and
    // plenty of others) leave in effect at the end of a page.
    let mut doc = Document::with_version("1.5");
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"0.25 0 0 0.25 10 10 cm 0 0 100 100 re f".to_vec(),
    ));
    let pages_id = doc.new_object_id();
    let page = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content,
        "Resources" => dictionary! {},
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page)],
    }));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.save(&src).unwrap();

    // Redact the whole page; the cover box should span the whole MediaBox.
    let region = pdf_core::RedactRegion { page: 1, x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    pdf_core::redact(&src, &out, &[region]).unwrap();

    let result = Document::load(&out).unwrap();
    let page_id = result.get_pages()[&1];
    let ops = lopdf::content::Content::decode(&result.get_page_content(page_id).unwrap())
        .unwrap()
        .operations;

    // Walk the stream the way a reader would, tracking the transform, and
    // check what is in force when our cover rectangle is drawn. The operands
    // were always in page space; the bug was that a stale `cm` scaled them.
    let ctm = ctm_at_last_rect(&ops);
    assert_eq!(
        ctm,
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        "the cover box is drawn under a leftover transform, so it will not          line up with the area the user selected"
    );
}

/// The current transformation matrix in effect at the final `re` operator.
fn ctm_at_last_rect(ops: &[lopdf::content::Operation]) -> [f64; 6] {
    fn mul(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
        [
            a[0] * b[0] + a[1] * b[2],
            a[0] * b[1] + a[1] * b[3],
            a[2] * b[0] + a[3] * b[2],
            a[2] * b[1] + a[3] * b[3],
            a[4] * b[0] + a[5] * b[2] + b[4],
            a[4] * b[1] + a[5] * b[3] + b[5],
        ]
    }
    let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let last_rect = ops.iter().rposition(|op| op.operator == "re").expect("no cover rectangle");

    let mut ctm = identity;
    let mut stack: Vec<[f64; 6]> = Vec::new();
    for op in &ops[..=last_rect] {
        match op.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(identity),
            "cm" => {
                let v: Vec<f64> = op.operands.iter().filter_map(|o| match o {
                    lopdf::Object::Integer(i) => Some(*i as f64),
                    lopdf::Object::Real(r) => Some(*r as f64),
                    _ => None,
                }).collect();
                if v.len() == 6 {
                    ctm = mul(&[v[0], v[1], v[2], v[3], v[4], v[5]], &ctm);
                }
            }
            _ => {}
        }
    }
    ctm
}
