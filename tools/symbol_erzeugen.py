"""Programmsymbol als PNG und ICO erzeugen: Schaltplan-Blatt mit einem kleinen Stromkreis (Schalter und Lampe) auf
blauem Grund.

Nutzer 24.11.2026: das frühere Symbol (ein einzelner schräger Blitz) erinnerte zu stark an eine Rune – bewusst ohne
Blitz und ohne kantige Zeichen.

Aufruf:  python tools/symbol_erzeugen.py resources
Ergebnis: elektroplanung.png (256 px) und elektroplanung.ico (16…256 px)
"""
import sys
from PIL import Image, ImageDraw

S = 1024  # Zeichenauflösung (wird verkleinert → glatte Kanten)

img = Image.new('RGBA', (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# Grund und Blatt
d.rounded_rectangle((40, 40, 984, 984), radius=190, fill=(30, 90, 160, 255))
d.rounded_rectangle((165, 140, 859, 884), radius=44, fill=(255, 255, 255, 255))

# Stromkreis: oben und unten je eine Leitung, links ein Schließer, rechts eine Lampe
L = (40, 40, 40, 255)
W = 44
x1, x2, y1, y2 = 300, 724, 300, 724
d.line((x1, y1, x2, y1), fill=L, width=W)
d.line((x1, y2, x2, y2), fill=L, width=W)
# Schließer: oberes Stück, Lücke, Schaltstück schräg, unteres Stück
d.line((x1, y1, x1, 440), fill=L, width=W)
d.line((x1, 584, x1, y2), fill=L, width=W)
d.line((x1, 584, 392, 452), fill=L, width=W)
# Lampe: Kreis mit Kreuz
cy, r = 512, 104
d.line((x2, y1, x2, cy - r), fill=L, width=W)
d.line((x2, cy + r, x2, y2), fill=L, width=W)
d.ellipse((x2 - r, cy - r, x2 + r, cy + r), fill=(245, 190, 20, 255), outline=L, width=36)
k = int(r * 0.70)
d.line((x2 - k, cy - k, x2 + k, cy + k), fill=L, width=30)
d.line((x2 - k, cy + k, x2 + k, cy - k), fill=L, width=30)
# Ecken rund schließen
for (x, y) in [(x1, y1), (x2, y1), (x1, y2), (x2, y2)]:
    d.ellipse((x - W // 2, y - W // 2, x + W // 2, y + W // 2), fill=L)

out = sys.argv[1] if len(sys.argv) > 1 else '.'
klein = img.resize((256, 256), Image.LANCZOS)
klein.save(f'{out}/elektroplanung.png')
klein.save(f'{out}/elektroplanung.ico', sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print('geschrieben:', out)
