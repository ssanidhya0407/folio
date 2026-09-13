use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// folio's CLI — offline PDF tools: merge, split, rotate, convert, all on your machine.
#[derive(Parser)]
#[command(name = "pdfx", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Merge PDFs (in the given order) into one file.
    Merge {
        /// Input PDFs, in output order.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Output PDF path.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract a page range (e.g. "1-3,5,8-10") into a new PDF.
    Split {
        input: PathBuf,
        /// Page range spec, 1-based and inclusive.
        #[arg(short, long)]
        range: String,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Burst a PDF into one single-page file per page.
    Burst {
        input: PathBuf,
        /// Directory to write `<stem>_<n>.pdf` files into.
        #[arg(short, long)]
        outdir: PathBuf,
    },

    /// Combine images into a PDF, one image per page.
    Img2pdf {
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
        /// Target DPI used to size each page to its image.
        #[arg(long, default_value_t = 300.0)]
        dpi: f32,
        /// Page size: fit | a4 | letter
        #[arg(long, default_value = "fit")]
        size: String,
    },

    /// Rotate pages by a multiple of 90 degrees (additive).
    Rotate {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Degrees to rotate (e.g. 90, 180, -90).
        #[arg(short, long, allow_hyphen_values = true)]
        degrees: i64,
        /// Optional page range; defaults to all pages.
        #[arg(short, long)]
        range: Option<String>,
    },

    /// Shrink a PDF (downsample images + deflate streams).
    Compress {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Compression strength: light | balanced | strong
        #[arg(short, long, default_value = "balanced")]
        level: String,
    },

    /// Reorder and/or delete pages by giving the new page order.
    Pages {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// New page order, 1-based (e.g. "3,1,2" or "1-3,5"). Pages left out
        /// are deleted; a page number may repeat to duplicate it.
        #[arg(long)]
        order: String,
    },

    /// Render pages to images (PNG/JPEG).
    Topng {
        input: PathBuf,
        /// Directory to write `<stem>_<n>.<ext>` files into.
        #[arg(short, long)]
        outdir: PathBuf,
        /// Resolution in DPI (72 ≈ original size).
        #[arg(long, default_value_t = 150.0)]
        dpi: f32,
        /// Output format: png | jpeg
        #[arg(long, default_value = "png")]
        format: String,
        /// Optional page range, e.g. "1-3,5"
        #[arg(long)]
        pages: Option<String>,
    },

    /// Password-protect a PDF.
    Encrypt {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Open password.
        #[arg(long, default_value = "")]
        password: String,
        /// Owner password (for permissions).
        #[arg(long, default_value = "")]
        owner: String,
        /// Key length: 256 | 128 | 40
        #[arg(long, default_value_t = 256)]
        bits: u32,
        #[arg(long)]
        no_print: bool,
        #[arg(long)]
        no_copy: bool,
        #[arg(long)]
        no_modify: bool,
    },

    /// Stamp a text watermark onto pages.
    Watermark {
        input: PathBuf,
        text: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 48.0)]
        size: f32,
        #[arg(long, default_value_t = 0.3)]
        opacity: f32,
        #[arg(long, default_value_t = 45.0)]
        angle: f32,
        #[arg(long)]
        tiled: bool,
        #[arg(long)]
        pages: Option<String>,
    },

    /// Remove a PDF's password.
    Decrypt {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "")]
        password: String,
    },

    /// Make a scanned PDF searchable by recognizing its text (OCR).
    Ocr {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Language code, or several joined with '+' (e.g. "eng", "eng+deu").
        #[arg(short, long, default_value = "eng")]
        lang: String,
        /// Rasterization resolution; 300 gives the best recognition.
        #[arg(long, default_value_t = 300.0)]
        dpi: f32,
        /// OCR even when the PDF already has selectable text (replaces the
        /// pages with images).
        #[arg(long)]
        force: bool,
    },

    /// List the OCR languages installed alongside the app.
    Langs,

    /// Permanently delete the content inside one or more areas of a page.
    Redact {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Area to redact as `page:x,y,w,h`, where x/y/w/h are fractions of
        /// the page (0-1) measured from the top-left. Repeatable.
        #[arg(short, long = "region", required = true, value_name = "PAGE:X,Y,W,H")]
        regions: Vec<String>,
        /// Colour of the box painted over each area, as hex. Only affects how
        /// the result looks; the content underneath is deleted either way.
        #[arg(long, default_value = "#ffffff")]
        color: String,
    },

    /// Read or remove document metadata.
    Meta {
        #[command(subcommand)]
        action: MetaAction,
    },
}

