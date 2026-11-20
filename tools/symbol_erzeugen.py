"""Programmsymbol (Blitz auf Blatt, wie app_icon() in src/main.rs) als PNG und ICO erzeugen.

Aufruf:  python tools/symbol_erzeugen.py resources
Ergebnis: elektroplanung.png (256 px) und elektroplanung.ico (16…256 px)
"""
import sys
from PIL import Image

S = 1024           # Zeichenauflösung (wird verkleinert → glatte Kanten)
k = S / 32.0       # Raster des 32-px-Originals

img = Image.new('RGBA', (S, S), (0, 0, 0, 0))
px = img.load()
for y in range(S):
    fy = y / k
    for x in range(S):
        fx = x / k
        inside = 3 <= fx < 29 and 2 <= fy < 30
        border = inside and (fx < 4 or fx >= 28 or fy < 3 or fy >= 29)
        bolt = (5.0 < fy < 17.0 and abs(fx - 0.5 - (20.0 - (fy - 0.5 - 5.0) * 0.6)) < 2.5) or \
               (15.0 <= fy < 27.0 and abs(fx - 0.5 - (17.0 - (fy - 0.5 - 15.0) * 0.6)) < 2.5)
        if border:
            px[x, y] = (40, 40, 40, 255)
        elif bolt:
            px[x, y] = (230, 180, 0, 255)
        elif inside:
            px[x, y] = (255, 255, 255, 255)

out = sys.argv[1] if len(sys.argv) > 1 else '.'
img.resize((256, 256), Image.LANCZOS).save(f'{out}/elektroplanung.png')
img.resize((256, 256), Image.LANCZOS).save(f'{out}/elektroplanung.ico', sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print('geschrieben:', out)
