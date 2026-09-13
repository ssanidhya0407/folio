//! # pdf-core
//!
//! Offline, native PDF operations with no network access and no AGPL
//! dependencies. This crate is the engine; UIs (Tauri, CLI) sit on top.
//!
//! ## Implemented (pure Rust)
//! - [`merge`] — concatenate PDFs in a given order
//! - [`split`] — extract page ranges, or burst into one file per page
//! - [`compress`] — downsample/re-encode images + deflate streams
//! - [`rotate`] — rotate pages (additive, multiples of 90°)
//! - [`pages`] — reorder / delete pages by an explicit new page order
//! - [`metadata`] — read or scrub document metadata (`/Info` + XMP)
//! - [`image_to_pdf`] — combine images into a PDF, one per page
//! - [`pdf_to_png`] — rasterize each page to a PNG (PDFium)
//! - [`redact`] — delete text/images inside a region, then cover it
//! - [`batch`] — parallel, fault-isolated batch processing (rayon)
//!
//! ## Native-backed (bundled libs — see README)
//! - [`compress`] — qpdf (lossless) + image downsampling (lossy)
//! - [`encrypt`] — password protection / removal via qpdf (lopdf is decrypt-only)
//! - [`ocr`] — scanned pages → searchable text layer via Tesseract

pub mod batch;
pub mod compress;
pub mod encrypt;
pub mod error;
pub mod image_to_pdf;
pub(crate) mod io;
pub(crate) mod qpdf;
pub mod merge;
pub mod metadata;
pub mod ocr;
pub mod pages;
pub mod pdf_to_png;
pub mod range;
pub mod redact;
pub mod rotate;
pub mod split;
pub mod tesseract;
pub mod watermark;

pub use error::{PdfError, Result};

// Convenience re-exports of the primary entry points.
pub use batch::{run_batch, BatchItemResult, BatchReport};
pub use compress::{compress, CompressLevel, CompressStats};
pub use encrypt::{decrypt, encrypt, EncryptOptions};
pub use image_to_pdf::{images_to_pdf, PageSize};
pub use merge::merge;
pub use metadata::{read_metadata, scrub_metadata, set_metadata, Metadata};
pub use ocr::{has_text, ocr, ocr_with_progress, OcrOptions, OcrStats};
pub use pages::reorder;
pub use pdf_to_png::{page_count, pdf_to_pngs, render_thumbnails, ImgFormat};
pub use range::PageRange;
pub use redact::{redact, RedactRegion, RedactStats};
pub use rotate::rotate;
pub use split::{extract, split_at, split_each_page, split_every};
pub use watermark::{watermark, WatermarkOptions};