#[derive(Subcommand)]
enum MetaAction {
    /// Print the document's /Info metadata.
    Get { input: PathBuf },
    /// Remove all metadata (/Info + XMP) and write a clean copy.
    Scrub {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> pdf_core::Result<()> {
    match command {
        Command::Merge { inputs, output } => {
            pdf_core::merge(&inputs, &output)?;
            println!("merged {} file(s) -> {}", inputs.len(), output.display());
        }
        Command::Split {
            input,
            range,
            output,
        } => {
            pdf_core::extract(&input, &output, &range)?;
            println!("extracted [{range}] -> {}", output.display());
        }
        Command::Burst { input, outdir } => {
            let outputs = pdf_core::split_each_page(&input, &outdir)?;
            println!("burst into {} page(s) -> {}", outputs.len(), outdir.display());
        }
        Command::Img2pdf {
            inputs,
            output,
            dpi,
            size,
        } => {
            pdf_core::images_to_pdf(&inputs, &output, dpi, pdf_core::PageSize::parse(&size))?;
            println!(
                "converted {} image(s) @ {dpi} dpi ({size}) -> {}",
                inputs.len(),
                output.display()
            );
        }
        Command::Rotate {
            input,
            output,
            degrees,
            range,
        } => {
            pdf_core::rotate(&input, &output, degrees, range.as_deref())?;
            println!("rotated {degrees}deg -> {}", output.display());
        }
        Command::Compress {
            input,
            output,
            level,
        } => {
            let lvl = pdf_core::CompressLevel::parse(&level)?;
            let s = pdf_core::compress(&input, &output, lvl)?;
            println!(
                "compressed {} KB -> {} KB ({:.0}% smaller)",
                s.bytes_before / 1024,
                s.bytes_after / 1024,
                s.percent_saved()
            );
        }
        Command::Pages {
            input,
            output,
            order,
        } => {
            let seq = parse_order(&order)?;
            pdf_core::reorder(&input, &output, &seq)?;
            println!("reordered to {} page(s) -> {}", seq.len(), output.display());
        }
        Command::Topng {
            input,
            outdir,
            dpi,
            format,
            pages,
        } => {
            let outputs = pdf_core::pdf_to_pngs(
                &input,
                &outdir,
                dpi,
                pdf_core::ImgFormat::parse(&format),
                pages.as_deref(),
            )?;
            println!(
                "rendered {} page(s) @ {dpi} dpi ({format}) -> {}",
                outputs.len(),
                outdir.display()
            );
        }
        Command::Encrypt {
            input,
            output,
            password,
            owner,
            bits,
            no_print,
            no_copy,
            no_modify,
        } => {
            let opts = pdf_core::EncryptOptions {
                user_password: password,
                owner_password: owner,
                bits,
                allow_print: !no_print,
                allow_copy: !no_copy,
                allow_modify: !no_modify,
            };
            pdf_core::encrypt(&input, &output, &opts)?;
            println!("protected -> {}", output.display());
        }
        Command::Watermark {
            input,
            text,
            output,
            size,
            opacity,
            angle,
            tiled,
            pages,
        } => {
            let opts = pdf_core::WatermarkOptions {
                text,
                font_size: size,
                opacity,
                color: (0.5, 0.5, 0.5),
                rotation: angle,
                tiled,
                pages,
            };
            pdf_core::watermark(&input, &output, &opts)?;
            println!("watermarked -> {}", output.display());
        }
        Command::Decrypt {
            input,
            output,
            password,
        } => {
            pdf_core::decrypt(&input, &output, &password)?;
            println!("unlocked -> {}", output.display());
        }
        Command::Ocr {
            input,
            output,
            lang,
            dpi,
            force,
        } => {
            let opts = pdf_core::OcrOptions { lang, dpi, force };
            let stats = pdf_core::ocr_with_progress(&input, &output, &opts, |done, total| {
                eprint!("\rreading page {done}/{total}");
            })?;
            eprintln!();
            println!(
                "recognized {} character(s) across {} page(s) -> {}",
                stats.chars,
                stats.pages,
                output.display()
            );
        }
        Command::Langs => {
            let langs = pdf_core::tesseract::languages();
            if langs.is_empty() {
                println!("no OCR languages found (is the tessdata folder next to the app?)");
            } else {
                println!("{}", langs.join("\n"));
            }
        }
        Command::Redact {
            input,
            output,
            regions,
            color,
        } => {
            let parsed: pdf_core::Result<Vec<_>> = regions.iter().map(|r| parse_region(r)).collect();
            let opts = pdf_core::RedactOptions { color: parse_hex(&color)? };
            let stats = pdf_core::redact_with(&input, &output, &parsed?, &opts)?;
            println!(
                "removed {} glyph(s), {} image(s), {} annotation(s) from {} page(s) -> {}",
                stats.glyphs_removed,
                stats.images_removed,
                stats.annotations_removed,
                stats.pages,
                output.display()
            );
        }
        Command::Meta { action } => match action {
            MetaAction::Get { input } => {
                let m = pdf_core::read_metadata(&input)?;
                print_meta(&m);
            }
            MetaAction::Scrub { input, output } => {
                pdf_core::scrub_metadata(&input, &output)?;
                println!("scrubbed metadata -> {}", output.display());
            }
        },
    }
    Ok(())
}

/// Parse an *ordered* page spec like `"3,1,2"` or `"1-3,5"`. Unlike a page
/// range, order is preserved and pages may repeat. Ranges count up or down
/// (`"3-1"` → 3,2,1). Bounds are validated against the document by the engine.
fn parse_order(spec: &str) -> pdf_core::Result<Vec<usize>> {
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((a, b)) => {
                let start = parse_page(a)?;
                let end = parse_page(b)?;
                if start <= end {
                    out.extend(start..=end);
                } else {
                    out.extend((end..=start).rev());
                }
            }
            None => out.push(parse_page(part)?),
        }
    }
    if out.is_empty() {
        return Err(pdf_core::PdfError::Invalid("empty page order".into()));
    }
    Ok(out)
}

