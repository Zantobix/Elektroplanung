"""Eigene Vektor-Icons (SVG, 24 × 24) für Werkzeugleisten, Projektbaum und Dialogknöpfe erzeugen.

Aufruf:  python tools/icons_erzeugen.py                      → schreibt resources/icons/*.svg
         python tools/icons_erzeugen.py --uebersicht X.html  → zusätzlich Übersichtsseite (alle Icons mit Verwendung)

Einheitliches System:
- Je Objektart EIN Grund-Icon (`obj_*`), das überall für diese Art steht (Werkzeugleiste, Projektbaum, Dialoge).
- Abzeichen unten rechts, immer gleich groß und an derselben Stelle: grünes „+“ = neu anlegen (`new_*`),
  Stift = bearbeiten (`edit_*`), rotes „−“ = entfernen (`del_*`).
- Ein Raster (24 × 24, Inhalt in 2…22), drei Strichstärken (DUENN 1,25 · NORMAL 1,75 · DICK 2,25) mit runden
  Enden, eine Farbpalette. Der Projektbaum zeigt dieselben Dateien in 16 × 16.
Die Icons sind eigene Entwürfe des Projekts (GPL-3.0 wie der übrige Quelltext); es werden keine Grafiken des
Originals verwendet.
"""
import math
import os
import re
import sys

# Palette
D = '#263238'   # Kontur
B = '#1e6fd9'   # Blau (Akzent)
R = '#d32f2f'   # Rot
Y = '#f2b705'   # Gelb
G = '#90a4ae'   # Grau
LG = '#cfd8dc'  # helles Grau (Flächen)
LB = '#dbe9fb'  # helles Blau (Flächen)
LY = '#ffe8a3'  # helles Gelb
W = '#ffffff'
GN = '#2e9e4f'  # Grün (Abzeichen „neu“, Übernehmen)
CU = '#c26a2e'  # Kupfer (Sammelschiene)
CUL = '#e9a46f'  # Kupfer hell
BR = '#8d5524'  # Braun (Ader)
CY = '#19c3d6'  # Türkis (Bücher der Symbolbibliothek)
DC = '#0b6f8a'  # dunkles Türkis (Buchrücken)

# Strichstärken
DUENN = 1.25
NORMAL = 1.75
DICK = 2.25


def st(col=D, w=NORMAL):
    return f'fill="none" stroke="{col}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"'


def arrow_head(x, y, dx, dy, col=B, size=3.2):
    """Gefüllte Pfeilspitze an (x, y) in Richtung (dx, dy)."""
    n = math.hypot(dx, dy)
    ux, uy = dx / n, dy / n
    bx, by = x - ux * size, y - uy * size
    px, py = -uy * size * 0.75, ux * size * 0.75
    return f'<path d="M{x:.2f} {y:.2f}L{bx + px:.2f} {by + py:.2f}L{bx - px:.2f} {by - py:.2f}z" fill="{col}"/>'


# ---------------------------------------------------------------------------------------------------------------
# Abzeichen (unten rechts, Mitte 18/18): weißer Rand, damit sie sich vom Grund-Icon abheben

BX, BY = 18.0, 18.0


def _scheibe(col):
    return f'<circle cx="{BX}" cy="{BY}" r="6" fill="{W}"/><circle cx="{BX}" cy="{BY}" r="4.75" fill="{col}"/>'


def badge_neu():
    return _scheibe(GN) + f'<path d="M{BX - 2.6} {BY}h5.2M{BX} {BY - 2.6}v5.2" {st(W, NORMAL)}/>'


def badge_entfernen():
    return _scheibe(R) + f'<path d="M{BX - 2.6} {BY}h5.2" {st(W, NORMAL)}/>'


def badge_bearbeiten():
    # kleiner gelber Stift auf weißer Scheibe
    return (f'<circle cx="{BX}" cy="{BY}" r="6" fill="{W}"/>'
            f'<path d="M{BX - 3} {BY + 3}L{BX + 2.6} {BY - 2.6}" {st(D, 3.6)}/>'
            f'<path d="M{BX - 2.2} {BY + 2.2}L{BX + 2.6} {BY - 2.6}" {st(Y, 1.6)}/>'
            f'<circle cx="{BX - 3.1}" cy="{BY + 3.1}" r="0.9" fill="{D}"/>')


