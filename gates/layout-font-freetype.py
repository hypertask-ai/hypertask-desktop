"""CI-only FreeType advance diagnostics for the source-derived font faces."""
import ctypes as C
from ctypes.util import find_library
import json
from pathlib import Path

from fontTools.ttLib import TTFont

ft = C.CDLL(find_library("freetype"))
ft.FT_Init_FreeType.argtypes = [C.POINTER(C.c_void_p)]
ft.FT_New_Face.argtypes = [C.c_void_p, C.c_char_p, C.c_long, C.POINTER(C.c_void_p)]
ft.FT_Set_Char_Size.argtypes = [C.c_void_p, C.c_long, C.c_long, C.c_uint, C.c_uint]
ft.FT_Get_Char_Index.argtypes = [C.c_void_p, C.c_ulong]
ft.FT_Get_Advance.argtypes = [C.c_void_p, C.c_uint, C.c_int32, C.POINTER(C.c_long)]
ft.FT_Done_Face.argtypes = [C.c_void_p]
ft.FT_Done_FreeType.argtypes = [C.c_void_p]
library = C.c_void_p()
assert ft.FT_Init_FreeType(C.byref(library)) == 0
receipts = []
try:
    for directory in ["evidence/web-fonts", "evidence/web-fonts/latin-probe", "crates/ht-desktop/assets/fonts"]:
        for weight, size, text in [(400, 14, "Command center"), (600, 14, "Important"), (400, 14, "Agents"), (400, 11, "1")]:
            path = Path(directory) / f"IBMPlexSans-{weight}.ttf"
            face = C.c_void_p()
            assert ft.FT_New_Face(library, str(path).encode(), 0, C.byref(face)) == 0
            try:
                assert ft.FT_Set_Char_Size(face, 0, size * 64, 72, 72) == 0
                probes = {}
                for name, flags in {"default": 0, "noHint": 2, "autoHint": 32, "light": 1 << 16, "autoLight": 32 | (1 << 16)}.items():
                    advances = {}
                    for char in sorted(set(text)):
                        advance = C.c_long()
                        glyph = ft.FT_Get_Char_Index(face, ord(char))
                        assert ft.FT_Get_Advance(face, glyph, flags, C.byref(advance)) == 0
                        advances[char] = advance.value / 65536
                    probes[name] = {"width": sum(advances[c] for c in text), "glyphs": advances}
                receipts.append({"path": str(path), "weight": TTFont(path)["OS/2"].usWeightClass, "size": size, "text": text, "probes": probes})
            finally:
                ft.FT_Done_Face(face)
finally:
    ft.FT_Done_FreeType(library)
Path("evidence/web-fonts/freetype.json").write_text(json.dumps(receipts, indent=2) + "\n")
for receipt in receipts:
    print("FREETYPE_METRIC", json.dumps({**receipt, "probes": {k: v["width"] for k, v in receipt["probes"].items()}}))
