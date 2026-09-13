#!/usr/bin/env python3
"""Generate folio's icon set from the brand logo.

Everything is derived from one source file, `assets/brand/folio-logo.png` —
the official logo: the "folio" wordmark over a pattern of PDF document icons.

The logo artwork is kept at every size, pattern included. Below roughly 64 px
the wordmark itself stops being readable, so those sizes keep the same
patterned background but swap the wordmark for the "f" cut from it — the same
letterform, just large enough to see. Everything therefore reads as one brand
from the Store tile down to the taskbar.

Run after any change to the logo:

    python assets/make-icons.py

Writes:
  src-tauri/icons/   the icons Tauri bundles into the .exe/.msi
  msix/Assets/       the tile and logo images the Microsoft Store requires
  ui/logo.png        the wordmark used in the app's sidebar
"""

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "assets" / "brand" / "folio-logo.png"
TAURI_ICONS = ROOT / "src-tauri" / "icons"
MSIX_ASSETS = ROOT / "msix" / "Assets"
UI = ROOT / "ui"

# Sampled from the logo itself, so anything drawn matches it exactly.
BRAND = (38, 44, 209, 255)

# Below this, the five-letter wordmark is a smudge and the monogram takes over.
WORDMARK_MIN_PX = 64

# Fraction of the tile the monogram occupies on the small sizes.
MONOGRAM_FILL = 0.60


def load_logo() -> Image.Image:
    if not LOGO.exists():
        raise SystemExit(
            f"brand logo not found at {LOGO.relative_to(ROOT)}\n"
            "Put the official logo there and re-run."
        )
    return Image.open(LOGO).convert("RGBA")


def wordmark_mask(logo: Image.Image) -> Image.Image:
    """The wordmark as an alpha mask, lifted off the pattern behind it.

    The lettering is a strongly saturated blue and the pattern is near-neutral
    grey, so "much bluer than it is red or green" separates the two cleanly
    without needing the original artwork.
    """
    r, g, b, _ = logo.split()
    blueness = ImageChops.subtract(b, ImageChops.lighter(r, g))
    mask = blueness.point(lambda v: 255 if v > 60 else 0)

    box = mask.getbbox()
    if box is None:
        raise SystemExit("no wordmark found in the logo — has the artwork changed?")
    return mask.crop(box)


def monogram(word: Image.Image) -> Image.Image:
    """The leading "f", cut from the wordmark at the first gap between letters."""
    width, height = word.size
    inked = [any(word.getpixel((x, y)) for y in range(height)) for x in range(width)]
    try:
        end = next(x for x, on in enumerate(inked) if not on and x > 0)
    except StopIteration:
        end = width
    return word.crop((0, 0, end, height))


