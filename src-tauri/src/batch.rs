//! Running one operation across many files, with a per-file pass/fail report.
//!
//! The engine already isolates failures (see `pdf_core::batch`); this module
//! adapts that to the GUI: it names the outputs, reports progress, and returns
//! a result per input so one bad file never looks like a total failure.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tauri::{AppHandle, Emitter};

/// The operations that make sense to apply to a folder full of PDFs.
///
/// Deliberately a subset: tools whose settings mean the same thing for every
/// file. Merge (one output) and Organize (per-file page choices) are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Compress,
    Rotate,
    Scrub,
    Watermark,
    Protect,
    Unlock,
    Ocr,
}

impl Op {
    fn parse(s: &str) -> Result<Op, String> {
        match s {
            "compress" => Ok(Op::Compress),
            "rotate" => Ok(Op::Rotate),
            "scrub" => Ok(Op::Scrub),
            "watermark" => Ok(Op::Watermark),
            "protect" => Ok(Op::Protect),
            "unlock" => Ok(Op::Unlock),
            "ocr" => Ok(Op::Ocr),
            other => Err(format!("unknown batch operation: {other}")),
        }
    }

    /// Appended to each input's name so a result never overwrites its source,
    /// even when the output folder is the input folder.
    fn suffix(self) -> &'static str {
        match self {
            Op::Compress => "-compressed",
            Op::Rotate => "-rotated",
            Op::Scrub => "-clean",
            Op::Watermark => "-watermarked",
            Op::Protect => "-protected",
            Op::Unlock => "-unlocked",
            Op::Ocr => "-searchable",
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    op: String,
    inputs: Vec<String>,
    outdir: String,
    // Per-operation settings; only the ones the chosen op needs are read.
    level: Option<String>,
    degrees: Option<i64>,
    range: Option<String>,
    password: Option<String>,
    owner_password: Option<String>,
    bits: Option<u32>,
    text: Option<String>,
    opacity: Option<f32>,
    tiled: Option<bool>,
    lang: Option<String>,
    dpi: Option<f32>,
    force: Option<bool>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemDto {
    name: String,
    ok: bool,
    /// Output path on success, error message on failure.
    detail: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchReportDto {
    succeeded: usize,
    failed: usize,
    items: Vec<BatchItemDto>,
    outdir: String,
}

/// Apply one operation to every input, writing results into `outdir`.
///
/// Never fails as a whole for a per-file problem: a corrupt or password-locked
/// file is reported in its own row and the rest still run.
#[tauri::command]
pub fn run_batch(app: AppHandle, req: BatchRequest) -> Result<BatchReportDto, String> {
    let op = Op::parse(&req.op)?;
    if req.inputs.is_empty() {
        return Err("Add some PDFs first.".into());
    }

    let outdir = PathBuf::from(&req.outdir);
    std::fs::create_dir_all(&outdir).map_err(|e| format!("could not use that folder: {e}"))?;

    let total = req.inputs.len();
    let done = AtomicUsize::new(0);

    let report = pdf_core::run_batch(req.inputs.clone(), |input| {
        let out = target(&outdir, input, op);
        let result = apply(op, input, &out, &req);

        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = app.emit(
            "batch-progress",
            serde_json::json!({ "done": n, "total": total, "label": "Processing" }),
        );

        result.map(|_| vec![out])
    });

    let items = report
        .results
        .iter()
        .map(|r| BatchItemDto {
            name: r
                .input
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| r.input.to_string_lossy().into_owned()),
            ok: r.is_ok(),
            detail: match &r.outcome {
                Ok(outs) => outs
                    .first()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                Err(e) => e.clone(),
            },
        })
        .collect();

    Ok(BatchReportDto {
        succeeded: report.succeeded(),
        failed: report.failed(),
        items,
        outdir: req.outdir,
    })
}

/// `<outdir>/<stem><suffix>.pdf` for one input.
fn target(outdir: &Path, input: &Path, op: Op) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    outdir.join(format!("{stem}{}.pdf", op.suffix()))
}

fn apply(op: Op, input: &Path, out: &Path, req: &BatchRequest) -> pdf_core::Result<()> {
    match op {
        Op::Compress => {
            let level = pdf_core::CompressLevel::parse(req.level.as_deref().unwrap_or("balanced"))?;
            pdf_core::compress(input, out, level)?;
        }
        Op::Rotate => {
            let range = req.range.as_deref().map(str::trim).filter(|s| !s.is_empty());
            pdf_core::rotate(input, out, req.degrees.unwrap_or(90), range)?;
        }
        Op::Scrub => pdf_core::scrub_metadata(input, out)?,
        Op::Watermark => {
            let opts = pdf_core::WatermarkOptions {
                text: req.text.clone().unwrap_or_default(),
                font_size: 48.0,
                opacity: req.opacity.unwrap_or(0.3),
                color: (0.5, 0.5, 0.5),
                rotation: 45.0,
                tiled: req.tiled.unwrap_or(false),
                pages: req.range.clone(),
            };
            pdf_core::watermark(input, out, &opts)?;
        }
        Op::Protect => {
            let opts = pdf_core::EncryptOptions {
                user_password: req.password.clone().unwrap_or_default(),
                owner_password: req.owner_password.clone().unwrap_or_default(),
                bits: req.bits.unwrap_or(256),
                allow_print: true,
                allow_copy: true,
                allow_modify: true,
            };
            pdf_core::encrypt(input, out, &opts)?;
        }
        Op::Unlock => {
            pdf_core::decrypt(input, out, req.password.as_deref().unwrap_or(""))?;
        }
        Op::Ocr => {
            let opts = pdf_core::OcrOptions {
                lang: req.lang.clone().unwrap_or_else(|| "eng".to_string()),
                dpi: req.dpi.unwrap_or(300.0),
                force: req.force.unwrap_or(false),
            };
            pdf_core::ocr(input, out, &opts)?;
        }
    }
    Ok(())
}
