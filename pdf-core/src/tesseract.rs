use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{PdfError, Result};

/// Locate the bundled `tesseract` executable, using the same search order as
/// [`crate::qpdf::locate`]: next to the app, one dir up (so tests running from
/// `target/*/deps` find `target/*/tesseract.exe`), the working dir, then PATH.
pub(crate) fn locate() -> Result<PathBuf> {
    let name = if cfg!(windows) { "tesseract.exe" } else { "tesseract" };
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

/// Locate the `tessdata` directory holding the `*.traineddata` language files.
/// Searched next to the executable and one dir up, mirroring [`locate`].
/// `None` means "let tesseract use its own default" (a system install).
pub(crate) fn tessdata_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("tessdata"));
            if let Some(up) = dir.parent() {
                candidates.push(up.join("tessdata"));
            }
        }
    }
    candidates.push(PathBuf::from("tessdata"));
    candidates.into_iter().find(|c| c.is_dir())
}

/// Every language available to OCR, as tesseract language codes (`eng`,
/// `deu`, …), derived from the `*.traineddata` files in [`tessdata_dir`].
///
/// Returns an empty list when no bundled `tessdata` directory is present.
pub fn languages() -> Vec<String> {
    let Some(dir) = tessdata_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut langs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension()?.eq_ignore_ascii_case("traineddata") {
                Some(path.file_stem()?.to_str()?.to_string())
            } else {
                None
            }
        })
        // `osd` is orientation/script detection, not a recognition language.
        .filter(|l| l != "osd")
        .collect();
    langs.sort();
    langs
}

/// True when the OCR engine is actually available to run.
pub fn is_available() -> bool {
    locate().map(|p| p.exists()).unwrap_or(false) || run(&["--version".into()]).is_ok()
}

/// Run tesseract with `args`, returning stdout. Surfaces a useful stderr line
/// on failure (tesseract reports missing language data there).
pub(crate) fn run(args: &[String]) -> Result<String> {
    let exe = locate()?;
    let mut cmd = Command::new(&exe);
    cmd.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().map_err(|e| {
        PdfError::Invalid(format!(
            "could not run the OCR engine ({}): {e}",
            exe.display()
        ))
    })?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let hint = stderr
        .lines()
        .find(|l| {
            let l = l.to_lowercase();
            l.contains("failed loading language")
                || l.contains("could not initialize")
                || l.contains("error")
        })
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| format!("OCR failed (exit {:?})", output.status.code()));
    Err(PdfError::Invalid(hint))
}
