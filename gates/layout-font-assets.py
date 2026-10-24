"""CI-only generation of static faces, retaining the existing full character map."""
import hashlib
from io import BytesIO
import json
from pathlib import Path
from urllib.request import urlopen

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

OUT = Path("evidence/web-fonts")
OUT.mkdir(parents=True, exist_ok=True)
SOURCES = {
    "web": ("https://app.hypertask.ai/_next/static/media/26d4368bf94c0ec4-s.woff2", "056e4e2459f57a0033c8c9c844ff19d6e42ac8602027803d4345823bcc939818"),
    "full": ("https://raw.githubusercontent.com/google/fonts/main/ofl/ibmplexsans/IBMPlexSans%5Bwdth%2Cwght%5D.ttf", "3b031aa4216174205bd8471f88a49b91f093169e9e87bd5262242bc5967fe2e3"),
}
fonts = {}
for name, (url, digest) in SOURCES.items():
    data = urlopen(url, timeout=60).read()
    assert hashlib.sha256(data).hexdigest() == digest, name
    fonts[name] = TTFont(BytesIO(data))

receipts = []
for weight in [400, 500, 600, 700]:
    faces = {}
    for name, font in fonts.items():
        axes = {axis.axisTag: weight if axis.axisTag == "wght" else axis.defaultValue for axis in font["fvar"].axes}
        face = instantiateVariableFont(font, axes, inplace=False, updateFontNames=True)
        face.flavor = None
        directory = OUT if name == "full" else OUT / "latin-probe"
        directory.mkdir(exist_ok=True)
        face.save(directory / f"IBMPlexSans-{weight}.ttf")
        faces[name] = face
    full, web = faces["full"], faces["web"]
    full_map, web_map = full.getBestCmap(), web.getBestCmap()
    old = TTFont(f"crates/ht-desktop/assets/fonts/IBMPlexSans-{weight}.ttf")
    missing = sorted(set(old.getBestCmap()) - set(full_map))
    mismatches = [code for code, glyph in web_map.items() if full["hmtx"][full_map[code]] != web["hmtx"][glyph]]
    receipts.append({"weight": weight, "fullGlyphs": len(full_map), "webGlyphs": len(web_map), "missingExisting": missing, "webMetricDifferences": mismatches})
    assert full["OS/2"].usWeightClass == weight
    assert not missing, receipts[-1]
    assert not mismatches, receipts[-1]
(OUT / "source.json").write_text(json.dumps({"sources": SOURCES, "faces": receipts}, indent=2) + "\n")
print("WEB_FONT_ASSETS_OK", json.dumps(receipts))
