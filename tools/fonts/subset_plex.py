"""Subset IBM Plex Sans (OFL) to Latin + Cyrillic + common symbols for embedding.

Usage: python tools/fonts/subset_plex.py <dir with IBMPlexSans-*.ttf>
Needs fontTools (pip install fonttools).
"""
import pathlib
import sys

from fontTools import subset

root = pathlib.Path(__file__).resolve().parents[2]
src = pathlib.Path(sys.argv[1])
out = root / "crates" / "paint-ui" / "assets" / "fonts"
ranges = [
    (0x0020, 0x024F),  # Latin, Latin-1, Latin Extended-A/B
    (0x0400, 0x04FF),  # Cyrillic
    (0x2000, 0x206F),  # General punctuation
    (0x20A0, 0x20CF),  # Currency
    (0x2100, 0x214F),  # Letterlike (№, ™)
    (0x2190, 0x21FF),  # Arrows
    (0x2200, 0x22FF),  # Math operators (−, ×, ≈)
    (0x25A0, 0x25FF),  # Geometric shapes
]
unicodes = [c for a, b in ranges for c in range(a, b + 1)]
for weight in ("Regular", "Medium", "SemiBold"):
    opts = subset.Options()
    opts.layout_features = ["kern", "liga"]
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    font = subset.load_font(str(src / f"IBMPlexSans-{weight}.ttf"), opts)
    sub = subset.Subsetter(opts)
    sub.populate(unicodes=unicodes)
    sub.subset(font)
    path = out / f"IBMPlexSans-{weight}.ttf"
    subset.save_font(font, str(path), opts)
    print(path.name, path.stat().st_size)
