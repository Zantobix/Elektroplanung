# Änderungen

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/), Versionen nach
[Semantic Versioning](https://semver.org/lang/de/): `MAJOR.MINOR.PATCH`; vor 1.0 können sich Bedienung und
eigenes Dateiformat (`.epj`) noch ändern.

Die Versionsnummer steht nur in `Cargo.toml`. Erreicht eine neue Nummer `main`, erzeugt die Pipeline das Tag
`v<Version>` und ein Release mit dem passenden Abschnitt dieser Datei als Text.

## [Unveröffentlicht]

## [0.1.2]

Neu:
- Eigene Vektor-Icons für alle Werkzeugleisten (scharf in jeder Größe); keine Grafiken des Originals mehr
- Klemmplan: Spalten „Von Ziel“/„Nach Ziel“ (Verbindungen werden verfolgt) und Brücken; alter Klemmenplan
  (Projektoption) mit 8 Spalten
- Leitungen ziehen beim Verschieben von Symbolen und Klemmstellen mit
- Fangpunkte: Anschlüsse in Mausnähe werden beim Zeichnen und Setzen markiert
- Symbole lassen sich aus der Bibliothek direkt aufs Blatt ziehen
- Eigenschaften für mehrere markierte Objekte (Stift, Fixierung, Textgröße/-stil)
- Suchen über alle Blätter (Bearbeiten › Suchen…, Strg+F) mit Sprung zum Objekt
- Tools › Projekt prüfen: offene Anschlüsse und doppelte BMK; Rückfrage vor dem Drucken (abschaltbar)
- Symbolvorschau als Tooltip im Symbolbaum
- Datei › Als Bild exportieren…: PNG mit 150/300/600 dpi, aktuelles Blatt oder alle Blätter der Anlage
- Sammelschienenassistent wie im Original (L1, L2, L3, N, PE, vorhandene Schienen werden wiederverwendet)
- Bauteil zuordnen: neues Bauteil direkt aus dem Dialog anlegen
- Variantenrad beim Setzen von Symbolen mit mehreren Bildern (wie im Original; abschaltbar in den Optionen)
- Querverweise: Doppelklick springt zum Ziel (auch Querverweistexte von Sammelschienen), Verweise lassen sich
  drehen, das Ziel wird gespeichert; im PDF sind Querverweise anklickbar
- Option „Rasterdarstellung“: Stärke des Rasters (Hell, Mittel, Kräftig oder eigene Farbe)

Behoben / geändert wie im Original:
- Strompfadausrichtung richtet Symbole beim Setzen und Verschieben auf die Mitte des Strompfads aus
- Automatische Verbindungen ziehen Leitungen von freien Anschlüssen; Symbole auf einer Leitung trennen sie auf
- Gedrückt halten von S/A kehrt Strompfadausrichtung/automatische Verbindungen vorübergehend um, Umschalt
  schaltet den Rasterfang ab
- Rahmen zur Größenänderung um Mehrfachmarkierungen mit Zeichenelementen (nicht um einzelne Symbole/Texte)
- Fixierte Objekte lassen sich bei aktiver Fixierung nicht markieren
- Eigentümer › Setzen: Eigentümer anklicken, auch auf einem anderen Blatt
- Rastergitter besser sichtbar

## [0.1.1]

- Releases als einzelne Dateien statt Archiven: Windows-Exe (mit Programmsymbol, ohne Visual-C++-Laufzeit),
  Linux-AppImage, macOS-Disk-Image mit App
- Behoben: Der Link im Info-Dialog öffnet jetzt den Browser

## [0.1.0]

Erste Veröffentlichung des Nachbaus:

- Öffnen, Bearbeiten, Speichern und Drucken von Projekten im Originalformat (`.epr`), byteidentische Rundreise
- Symbolbibliothek aus der Original-Datenbank (`Symbole.tfi`), Symboleditor mit Seiten und Zusatzoptionen
- Dialoge und Kontextmenüs nach dem Original, einstellbare Tastenkürzel, Blätter per Ziehen umsortieren
- MCP-Schnittstelle (`--mcp`) zum Bearbeiten von Projekten durch KI-Assistenten
- Windows, Linux, macOS
