"""CI-only generation of static faces, retaining the existing full character map."""
import hashlib
from io import BytesIO
import json
from pathlib import Path
import re
import sys
from urllib.parse import urljoin
from urllib.request import urlopen

from fontTools.ttLib import TTFont
from fontTools import subset
from fontTools.varLib.instancer import instantiateVariableFont

APP = "https://app.hypertask.ai/"
FAMILY = "IBM Plex Sans"
LATIN_PROBES = (0x41, 0x61, 0x31, 0x20)
WEB_DIGEST = "056e4e2459f57a0033c8c9c844ff19d6e42ac8602027803d4345823bcc939818"
FULL_URL = "https://raw.githubusercontent.com/google/fonts/8409f033cd7dee08914990602f0df5f5e70e0c14/ofl/ibmplexsans/IBMPlexSans%5Bwdth%2Cwght%5D.ttf"
FULL_DIGEST = "3b031aa4216174205bd8471f88a49b91f093169e9e87bd5262242bc5967fe2e3"
OUT = Path("evidence/web-fonts")


def unicode_ranges(declaration):
    for token in declaration.split(","):
        token = token.strip().lower()
        if not token.startswith("u+"):
            continue
        body = token[2:]
        if "?" in body:
            yield int(body.replace("?", "0"), 16), int(body.replace("?", "f"), 16)
        elif "-" in body:
            low, high = body.split("-", 1)
            yield int(low, 16), int(high, 16)
        else:
            yield int(body, 16), int(body, 16)


def latin_faces(html, stylesheet):
    """The app's woff2 URLs for the latin range of FAMILY, in stylesheet order."""
    urls = []
    for sheet in dict.fromkeys(re.findall(r"/_next/static/css/[^\"']+\.css", html)):
        for block in re.findall(r"@font-face\{[^}]*\}", stylesheet(sheet)):
            if not re.search(r"font-family:\s*[\"']?" + FAMILY + r"[\"']?\s*[;}]", block):
                continue
            source = re.search(r"url\(\s*([^)\s]+\.woff2[^)\s]*)\s*\)", block)
            declared = re.search(r"unicode-range:([^;}]+)", block)
            if not source or not declared:
                continue
            spans = list(unicode_ranges(declared.group(1)))
            if all(any(low <= point <= high for low, high in spans) for point in LATIN_PROBES):
                urls.append(urljoin(APP, source.group(1)))
    return list(dict.fromkeys(urls))


def resolve_web_face(fetch):
    html = fetch(APP).decode("utf-8", "replace")
    faces = latin_faces(html, lambda sheet: fetch(urljoin(APP, sheet)).decode("utf-8", "replace"))
    assert len(faces) == 1, f"expected one latin {FAMILY} face, found {faces}"
    return faces[0]


def pinned_web_face(fetch, expected=WEB_DIGEST):
    url = resolve_web_face(fetch)
    data = fetch(url)
    digest = hashlib.sha256(data).hexdigest()
    assert digest == expected, f"{url} hashes to {digest}, not the pinned {FAMILY} face"
    return url, data


