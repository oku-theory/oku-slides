# /// script
# requires-python = ">=3.11"
# dependencies = ["fonttools==4.60.1"]
# ///
"""Sous-ensembles des polices Noto Sans CJK livrées avec Oku Slides.

Usage : uv run assets/fonts/subset-cjk.py <dossier des OTF sources>

Sources : notofonts/noto-cjk, Sans/SubsetOTF/{JP,SC,TC,KR}/NotoSans{R}-{Regular,Bold}.otf.
Chaque police garde l'ASCII, la ponctuation CJK, les kana, les formes pleine
chasse et le jeu de caractères national de sa région :
JP = JIS X 0208 (Shift_JIS), SC = GB 2312, TC = Big5,
KR = KS X 1001 plus toutes les syllabes hangûl.
"""

import sys
from pathlib import Path

from fontTools import subset

COMMON = [
    *range(0x20, 0x7F),  # ASCII
    *range(0x2000, 0x2070),  # ponctuation générale
    *range(0x3000, 0x3100),  # ponctuation CJK, hiragana, katakana
    *range(0x31F0, 0x3200),  # katakana phonétiques
    *range(0xFF00, 0xFFF0),  # demi et pleine chasse
]
HANGUL = [*range(0x1100, 0x1200), *range(0x3130, 0x3190), *range(0xAC00, 0xD7A4)]
REGIONS = {"JP": ("shift_jis", []), "SC": ("gb2312", []), "TC": ("big5", []), "KR": ("euc_kr", HANGUL)}
WEIGHTS = {"Regular": 400, "Bold": 700}


def encodable(codec: str) -> list[int]:
    """Points de code non ASCII que `codec` sait encoder."""
    out = []
    for cp in range(0x80, 0x10000):
        if 0xD800 <= cp < 0xE000:
            continue
        try:
            chr(cp).encode(codec)
        except UnicodeEncodeError:
            continue
        out.append(cp)
    return out


def main(src: Path, dst: Path) -> None:
    for region, (codec, extra) in REGIONS.items():
        unicodes = sorted(set(COMMON + extra + encodable(codec)))
        for style, weight in WEIGHTS.items():
            options = subset.Options()
            options.layout_features = ["*"]  # locl, vert, palt… restent disponibles
            options.name_IDs = ["*"]
            options.notdef_outline = True
            font = subset.load_font(src / f"NotoSans{region}-{style}.otf", options)
            subsetter = subset.Subsetter(options)
            subsetter.populate(unicodes=unicodes)
            subsetter.subset(font)
            out = dst / f"noto-sans-{region.lower()}-{weight}.otf"
            subset.save_font(font, out, options)
            print(f"{out.name}: {len(unicodes)} caractères, {out.stat().st_size // 1024} Kio")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(__file__).parent)