# ---------------------------------------------------------------------------------------------------------------
# Grund-Icons je Objektart

def blatt():
    """Blatt mit Eselsohr und Textzeilen."""
    return (f'<path d="M5 2.5h10l4 4v15H5z" fill="{W}" {st()}/><path d="M15 2.5v4h4" {st()}/>'
            f'<path d="M8 10.5h8M8 14h8M8 17.5h5" {st(G, DUENN)}/>')


def projekt():
    """Projekt: Aktenordner mit Rückenschild und Griffloch."""
    return (f'<rect x="5" y="2.5" width="14" height="19" rx="1.5" fill="{B}" {st()}/>'
            f'<rect x="8" y="5.5" width="8" height="6" rx="0.8" fill="{W}" {st(D, DUENN)}/>'
            f'<path d="M9.8 8.5h4.4" {st(G, DUENN)}/>'
            f'<circle cx="12" cy="16.5" r="2" fill="{W}" {st(D, DUENN)}/>')


def anlage():
    """Anlage: Schaltschrank mit Warndreieck, Griff und Füßen."""
    return (f'<path d="M6.5 19.5v2M17.5 19.5v2" {st(D, DICK)}/>'
            f'<rect x="4" y="2.5" width="16" height="17" rx="1" fill="{LG}" {st()}/>'
            f'<path d="M16.8 8.5v4" {st(D, DICK)}/>'
            f'<path d="M10.5 6l3.4 6H7.1z" fill="{Y}" {st(D, DUENN)}/>'
            f'<path d="M7 15.5h7" {st(G, DUENN)}/>')


def potential():
    """Potential: Blitz."""
    return f'<path d="M13.5 2.5L5 13.5h6l-1.5 8 9-11.5h-6z" fill="{Y}" {st()}/>'


def sammelschiene():
    """Sammelschiene: Kupferschiene mit zwei Schrauben."""
    schraube = ''.join(f'<circle cx="{x}" cy="12" r="2" fill="{LG}" {st(D, DUENN)}/>'
                       f'<path d="M{x - 1.1} 13.1l2.2-2.2" {st(D, DUENN)}/>' for x in (7, 17))
    return (f'<rect x="2" y="8" width="20" height="8" rx="1" fill="{CU}"/>'
            f'<rect x="2" y="8" width="20" height="2.2" rx="1" fill="{CUL}"/>'
            f'<rect x="2" y="8" width="20" height="8" rx="1" {st()}/>' + schraube)


def leitung():
    """Leitung: Kabel (Mantel) mit zwei abisolierten Adern."""
    return (f'<path d="M3 20.5C3 14 9 14 11 11s3-6 5.5-6" {st(D, 3.4)}/>'
            f'<path d="M16.5 5L21 3M16.5 5L21 8" {st(D, 3.2)}/>'
            f'<path d="M16.8 5L21 3" {st(B, NORMAL)}/><path d="M16.8 5L21 8" {st(BR, NORMAL)}/>')


def klemme():
    """Klemme: Schaltzeichen Klemmstelle (Kreis mit Schrägstrich) mit Anschlüssen."""
    return (f'<path d="M2.5 9h6.5M15.5 9h6" {st(D, DICK)}/>'
            f'<circle cx="12.25" cy="9" r="3.25" fill="{W}" {st()}/><path d="M12.25 12.25V21" {st(D, DICK)}/>'
            f'<path d="M10 6.75l4.5 4.5" {st(D, DUENN)}/>')


def klemmleiste():
    """Klemmleiste: vier Reihenklemmen (grau, grau, blau, grün-gelb) mit Klemmschrauben."""
    farben = [LG, LG, '#5c9ded', GN]
    s = ''
    for i, c in enumerate(farben):
        x = 2.5 + i * 4.75
        s += f'<rect x="{x}" y="3" width="4.75" height="18" fill="{c}"/>'
        if c == GN:
            s += f'<rect x="{x + 1.8}" y="3" width="1.15" height="18" fill="{Y}"/>'
    s += f'<rect x="2.5" y="3" width="19" height="18" rx="0.8" {st()}/>'
    s += f'<path d="M7.25 3v18M12 3v18M16.75 3v18" {st(D, DUENN)}/>'
    for i in range(4):
        cx = 4.875 + i * 4.75
        for cy in (7, 17):
            s += f'<circle cx="{cx}" cy="{cy}" r="1.45" fill="{W}" stroke="{D}" stroke-width="1"/>'
            s += f'<path d="M{cx - 0.8} {cy + 0.8}l1.6-1.6" stroke="{D}" stroke-width="0.9"/>'
    return s


