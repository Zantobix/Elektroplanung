"""Eigene Vektor-Icons (SVG, 24 × 24) für die Werkzeugleisten erzeugen.

Aufruf:  python tools/icons_erzeugen.py        → schreibt resources/icons/*.svg
Einheitlicher Stil: Linien 1,75 px mit runden Enden, dunkle Kontur, wenige Akzentfarben.
Die Icons sind eigene Entwürfe des Projekts (GPL-3.0 wie der übrige Quelltext).
"""
import os

D = '#263238'   # Kontur
B = '#1e6fd9'   # Blau (Akzent)
R = '#d32f2f'   # Rot
Y = '#f2b705'   # Gelb
G = '#90a4ae'   # Grau
LB = '#dbe9fb'  # helles Blau (Flächen)
LY = '#ffe8a3'  # helles Gelb
W = '#ffffff'

def st(col=D, w=1.75):
    return f'fill="none" stroke="{col}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"'

def doc(x0=5, y0=2.5, x1=19, y1=21.5, fill=W):
    """Blatt mit Eselsohr."""
    return (f'<path d="M{x0} {y0}h{x1 - x0 - 4}l4 4v{y1 - y0 - 4}H{x0}z" fill="{fill}" {st()}/>'
            f'<path d="M{x1 - 4} {y0}v4h4" {st()}/>')

def plus(cx, cy, r=3.2, col=B):
    return (f'<circle cx="{cx}" cy="{cy}" r="{r + 1.2}" fill="{W}"/>'
            f'<path d="M{cx - r} {cy}h{2 * r}M{cx} {cy - r}v{2 * r}" {st(col, 2)}/>')

def arrow_head(x, y, dx, dy, col=B, size=3.2):
    """Gefüllte Pfeilspitze an (x, y) in Richtung (dx, dy)."""
    import math
    n = math.hypot(dx, dy)
    ux, uy = dx / n, dy / n
    bx, by = x - ux * size, y - uy * size
    px, py = -uy * size * 0.75, ux * size * 0.75
    return f'<path d="M{x:.2f} {y:.2f}L{bx + px:.2f} {by + py:.2f}L{bx - px:.2f} {by - py:.2f}z" fill="{col}"/>'

