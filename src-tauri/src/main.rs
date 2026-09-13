// Prevent a console window from popping up on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod batch;
mod settings;

use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "tiff", "webp"];

// ---------------------------------------------------------------------------
// File pickers (native dialogs, run on a command thread)
// ---------------------------------------------------------------------------

/// Start every dialog in the folder the user last used, so folio stops
/// dumping people in their home directory on every operation.
fn dialog(app: &AppHandle, saving: bool) -> FileDialogBuilder<tauri::Wry> {
    let builder = app.dialog().file();
    match settings::start_dir(app, saving) {
        Some(dir) => builder.set_directory(dir),
        None => builder,
    }
}

#[tauri::command]
fn browse_pdfs(app: AppHandle) -> Vec<String> {
    let picked: Vec<String> = dialog(&app, false)
        .add_filter("PDF", &["pdf"])
        .blocking_pick_files()
        .map(|fs| fs.into_iter().filter_map(to_string).collect())
        .unwrap_or_default();
    if let Some(first) = picked.first() {
        settings::remember_open(&app, first);
    }
    picked
}

#[tauri::command]
fn browse_pdf(app: AppHandle) -> Option<String> {
    let picked = dialog(&app, false)
        .add_filter("PDF", &["pdf"])
        .blocking_pick_file()
        .and_then(to_string);
    if let Some(p) = &picked {
        settings::remember_open(&app, p);
    }
    picked
}

#[tauri::command]
fn browse_images(app: AppHandle) -> Vec<String> {
    let picked: Vec<String> = dialog(&app, false)
        .add_filter("Images", IMAGE_EXTS)
        .blocking_pick_files()
        .map(|fs| fs.into_iter().filter_map(to_string).collect())
        .unwrap_or_default();
    if let Some(first) = picked.first() {
        settings::remember_open(&app, first);
    }
    picked
}

#[tauri::command]
fn save_pdf(app: AppHandle, default_name: String) -> Option<String> {
    let picked = dialog(&app, true)
        .add_filter("PDF", &["pdf"])
        .set_file_name(&default_name)
        .blocking_save_file()
        .and_then(to_string);
    if let Some(p) = &picked {
        settings::remember_save(&app, p, false);
    }
    picked
}

#[tauri::command]
fn choose_folder(app: AppHandle) -> Option<String> {
    let picked = dialog(&app, true).blocking_pick_folder().and_then(to_string);
    if let Some(p) = &picked {
        settings::remember_save(&app, p, true);
    }
    picked
}

// ---------------------------------------------------------------------------
// Inspection
// ---------------------------------------------------------------------------

/// Files handed to folio on the command line — "Open with", or a PDF dropped
/// onto the executable. The UI asks for these once, at startup.
#[tauri::command]
fn startup_files() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .filter(|a| Path::new(a).is_file())
        .collect()
}