def stift():
    """Stift: gelber Zeichenstift."""
    return (f'<path d="M4 20l1-4.5L15.5 5l3.5 3.5L8.5 19z" fill="{LY}" {st()}/>'
            f'<path d="M13.5 7l3.5 3.5" {st()}/><path d="M4 20l1-4.5 3.5 3.5z" fill="{D}"/>')


def symbol():
    """Symbol (Objektkatalog): Schaltzeichen Schließer auf hellblauer Karte."""
    return (f'<rect x="3" y="3" width="18" height="18" rx="2" fill="{LB}" {st(B, DUENN)}/>'
            f'<path d="M12 3v5M12 16v5M12 16L7.8 8.2M12 8h-2" {st()}/>')


def bauteil():
    """Bauteil: Reiheneinbaugerät auf der Hutschiene."""
    return (f'<path d="M2.5 12h4M17.5 12h4" {st(D, DICK)}/>'
            f'<rect x="6.5" y="2.5" width="11" height="19" rx="1" fill="{W}" {st()}/>'
            f'<rect x="9" y="8.5" width="6" height="7" rx="0.6" fill="{LB}" {st(D, DUENN)}/>'
            f'<rect x="10.6" y="9.6" width="2.8" height="2.6" fill="{B}"/>'
            f'<circle cx="12" cy="5.2" r="1.1" fill="{D}"/><circle cx="12" cy="18.8" r="1.1" fill="{D}"/>')


def platzhalter():
    """Platzhaltergruppe: geschweifte Klammern mit Auslassung."""
    return (f'<path d="M9 3.5c-2 0-2.5 1-2.5 3V9c0 1.5-1 3-2.5 3 1.5 0 2.5 1.5 2.5 3v2.5c0 2 .5 3 2.5 3" {st(B)}/>'
            f'<path d="M15 3.5c2 0 2.5 1 2.5 3V9c0 1.5 1 3 2.5 3-1.5 0-2.5 1.5-2.5 3v2.5c0 2-.5 3-2.5 3" {st(B)}/>'
            + ''.join(f'<circle cx="{x}" cy="12" r="1.1" fill="{D}"/>' for x in (9.5, 12, 14.5)))


def tabelle():
    return (f'<rect x="3" y="4" width="18" height="16" rx="1" fill="{W}"/>'
            f'<rect x="3" y="4" width="18" height="5" fill="{LB}"/>'
            f'<path d="M3 9h18M3 14.5h18M9 4v16M15 4v16" {st(D, DUENN)}/>'
            f'<rect x="3" y="4" width="18" height="16" rx="1" {st()}/>')


def text():
    return f'<path d="M5 20L12 4l7 16M7.6 14h8.8" {st(D, DICK)}/>'


def textbox():
    return (f'<rect x="2.5" y="3.5" width="19" height="17" rx="1.2" fill="{W}" {st()}/>'
            f'<path d="M5.5 16.5l3-8.5 3 8.5M6.6 13.8h3.8" {st(D, NORMAL)}/>'
            f'<path d="M14 9h4.5M14 12.5h4.5M14 16h3" {st(G, DUENN)}/>')


def bild():
    return (f'<rect x="2.5" y="4" width="19" height="16" rx="1.5" fill="{W}" {st()}/>'
            f'<path d="M2.5 17l5.5-5.5 4.5 4.5 3-3 6 6" {st(B, NORMAL)}/>'
            f'<circle cx="16" cy="8.5" r="1.8" fill="{Y}"/>')


def seite():
    """Seite eines Symbols (Bildvariante): zwei Blätter übereinander."""
    return (f'<rect x="7" y="2.5" width="12" height="15" rx="1" fill="{W}" {st(G)}/>'
            f'<rect x="4" y="5.5" width="12" height="15" rx="1" fill="{W}" {st()}/>')


