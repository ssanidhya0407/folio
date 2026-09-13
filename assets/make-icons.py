#!/usr/bin/env python3
"""Generate folio's icon set from the brand mark.

The mark is drawn geometrically rather than set in a typeface, so the whole
icon set is reproducible from this file alone with no font licensing to worry
about. Run it after changing the brand:

    python assets/make-icons.py

Writes:
  src-tauri/icons/      the icons Tauri bundles into the .exe/.msi
  msix/Assets/          the tile and logo images the Microsoft Store requires
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
TAURI_ICONS = ROOT / "src-tauri" / "icons"
MSIX_ASSETS = ROOT / "msix" / "Assets"

# The brand blue from the folio wordmark. If the icon ever looks slightly off
# against the official logo, this is the single line to correct.
BRAND = (43, 43, 224, 255)
FOLD = (28, 28, 168, 255)     # the turned corner, a shade down from BRAND
WHITE = (255, 255, 255, 255)

# The mark is drawn on this grid, then scaled down to each output size.
GRID = 1024


def draw_mark(canvas_size: int) -> Image.Image:
    """folio's mark — a sheet of paper with a turned corner — on a transparent
    square.

    It is the same shape as the mark in the app's own header, so the icon in
    the taskbar and the logo in the window read as one thing. Drawn as plain
    geometry, which survives being scaled down to 16 px far better than a
    detailed illustration would.
    """
    # Page box and the size of the turned-down corner, on the 1024 grid.
    left, top, right, bottom = 296, 176, 728, 848
    fold = 168
    radius = 40

    # Build the page as a mask so the folded corner can be cut away cleanly.
    mask = Image.new("L", (GRID, GRID), 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle([left, top, right, bottom], radius=radius, fill=255)
    md.polygon([(right - fold, top - 2), (right + 2, top - 2), (right + 2, top + fold)], fill=0)

    img = Image.new("RGBA", (GRID, GRID), (0, 0, 0, 0))
    img.paste(WHITE, (0, 0), mask)

    d = ImageDraw.Draw(img)
    # The underside of the turned corner, in a deeper orange so it reads as a
    # fold rather than a bite taken out of the page.
    d.polygon(
        [(right - fold, top), (right, top + fold), (right - fold, top + fold)],
        fill=FOLD,
    )
    return img.resize((canvas_size, canvas_size), Image.LANCZOS)


def tile(width: int, height: int, *, plated: bool = True, fill: float = 0.62) -> Image.Image:
    """The mark on a rounded orange tile (or transparent, when unplated).

    `fill` is how much of the shorter side the glyph occupies — Microsoft asks
    for clear space around tile logos, and it keeps the mark legible at 16 px.
    """
    img = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    if plated:
        d = ImageDraw.Draw(img)
        radius = round(min(width, height) * 0.22)
        d.rounded_rectangle([0, 0, width - 1, height - 1], radius=radius, fill=BRAND)

    glyph_size = max(1, round(min(width, height) * fill))
    glyph = draw_mark(glyph_size)
    img.paste(glyph, ((width - glyph_size) // 2, (height - glyph_size) // 2), glyph)
    return img


def square(size: int, **kwargs) -> Image.Image:
    return tile(size, size, **kwargs)


def write(img: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)
    print(f"  {path.relative_to(ROOT)}  {img.width}x{img.height}")


def build_tauri_icons() -> None:
    print("Tauri icons:")
    for size in (32, 128, 256, 512):
        write(square(size), TAURI_ICONS / f"{size}x{size}.png")
    # Tauri's own naming for the 2x 128 icon.
    write(square(256), TAURI_ICONS / "128x128@2x.png")
    write(square(1024), TAURI_ICONS / "icon.png")

    # A multi-resolution .ico is what Windows Explorer and the taskbar read.
    ico_sizes = [(s, s) for s in (16, 24, 32, 48, 64, 128, 256)]
    square(256).save(TAURI_ICONS / "icon.ico", format="ICO", sizes=ico_sizes)
    print(f"  {(TAURI_ICONS / 'icon.ico').relative_to(ROOT)}  multi-size")


# Every logo the Store/MSIX manifest can reference, with its base size.
MSIX_SQUARES = {
    "Square44x44Logo": 44,
    "Square71x71Logo": 71,
    "Square150x150Logo": 150,
    "Square310x310Logo": 310,
    "StoreLogo": 50,
}
SCALES = (100, 125, 150, 200, 400)
# Sizes Windows picks from for the taskbar, Start list and Alt-Tab.
TARGET_SIZES = (16, 24, 32, 48, 256)


def build_msix_assets() -> None:
    print("MSIX assets:")
    for name, base in MSIX_SQUARES.items():
        # The unqualified name is what the manifest references, and what
        # Windows falls back to when the package has no resources.pri.
        write(square(base), MSIX_ASSETS / f"{name}.png")
        for scale in SCALES:
            size = max(1, round(base * scale / 100))
            write(square(size), MSIX_ASSETS / f"{name}.scale-{scale}.png")

    # Wide tile: the same mark, sized to the tile's height.
    write(tile(310, 150, fill=0.58), MSIX_ASSETS / "Wide310x150Logo.png")
    for scale in SCALES:
        w, h = round(310 * scale / 100), round(150 * scale / 100)
        write(tile(w, h, fill=0.58), MSIX_ASSETS / f"Wide310x150Logo.scale-{scale}.png")

    # Target-size assets are used at exact pixel sizes. The "unplated" variants
    # are the ones Windows shows *without* adding its own background plate —
    # they carry our orange tile instead, since the mark itself is white and
    # would vanish on a light background.
    for size in TARGET_SIZES:
        write(square(size), MSIX_ASSETS / f"Square44x44Logo.targetsize-{size}.png")
        write(square(size), MSIX_ASSETS / f"Square44x44Logo.targetsize-{size}_altform-unplated.png")


if __name__ == "__main__":
    build_tauri_icons()
    build_msix_assets()
    print("\nDone. Re-run after any brand change.")
