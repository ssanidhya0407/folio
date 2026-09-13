#!/usr/bin/env python3
"""Fetch the native libraries folio ships alongside its executable.

These are author/build-time downloads. None of them is a network call made by
the app — the shipped binary still has no networking code in it at all, which
is what `cargo deny check bans` enforces on every build.

    python assets/fetch-runtime.py                 # PDFium, qpdf, OCR language data
    python assets/fetch-runtime.py --lang eng deu  # extra OCR languages
    python assets/fetch-runtime.py --tesseract     # also install the OCR engine

Everything lands in `src-tauri/runtime/`, which the Tauri bundler copies next
to the installed `folio.exe` (see `bundle.resources` in tauri.conf.json).

Licensing, all GPL-compatible and all permissive:
  PDFium    BSD-3-Clause   qpdf  Apache-2.0   Tesseract  Apache-2.0
"""

import argparse
import io
import json
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNTIME = ROOT / "src-tauri" / "runtime"
TESSDATA = RUNTIME / "tessdata"

PDFIUM_RELEASE = "https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest"
PDFIUM_ASSET = "pdfium-win-x64.tgz"

QPDF_RELEASE = "https://api.github.com/repos/qpdf/qpdf/releases/latest"
QPDF_ASSET_SUFFIX = "-msvc64.zip"

TESSERACT_RELEASE = "https://api.github.com/repos/tesseract-ocr/tesseract/releases/latest"
# `tessdata_fast` is the size/accuracy compromise meant for end users; the
# `_best` models are several times larger for a marginal accuracy gain.
TESSDATA_URL = "https://github.com/tesseract-ocr/tessdata_fast/raw/main/{lang}.traineddata"

# Tesseract writes a searchable PDF by laying invisible text over the page
# image, which needs a glyph-less font to do it with. The copy in the tessdata
# repo is a symlink, so take the real one from tessconfigs.
PDF_FONT_URL = "https://github.com/tesseract-ocr/tessconfigs/raw/main/pdf.ttf"


def get(url: str) -> bytes:
    print(f"  fetching {url.split('/')[-1]} …", flush=True)
    with urllib.request.urlopen(url, timeout=180) as r:
        return r.read()


def latest_asset(release_api: str, match) -> str:
    """URL of the first asset in a GitHub release whose name satisfies `match`."""
    with urllib.request.urlopen(release_api, timeout=60) as r:
        release = json.load(r)
    for asset in release["assets"]:
        if match(asset["name"]):
            return asset["browser_download_url"]
    raise SystemExit(f"no matching asset in {release['tag_name']} at {release_api}")


def fetch_pdfium() -> None:
    print("PDFium (PDF rendering, BSD-3-Clause)")
    url = latest_asset(PDFIUM_RELEASE, lambda n: n == PDFIUM_ASSET)
    data = get(url)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        member = next(m for m in tar.getmembers() if m.name.endswith("bin/pdfium.dll"))
        src = tar.extractfile(member)
        (RUNTIME / "pdfium.dll").write_bytes(src.read())
    print("  -> pdfium.dll")


def fetch_qpdf() -> None:
    print("qpdf (compression + encryption, Apache-2.0)")
    url = latest_asset(QPDF_RELEASE, lambda n: n.endswith(QPDF_ASSET_SUFFIX))
    data = get(url)
    with zipfile.ZipFile(io.BytesIO(data)) as zf:
        # The zip holds `qpdf-<version>/bin/*`; take the executable and every
        # DLL it links against, including the MSVC runtime.
        # Only the engine we actually shell out to, plus the libraries it
        # links against — qpdf also ships helper tools folio never calls.
        wanted = [
            n for n in zf.namelist()
            if "/bin/" in n
            and (n.lower().endswith(".dll") or Path(n).name.lower() == "qpdf.exe")
        ]
        if not wanted:
            raise SystemExit("qpdf archive did not contain a bin/ directory")
        for name in wanted:
            target = RUNTIME / Path(name).name
            target.write_bytes(zf.read(name))
        print(f"  -> {len(wanted)} file(s), including qpdf.exe")


def fetch_tessdata(langs) -> None:
    print("OCR language data (Apache-2.0)")
    TESSDATA.mkdir(parents=True, exist_ok=True)
    for lang in langs:
        data = get(TESSDATA_URL.format(lang=lang))
        (TESSDATA / f"{lang}.traineddata").write_bytes(data)
        print(f"  -> tessdata/{lang}.traineddata")

    # Without these two, tesseract runs, recognizes the text, and then fails to
    # emit a PDF with only "read_params_file: Can't open pdf" to explain why.
    configs = TESSDATA / "configs"
    configs.mkdir(exist_ok=True)
    (configs / "pdf").write_text("tessedit_create_pdf 1\n", encoding="ascii")
    print("  -> tessdata/configs/pdf")

    (TESSDATA / "pdf.ttf").write_bytes(get(PDF_FONT_URL))
    print("  -> tessdata/pdf.ttf")