def background(logo: Image.Image, width: int, height: int) -> Image.Image:
    """The logo's patterned background, cropped to an aspect ratio and resized.

    The wordmark is masked out first and the gap filled from the pattern, so
    the small sizes get texture without a ghost of the lettering behind the
    monogram.
    """
    pattern = logo.copy()
    # The wordmark sits across the middle; take the pattern from above it and
    # mirror it down, which keeps the icon-density even.
    top = pattern.crop((0, 0, pattern.width, pattern.height // 3))
    filled = Image.new("RGBA", pattern.size)
    y = 0
    while y < pattern.height:
        filled.paste(top, (0, y))
        y += top.height
    # Keep the genuine corners of the original; only the middle band is rebuilt.
    band = pattern.height // 4
    filled.paste(pattern.crop((0, 0, pattern.width, band)), (0, 0))
    filled.paste(
        pattern.crop((0, pattern.height - band, pattern.width, pattern.height)),
        (0, pattern.height - band),
    )

    source_aspect = filled.width / filled.height
    target_aspect = width / height
    if target_aspect > source_aspect:
        new_height = round(filled.width / target_aspect)
        offset = (filled.height - new_height) // 2
        filled = filled.crop((0, offset, filled.width, offset + new_height))
    else:
        new_width = round(filled.height * target_aspect)
        offset = (filled.width - new_width) // 2
        filled = filled.crop((offset, 0, offset + new_width, filled.height))

    return filled.resize((width, height), Image.LANCZOS)


def rounded(img: Image.Image, radius_fraction: float = 0.22) -> Image.Image:
    """Round the corners, which is what every platform expects of an app icon."""
    radius = round(min(img.size) * radius_fraction)
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, img.width - 1, img.height - 1],
                                          radius=radius, fill=255)
    out = Image.new("RGBA", img.size, (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    return out


def tile(logo: Image.Image, word: Image.Image, mono: Image.Image,
         width: int, height: int) -> Image.Image:
    """One icon: patterned background, with wordmark or monogram on top."""
    img = background(logo, width, height)

    small = min(width, height) < WORDMARK_MIN_PX
    art = mono if small else word
    fill = MONOGRAM_FILL if small else 0.78

    # At taskbar sizes the pattern and the mark are drawn at similar weights and
    # compete, leaving a grey smudge. Fading the pattern keeps the texture the
    # brand is built on while letting the mark stay the thing you actually see.
    if min(width, height) < 32:
        veil = Image.new("RGBA", img.size, (255, 255, 255, 150))
        img = Image.alpha_composite(img, veil)

    scaled = art.copy()
    scaled.thumbnail(
        (max(1, round(width * fill)), max(1, round(height * (0.62 if small else 0.34)))),
        Image.LANCZOS,
    )
    img.paste(BRAND, ((width - scaled.width) // 2, (height - scaled.height) // 2), scaled)
    return rounded(img)


def write(img: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)


def build_tauri_icons(logo, word, mono) -> None:
    print("Tauri icons:")
    for size in (32, 128, 256, 512):
        write(tile(logo, word, mono, size, size), TAURI_ICONS / f"{size}x{size}.png")
    write(tile(logo, word, mono, 256, 256), TAURI_ICONS / "128x128@2x.png")
    write(tile(logo, word, mono, 1024, 1024), TAURI_ICONS / "icon.png")

    ico_sizes = (16, 24, 32, 48, 64, 128, 256)
    # Each .ico frame is rendered at its own size so the small ones get the
    # monogram rather than a downscaled, unreadable wordmark.
    frames = [tile(logo, word, mono, s, s).convert("RGBA") for s in ico_sizes]
    frames[-1].save(
        TAURI_ICONS / "icon.ico",
        format="ICO",
        sizes=[(s, s) for s in ico_sizes],
        append_images=frames[:-1],
    )
    print(f"  6 PNGs + icon.ico ({len(ico_sizes)} frames)")


MSIX_SQUARES = {
    "Square44x44Logo": 44,
    "Square71x71Logo": 71,
    "Square150x150Logo": 150,
    "Square310x310Logo": 310,
    "StoreLogo": 50,
}
SCALES = (100, 125, 150, 200, 400)
TARGET_SIZES = (16, 24, 32, 48, 256)


def build_msix_assets(logo, word, mono) -> None:
    print("MSIX assets:")
    count = 0
    for name, base in MSIX_SQUARES.items():
        # The unqualified name is the manifest's reference, and the fallback
        # when the package has no resources.pri.
        write(tile(logo, word, mono, base, base), MSIX_ASSETS / f"{name}.png")
        count += 1
        for scale in SCALES:
            size = max(1, round(base * scale / 100))
            write(tile(logo, word, mono, size, size), MSIX_ASSETS / f"{name}.scale-{scale}.png")
            count += 1

    write(tile(logo, word, mono, 310, 150), MSIX_ASSETS / "Wide310x150Logo.png")
    count += 1
    for scale in SCALES:
        w, h = round(310 * scale / 100), round(150 * scale / 100)
        write(tile(logo, word, mono, w, h), MSIX_ASSETS / f"Wide310x150Logo.scale-{scale}.png")
        count += 1

    for size in TARGET_SIZES:
        art = tile(logo, word, mono, size, size)
        write(art, MSIX_ASSETS / f"Square44x44Logo.targetsize-{size}.png")
        write(art, MSIX_ASSETS / f"Square44x44Logo.targetsize-{size}_altform-unplated.png")
        count += 2

    print(f"  {count} tile images")


def build_ui_logo(word: Image.Image) -> None:
    """The sidebar wordmark: brand blue on transparent, for a light background."""
    height = 96  # rendered around 24 px, so 4x keeps it sharp on HiDPI
    scaled = word.copy()
    scaled.thumbnail((word.width * height // word.height, height), Image.LANCZOS)
    img = Image.new("RGBA", scaled.size, (0, 0, 0, 0))
    img.paste(BRAND, (0, 0), scaled)
    write(img, UI / "logo.png")
    print(f"UI wordmark:\n  ui/logo.png  {img.width}x{img.height}")


if __name__ == "__main__":
    logo = load_logo()
    word = wordmark_mask(logo)
    mono = monogram(word)
    print(f"Source: {LOGO.relative_to(ROOT)}   wordmark {word.size}, monogram {mono.size}\n")

    build_tauri_icons(logo, word, mono)
    build_msix_assets(logo, word, mono)
    build_ui_logo(word)
    print("\nDone.")
