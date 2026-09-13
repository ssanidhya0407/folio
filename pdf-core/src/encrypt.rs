use std::path::Path;

use crate::error::{PdfError, Result};

/// Options for [`encrypt`].
#[derive(Debug, Clone)]
pub struct EncryptOptions {
    /// Password required to open the document (empty = none).
    pub user_password: String,
    /// Password required to change permissions (empty = same as user).
    pub owner_password: String,
    /// Key length: 256 (AES, default), 128, or 40.
    pub bits: u32,
    pub allow_print: bool,
    pub allow_copy: bool,
    pub allow_modify: bool,
}

impl Default for EncryptOptions {
    fn default() -> Self {
        EncryptOptions {
            user_password: String::new(),
            owner_password: String::new(),
            bits: 256,
            allow_print: true,
            allow_copy: true,
            allow_modify: true,
        }
    }
}

/// Encrypt/password-protect `input` into `output` via bundled qpdf.
pub fn encrypt(input: &Path, output: &Path, opts: &EncryptOptions) -> Result<()> {
    if opts.user_password.is_empty() && opts.owner_password.is_empty() {
        return Err(PdfError::Invalid("set at least one password".into()));
    }
    let bits = match opts.bits {
        40 | 128 | 256 => opts.bits,
        _ => 256,
    };
    // qpdf classic syntax: --encrypt USER OWNER BITS [perms] -- in out
    let owner = if opts.owner_password.is_empty() {
        opts.user_password.clone()
    } else {
        opts.owner_password.clone()
    };

    let mut args: Vec<String> = vec![
        "--encrypt".into(),
        opts.user_password.clone(),
        owner,
        bits.to_string(),
    ];
    // Permission flags only apply to 128/256-bit encryption.
    if bits != 40 {
        args.push(format!("--print={}", if opts.allow_print { "full" } else { "none" }));
        args.push(format!("--modify={}", if opts.allow_modify { "all" } else { "none" }));
        args.push(format!("--extract={}", if opts.allow_copy { "y" } else { "n" }));
    }
    args.push("--".into());
    args.push(input.to_string_lossy().into_owned());
    args.push(output.to_string_lossy().into_owned());

    crate::qpdf::run(&args)
}

/// Remove encryption from `input` into `output` using `password` (the open or
/// owner password). Empty password works for files with no open password.
pub fn decrypt(input: &Path, output: &Path, password: &str) -> Result<()> {
    let mut args: Vec<String> = Vec::new();
    if !password.is_empty() {
        args.push(format!("--password={password}"));
    }
    args.push("--decrypt".into());
    args.push(input.to_string_lossy().into_owned());
    args.push(output.to_string_lossy().into_owned());

    crate::qpdf::run(&args)
}