ICONS = {
    # Datei
    'new_project': doc() + plus(17, 17),
    'new_sheet': doc(fill=W) + f'<path d="M8 11h7M8 14h7M8 17h4" {st(G, 1.5)}/>' + plus(17.5, 17.5),
    'open': (f'<path d="M2.5 7.5V19a1.5 1.5 0 0 0 1.5 1.5h14" {st()}/>'
             f'<path d="M2.5 7.5V5a1 1 0 0 1 1-1h5l2 2.5h7a1 1 0 0 1 1 1V10" fill="{LY}" {st()}/>'
             f'<path d="M5.5 10.5h16l-3 10H2.5z" fill="{Y}" {st()}/>'),
    'save': (f'<path d="M4 3.5h12.5l3.5 3.5v13.5H4z" fill="{LB}" {st()}/>'
             f'<path d="M7.5 3.5v5h8v-5" fill="{W}" {st()}/>'
             f'<rect x="7" y="13" width="10" height="7.5" fill="{W}" {st()}/>'),
    'save_project': (f'<path d="M7 2h10.5l3 3v11.5" {st(G)}/>'
                     f'<path d="M3 5.5h11.5l3.5 3.5v12H3z" fill="{LB}" {st()}/>'
                     f'<path d="M6.5 5.5v4.5h7V5.5" fill="{W}" {st()}/>'
                     f'<rect x="6" y="14" width="9" height="7" fill="{W}" {st()}/>'),
    'print': (f'<path d="M6.5 9V3h11v6" fill="{W}" {st()}/>'
              f'<rect x="2.5" y="9" width="19" height="8" rx="1.5" fill="{LB}" {st()}/>'
              f'<path d="M6.5 14h11v7.5h-11z" fill="{W}" {st()}/><path d="M9 17.5h6" {st(G, 1.5)}/>'
              f'<circle cx="18" cy="12" r="0.9" fill="{B}"/>'),
    # Bearbeiten
    'cut': (f'<circle cx="6.5" cy="17.5" r="3" {st()}/><circle cx="17.5" cy="17.5" r="3" {st()}/>'
            f'<path d="M8.6 15.3L18 3M15.4 15.3L6 3" {st()}/>'),
    'copy': (f'<rect x="3" y="3" width="11.5" height="13.5" rx="1.5" fill="{W}" {st()}/>'
             f'<rect x="9.5" y="7.5" width="11.5" height="13.5" rx="1.5" fill="{LB}" {st()}/>'),
    'paste': (f'<path d="M15.5 5h2a1.5 1.5 0 0 1 1.5 1.5V9M8.5 5h-2A1.5 1.5 0 0 0 5 6.5v13A1.5 1.5 0 0 0 6.5 21H10" fill="{LY}" {st()}/>'
              f'<rect x="8.5" y="3" width="7" height="4" rx="1" fill="{W}" {st()}/>'
              f'<rect x="12" y="11" width="9.5" height="10.5" rx="1.2" fill="{W}" {st()}/>'),
    'undo': (f'<path d="M5 9h10a5 5 0 0 1 0 10h-4" {st(B, 2.2)}/>' + arrow_head(3.2, 9, -1, 0, size=4.4)),
    'redo': (f'<path d="M19 9H9a5 5 0 0 0 0 10h4" {st(B, 2.2)}/>' + arrow_head(20.8, 9, 1, 0, size=4.4)),
    'zoom': (f'<circle cx="10" cy="10" r="6.5" fill="{LB}" {st()}/><path d="M15 15l6 6" {st(D, 2.6)}/>'
             f'<path d="M7 10h6M10 7v6" {st(B, 1.6)}/>'),
    'new_pen': (f'<path d="M4 20l1-4.5L15.5 5l3.5 3.5L8.5 19z" fill="{LB}" {st()}/>'
                f'<path d="M13.5 7l3.5 3.5" {st()}/><path d="M4 20l1-4.5 3.5 3.5z" fill="{D}"/>' + plus(18.5, 18.5, 2.8)),
    # Raster und Hilfen
    'grid': ''.join(f'<circle cx="{x}" cy="{y}" r="1.15" fill="{B}"/>' for x in (4, 9, 14, 19) for y in (4, 9, 14, 19)),
    'path_align': (f'<path d="M3 12h18" {st(B, 1.5)} stroke-dasharray="2 2.2"/>'
                   f'<rect x="7.5" y="4" width="9" height="16" rx="1" fill="{W}" {st()}/>'
                   f'<path d="M7.5 12H3M16.5 12H21" {st(D, 2)}/><circle cx="12" cy="12" r="1.8" fill="{R}"/>'),
    'auto_conn': (f'<path d="M2.5 12h6M15.5 12h6" {st(D, 2)}/>'
                  f'<rect x="8.5" y="7.5" width="7" height="9" rx="1" fill="{LB}" {st()}/>'
                  f'<path d="M18 3.5l-1.5 3.5h3l-1.5 3.5" {st(Y, 1.8)}/>'
                  f'<circle cx="8.5" cy="12" r="1.5" fill="{B}"/><circle cx="15.5" cy="12" r="1.5" fill="{B}"/>'),
    'size_frame': (f'<rect x="4.5" y="4.5" width="15" height="15" {st(D, 1.4)} stroke-dasharray="2.5 2"/>'
                   + ''.join(f'<rect x="{x - 2}" y="{y - 2}" width="4" height="4" fill="{B}"/>'
                             for x, y in ((4.5, 4.5), (19.5, 4.5), (19.5, 19.5), (4.5, 19.5), (12, 4.5), (12, 19.5), (4.5, 12), (19.5, 12)))),
    'fix': (f'<path d="M12 14v7.5" {st(D, 2)}/>'
            f'<path d="M8 3.5h8l-1 3.5 3 4H6l3-4z" fill="{R}" {st(D, 1.5)}/>'
            f'<path d="M5 14h14" {st(D, 2)}/>'),
    # Neues Element
    'new_potential': (doc() + f'<path d="M8 10h7M8 13h7" {st(B, 1.6)}/>'
                      f'<path d="M14 15.5l-2.5 4h3l-2.5 4" {st(R, 1.8)}/>'),
    'new_busbar': (f'<rect x="2.5" y="10" width="19" height="3.2" rx="0.6" fill="{R}"/>'
                   f'<rect x="2.5" y="15.5" width="19" height="3.2" rx="0.6" fill="{B}"/>'
                   f'<path d="M7 3.5V10M17 3.5V10" {st()}/><circle cx="7" cy="11.6" r="1.1" fill="{W}"/><circle cx="17" cy="11.6" r="1.1" fill="{W}"/>'),
    'new_leitung': (f'<path d="M3 16c4 0 4-8 9-8s5 8 9 8" {st(B, 2.4)}/>'
                    f'<circle cx="3" cy="16" r="2" fill="{D}"/><circle cx="21" cy="16" r="2" fill="{D}"/>'),
    'new_klemme': (''.join(f'<rect x="{x}" y="7" width="5" height="14" rx="0.8" fill="{LB}" {st(D, 1.4)}/>'
                           f'<circle cx="{x + 2.5}" cy="10.5" r="1.3" fill="{Y}" stroke="{D}" stroke-width="0.8"/>' for x in (3, 9.5, 16))
                   + f'<path d="M2.5 3.5h19" {st(R, 2)}/>'),
    'klemmstelle': (f'<path d="M2.5 9h6.5M15.5 9h6" {st(D, 2)}/>'
                    f'<circle cx="12.25" cy="9" r="3.25" fill="{W}" {st(D, 1.8)}/><path d="M12.25 12.25V21" {st(D, 2)}/>'
                    f'<path d="M10 7l4.5 4" {st(D, 1.4)}/>'),
    # Transformieren
    'rotate': (f'<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3" {st(B, 2.2)}/>' + arrow_head(19.8, 4.2, 0.55, -1, size=4.6)),
    'mirror_v': (f'<path d="M12 2.5v19" {st(G, 1.5)} stroke-dasharray="2 2"/>'
                 f'<path d="M9.5 6v12.5H3z" fill="{B}" {st(B, 1.4)}/><path d="M14.5 6v12.5H21z" fill="{W}" {st(B, 1.6)}/>'),
    'mirror_h': (f'<path d="M2.5 12h19" {st(G, 1.5)} stroke-dasharray="2 2"/>'
                 f'<path d="M6 9.5h12.5V3z" fill="{B}" {st(B, 1.4)}/><path d="M6 14.5h12.5V21z" fill="{W}" {st(B, 1.6)}/>'),
    'group': (f'<rect x="2.5" y="2.5" width="19" height="19" rx="1.5" {st(D, 1.4)} stroke-dasharray="2.5 2"/>'
              f'<rect x="6" y="6" width="7" height="7" fill="{LB}" {st(B, 1.6)}/><circle cx="15" cy="15" r="3.8" fill="{W}" {st(R, 1.6)}/>'),
    'ungroup': (f'<rect x="3" y="3" width="8" height="8" fill="{LB}" {st(B, 1.6)}/><circle cx="16.5" cy="16.5" r="4.2" fill="{W}" {st(R, 1.6)}/>'
                f'<path d="M3 21L21 3" {st(D, 1.4)} stroke-dasharray="2.5 2"/>'),
    # Zeichenwerkzeuge
    'select': f'<path d="M5 3l13 8.5-5.5 1.3 3.3 6.7-2.6 1.2-3.2-6.7L5 18z" fill="{W}" {st()}/>',
    'line': f'<path d="M4 20L20 4" {st(D, 2)}/><circle cx="4" cy="20" r="1.8" fill="{B}"/><circle cx="20" cy="4" r="1.8" fill="{B}"/>',
    'curve': f'<path d="M3 17C7 1 13 23 21 6" {st(D, 2)}/>',
    'rect': f'<rect x="3.5" y="5.5" width="17" height="13" rx="0.8" {st(D, 2)}/>',
    'triangle': f'<path d="M12 4l8.5 15.5h-17z" {st(D, 2)}/>',
    'circle': f'<circle cx="12" cy="12" r="8.5" {st(D, 2)}/>',
    'text': f'<path d="M5 20L12 4l7 16M7.6 14h8.8" {st(D, 2.2)}/>',
    'text_box': (f'<rect x="2.5" y="3.5" width="19" height="17" rx="1.2" fill="{W}" {st()}/>'
                 f'<path d="M6 8h12M6 12h12M6 16h8" {st(G, 1.6)}/>'),
    'table': (f'<rect x="3" y="4" width="18" height="16" rx="1" fill="{W}" {st()}/>'
              f'<path d="M3 9h18" {st()}/><rect x="3" y="4" width="18" height="5" fill="{LB}" {st()}/>'
              f'<path d="M3 14.5h18M9.5 4v16M15 4v16" {st(D, 1.3)}/>'),
    'image': (f'<rect x="2.5" y="4" width="19" height="16" rx="1.5" fill="{W}" {st()}/>'
              f'<path d="M2.5 17l5.5-5.5 4.5 4.5 3-3 6 6" fill="none" {st(B, 1.6)}/>'
              f'<circle cx="16" cy="8.5" r="1.8" fill="{Y}"/>'),
    # Suche
    'case_sensitive': (f'<path d="M2.5 17L7 6l4.5 11M4.3 13.2h5.4" {st(B, 1.8)}/>'
                       f'<circle cx="17" cy="14" r="3" {st(B, 1.8)}/><path d="M20 11v6" {st(B, 1.8)}/><path d="M2.5 21h19" {st(B, 1.2)}/>'),
    'find': (f'<circle cx="10" cy="10" r="6.5" fill="{W}" {st()}/><path d="M15 15l6 6" {st(D, 2.6)}/>'),
    # Symboleditor (Leiste BLeiste) und Ausrichtung
    'new_page': (f'<rect x="7" y="2.5" width="12" height="15" rx="1" fill="{W}" {st(G)}/>'
                 f'<rect x="4" y="5.5" width="12" height="15" rx="1" fill="{W}" {st()}/>' + plus(17, 17)),
    'new_option': (f'<rect x="3" y="4" width="13" height="13" rx="2" fill="{LB}" {st()}/>'
                   f'<path d="M6.5 10.5l2.5 2.5 4-5" {st(B, 2)}/>' + plus(17.5, 17.5)),
    'properties': (f'<rect x="3" y="3" width="18" height="18" rx="1.5" fill="{W}" {st()}/>'
                   f'<path d="M7 8h2M11.5 8H17M7 12h2M11.5 12H17M7 16h2M11.5 16H15" {st(D, 1.6)}/>'),
    'apply': f'<circle cx="12" cy="12" r="9" fill="#2e9e4f"/><path d="M7.5 12.5l3 3 6-7" {st(W, 2.4)}/>',
    'cancel': f'<circle cx="12" cy="12" r="9" fill="{R}"/><path d="M8.5 8.5l7 7M15.5 8.5l-7 7" {st(W, 2.4)}/>',
    'option_toggle': (f'<rect x="2.5" y="7" width="19" height="10" rx="5" fill="{LB}" {st()}/>'
                      f'<circle cx="15.5" cy="12" r="3.3" fill="{B}"/>'),
    'align_hor': (f'<path d="M2.5 12h19" {st(R, 1.5)} stroke-dasharray="2 2"/>'
                  f'<rect x="4" y="7" width="6" height="10" fill="{LB}" {st()}/><rect x="14" y="9" width="6" height="6" fill="{LB}" {st()}/>'),
    'align_ver': (f'<path d="M12 2.5v19" {st(R, 1.5)} stroke-dasharray="2 2"/>'
                  f'<rect x="7" y="4" width="10" height="6" fill="{LB}" {st()}/><rect x="9" y="14" width="6" height="6" fill="{LB}" {st()}/>'),
    'space_hor': (f'<rect x="2.5" y="8" width="4" height="8" fill="{LB}" {st()}/><rect x="10" y="8" width="4" height="8" fill="{LB}" {st()}/>'
                  f'<rect x="17.5" y="8" width="4" height="8" fill="{LB}" {st()}/><path d="M6.5 19.5h3.5M14 19.5h3.5" {st(R, 1.5)}/>'),
    'space_ver': (f'<rect x="8" y="2.5" width="8" height="4" fill="{LB}" {st()}/><rect x="8" y="10" width="8" height="4" fill="{LB}" {st()}/>'
                  f'<rect x="8" y="17.5" width="8" height="4" fill="{LB}" {st()}/><path d="M19.5 6.5V10M19.5 14v3.5" {st(R, 1.5)}/>'),
    'part_list': (f'<rect x="3" y="4" width="18" height="16" rx="1.5" fill="{W}" {st()}/>'
                  f'<path d="M6.5 9h2M11 9h6.5M6.5 13h2M11 13h6.5M6.5 17h2M11 17h4" {st(D, 1.6)}/>'),
    # Listen (Tabellendialog TSTab): Liste laden, Liste speichern, Alles löschen, Zeile löschen
    'list_load': (f'<path d="M2.5 7V19.5h15" {st()}/>'
                  f'<path d="M2.5 7V4.5a1 1 0 0 1 1-1h4.5l2 2.5h6a1 1 0 0 1 1 1V9" fill="{LY}" {st()}/>'
                  f'<rect x="6" y="7.5" width="12" height="9" fill="{W}" {st(D, 1.3)}/>'
                  f'<path d="M8.5 10.5h7M8.5 13.5h7" {st(B, 1.4)}/>'
                  f'<path d="M5 12.5h16.5l-3 8H2.5z" fill="{Y}" {st()}/>'),
    'list_save': (f'<path d="M3 3h13.5l4.5 4.5V21H3z" fill="{LB}" {st()}/>'
                  f'<path d="M7 3v5h8.5V3" fill="{W}" {st()}/>'
                  f'<rect x="6.5" y="12" width="11" height="9" fill="{W}" {st()}/>'
                  f'<path d="M9 15h6M9 18h6" {st(B, 1.4)}/>'),
    'clear_all': (f'<rect x="3" y="4" width="15" height="14" rx="1" fill="{W}" {st()}/>'
                  f'<path d="M3 8.7h15M3 13.3h15M8 4v14M13 4v14" {st(G, 1.2)}/>'
                  f'<circle cx="17.5" cy="17.5" r="4.8" fill="{R}"/>'
                  f'<path d="M15.3 15.3l4.4 4.4M19.7 15.3l-4.4 4.4" {st(W, 1.8)}/>'),
    'delete_row': (f'<rect x="3" y="3.5" width="15" height="15" rx="1" fill="{W}" {st()}/>'
                   f'<rect x="3" y="8.5" width="15" height="5" fill="#f8d0d0"/>'
                   f'<path d="M3 8.5h15M3 13.5h15M8.5 3.5v15" {st(G, 1.2)}/>'
                   f'<rect x="3" y="3.5" width="15" height="15" rx="1" {st()}/>'
                   f'<circle cx="17.5" cy="17.5" r="4.8" fill="{R}"/>'
                   f'<path d="M14.7 17.5h5.6" {st(W, 2)}/>'),
}


