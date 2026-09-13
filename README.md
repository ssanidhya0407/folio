<div align="center">

<img src="src-tauri/icons/256x256.png" width="112" alt="folio">

# folio

### **Folio : Local PDF Studio**

**Every PDF tool you need — running entirely on your own computer.**
No cloud. No account. No upload. Free forever.

[![License](https://img.shields.io/badge/license-GPL--3.0-2b2be0)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2B-2b2be0)](#install)
[![Offline](https://img.shields.io/badge/network%20code-none-2b2be0)](#-the-offline-guarantee)
[![Built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-2b2be0)](#-how-it-is-put-together)

</div>

---

Most free PDF tools are websites. You upload a contract, a payslip, a passport
scan — to a server you know nothing about, run by a company whose business model
you cannot see.

folio never does that, because **it cannot**. There is no networking code in the
app at all, and the build fails if any ever gets added.

---

## 📦 Install

| | |
|---|---|
| 🏪 **Microsoft Store** | The easy way. Store signing means no SmartScreen warning. |
| 💾 **Direct download** | `.msi` or `.exe` installer from [Releases](../../releases). Unsigned, so Windows shows *"Windows protected your PC"* → **More info → Run anyway**. |
| 🔧 **From source** | See [Build it yourself](#-build-it-yourself). |

Requires Windows 10 version 1809 or later, 64-bit.

---

## 🧰 What it does

<table>
<tr><td width="33%" valign="top">

**Organise**
- Merge in any order
- Split by range or burst
- Reorder / delete pages
- Rotate

</td><td width="33%" valign="top">

**Optimise & convert**
- Compress (3 strengths)
- Images → PDF
- PDF → PNG / JPEG
- **OCR** scans → searchable

</td><td width="33%" valign="top">

**Protect**
- Password + permissions
- Remove password
- **Redact** (truly)
- Strip metadata
- Watermark

</td></tr>
</table>

Plus **batch mode** — run any of the above across every open file at once, with
a per-file pass/fail report. One bad file never stops the run.

---

## 🔒 The offline guarantee

> "Never goes online" is **enforced**, not promised.

<details>
<summary><b>How you can verify it yourself</b> (click to expand)</summary>

<br>

**1. The dependency tree cannot contain networking code.**
[`deny.toml`](deny.toml) bans every HTTP / TLS / socket / async-networking
crate. CI runs `cargo deny check bans` on every change, so the build fails the
moment networking code enters the tree. The tripwire is `socket2` + `mio` —
Rust's raw socket primitives. Without them, *nothing* can open a TCP or UDP
socket, not even indirectly.

```sh
cargo deny check bans     # try it
```

**2. The Store package declares no network capabilities.**
See [`msix/AppxManifest.xml`](msix/AppxManifest.xml). Windows will not grant a
packaged app a capability it has not declared, so this is enforced by the
operating system regardless of what the code does.

**3. Just unplug the network.**
folio works identically air-gapped. Every feature, every time.

</details>

<details>
<summary><b>What folio stores on your machine</b></summary>

<br>

One file, holding the folders you last opened and saved from so the dialogs
start somewhere useful:

```
%APPDATA%\app.folio.desktop\settings.json
```

Two folder paths. That is the entire extent of it. Full detail in
[PRIVACY.md](PRIVACY.md).

</details>

---

## ✂️ What "redact" actually means here

Most editors draw a rectangle over the text. **The text is still in the file**,
one *select-all → copy* away. That is not redaction; it is a sticker.

folio does the real thing:

1. Parses the page's content stream and works out where **every glyph** lands.
2. **Deletes** the ones inside your selection, replacing each with an equivalent
   position adjustment so surviving text does not shift.
3. Removes image draw operations that overlap, and annotations (links, form
   fields, comments) centred in the region.
4. *Then* paints the covering box — **white by default**, with black, blue,
   grey or any custom colour a click away in the Redact panel. The colour is
   purely cosmetic; the content underneath is gone either way.

<details>
<summary><b>The one approximation, stated honestly</b></summary>

<br>

Glyph widths are estimated rather than read from font metrics, and the hit test
is padded so a borderline glyph is **removed rather than kept** — deliberately
biased toward over-removal, which is the safe direction for a redaction tool.
You may occasionally lose a character just outside the box.

**Check the output before you rely on it**, as you should with any redaction
tool.

This is covered by tests that read the output back and assert the secret text
is gone while its neighbours survive — including on rotated pages and on pages
sharing a header via a Form XObject:

```sh
cargo test -p pdf-core redact
```

</details>

---

## 🚫 What folio will never do

| | |
|---|---|
| ❌ | Telemetry — not even opt-in |
| ❌ | Accounts, sign-in, cloud sync |
| ❌ | A paid tier, user limit, or watermark on your output |
| ❌ | Feature bloat — the surface is capped at ~12–15 well-chosen tools |
| ❌ | Closed or proprietary parts |

Free for everyone, including your company. Forever.

---

## 🏗 How it is put together

```
folio/
├── pdf-core/     the engine — every operation, headless and unit-tested
├── cli/          pdfx, a thin CLI over the engine
├── src-tauri/    the desktop backend (commands, batch, settings)
├── ui/           the front end — no framework, no build step
├── msix/         Microsoft Store package: manifest, tiles, build script
└── assets/       scripts that generate icons and fetch native engines
```

Keeping the engine in its own crate means it is testable without a GUI, and the
CLI wraps it for free.

<details>
<summary><b>Why these libraries (and why not Ghostscript)</b></summary>

<br>

Every dependency is permissively licensed, which is GPL-compatible. The usual
compress/rasterize engines — **Ghostscript and MuPDF — are AGPL** and are
deliberately avoided; `deny.toml` blocks AGPL too.

| Concern | Library | Licence |
|---|---|---|
| Page ops, redaction | [`lopdf`](https://crates.io/crates/lopdf) | MIT |
| Image → PDF | [`printpdf`](https://crates.io/crates/printpdf) + [`image`](https://crates.io/crates/image) | MIT / Apache-2.0 |
| Parallel batch | [`rayon`](https://crates.io/crates/rayon) | MIT / Apache-2.0 |
| Rendering | [PDFium](https://github.com/bblanchon/pdfium-binaries) | BSD-3 |
| Compress / encrypt | [qpdf](https://github.com/qpdf/qpdf) | Apache-2.0 |
| OCR | [Tesseract](https://github.com/tesseract-ocr/tesseract) | Apache-2.0 |
| Shell | [Tauri](https://tauri.app) | MIT / Apache-2.0 |

</details>

---

## 🔨 Build it yourself

Needs the [Rust toolchain](https://rustup.rs) and Python 3 with Pillow.

```sh
python assets/fetch-runtime.py --lang eng   # native engines, once
cargo test                                  # 37 tests
cargo build --release
```

<details>
<summary><b>Producing installers and the Store package</b></summary>

<br>

```sh
cargo install tauri-cli --version "^2.0" --locked

cargo tauri build                            # .msi + NSIS installer
powershell -File msix/build-msix.ps1         # Microsoft Store .msix
```

See [`msix/SUBMISSION.md`](msix/SUBMISSION.md) for the full Store walkthrough.

</details>

<details>
<summary><b>⚠️ The OCR engine needs an elevated prompt</b></summary>

<br>

Tesseract publishes Windows builds **only** as an installer that demands
administrator rights, so this one step cannot run unattended:

```sh
python assets/fetch-runtime.py --skip pdfium qpdf tessdata --tesseract
```

Without it every other tool works normally and the OCR panel says so. CI
runners are already elevated, so release builds pick it up automatically.

Add more languages with `--lang eng deu fra`; folio lists whatever
`*.traineddata` files it finds.

</details>

<details>
<summary><b>Known limitations</b></summary>

<br>

- **OCR replaces pages with images.** Inherent to making a scan searchable, but
  it means a born-digital PDF would lose its vector text and grow. folio detects
  that case and refuses unless you tick *"OCR anyway"*.
- **Redaction estimates glyph widths** — see [above](#-what-redact-actually-means-here).
- **`image_to_pdf` output is slightly non-conformant** (a `printpdf` quirk).
  qpdf-based ops handle it fine, but lopdf-based ones can drop its image on
  round-trip. Real-world PDFs are unaffected.
- **A terminal appears in debug builds only** — release builds are windowed.

</details>

---

## ⌨️ CLI (`pdfx`)

<details>
<summary><b>Every command</b></summary>

<br>

```sh
pdfx merge a.pdf b.pdf -o out.pdf
pdfx split in.pdf -r "1-3,5,8-10" -o pages.pdf
pdfx burst in.pdf -o out_dir/
pdfx pages in.pdf -o reordered.pdf --order "3,1,2"
pdfx img2pdf a.png b.jpg -o photos.pdf --dpi 300
pdfx compress in.pdf -o out.pdf -l balanced        # light | balanced | strong
pdfx rotate in.pdf -o rot.pdf -d 90 [-r "1-2"]
pdfx topng in.pdf -o out_dir/ --dpi 150 --format png
pdfx encrypt in.pdf -o locked.pdf --password open123 [--no-copy]
pdfx decrypt locked.pdf -o out.pdf --password open123
pdfx watermark in.pdf "CONFIDENTIAL" -o wm.pdf [--tiled]
pdfx ocr scan.pdf -o searchable.pdf [-l eng] [--force]
pdfx langs
pdfx redact in.pdf -o clean.pdf -r "1:0.1,0.2,0.5,0.04" [--color "#000000"]
pdfx meta get in.pdf
pdfx meta scrub in.pdf -o clean.pdf
```

Non-zero exit code on failure, for scripting.

</details>

---

## 🗺 Status

**Shipped in 1.0** — merge · split · pages · rotate · compress · protect ·
unlock · watermark · metadata · images↔PDF · **redact** · **OCR** · **batch**,
plus installers and a Store package.

**Next** — code signing for direct downloads · macOS and Linux builds ·
winget / scoop manifests.

Full detail in [ROADMAP.md](ROADMAP.md) and [CHANGELOG.md](CHANGELOG.md).

---

<div align="center">

**folio** is free software under the [GPL-3.0](LICENSE).

Licensed so it — and anything built from it — stays free and open forever.

</div>
