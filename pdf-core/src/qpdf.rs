use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{PdfError, Result};

/// Locate the bundled `qpdf` executable: next to the app, one dir up (so tests
/// running from `target/*/deps` find `target/*/qpdf.exe`), the working dir,
/// then PATH.
pub(crate) fn locate() -> Result<PathBuf> {
    let name = if cfg!(windows) { "qpdf.exe" } else { "qpdf" };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(name));
            if let Some(up) = dir.parent() {
                candidates.push(up.join(name));
            }
        }
    }
    candidates.push(Path::new(".").join(name));
    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    Ok(PathBuf::from(name))
}

/// Run qpdf with `args`. Treats exit 0 (ok) and 3 (ok-with-warnings) as success.
/// On failure, surfaces a relevant stderr line (e.g. "invalid password").
pub(crate) fn run(args: &[String]) -> Result<()> {
    let qpdf = locate()?;
    let mut cmd = Command::new(&qpdf);
    cmd.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().map_err(|e| {
        PdfError::Invalid(format!("could not run qpdf ({}): {e}", qpdf.display()))
    })?;

    match output.status.code() {
        Some(0) | Some(3) => Ok(()),
        other => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let hint = stderr
                .lines()
                .find(|l| l.to_lowercase().contains("password"))
                .map(|l| l.trim().trim_start_matches("qpdf: ").to_string())
                .unwrap_or_else(|| format!("qpdf failed (exit {other:?})"));
            Err(PdfError::Invalid(hint))
        }
    }
}