# Symbole des Projektexplorers (16 × 16 wie die Baumsymbole des Originals, aber eigene Entwürfe):
# kräftigere Formen und Farben, damit sie in Originalgröße lesbar bleiben.
CY = '#19d3e6'  # Türkis (Bücher der Symbolbibliothek)
DC = '#0b6f8a'  # dunkles Türkis (Buchrücken)
BR = '#b8401f'  # Rotbraun (Sammelschiene)
GN = '#2e9e4f'  # Grün


def book(x, y, w=6.5, h=10, shadow=True):
    """Buch der Symbolbibliothek: türkiser Einband mit dunklem Rücken, grauer Schatten."""
    s = f'<rect x="{x - 1.3}" y="{y - 1.3}" width="{w}" height="{h}" fill="{G}"/>' if shadow else ''
    return (s + f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{CY}" {st(D, 1)}/>'
            f'<path d="M{x + 1.6} {y}v{h}" {st(DC, 1.2)}/>')


def arrow16(x, y0, y1, col):
    """Senkrechter Pfeil von y0 nach y1 mit gefüllter Spitze."""
    d = 1 if y1 > y0 else -1
    return (f'<path d="M{x} {y0}V{y1 - d * 2.2}" {st(col, 1.4)}/>'
            f'<path d="M{x} {y1}l-2.2 {-d * 2.8}h4.4z" fill="{col}"/>')