def option():
    return (f'<rect x="3" y="4" width="13" height="13" rx="2" fill="{LB}" {st()}/>'
            f'<path d="M6.5 10.5l2.5 2.5 4-5" {st(B, DICK)}/>')


def book(x, y, w=8, h=14, schatten=True):
    """Buch der Symbolbibliothek: türkiser Einband mit dunklem Rücken, grauer Schatten."""
    s = f'<rect x="{x - 2}" y="{y - 2}" width="{w}" height="{h}" fill="{G}"/>' if schatten else ''
    return (s + f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{CY}" {st(D, DUENN)}/>'
            f'<path d="M{x + 2.2} {y}v{h}" {st(DC, NORMAL)}/>')


OBJ = {
    'projekt': projekt, 'blatt': blatt, 'anlage': anlage, 'potential': potential,
    'sammelschiene': sammelschiene, 'leitung': leitung, 'klemmleiste': klemmleiste, 'klemme': klemme,
    'stift': stift, 'symbol': symbol, 'bauteil': bauteil, 'platzhalter': platzhalter, 'bild': bild,
    'tabelle': tabelle, 'text': text, 'textbox': textbox, 'seite': seite, 'option': option,
    'buch': lambda: book(9, 5.5), 'buecher': lambda: book(3, 2.5, schatten=False) + book(7.5, 5, schatten=False) + book(12, 7.5, schatten=False),
}

ICONS = {f'obj_{k}': f() for k, f in OBJ.items()}
# „Neu“: Grund-Icon + grünes Plus
for k in ['projekt', 'blatt', 'potential', 'sammelschiene', 'leitung', 'klemmleiste', 'stift', 'seite', 'option']:
    ICONS[f'new_{k}'] = OBJ[k]() + badge_neu()
# „Bearbeiten“: Grund-Icon + Stift
for k in ['sammelschiene', 'bauteil', 'symbol']:
    ICONS[f'edit_{k}'] = OBJ[k]() + badge_bearbeiten()
# „Entfernen“: Grund-Icon + rotes Minus
ICONS['del_zeile'] = tabelle() + badge_entfernen()

