//! The handful of preferences folio remembers between runs.
//!
//! Stored as a small JSON file in the OS config directory. Nothing here leaves
//! the machine — it exists so the app stops asking where your files live.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Folder the last file was opened from.
    pub last_open_dir: Option<String>,
    /// Folder the last result was saved to.
    pub last_save_dir: Option<String>,
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) {
    if let Some(p) = path(app) {
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            let _ = std::fs::write(p, json);
        }
    }
}

/// Remember the folder `file` lives in as the last place opened from.
pub fn remember_open(app: &AppHandle, file: &str) {
    if let Some(dir) = parent_of(file) {
        let mut s = load(app);
        s.last_open_dir = Some(dir);
        save(app, &s);
    }
}

/// Remember where a result was written. `is_dir` distinguishes "saved this
/// file" from "chose this output folder".
pub fn remember_save(app: &AppHandle, target: &str, is_dir: bool) {
    let dir = if is_dir {
        Some(target.to_string())
    } else {
        parent_of(target)
    };
    if let Some(dir) = dir {
        let mut s = load(app);
        s.last_save_dir = Some(dir);
        save(app, &s);
    }
}

fn parent_of(file: &str) -> Option<String> {
    Path::new(file)
        .parent()
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
}

/// The directory a file dialog should start in, if we still have a valid one.
pub fn start_dir(app: &AppHandle, saving: bool) -> Option<PathBuf> {
    let s = load(app);
    let chosen = if saving {
        s.last_save_dir.or(s.last_open_dir)
    } else {
        s.last_open_dir.or(s.last_save_dir)
    };
    chosen.map(PathBuf::from).filter(|p| p.is_dir())
}