/// Parse a `#rrggbb` (or `rrggbb`) colour into RGB components in `0.0..=1.0`.
fn parse_hex(spec: &str) -> pdf_core::Result<(f32, f32, f32)> {
    let hex = spec.trim().trim_start_matches('#');
    let bad = || pdf_core::PdfError::Invalid(format!("invalid colour {spec:?} — expected #rrggbb"));
    if hex.len() != 6 {
        return Err(bad());
    }
    let channel = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .map(|v| v as f32 / 255.0)
            .map_err(|_| bad())
    };
    Ok((channel(0)?, channel(2)?, channel(4)?))
}

/// Parse a redaction area written as `page:x,y,w,h`, with the geometry given
/// as fractions of the page measured from the top-left corner.
fn parse_region(spec: &str) -> pdf_core::Result<pdf_core::RedactRegion> {
    let bad = || {
        pdf_core::PdfError::Invalid(format!(
            "invalid region {spec:?} — expected PAGE:X,Y,W,H (e.g. 1:0.1,0.2,0.5,0.05)"
        ))
    };
    let (page, geometry) = spec.split_once(':').ok_or_else(bad)?;
    let values: Vec<f32> = geometry
        .split(',')
        .map(|v| v.trim().parse::<f32>().map_err(|_| bad()))
        .collect::<pdf_core::Result<_>>()?;
    if values.len() != 4 {
        return Err(bad());
    }
    if values.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
        return Err(pdf_core::PdfError::Invalid(format!(
            "region {spec:?} is out of bounds — x/y/w/h are fractions between 0 and 1"
        )));
    }
    Ok(pdf_core::RedactRegion {
        page: parse_page(page)?,
        x: values[0],
        y: values[1],
        w: values[2],
        h: values[3],
    })
}

fn parse_page(s: &str) -> pdf_core::Result<usize> {
    let n: usize = s
        .trim()
        .parse()
        .map_err(|_| pdf_core::PdfError::Invalid(format!("invalid page number: {s:?}")))?;
    if n == 0 {
        return Err(pdf_core::PdfError::Invalid("page numbers are 1-based".into()));
    }
    Ok(n)
}

fn print_meta(m: &pdf_core::Metadata) {
    if m.is_empty() {
        println!("(no metadata)");
        return;
    }
    let row = |label: &str, val: &Option<String>| {
        if let Some(v) = val {
            println!("{label:<14} {v}");
        }
    };
    row("Title:", &m.title);
    row("Author:", &m.author);
    row("Subject:", &m.subject);
    row("Keywords:", &m.keywords);
    row("Creator:", &m.creator);
    row("Producer:", &m.producer);
    row("CreationDate:", &m.creation_date);
    row("ModDate:", &m.mod_date);
}

#[cfg(test)]
mod tests {
    use super::{parse_hex, parse_order, parse_region};

    #[test]
    fn parses_explicit_order() {
        assert_eq!(parse_order("3,1,2").unwrap(), vec![3, 1, 2]);
    }

    #[test]
    fn expands_ascending_and_descending_ranges() {
        assert_eq!(parse_order("1-3,5").unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse_order("3-1").unwrap(), vec![3, 2, 1]);
    }

    #[test]
    fn parses_a_redaction_region() {
        let r = parse_region("2:0.1,0.2,0.3,0.4").unwrap();
        assert_eq!(r.page, 2);
        assert_eq!((r.x, r.y, r.w, r.h), (0.1, 0.2, 0.3, 0.4));
    }

    #[test]
    fn parses_hex_colours() {
        assert_eq!(parse_hex("#ffffff").unwrap(), (1.0, 1.0, 1.0));
        assert_eq!(parse_hex("000000").unwrap(), (0.0, 0.0, 0.0));
        let (r, g, b) = parse_hex("#2b2be0").unwrap();
        assert!((r - 43.0 / 255.0).abs() < 1e-6 && (g - 43.0 / 255.0).abs() < 1e-6 && (b - 224.0 / 255.0).abs() < 1e-6);
        assert!(parse_hex("#fff").is_err());
        assert!(parse_hex("#gggggg").is_err());
    }

    #[test]
    fn rejects_malformed_or_out_of_bounds_regions() {
        assert!(parse_region("1:0.1,0.2,0.3").is_err());
        assert!(parse_region("0.1,0.2,0.3,0.4").is_err());
        assert!(parse_region("1:0.1,0.2,0.3,1.4").is_err());
        assert!(parse_region("1:a,b,c,d").is_err());
    }

    #[test]
    fn allows_duplicates_and_rejects_empty_or_zero() {
        assert_eq!(parse_order("1,1,2").unwrap(), vec![1, 1, 2]);
        assert!(parse_order("").is_err());
        assert!(parse_order("0").is_err());
    }
}