ICONS.update({
    # Datei
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
              f'<path d="M6.5 14h11v7.5h-11z" fill="{W}" {st()}/><path d="M9 17.5h6" {st(G, DUENN)}/>'
              f'<circle cx="18" cy="12" r="0.9" fill="{B}"/>'),
    'delete': (f'<path d="M6 6.5l1 14.5h10l1-14.5" fill="{W}" {st()}/>'
               f'<path d="M3.5 6.5h17M9.5 6.5V3.5h5v3" {st()}/>'
               f'<path d="M10 10v7.5M14 10v7.5" {st(R, DUENN)}/>'),
    # Bearbeiten
    'cut': (f'<circle cx="6.5" cy="17.5" r="3" {st()}/><circle cx="17.5" cy="17.5" r="3" {st()}/>'
            f'<path d="M8.6 15.3L18 3M15.4 15.3L6 3" {st()}/>'),
    'copy': (f'<rect x="3" y="3" width="11.5" height="13.5" rx="1.5" fill="{W}" {st()}/>'
             f'<rect x="9.5" y="7.5" width="11.5" height="13.5" rx="1.5" fill="{LB}" {st()}/>'),
    'paste': (f'<path d="M15.5 5h2a1.5 1.5 0 0 1 1.5 1.5V9M8.5 5h-2A1.5 1.5 0 0 0 5 6.5v13A1.5 1.5 0 0 0 6.5 21H10" fill="{LY}" {st()}/>'
              f'<rect x="8.5" y="3" width="7" height="4" rx="1" fill="{W}" {st()}/>'
              f'<rect x="12" y="11" width="9.5" height="10.5" rx="1.2" fill="{W}" {st()}/>'),
    'undo': (f'<path d="M5 9h10a5 5 0 0 1 0 10h-4" {st(B, DICK)}/>' + arrow_head(3.2, 9, -1, 0, size=4.4)),
    'redo': (f'<path d="M19 9H9a5 5 0 0 0 0 10h4" {st(B, DICK)}/>' + arrow_head(20.8, 9, 1, 0, size=4.4)),
    'zoom': (f'<circle cx="10" cy="10" r="6.5" fill="{LB}" {st()}/><path d="M15 15l6 6" {st(D, 2.75)}/>'
             f'<path d="M7 10h6M10 7v6" {st(B, NORMAL)}/>'),
    # Raster und Hilfen
    'grid': ''.join(f'<circle cx="{x}" cy="{y}" r="1.15" fill="{B}"/>' for x in (4, 9, 14, 19) for y in (4, 9, 14, 19)),
    'path_align': (f'<path d="M3 12h18" {st(B, DUENN)} stroke-dasharray="2 2.2"/>'
                   f'<rect x="7.5" y="4" width="9" height="16" rx="1" fill="{W}" {st()}/>'
                   f'<path d="M7.5 12H3M16.5 12H21" {st(D, DICK)}/><circle cx="12" cy="12" r="1.8" fill="{R}"/>'),
    'auto_conn': (f'<path d="M2.5 14h6M15.5 14h6" {st(D, DICK)}/>'
                  f'<rect x="8.5" y="9.5" width="7" height="9" rx="1" fill="{LB}" {st()}/>'
                  f'<path d="M18.5 2.5l1 2.5 2.5 1-2.5 1-1 2.5-1-2.5-2.5-1 2.5-1z" fill="{B}"/>'
                  f'<circle cx="8.5" cy="14" r="1.5" fill="{B}"/><circle cx="15.5" cy="14" r="1.5" fill="{B}"/>'),
    'size_frame': (f'<rect x="4.5" y="4.5" width="15" height="15" {st(D, DUENN)} stroke-dasharray="2.5 2"/>'
                   + ''.join(f'<rect x="{x - 2}" y="{y - 2}" width="4" height="4" fill="{B}"/>'
                             for x, y in ((4.5, 4.5), (19.5, 4.5), (19.5, 19.5), (4.5, 19.5), (12, 4.5), (12, 19.5), (4.5, 12), (19.5, 12)))),
    'fix': (f'<path d="M12 14v7.5" {st(D, DICK)}/>'
            f'<path d="M8 3.5h8l-1 3.5 3 4H6l3-4z" fill="{R}" {st(D, DUENN)}/>'
            f'<path d="M5 14h14" {st(D, DICK)}/>'),
    # Transformieren
    'rotate': (f'<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3" {st(B, DICK)}/>' + arrow_head(19.8, 4.2, 0.55, -1, size=4.6)),
    'mirror_v': (f'<path d="M12 2.5v19" {st(G, DUENN)} stroke-dasharray="2 2"/>'
                 f'<path d="M9.5 6v12.5H3z" fill="{B}" {st(B, DUENN)}/><path d="M14.5 6v12.5H21z" fill="{W}" {st(B, NORMAL)}/>'),
    'mirror_h': (f'<path d="M2.5 12h19" {st(G, DUENN)} stroke-dasharray="2 2"/>'
                 f'<path d="M6 9.5h12.5V3z" fill="{B}" {st(B, DUENN)}/><path d="M6 14.5h12.5V21z" fill="{W}" {st(B, NORMAL)}/>'),
    'group': (f'<rect x="2.5" y="2.5" width="19" height="19" rx="1.5" {st(D, DUENN)} stroke-dasharray="2.5 2"/>'
              f'<rect x="6" y="6" width="7" height="7" fill="{LB}" {st(B, NORMAL)}/><circle cx="15" cy="15" r="3.8" fill="{W}" {st(R, NORMAL)}/>'),
    'ungroup': (f'<rect x="3" y="3" width="8" height="8" fill="{LB}" {st(B, NORMAL)}/><circle cx="16.5" cy="16.5" r="4.2" fill="{W}" {st(R, NORMAL)}/>'
                f'<path d="M3 21L21 3" {st(D, DUENN)} stroke-dasharray="2.5 2"/>'),
    # Zeichenwerkzeuge (Werkzeugwahl, daher ohne Abzeichen)
    'select': f'<path d="M5 3l13 8.5-5.5 1.3 3.3 6.7-2.6 1.2-3.2-6.7L5 18z" fill="{W}" {st()}/>',
    'line': f'<path d="M4 20L20 4" {st(D, DICK)}/><circle cx="4" cy="20" r="1.8" fill="{B}"/><circle cx="20" cy="4" r="1.8" fill="{B}"/>',
    'curve': f'<path d="M3 17C7 1 13 23 21 6" {st(D, DICK)}/>',
    'rect': f'<rect x="3.5" y="5.5" width="17" height="13" rx="0.8" {st(D, DICK)}/>',
    'triangle': f'<path d="M12 4l8.5 15.5h-17z" {st(D, DICK)}/>',
    'circle': f'<circle cx="12" cy="12" r="8.5" {st(D, DICK)}/>',
    # Suche
    'case_sensitive': (f'<path d="M2.5 17L7 6l4.5 11M4.3 13.2h5.4" {st(B, NORMAL)}/>'
                       f'<circle cx="17" cy="14" r="3" {st(B, NORMAL)}/><path d="M20 11v6" {st(B, NORMAL)}/><path d="M2.5 21h19" {st(B, DUENN)}/>'),
    'find': (f'<circle cx="10" cy="10" r="6.5" fill="{W}" {st()}/><path d="M15 15l6 6" {st(D, 2.75)}/>'),
    # Symboleditor
    'apply': f'<circle cx="12" cy="12" r="9" fill="{GN}"/><path d="M7.5 12.5l3 3 6-7" {st(W, DICK)}/>',
    'cancel': f'<circle cx="12" cy="12" r="9" fill="{R}"/><path d="M8.5 8.5l7 7M15.5 8.5l-7 7" {st(W, DICK)}/>',
    'option_toggle': (f'<rect x="2.5" y="7" width="19" height="10" rx="5" fill="{LB}" {st()}/>'
                      f'<circle cx="15.5" cy="12" r="3.3" fill="{B}"/>'),
    # Ausrichten
    'align_hor': (f'<path d="M2.5 12h19" {st(R, DUENN)} stroke-dasharray="2 2"/>'
                  f'<rect x="4" y="7" width="6" height="10" fill="{LB}" {st()}/><rect x="14" y="9" width="6" height="6" fill="{LB}" {st()}/>'),
    'align_ver': (f'<path d="M12 2.5v19" {st(R, DUENN)} stroke-dasharray="2 2"/>'
                  f'<rect x="7" y="4" width="10" height="6" fill="{LB}" {st()}/><rect x="9" y="14" width="6" height="6" fill="{LB}" {st()}/>'),
    'space_hor': (f'<rect x="2.5" y="8" width="4" height="8" fill="{LB}" {st()}/><rect x="10" y="8" width="4" height="8" fill="{LB}" {st()}/>'
                  f'<rect x="17.5" y="8" width="4" height="8" fill="{LB}" {st()}/><path d="M6.5 19.5h3.5M14 19.5h3.5" {st(R, DUENN)}/>'),
    'space_ver': (f'<rect x="8" y="2.5" width="8" height="4" fill="{LB}" {st()}/><rect x="8" y="10" width="8" height="4" fill="{LB}" {st()}/>'
                  f'<rect x="8" y="17.5" width="8" height="4" fill="{LB}" {st()}/><path d="M19.5 6.5V10M19.5 14v3.5" {st(R, DUENN)}/>'),
})