ICONS16 = {
    # Potentiale: Formelzeichen U mit Pfeilen nach oben und unten
    'tree_potentiale': (f'<path d="M2 2.5v6.5a3.25 3.25 0 0 0 6.5 0V2.5" {st(D, 2)}/>'
                        + arrow16(12.5, 7, 1.2, R) + arrow16(12.5, 9, 14.8, B)),
    # Leitungen: aufgewickelte Leitung mit Aderende
    'tree_leitungen': (f'<circle cx="7" cy="6" r="4.6" {st(D, 2.2)}/><circle cx="7" cy="6" r="2" {st(G, 1)}/>'
                       f'<path d="M10.5 9c1.5 1.6 1.5 3 0.5 4.5" {st(D, 1.4)}/>'
                       f'<rect x="9" y="12.5" width="3.2" height="3" rx="0.6" fill="{B}"/><rect x="12.2" y="13.3" width="2.3" height="1.4" fill="{Y}"/>'),
    # Objektkatalog: Rahmen mit schräger Linie (Zeichenobjekt)
    'tree_objektkatalog': (f'<rect x="1" y="3" width="14" height="10" fill="{W}" {st(D, 1.2)}/>'
                           f'<path d="M3 11L13 5" {st(D, 1.4)}/>'),
    # Stifte: schräger Zeichenstift
    'tree_stifte': (f'<path d="M1.5 3.5l2-2 9 9-2 2z" fill="{D}" {st(D, 1)}/>'
                    f'<path d="M10.5 12.5l2-2 2.5 4.5z" fill="{LY}" {st(D, 0.9)}/><path d="M14.2 14.2l0.8 0.8" {st(D, 1.4)}/>'),
    # Grafiken: Fotoapparat
    'tree_grafiken': (f'<rect x="1" y="5" width="14" height="9" rx="1.2" fill="{D}"/><rect x="3" y="3" width="4" height="2.5" fill="{D}"/>'
                      f'<circle cx="9.5" cy="9.5" r="3" fill="{G}" stroke="{W}" stroke-width="1.2"/><circle cx="3.5" cy="7" r="0.8" fill="{Y}"/>'),
    # Bauteile: Reiheneinbaugerät mit Anschlussklemmen und Hutschiene
    'tree_bauteile': (f'<rect x="3" y="1.5" width="10" height="7.5" fill="{LB}" {st(D, 1.2)}/>'
                      + ''.join(f'<circle cx="{x}" cy="4" r="0.9" fill="{D}"/>' for x in (5.5, 8, 10.5))
                      + f'<path d="M5.5 9v4.5M10.5 9v4.5M1.5 14.5h13" {st(D, 1.4)}/>'),
    # Platzhaltergruppen: offener Ring („C“) mit T (Textersetzung)
    'tree_platzhalter': (f'<path d="M13.2 4.2A6.5 6.5 0 1 0 13.2 11.8" {st(D, 2)}/>'
                         f'<path d="M5 5h6M8 5v6.5" {st(D, 1.8)}/><circle cx="13.6" cy="4" r="1.2" fill="{R}"/>'),
    # Sammelschienen: Kupferschiene mit zwei Schrauben
    'tree_sammelschienen': (f'<rect x="0.5" y="4.5" width="15" height="7" rx="0.8" fill="{BR}"/>'
                            f'<rect x="0.5" y="4.5" width="15" height="1.6" fill="{Y}"/>'
                            + ''.join(f'<circle cx="{x}" cy="8.4" r="1.9" fill="#d0d0d0" stroke="{D}" stroke-width="0.6"/>'
                                      f'<path d="M{x - 1.1} {8.4 + 1.1}l2.2 -2.2" stroke="{D}" stroke-width="0.6"/>' for x in (4.5, 11.5))),
    # Klemmleisten: Reihe von Reihenklemmen (grau, blau, grün-gelb)
    'tree_klemmleisten': ''.join(f'<rect x="{0.5 + i * 3}" y="2" width="3" height="12.5" fill="{c}" stroke="{D}" stroke-width="0.6"/>'
                                 f'<rect x="{1.4 + i * 3}" y="3.2" width="1.2" height="2" fill="{D}"/>'
                                 for i, c in enumerate(['#a8a8a8', '#a8a8a8', B, B, GN]))
                         + f'<path d="M12.5 7h3M12.5 10.5h3" stroke="{Y}" stroke-width="1.4"/>',
    # Blätter: Stapel von Blättern
    'tree_blaetter': (f'<rect x="6" y="1" width="8" height="11" fill="{W}" {st(G, 1)}/>'
                      f'<rect x="4" y="2.5" width="8" height="11" fill="{W}" {st(G, 1)}/>'
                      f'<rect x="2" y="4" width="8" height="11" fill="{W}" {st(D, 1.1)}/>'
                      f'<path d="M3.8 7.5h4.4M3.8 9.8h4.4M3.8 12.1h3" {st(G, 0.9)}/>'),
    # Gruppe der Symbolbibliothek: geschlossen ein Buch, geöffnet drei Bücher
    'tree_buch': book(5.5, 4.5),
    'tree_buecher': book(1.5, 1.5, shadow=False) + book(4.5, 3.5, shadow=False) + book(7.5, 5.5, shadow=False),
}


def write_icon(out, name, body, size):
    import re
    # Elemente mit eigener Füllung: das fill="none" aus st() entfernen (doppelte Attribute sind ungültig)
    body = re.sub(r'<[^>]+>', lambda m: m.group(0).replace(' fill="none"', '', 1) if m.group(0).count('fill=') > 1 else m.group(0), body)
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">{body}</svg>\n'
    with open(os.path.join(out, name + '.svg'), 'w', encoding='utf-8', newline='\n') as f:
        f.write(svg)


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = os.path.join(root, 'resources', 'icons')
    os.makedirs(out, exist_ok=True)
    for name, body in ICONS.items():
        write_icon(out, name, body, 24)
    for name, body in ICONS16.items():
        write_icon(out, name, body, 16)
    print(f'{len(ICONS) + len(ICONS16)} Icons nach {out}')


if __name__ == '__main__':
    main()