#[tauri::command]
fn is_pdf(path: String) -> bool {
    Path::new(&path)
        .extension()
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

#[tauri::command]
fn page_count(path: String) -> Result<usize, String> {
    pdf_core::page_count(&path).map_err(|e| e.to_string())
}

/// Render up to `max_pages` pages as base64 PNG data URLs at `width` px wide,
/// for the in-app PDF viewer. The UI asks for the display's real pixel width,
/// so the upper bound has to leave room for a 2x HiDPI screen.
#[tauri::command]
fn render_view(path: String, max_pages: usize, width: u32) -> Result<Vec<String>, String> {
    let pngs = pdf_core::render_thumbnails(&path, max_pages, width.clamp(80, 3000))
        .map_err(|e| e.to_string())?;
    let engine = base64::engine::general_purpose::STANDARD;
    Ok(pngs
        .into_iter()
        .map(|bytes| format!("data:image/png;base64,{}", engine.encode(bytes)))
        .collect())
}

// ---------------------------------------------------------------------------
// Operations (explicit paths; the UI handles file selection + drag-drop)
// ---------------------------------------------------------------------------

#[tauri::command]
fn run_merge(inputs: Vec<String>, output: String) -> Result<String, String> {
    let paths: Vec<PathBuf> = inputs.iter().map(PathBuf::from).collect();
    pdf_core::merge(&paths, Path::new(&output)).map_err(|e| e.to_string())?;
    Ok(format!("Merged {} files.", paths.len()))
}

#[tauri::command]
fn run_split(input: String, output: String, range: String) -> Result<String, String> {
    pdf_core::extract(&input, Path::new(&output), &range).map_err(|e| e.to_string())?;
    Ok(format!("Extracted pages {range}."))
}

#[tauri::command]
fn run_burst(input: String, outdir: String) -> Result<String, String> {
    let outs = pdf_core::split_each_page(&input, Path::new(&outdir)).map_err(|e| e.to_string())?;
    Ok(format!("Split into {} single-page PDFs.", outs.len()))
}

#[tauri::command]
fn run_split_every(input: String, outdir: String, n: usize) -> Result<String, String> {
    let outs = pdf_core::split_every(&input, Path::new(&outdir), n).map_err(|e| e.to_string())?;
    Ok(format!("Split into {} files of up to {n} page(s).", outs.len()))
}

#[tauri::command]
fn run_split_at(input: String, outdir: String, points: Vec<usize>) -> Result<String, String> {
    let outs = pdf_core::split_at(&input, Path::new(&outdir), &points).map_err(|e| e.to_string())?;
    Ok(format!("Split into {} files.", outs.len()))
}

#[tauri::command]
fn run_rotate(input: String, output: String, degrees: i64, range: Option<String>) -> Result<String, String> {
    let spec = range.as_deref().map(str::trim).filter(|s| !s.is_empty());
    pdf_core::rotate(&input, Path::new(&output), degrees, spec).map_err(|e| e.to_string())?;
    Ok(format!("Rotated {degrees}°."))
}

#[tauri::command]
fn run_reorder(input: String, output: String, order: Vec<usize>) -> Result<String, String> {
    pdf_core::reorder(&input, Path::new(&output), &order).map_err(|e| e.to_string())?;
    Ok(format!("Saved {} page(s) in the new order.", order.len()))
}

#[tauri::command]
fn run_img2pdf(inputs: Vec<String>, output: String, page_size: String) -> Result<String, String> {
    let paths: Vec<PathBuf> = inputs.iter().map(PathBuf::from).collect();
    pdf_core::images_to_pdf(&paths, Path::new(&output), 300.0, pdf_core::PageSize::parse(&page_size))
        .map_err(|e| e.to_string())?;
    Ok(format!("Converted {} images into a PDF.", paths.len()))
}

#[tauri::command]
fn run_compress(input: String, output: String, level: String) -> Result<String, String> {
    let lvl = pdf_core::CompressLevel::parse(&level).map_err(|e| e.to_string())?;
    let stats = pdf_core::compress(&input, Path::new(&output), lvl).map_err(|e| e.to_string())?;
    let saved = stats.percent_saved();
    if saved < 1.0 {
        Ok(format!("Already optimized ({}) — no further savings.", human(stats.bytes_after)))
    } else {
        Ok(format!(
            "{} → {}  ({saved:.0}% smaller)",
            human(stats.bytes_before),
            human(stats.bytes_after)
        ))
    }
}

#[tauri::command]
fn run_topng(input: String, outdir: String, dpi: f32, format: String, range: Option<String>) -> Result<String, String> {
    let outs = pdf_core::pdf_to_pngs(
        &input,
        Path::new(&outdir),
        dpi,
        pdf_core::ImgFormat::parse(&format),
        range.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    Ok(format!("Saved {} image(s).", outs.len()))
}

fn human(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

// ----- metadata read/write -----
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaDto {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    mod_date: Option<String>,
}
impl From<pdf_core::Metadata> for MetaDto {
    fn from(m: pdf_core::Metadata) -> Self {
        MetaDto {
            title: m.title, author: m.author, subject: m.subject, keywords: m.keywords,
            creator: m.creator, producer: m.producer, creation_date: m.creation_date, mod_date: m.mod_date,
        }
    }
}
impl MetaDto {
    fn into_core(self) -> pdf_core::Metadata {
        pdf_core::Metadata {
            title: self.title, author: self.author, subject: self.subject, keywords: self.keywords,
            creator: self.creator, producer: self.producer, creation_date: self.creation_date, mod_date: self.mod_date,
        }
    }
}

#[tauri::command]
fn get_metadata(path: String) -> Result<MetaDto, String> {
    pdf_core::read_metadata(&path).map(MetaDto::from).map_err(|e| e.to_string())
}

#[tauri::command]
fn run_set_metadata(input: String, output: String, meta: MetaDto) -> Result<String, String> {
    pdf_core::set_metadata(&input, Path::new(&output), &meta.into_core()).map_err(|e| e.to_string())?;
    Ok("Saved metadata.".into())
}

// ----- encrypt / decrypt -----
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptDto {
    user_password: String,
    owner_password: String,
    bits: u32,
    allow_print: bool,
    allow_copy: bool,
    allow_modify: bool,
}

#[tauri::command]
fn run_encrypt(input: String, output: String, opts: EncryptDto) -> Result<String, String> {
    let o = pdf_core::EncryptOptions {
        user_password: opts.user_password,
        owner_password: opts.owner_password,
        bits: opts.bits,
        allow_print: opts.allow_print,
        allow_copy: opts.allow_copy,
        allow_modify: opts.allow_modify,
    };
    pdf_core::encrypt(Path::new(&input), Path::new(&output), &o).map_err(|e| e.to_string())?;
    Ok("Password protection added.".into())
}

#[tauri::command]
fn run_decrypt(input: String, output: String, password: String) -> Result<String, String> {
    pdf_core::decrypt(Path::new(&input), Path::new(&output), &password).map_err(|e| e.to_string())?;
    Ok("Password removed.".into())
}

// ----- watermark -----
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WatermarkDto {
    text: String,
    font_size: f32,
    opacity: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    rotation: f32,
    tiled: bool,
    pages: Option<String>,
}

#[tauri::command]
fn run_watermark(input: String, output: String, opts: WatermarkDto) -> Result<String, String> {
    let o = pdf_core::WatermarkOptions {
        text: opts.text,
        font_size: opts.font_size,
        opacity: opts.opacity,
        color: (opts.color_r, opts.color_g, opts.color_b),
        rotation: opts.rotation,
        tiled: opts.tiled,
        pages: opts.pages,
    };
    pdf_core::watermark(Path::new(&input), Path::new(&output), &o).map_err(|e| e.to_string())?;
    Ok("Watermark added.".into())
}

#[tauri::command]
fn run_scrub(input: String, output: String) -> Result<String, String> {
    pdf_core::scrub_metadata(&input, Path::new(&output)).map_err(|e| e.to_string())?;
    Ok("Removed all metadata.".into())
}

// ----- OCR -----

/// OCR languages installed next to the app, as `(code, human name)` pairs.
#[tauri::command]
fn ocr_languages() -> Vec<(String, String)> {
    pdf_core::tesseract::languages()
        .into_iter()
        .map(|code| {
            let name = language_name(&code);
            (code, name)
        })
        .collect()
}

#[tauri::command]
fn ocr_available() -> bool {
    pdf_core::tesseract::is_available()
}

/// Whether the PDF already has selectable text — the UI warns before OCR
/// replaces good text with pictures of text.
#[tauri::command]
fn pdf_has_text(path: String) -> Result<bool, String> {
    pdf_core::has_text(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn run_ocr(
    app: AppHandle,
    input: String,
    output: String,
    lang: String,
    dpi: f32,
    force: bool,
) -> Result<String, String> {
    let opts = pdf_core::OcrOptions { lang, dpi, force };
    let stats = pdf_core::ocr_with_progress(&input, Path::new(&output), &opts, |done, total| {
        progress(&app, "ocr-progress", done, total, "Reading pages");
    })
    .map_err(|e| e.to_string())?;

    if stats.chars == 0 {
        return Ok(format!(
            "Processed {} page(s), but no text was recognized — try a higher \
             resolution or a different language.",
            stats.pages
        ));
    }
    Ok(format!(
        "{} page(s) are now searchable ({} characters recognized).",
        stats.pages, stats.chars
    ))
}

// ----- Redact -----

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionDto {
    page: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[tauri::command]
fn run_redact(
    input: String,
    output: String,
    regions: Vec<RegionDto>,
    color: Option<String>,
) -> Result<String, String> {
    let regions: Vec<pdf_core::RedactRegion> = regions
        .into_iter()
        .map(|r| pdf_core::RedactRegion {
            page: r.page,
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        })
        .collect();

    let opts = pdf_core::RedactOptions {
        color: color.as_deref().and_then(hex_rgb).unwrap_or((1.0, 1.0, 1.0)),
    };
    let stats = pdf_core::redact_with(&input, Path::new(&output), &regions, &opts)
        .map_err(|e| e.to_string())?;

    let mut parts = vec![format!("{} glyph(s)", stats.glyphs_removed)];
    if stats.images_removed > 0 {
        parts.push(format!("{} image(s)", stats.images_removed));
    }
    if stats.annotations_removed > 0 {
        parts.push(format!("{} link(s)/field(s)", stats.annotations_removed));
    }
    Ok(format!(
        "Permanently removed {} from {} page(s).",
        parts.join(", "),
        stats.pages
    ))
}

/// `#rrggbb` -> RGB in `0.0..=1.0`. `None` when the string is not a colour.
fn hex_rgb(spec: &str) -> Option<(f32, f32, f32)> {
    let hex = spec.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok().map(|v| v as f32 / 255.0);
    Some((channel(0)?, channel(2)?, channel(4)?))
}

// ----- About / trust -----

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AboutDto {
    version: String,
    license: String,
    ocr_available: bool,
    ocr_languages: usize,
}

#[tauri::command]
fn about() -> AboutDto {
    AboutDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
        license: "GPL-3.0-or-later".to_string(),
        ocr_available: pdf_core::tesseract::is_available(),
        ocr_languages: pdf_core::tesseract::languages().len(),
    }
}

/// Tell the UI how far along a long operation is.
fn progress(app: &AppHandle, event: &str, done: usize, total: usize, label: &str) {
    use tauri::Emitter;
    let _ = app.emit(
        event,
        serde_json::json!({ "done": done, "total": total, "label": label }),
    );
}

/// Human-readable names for the languages we ship. Unknown codes fall back to
/// the code itself, so a user-added `traineddata` file still shows up.
fn language_name(code: &str) -> String {
    let name = match code {
        "eng" => "English",
        "deu" => "German",
        "fra" => "French",
        "spa" => "Spanish",
        "ita" => "Italian",
        "por" => "Portuguese",
        "nld" => "Dutch",
        "rus" => "Russian",
        "pol" => "Polish",
        "tur" => "Turkish",
        "ara" => "Arabic",
        "hin" => "Hindi",
        "jpn" => "Japanese",
        "kor" => "Korean",
        "chi_sim" => "Chinese (Simplified)",
        "chi_tra" => "Chinese (Traditional)",
        _ => code,
    };
    name.to_string()
}

// ---------------------------------------------------------------------------
// Reveal / open results
// ---------------------------------------------------------------------------

#[tauri::command]
fn reveal(path: String) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{path}\""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = open_parent(&path);
    }
}

#[tauri::command]
fn open_path(path: String) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    }
}

#[cfg(not(windows))]
fn open_parent(path: &str) -> std::io::Result<std::process::Child> {
    let parent = Path::new(path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    std::process::Command::new("xdg-open").arg(parent).spawn()
}

// ---------------------------------------------------------------------------

fn to_string(f: tauri_plugin_dialog::FilePath) -> Option<String> {
    f.into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            browse_pdfs,
            browse_pdf,
            browse_images,
            save_pdf,
            choose_folder,
            startup_files,
            is_pdf,
            page_count,
            render_view,
            get_metadata,
            run_set_metadata,
            run_encrypt,
            run_decrypt,
            run_watermark,
            run_merge,
            run_split,
            run_burst,
            run_split_every,
            run_split_at,
            run_rotate,
            run_reorder,
            run_img2pdf,
            run_compress,
            run_topng,
            run_scrub,
            ocr_languages,
            ocr_available,
            pdf_has_text,
            run_ocr,
            run_redact,
            about,
            batch::run_batch,
            reveal,
            open_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running folio");
}