# Verwendungsorte für die Übersichtsseite (Name → Beschreibung); jedes Icon braucht einen Eintrag.
VERWENDUNG = {
    'obj_projekt': 'Grund-Icon Projekt',
    'obj_blatt': 'Projektbaum: Gruppe „Blätter“ je Anlage',
    'obj_anlage': 'Projektbaum: Anlagen',
    'obj_potential': 'Projektbaum: „Potentiale“',
    'obj_sammelschiene': 'Projektbaum: „Sammelschienen“ je Anlage',
    'obj_leitung': 'Projektbaum: „Leitungen“',
    'obj_klemmleiste': 'Projektbaum: „Klemmleisten“ je Anlage',
    'obj_klemme': 'Linke Leiste: „Neue Klemmstelle“ (Werkzeug)',
    'obj_stift': 'Projektbaum: „Stifte“',
    'obj_symbol': 'Projektbaum: „Objektkatalog“',
    'obj_bauteil': 'Projektbaum: „Bauteile“',
    'obj_platzhalter': 'Projektbaum: „Platzhaltergruppen“',
    'obj_bild': 'Projektbaum: „Grafiken“; linke Leiste: „Neues Bild“ (Werkzeug)',
    'obj_tabelle': 'Linke Leiste: „Neue Tabelle“ (Werkzeug)',
    'obj_text': 'Linke Leiste: „Neuer Text“ (Werkzeug)',
    'obj_textbox': 'Linke Leiste: „Neue Textbox“ (Werkzeug)',
    'obj_seite': 'Grund-Icon Seite eines Symbols',
    'obj_option': 'Grund-Icon Option eines Symbols',
    'obj_buch': 'Bibliotheksbäume (Symbole, Bauteile): Gruppe zugeklappt',
    'obj_buecher': 'Bibliotheksbäume (Symbole, Bauteile): Gruppe aufgeklappt',
    'new_projekt': 'Werkzeugleiste oben: „Neues Projekt“',
    'new_blatt': 'Werkzeugleiste 2: „Neues Blatt“',
    'new_potential': 'Werkzeugleiste 2: „Neues Potential“',
    'new_sammelschiene': 'Werkzeugleiste 2: „Neue Sammelschiene“',
    'new_leitung': 'Werkzeugleiste 2: „Neue Leitung“',
    'new_klemmleiste': 'Werkzeugleiste 2: „Neue Klemmleiste“',
    'new_stift': 'Werkzeugleiste oben und Symboleditor: „Neuer Stift“',
    'new_seite': 'Symboleditor: „Neue Seite“',
    'new_option': 'Symboleditor: „Neue Option“',
    'edit_sammelschiene': 'Dialog Sammelschiene: „Bearbeiten“ (Schiene)',
    'edit_bauteil': 'Dialoge Symbol und Klemme: „Bearbeiten“ (Bauteil)',
    'edit_symbol': 'Symboleditor: „Symboleigenschaften“',
    'del_zeile': 'Tabellendialog: „Zeile löschen“',
    'open': 'Werkzeugleiste oben: „Projekt laden“; Tabellendialog: „Liste laden“',
    'save': 'Werkzeugleiste oben: „Speichern“; Tabellendialog: „Liste speichern“',
    'save_project': 'Werkzeugleiste oben: „Projekt speichern“',
    'print': 'Werkzeugleiste oben: „Drucken“',
    'delete': 'Tabellendialog: „Alles löschen“',
    'cut': 'Werkzeugleiste oben: „Ausschneiden“',
    'copy': 'Werkzeugleiste oben: „Kopieren“',
    'paste': 'Werkzeugleiste oben: „Einfügen“',
    'undo': 'Werkzeugleiste oben: „Rückgängig“',
    'redo': 'Werkzeugleiste oben: „Wiederherstellen“',
    'zoom': 'Werkzeugleiste oben: „Vergrößern/Verkleinern“',
    'grid': 'Werkzeugleiste oben: Rasterfang',
    'path_align': 'Werkzeugleiste oben: Strompfadausrichtung',
    'auto_conn': 'Werkzeugleiste oben: Automatische Verbindungen',
    'size_frame': 'Werkzeugleiste oben: Rahmen zur Größenänderung',
    'fix': 'Werkzeugleiste oben: Fixierung',
    'rotate': 'Werkzeugleiste 2 und Symboleditor: „Drehen“',
    'mirror_v': 'Werkzeugleiste 2 und Symboleditor: „Vertikal Spiegeln“',
    'mirror_h': 'Werkzeugleiste 2 und Symboleditor: „Horizontal Spiegeln“',
    'group': 'Werkzeugleiste 2 und Symboleditor: „Gruppieren“',
    'ungroup': 'Werkzeugleiste 2 und Symboleditor: „Gruppe auflösen“',
    'select': 'Linke Leiste: „Auswahl“',
    'line': 'Linke Leiste: „Neue Linie“ (Werkzeug)',
    'curve': 'Linke Leiste: „Neue Kurve“ (Werkzeug)',
    'rect': 'Linke Leiste: „Neues Rechteck“ (Werkzeug)',
    'triangle': 'Linke Leiste: „Neues Dreieck“ (Werkzeug)',
    'circle': 'Linke Leiste: „Neuer Kreis“ (Werkzeug)',
    'case_sensitive': 'Suchfeld unter dem Projektbaum: Groß-/Kleinschreibung',
    'find': 'Suchfeld unter dem Projektbaum: Weitersuchen',
    'apply': 'Symboleditor: „Übernehmen“',
    'cancel': 'Symboleditor: „Abbruch“',
    'option_toggle': 'Symboleditor: Option ein/aus',
    'align_hor': 'Fenster Ausrichten: „Horizontal“',
    'align_ver': 'Fenster Ausrichten: „Vertikal“',
    'space_hor': 'Fenster Ausrichten: „Abstand Horizontal“',
    'space_ver': 'Fenster Ausrichten: „Abstand Vertikal“',
}


