# Änderungen

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/), Versionen nach
[Semantic Versioning](https://semver.org/lang/de/): `MAJOR.MINOR.PATCH`; vor 1.0 können sich Bedienung und
eigenes Dateiformat (`.epj`) noch ändern.

Die Versionsnummer steht nur in `Cargo.toml`. Erreicht eine neue Nummer `main`, erzeugt die Pipeline das Tag
`v<Version>` und ein Release mit dem passenden Abschnitt dieser Datei als Text.

## [Unveröffentlicht]

- Releases als einzelne Dateien statt Archiven: Windows-Exe (mit Programmsymbol, ohne Visual-C++-Laufzeit),
  Linux-AppImage, macOS-Disk-Image mit App

## [0.1.0]

Erste Veröffentlichung des Nachbaus:

- Öffnen, Bearbeiten, Speichern und Drucken von Projekten im Originalformat (`.epr`), byteidentische Rundreise
- Symbolbibliothek aus der Original-Datenbank (`Symbole.tfi`), Symboleditor mit Seiten und Zusatzoptionen
- Dialoge und Kontextmenüs nach dem Original, einstellbare Tastenkürzel, Blätter per Ziehen umsortieren
- MCP-Schnittstelle (`--mcp`) zum Bearbeiten von Projekten durch KI-Assistenten
- Windows, Linux, macOS
