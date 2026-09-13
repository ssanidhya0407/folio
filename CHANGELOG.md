# Changelog

All notable changes to folio are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and folio uses [semantic versioning](https://semver.org/).

## [1.0.0] — 2026-09-13

The first release meant for other people. Everything below the surface was
already working; this version finishes the two features that were missing, adds
the polish that makes it usable by someone who has never heard of a PDF content
stream, and packages it for distribution.

### Added

- **Redaction.** Draw boxes over a page and the glyphs, images and annotations
  underneath are deleted from the file, not covered up. Surviving text keeps
  its position. Verified by a test that reads the output back and asserts the
  redacted string is gone.
- **OCR.** Make a scanned PDF searchable using a bundled Tesseract engine,
  entirely offline. Picks up any language you add to `tessdata/`, shows
  per-page progress, and refuses to run on a PDF that already has good text
  unless you insist.
- **Batch processing in the app.** Apply compress, rotate, metadata scrub,
  watermark, protect, unlock or OCR to every open PDF at once, into a folder
  you choose, with a per-file pass/fail report. One bad file no longer stops
  the run.
- **About panel** stating the offline guarantee in the app's own words, with
  the engine and language status.
- **Progress bars** for the operations slow enough to need them.
- **Remembered folders.** File dialogs open where you last were instead of your
  home directory.
- **File association.** folio appears in "Open with" for PDFs, and opens a file
  passed on the command line.
- **App icon set**, generated reproducibly from geometry by
  `assets/make-icons.py`.
- **Installers.** `.msi` and NSIS builds via the Tauri bundler, with the native
  engines bundled alongside.
- **Microsoft Store package.** A complete MSIX pipeline — manifest, tile
  assets, build script, listing copy and a submission walkthrough — under
  `msix/`. The package declares no network capabilities, which makes the
  privacy claim enforceable by Windows itself.
- **`assets/fetch-runtime.py`** replaces the manual "download these DLLs by
  hand" instructions.
- CLI: `pdfx ocr`, `pdfx redact`, `pdfx langs`.
- `PRIVACY.md`, for the Store's required privacy policy URL and for anyone who
  wants the claim written down.
- A release workflow that builds all three artifacts from a tag.

### Changed

- `pdf_to_png::load_bindings` is now crate-visible so OCR can share the
  PDFium binding logic rather than duplicating it.
- The README documents precisely what redaction does and where it approximates,
  rather than claiming more than the implementation delivers.

### Fixed

- The "open a PDF to use this tool" prompt rendered a broken icon: it asked for
  an `upload` glyph that was never defined.

### Known limitations

- Redaction estimates glyph widths instead of reading font metrics, and pads
  the hit test so borderline glyphs are removed rather than kept. Check the
  output before relying on it.
- OCR rasterizes pages, so it grows a born-digital PDF and loses its vector
  text. folio detects this and warns before doing it.
- The direct-download installers are unsigned, so SmartScreen warns on first
  run. The Store build does not have this problem.
