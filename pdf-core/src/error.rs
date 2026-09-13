use std::path::PathBuf;

/// Errors produced by pdf-core operations.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("PDF parse/build error: {0}")]
    Pdf(#[from] lopdf::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("no input files provided")]
    NoInput,

    #[error("page range {0} is out of bounds (document has {1} pages)")]
    PageOutOfRange(String, usize),
}

pub type Result<T> = std::result::Result<T, PdfError>;

impl PdfError {
    /// Helper to attach a path to an [`std::io::Error`].
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        PdfError::Io {
            path: path.into(),
            source,
        }
    }
}