def fetch_tesseract() -> None:
    """Install the Tesseract engine into `runtime/`.

    Tesseract publishes Windows builds only as an NSIS installer, so there is
    nothing to simply unzip. We run it in silent mode pointed at our own
    directory; it stays self-contained and is removed by deleting the folder.
    """
    print("Tesseract OCR engine (Apache-2.0)")
    url = latest_asset(TESSERACT_RELEASE, lambda n: n.endswith(".exe") and "w64" in n)
    data = get(url)

    with tempfile.TemporaryDirectory() as tmp:
        installer = Path(tmp) / "tesseract-setup.exe"
        installer.write_bytes(data)
        staging = RUNTIME / "_tesseract"
        if staging.exists():
            shutil.rmtree(staging)
        # NSIS: /S is silent, /D=<abs path> sets the destination and must come last.
        try:
            result = subprocess.run(
                [str(installer), "/S", f"/D={staging}"],
                capture_output=True,
                timeout=900,
            )
        except OSError as e:
            raise SystemExit(
                f"could not start the Tesseract installer: {e}\n\n"
                "Tesseract ships only as an installer that asks for administrator\n"
                "rights, so this step has to run from an elevated prompt:\n\n"
                "    python assets/fetch-runtime.py --skip pdfium qpdf tessdata --tesseract\n\n"
                "Everything else in folio works without it; OCR simply reports that\n"
                "the engine is missing."
            ) from e

        if result.returncode != 0 or not staging.exists():
            raise SystemExit(
                "the Tesseract installer did not complete. Install Tesseract\n"
                "manually, then copy tesseract.exe and its DLLs into\n"
                "src-tauri/runtime/ and the language files into\n"
                "src-tauri/runtime/tessdata/."
            )

    # Flatten: the engine binary and its DLLs sit next to folio.exe, and the
    # language files go in runtime/tessdata where pdf-core looks for them.
    for item in staging.iterdir():
        if item.is_file() and item.suffix.lower() in (".exe", ".dll"):
            shutil.copy2(item, RUNTIME / item.name)
    installed_data = staging / "tessdata"
    if installed_data.is_dir():
        TESSDATA.mkdir(parents=True, exist_ok=True)
        for item in installed_data.glob("*.traineddata"):
            shutil.copy2(item, TESSDATA / item.name)
    shutil.rmtree(staging, ignore_errors=True)
    print("  -> tesseract.exe + libraries")


# ---------------------------------------------------------------- pruning ---

# The only binaries folio actually invokes. Everything else in `runtime/` is
# kept only if one of these links against it.
ENTRY_POINTS = ("tesseract.exe", "qpdf.exe", "pdfium.dll")


def dll_imports(path: Path):
    """DLL names in a PE file's import table.

    Tesseract's installer drops ~80 files, most of them training tools and the
    Pango/Cairo/ICU stack that only `text2image` needs. Reading the import
    table lets us keep exactly what is reachable and delete the rest, instead
    of guessing from filenames.
    """
    data = path.read_bytes()
    try:
        pe = struct.unpack_from("<I", data, 0x3C)[0]
        if data[pe:pe + 4] != b"PE\0\0":
            return []
        sections = struct.unpack_from("<H", data, pe + 6)[0]
        opt_size = struct.unpack_from("<H", data, pe + 20)[0]
        magic = struct.unpack_from("<H", data, pe + 24)[0]
        directory = pe + 24 + (112 if magic == 0x20B else 96)
        import_rva = struct.unpack_from("<I", data, directory + 8)[0]
        if not import_rva:
            return []

        table = []
        base = pe + 24 + opt_size
        for i in range(sections):
            _, vsize, vaddr, rsize, raw = struct.unpack_from("<8sIIII", data, base + i * 40)
            table.append((vaddr, max(vsize, rsize), raw))

        def offset_of(rva):
            for vaddr, size, raw in table:
                if vaddr <= rva < vaddr + size:
                    return raw + (rva - vaddr)
            return None

        names, cursor = [], offset_of(import_rva)
        while cursor is not None:
            entry = data[cursor:cursor + 20]
            if len(entry) < 20 or entry == b"\0" * 20:
                break
            name_rva = struct.unpack_from("<I", entry, 12)[0]
            if not name_rva:
                break
            at = offset_of(name_rva)
            if at is None:
                break
            names.append(data[at:data.index(b"\0", at)].decode("ascii", "ignore"))
            cursor += 20
        return names
    except Exception:
        return []


def prune() -> None:
    """Delete anything in `runtime/` that no entry point depends on."""
    present = {p.name.lower(): p for p in RUNTIME.iterdir() if p.is_file()}
    keep, queue = set(), [e for e in ENTRY_POINTS if e in present]

    while queue:
        name = queue.pop().lower()
        if name in keep or name not in present:
            continue
        keep.add(name)
        queue.extend(d for d in dll_imports(present[name]) if d.lower() in present)

    removed = freed = 0
    for name, path in present.items():
        if name not in keep:
            freed += path.stat().st_size
            path.unlink()
            removed += 1

    if removed:
        print(f"  pruned {removed} unused file(s), {freed // (1024 * 1024)} MB")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--lang",
        nargs="+",
        default=["eng"],
        help="OCR languages to bundle (default: eng)",
    )
    parser.add_argument(
        "--tesseract",
        action="store_true",
        help="also install the OCR engine itself (runs the vendor's installer)",
    )
    parser.add_argument("--skip", nargs="*", default=[], choices=["pdfium", "qpdf", "tessdata"])
    args = parser.parse_args()

    RUNTIME.mkdir(parents=True, exist_ok=True)
    if "pdfium" not in args.skip:
        fetch_pdfium()
    if "qpdf" not in args.skip:
        fetch_qpdf()
    if "tessdata" not in args.skip:
        fetch_tessdata(args.lang)
    if args.tesseract:
        fetch_tesseract()
        prune()
    else:
        print("\nSkipping the Tesseract engine (pass --tesseract to install it).")
        print("Without it, every tool works except OCR, which says so in the app.")

    print(f"\nRuntime ready in {RUNTIME.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