def self_test():
    sheet = (
        "@font-face{font-family:Other;src:url(/_next/static/media/aaaa-s.woff2);unicode-range:u+00??}"
        '@font-face{font-family:"IBM Plex Sans";font-weight:100 700;'
        "src:url(/_next/static/media/bbbb-s.p.woff2?dpl=x);unicode-range:u+0460-052f,u+0131}"
        "@font-face{font-family:IBM Plex Sans;font-weight:100 700;"
        "src:url(/_next/static/media/cccc-s.p.woff2);unicode-range:u+00??,u+2000-206f}"
        "@font-face{font-family:IBM Plex Sans;font-weight:100 700;"
        "src:url(/_next/static/media/dddd-s.woff2);unicode-range:u+0301,u+0400-045f}"
    )
    html = '<link rel="stylesheet" href="/_next/static/css/aaaa.css"><link rel="stylesheet" href="/_next/static/css/aaaa.css">'
    latin = APP + "_next/static/media/cccc-s.p.woff2"
    pinned = hashlib.sha256(b"pinned bytes").hexdigest()

    def served(url, css=sheet, font=b"pinned bytes"):
        if url.endswith(".css"):
            return css.encode()
        if url == APP:
            return html.encode()
        return font

    assert pinned_web_face(served, pinned)[0] == latin

    def refused(fetch, why):
        try:
            pinned_web_face(fetch, pinned)
        except AssertionError:
            return
        raise AssertionError(f"accepted {why}")

    refused(lambda url: served(url, font=b"changed bytes"), "changed font bytes")
    refused(lambda url: served(url, css=sheet.replace("IBM Plex Sans", "IBM Plex Mono")), "a missing family")
    refused(lambda url: served(url, css=sheet.replace("u+00??,u+2000-206f", "u+2000-206f")), "a non-latin subset")
    refused(
        lambda url: served(
            url,
            css=sheet + "@font-face{font-family:IBM Plex Sans;src:url(/_next/static/media/eeee-s.woff2);unicode-range:u+00??}",
        ),
        "two latin faces",
    )
    print("WEB_FONT_SOURCE_SELFTEST_OK")


def fetch(url):
    return urlopen(url, timeout=60).read()


def main():
    if "--self-test" in sys.argv:
        self_test()
        return
    if "--resolve" in sys.argv:
        url, data = pinned_web_face(fetch)
        print("WEB_FONT_SOURCE_OK", url, hashlib.sha256(data).hexdigest())
        return

    web_url, web_data = pinned_web_face(fetch)
    full_data = fetch(FULL_URL)
    assert hashlib.sha256(full_data).hexdigest() == FULL_DIGEST, FULL_URL
    SOURCES = {"web": (web_url, WEB_DIGEST), "full": (FULL_URL, FULL_DIGEST)}
    fonts = {"web": TTFont(BytesIO(web_data)), "full": TTFont(BytesIO(full_data))}

    OUT.mkdir(parents=True, exist_ok=True)
    receipts = []
    for weight in [400, 500, 600, 700]:
        faces = {}
        for name, font in fonts.items():
            axes = {axis.axisTag: weight if axis.axisTag == "wght" else axis.defaultValue for axis in font["fvar"].axes}
            face = instantiateVariableFont(font, axes, inplace=False, updateFontNames=True)
            face.flavor = None
            # The web asset (ibmPlexSans.ts:3) is unhinted. Preserve every glyph,
            # but do not import the full source's different TrueType hint programs.
            if name == "full":
                unhinted = subset.Subsetter(options=subset.Options(hinting=False, retain_gids=True, name_IDs=["*"]))
                unhinted.populate(glyphs=face.getGlyphOrder())
                unhinted.subset(face)
            directory = OUT if name == "full" else OUT / "latin-probe"
            directory.mkdir(exist_ok=True)
            face.save(directory / f"IBMPlexSans-{weight}.ttf")
            faces[name] = face
        full, web = faces["full"], faces["web"]
        full_map, web_map = full.getBestCmap(), web.getBestCmap()
        old = TTFont(f"crates/ht-desktop/assets/fonts/IBMPlexSans-{weight}.ttf")
        missing = sorted(set(old.getBestCmap()) - set(full_map))
        mismatches = [code for code, glyph in web_map.items() if full["hmtx"][full_map[code]] != web["hmtx"][glyph]]
        family = full["name"].getDebugName(16) or full["name"].getDebugName(1)
        assert family == "IBM Plex Sans", family
        assert not any(tag in full for tag in ["fpgm", "prep", "cvt "])
        receipts.append({"family": family, "weight": weight, "fullGlyphs": len(full_map), "webGlyphs": len(web_map), "missingExisting": missing, "webMetricDifferences": mismatches})
        assert full["OS/2"].usWeightClass == weight
        assert not missing, receipts[-1]
        assert not mismatches, receipts[-1]
    web = {"url": web_url, "digest": WEB_DIGEST, "family": FAMILY}
    (OUT / "source.json").write_text(json.dumps({"web": web, "sources": SOURCES, "faces": receipts}, indent=2) + "\n")
    print("WEB_FONT_ASSETS_OK", json.dumps(receipts))


main()
