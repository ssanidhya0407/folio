use std::path::Path;

use crate::error::{PdfError, Result};

/// Compression strength. We delegate to **qpdf** (Apache-2.0), which is robust
/// (it even recovers malformed PDFs) and never drops content. Stronger levels
/// additionally re-encode embedded images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressLevel {
    /// Lossless only: repack objects + recompress streams. No quality loss.
    Light,
    /// Lossless + optimize larger images (default).
    Balanced,
    /// Lossless + optimize all images, including small ones. Smallest files.
    Strong,
}

impl CompressLevel {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" | "low" => Ok(CompressLevel::Light),
            "balanced" | "medium" | "med" => Ok(CompressLevel::Balanced),
            "strong" | "high" => Ok(CompressLevel::Strong),
            other => Err(PdfError::Invalid(format!(
                "unknown compression level {other:?} (use light|balanced|strong)"
            ))),
        }
    }

    /// Extra qpdf args for image handling, beyond the lossless baseline.
    fn image_args(self) -> &'static [&'static str] {
        match self {
            CompressLevel::Light => &[],
            CompressLevel::Balanced => &["--optimize-images"],
            CompressLevel::Strong => &[
                "--optimize-images",
                "--oi-min-width=0",
                "--oi-min-height=0",
                "--oi-min-area=0",
            ],
        }
    }
}

/// Result of a compression pass.
#[derive(Debug, Clone, Copy)]
pub struct CompressStats {
    pub bytes_before: u64,
    pub bytes_after: u64,
}

impl CompressStats {
    /// Percentage saved (0..=100), 0 if it didn't shrink.
    pub fn percent_saved(&self) -> f64 {
        if self.bytes_before == 0 || self.bytes_after >= self.bytes_before {
            return 0.0;
        }
        (1.0 - self.bytes_after as f64 / self.bytes_before as f64) * 100.0
    }
}

/// Compress `input` into `output` using bundled qpdf. Never returns a file
/// larger than the input (keeps the original bytes if qpdf can't shrink it).
pub fn compress<P: AsRef<Path>>(
    input: P,
    output: &Path,
    level: CompressLevel,
) -> Result<CompressStats> {
    let input = input.as_ref();
    let bytes_before = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);

    let mut args: Vec<String> = vec![
        "--object-streams=generate".into(),
        "--recompress-flate".into(),
        "--compression-level=9".into(),
    ];
    for a in level.image_args() {
        args.push((*a).to_string());
    }
    args.push(input.to_string_lossy().into_owned());
    args.push(output.to_string_lossy().into_owned());
    crate::qpdf::run(&args)?;

    let mut bytes_after = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);

    // Inflation guard — never hand back a larger file.
    if bytes_before > 0 && bytes_after >= bytes_before {
        std::fs::copy(input, output).map_err(|e| PdfError::io(output, e))?;
        bytes_after = bytes_before;
    }

    Ok(CompressStats {
        bytes_before,
        bytes_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_level_aliases() {
        assert_eq!(CompressLevel::parse("low").unwrap(), CompressLevel::Light);
        assert_eq!(CompressLevel::parse("MEDIUM").unwrap(), CompressLevel::Balanced);
        assert_eq!(CompressLevel::parse("strong").unwrap(), CompressLevel::Strong);
        assert!(CompressLevel::parse("nonsense").is_err());
    }

    #[test]
    fn percent_saved_math() {
        let s = CompressStats { bytes_before: 1000, bytes_after: 250 };
        assert_eq!(s.percent_saved().round() as i64, 75);
        let grew = CompressStats { bytes_before: 100, bytes_after: 120 };
        assert_eq!(grew.percent_saved(), 0.0);
    }
}