def svg_text(body, size=24):
    # Elemente mit eigener Füllung: das fill="none" aus st() entfernen (doppelte Attribute sind ungültig)
    body = re.sub(r'<[^>]+>', lambda m: m.group(0).replace(' fill="none"', '', 1) if m.group(0).count('fill=') > 1 else m.group(0), body)
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">{body}</svg>\n'


GRUPPEN = [
    ('Grund-Icons je Objektart', lambda n: n.startswith('obj_')),
    ('Neu anlegen (Grund-Icon + grünes Plus)', lambda n: n.startswith('new_')),
    ('Bearbeiten / Entfernen (Grund-Icon + Stift bzw. rotes Minus)', lambda n: n.startswith(('edit_', 'del_'))),
    ('Aktionen und Werkzeuge', lambda n: not n.startswith(('obj_', 'new_', 'edit_', 'del_'))),
]


def uebersicht(pfad):
    zeilen = []
    for titel, f in GRUPPEN:
        zeilen.append(f'<h2>{titel}</h2><table><tr><th>16 px</th><th>20 px</th><th>48 px</th><th>Datei</th><th>Verwendung</th></tr>')
        for n in sorted(k for k in ICONS if f(k)):
            s = svg_text(ICONS[n]).strip()
            img = lambda px: s.replace('width="24" height="24"', f'width="{px}" height="{px}"', 1)
            zeilen.append(f'<tr><td class="bg">{img(16)}</td><td class="bg">{img(20)}</td><td>{img(48)}</td>'
                          f'<td><code>{n}.svg</code></td><td>{VERWENDUNG.get(n, "")}</td></tr>')
        zeilen.append('</table>')
    html = ('<!doctype html><html lang="de"><head><meta charset="utf-8"><title>Icon-Übersicht</title><style>'
            'body{font-family:Segoe UI,sans-serif;background:#fff;color:#222;margin:16px}'
            'table{border-collapse:collapse;margin-bottom:20px}td,th{border:1px solid #ddd;padding:4px 8px;text-align:left}'
            'td.bg{background:#f0f0f0;text-align:center}svg{display:block;margin:auto}</style></head><body>'
            '<h1>Icons der Elektroplanung</h1><p>Erzeugt mit tools/icons_erzeugen.py. 16 px = Projektbaum, '
            '20 px = Werkzeugleisten (Hintergrund wie die Leisten).</p>' + ''.join(zeilen) + '</body></html>\n')
    with open(pfad, 'w', encoding='utf-8', newline='\n') as f:
        f.write(html)


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = os.path.join(root, 'resources', 'icons')
    os.makedirs(out, exist_ok=True)
    fehlt = [n for n in ICONS if n not in VERWENDUNG]
    assert not fehlt, f'Verwendung fehlt: {fehlt}'
    # alte Dateien entfernen, damit keine verwaisten Icons liegen bleiben
    for f in os.listdir(out):
        if f.endswith('.svg') and f[:-4] not in ICONS:
            os.remove(os.path.join(out, f))
    for name, body in ICONS.items():
        with open(os.path.join(out, name + '.svg'), 'w', encoding='utf-8', newline='\n') as f:
            f.write(svg_text(body))
    print(f'{len(ICONS)} Icons nach {out}')
    if '--uebersicht' in sys.argv:
        p = sys.argv[sys.argv.index('--uebersicht') + 1]
        uebersicht(p)
        print(f'Übersicht: {p}')


if __name__ == '__main__':
    main()
