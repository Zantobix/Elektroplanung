# Elektroplanung – Nachbau in Rust

Nachbau der Planungssoftware **Elektroplanung 6.4.2** (Borland C++ Builder, MD Evolution) in Rust mit
egui/eframe, mit eingebauter **MCP-Schnittstelle**. Grundlage: Analyse der Original-`Elektroplanung.exe`
mit REA/Rizin (Formate, Formularressource, Menüs), ein eigenes Beispielprojekt (`W2.epr`, nicht im Repository)
und die Original-Icons aus [BYTE-RANGER-Software/Elektroplanung](https://github.com/BYTE-RANGER-Software/Elektroplanung).

> **Unabhängiger Nachbau.** Dieses Projekt ist nicht mit MD Evolution verbunden und enthält keinen Code des
> Originals. Es liest und schreibt dessen Dateiformat (`.epr`) und die Symbol-Datenbank (`Symbole.tfi`);
> beides muss der Nutzer selbst mitbringen.

> **Fork** von [BYTE-RANGER-Software/Elektroplanung](https://github.com/BYTE-RANGER-Software/Elektroplanung).
> Die Weiterentwicklung hier ist ein Nachbau in Rust; der frühere PureBasic-Prototyp ist in der Git-Historie
> enthalten.

## Lizenz und Herkunft

- Quelltext: **GPL-3.0** (siehe [LICENSE](LICENSE)).
- Icons in `resources/` (`Icons.*`, `ImageList*.*`, `Button5/10.Glyph.Data.png`, `1.ico`): unverändert aus
  [BYTE-RANGER-Software/Elektroplanung](https://github.com/BYTE-RANGER-Software/Elektroplanung),
  Copyright (c) 2025 Daniel Metz / md-evolution, GPL-3.0.
- Weitere Knopf-Glyphen des Originals sind nicht enthalten; liegen sie lokal vor (Ordner über die
  Umgebungsvariable `EP_GLYPHEN`), bettet `build.rs` sie ein, sonst zeigen diese Knöpfe Text.
- `resources/template.epr`: leeres Projekt im Originalformat als Vorlage für neue Projekte (ohne persönliche Daten).

## Bauen und starten

```bash
cargo build --release
```

```bash
target/release/Elektroplanung.exe "Pfad/zum/Projekt.epr"
```

Die Original-Symbolbibliothek (`Datenbank\Symbole.tfi`, `Gruppen.tfi`) wird automatisch neben der
Original-Exe, im aktuellen Ordner und im Einstellungsordner (`%APPDATA%\Elektroplanung\Datenbank`,
unter Linux `~/.config/elektroplanung/Datenbank`) gesucht; anderer Ort: Umgebungsvariable `EP_DATENBANK` oder
Optionen › Datenbank. Lokale Einstellungen für Tests und `cargo run` gehören in `.cargo/config.toml`
(nicht im Repository), z. B.:

```toml
[env]
EP_SAMPLE = 'C:\Pfad\zu\Beispiel.epr'
EP_DATENBANK = 'C:\Pfad\zur\Datenbank'
```

Für die MCP-Schnittstelle: Vorlage `.mcp.json.example` nach `.mcp.json` kopieren und anpassen.

### Linux

```bash
cargo build --release            # nativ unter Linux
cargo check --target x86_64-unknown-linux-gnu   # Prüfung von Windows aus
```

Schriften kommen unter Linux aus `/usr/share/fonts` u. a. (Liberation Serif als metrisch gleicher Ersatz für
Times New Roman, Liberation Sans/DejaVu für die Oberfläche; fehlen sie, die eingebaute egui-Schrift).
Drucken geht über `lp` (Druckerliste `lpstat -e`), ersatzweise `xdg-open`. Einstellungen und Benutzerbibliothek
liegen in `~/.config/elektroplanung` (Windows: `%APPDATA%\Elektroplanung`; für Tests `EP_APPDIR`).

## Originalformat (.epr)

- **Öffnen und Speichern im Originalformat.** Unveränderte Teile werden Byte für Byte übernommen,
  geänderte Objekte im Originalblock nachgeführt, neue aus Vorlageblöcken erzeugt. Vor dem ersten
  Überschreiben einer `.epr` entsteht eine Sicherung `*.epr.bak`.
- Alle Blockarten von W2.epr werden dargestellt: Zeichenelemente, Texte, Tabellen (ZCN 13), Symbole mit
  BMK-Vorlagen (`-X1.%C+%P` → `-X1.1+W2`), Parametern (`%<Typ>` → `B10`) und Instanztexten, Verbindungen mit
  Leitungsart und Aderbezeichnungen (BZD), Sammelschienen mit Querverweisen (`4/10►`), Klemmstellen
  (`-X1.8B`), Verweise, Projektbilder (BLD/JPG). Sicherungsübersicht und Klemmplan berechnet der Nachbau
  wie das Original beim Anzeigen. Einzige nicht dargestellte Blöcke in W2.epr: zwei Verbindungen mit nur
  einem Punkt und eine Sammelschiene ohne Länge (`cargo run --example unmapped`).
- Im Symboleditor geänderte Projektsymbole werden als neuer SYM-Block geschrieben, alle anderen bleiben byteidentisch.
- Format-Details: [MECHANICS.md](MECHANICS.md).

## MCP-Schnittstelle

`Elektroplanung.exe --mcp` startet statt des Fensters einen MCP-Server (JSON-RPC 2.0 über stdio,
Protokoll 2025-06-18). Für Claude Code ist er in [.mcp.json](.mcp.json) eingetragen.

| Werkzeug | Zweck |
|---|---|
| `open_project`, `new_project`, `save_project` | Projekt öffnen (.epr/.epj), neu, speichern (`format: epr` oder `epj`) |
| `project_info`, `set_project_info` | Projektdaten lesen/ändern |
| `list_sheets`, `get_sheet`, `add_anlage`, `add_sheet`, `set_sheet` | Anlagen und Blätter |
| `add_object`, `update_object`, `delete_object` | Linie, Rechteck, Kreis, Kurve, Text, Verbindung; verschieben, Texte/BMK ändern |
| `list_symbols`, `place_symbol` | Symbole aus Projekt und Bibliothek suchen und platzieren (inkl. Zusatzoptionen) |
| `export_pdf` | alle Blätter als Vektor-PDF (gleiche Zeichenroutinen wie die Oberfläche) |
| `symbol_open`, `symbol_new`, `symbol_get` | Symboleditor: Symbol aus Projekt/Bibliothek öffnen oder neu anlegen, Elemente auflisten |
| `symbol_add_element`, `symbol_update_element`, `symbol_delete_element` | Linie, Polygon, Kurve, Ellipse, Text (mit Ausrichtung/Drehmodus), Anschluss, Titel-/Wertanker; Seite, Zusatzoption, Füllung |
| `symbol_pages`, `symbol_options`, `symbol_alt_texts`, `symbol_set_properties` | Seiten (Bildvarianten), Zusatzoptionen, Alternativtexte, Name/BMK/Gruppe/Freigaben |
| `symbol_commit`, `symbol_close` | Übernehmen ins Projekt (wird im .epr geschrieben) bzw. in die Benutzerbibliothek; Abbruch |
| `verify_epr_roundtrip` | .epr laden, neu schreiben, Byte-Vergleich |

## Versionen und fertige Programme

- Jeder Push auf `main` und jeder Pull Request wird auf Linux, macOS und Windows gebaut und getestet
  ([.github/workflows/build.yml](.github/workflows/build.yml)); die Programme hängen als Artefakte an der Ausführung.
- Die Versionsnummer steht **nur** in `Cargo.toml` (`version = "…"`); Info-Dialog, Paketnamen, Tag und Release
  werden daraus abgeleitet. Für eine neue Version: Nummer in `Cargo.toml` erhöhen, Abschnitt
  `## [<Version>]` in [CHANGELOG.md](CHANGELOG.md) schreiben, auf `main` bringen. Die Pipeline legt dann nach
  erfolgreichem Bauen selbst das Tag `v<Version>` und ein Release mit den Programmen für alle drei Systeme an.
  Tags werden nicht von Hand gesetzt.
- Neue Funktionen entstehen auf eigenen Zweigen und kommen per Pull Request nach `main`.

## Nachweise

Die automatischen Tests stehen in `cargo test --release` ([tests/epr_w2.rs](tests/epr_w2.rs)) und
benutzen ein Beispielprojekt des Originals, das über die Umgebungsvariable `EP_SAMPLE` angegeben wird
(ohne sie werden diese Prüfungen übersprungen).

| Prüfung | Ergebnis |
|---|---|
| Blockebene: alle 6557 Blöcke, Prüfsummen, keine Restbytes | W2.epr lesen und schreiben ist byteidentisch (815 515 Byte) |
| Projektebene: Import in das Datenmodell und Rückschreiben | byteidentisch |
| Änderungen (Symbol verschieben, BMK, Blattname, neue Linie, neuer Text) | werden geschrieben und korrekt wieder gelesen |
| Neues Projekt als .epr | gültig, Querformat, Rahmen wie im Original |
| Rahmenlage aus den Blattdaten | Altbau EG: Leiste bei 10 mm wie im Original-Ausdruck |
| MCP über stdio | initialize, tools/list, Öffnen, Ändern, Speichern, PDF (39 Seiten) |
| **Gegenprobe mit dem Original** | Die vom Nachbau geänderte Datei öffnet das Original fehlerfrei („Version:EP16“) und zeigt die Änderungen |
| Sichtvergleich | PDF Seite 1 und 3 entsprechen dem Original-Ausdruck „Elektroplanung Wohnung 2.pdf“ (Symbole, BMK, Zusatzoptionen „²“/„SPD“, Alternativtexte, Verweise, Fettschrift, Schriftfeld) |
| Änderungen **im Original** gespeichert (Rahmen, Fixiert, Ausrichtung, Freigaben, Spiegeln) | Der Nachbau schreibt bei derselben Änderung dieselben Feldwerte (`tests/original_edits.rs`) |
| Textstil und Bauteiloptionen vom Nachbau gesetzt | Das Original zeigt „Raum 1“ fett, kursiv, rot und an -X2.4 „SPD“ und „⁴“ |
| Neues Bibliothekssymbol vom Nachbau eingefügt | Das Original zeigt es |
| Eigenschaftendialoge Text/Symbol/Blatt | Felder, Reihenfolge und Sperren wie im Original |
| Kontextmenüs Symbol/Text/Fläche inkl. Optionen und Alternativtext | Einträge und Reihenfolge wie im Original |
| Import-Vollständigkeit (`tests/import_vollstaendig.rs`) | Sammelschienen und Querverweise wie Ausdruck Seite 6/18, Klemmstellen, Aderfarben, Tabellen, Bilder, BMK-Vorlagen, Sicherungsübersicht Seite 5, Klemmplan Seite 12 |
| Symboltexte in gedrehten Symbolen (`tests/motor_lage.rs`) | "M" im gedrehten Motorsymbol (Seite 1) dreht mit, im Symbol "Motor" (Seite 7) bleibt es waagerecht; hochkant BMK "-M2.1" an derselben Stelle wie im Ausdruck |
| Maustests in einer Projektkopie, Gegenprobe im Original | Kontextmenüs (Optionen, Bildindex, Symbol bearbeiten), Dialoge verschieben, Zoom/Bildlauf, Zeichnen, Verschieben, Rückgängig, Symbol setzen, Symboleditor mit Titel-/Wertfeld, Speichern → Original zeigt die Änderungen; Rückfrage „Geänderte Daten Speichern?“ |
| Symboleditor über MCP (`tests/mcp_symbol.rs`) | neues Symbol mit Seiten, Zusatzoption, Alternativtext → .epr → wieder gelesen; geändertes W2-Symbol wird geschrieben, alle anderen bleiben gleich |

Analyse-Grundlagen: die 47 Formulare des Originals (Beschriftungen und Anordnung) und das Verhalten des
Original-Programms; Formatdetails in [MECHANICS.md](MECHANICS.md). Analysematerial des Originals ist nicht Teil
dieses Repositorys.

### Übereinstimmung messen

Eine Binär-Übereinstimmung zwischen Original (Borland C++, 32 Bit) und Nachbau (Rust, 64 Bit) gibt es grundsätzlich
nicht. Gemessen werden stattdessen:

- **Dateiformat:** W2.epr wird zu 100 % byteidentisch gelesen und geschrieben; vom Original gespeicherte Änderungen
  stimmen Feld für Feld mit denen des Nachbaus überein (Tests).
- **Oberfläche:** `python tools/abdeckung.py [--fehlend]` prüft jede sichtbare Beschriftung aller 47 Formulare
  (Captions, Hinweise, Listeneinträge) gegen den Nachbau. Aktueller Stand: **589 von 604 = 97,5 %**,
  je Formular in [docs/abdeckung.txt](docs/abdeckung.txt). Es fehlen bewusst: die Info-Box des Autors (6),
  interne Komponentennamen ohne Anzeige (ToolBar1–3, OB_Fang, SizeRahmen), An-/Abmelden am Datenbankserver und
  der S7-Import (kein Gegenstück im Nachbau).

## Bekannte Grenzen

- Textboxen, Tabellen, Bilder und Klemmstellen, die im Nachbau **neu** angelegt werden, kann das `.epr`-Format
  des Nachbaus noch nicht schreiben (vorhandene bleiben erhalten); der Nachbau meldet sie beim Speichern.
- Eigene Schriftarten pro Text: Schnitt, Grad, Farbe und Unterstreichung werden geschrieben, der Schriftname
  (Original: Zeiger in Feld 9) noch nicht; gezeichnet wird in Times wie die Projektschrift.
- Tools › Ausrichtung: Knöpfe und Referenzpunkt wie im Original; die genaue Auswahl im Feld „Referenzpunkt“
  füllt das Original zur Laufzeit – der Nachbau listet die markierten Objekte.
- Symboleditor: Parameter (PRA) werden nicht bearbeitet; Freigaben eines Symbols gelten als Vorgabe für neu
  gesetzte Symbole, ihr Ablageort im SYM-Block ist noch unbekannt.
- Drucken: „Änderung seit …“ druckt alle Blätter der Anlage (der Nachbau führt kein Änderungsdatum je Blatt).
- Optionen › Datenbank: Serverangaben werden nur gespeichert; der Nachbau liest die Bibliothek direkt aus Symbole.tfi.
- Info-Dialog bewusst nicht 1:1 (Copyright, E-Mail und Registrierungsschlüssel des Original-Autors).
- Bewusste Abweichungen in der Bedienung: Einfügen (Strg+V) versetzt die Kopie um ein Rasterfeld und vergibt eine
  neue BMK (das Original legt sie verdeckt auf das Original mit gleicher BMK); das Mausrad zoomt feiner
  (≈ 5 % je Rastung statt ≈ 1,8-fach).
- Klemmplan: Kopf, Klemmennummern und Brückenspalte werden berechnet, die Spalten "Von Ziel"/"Nach Ziel"
  (Verfolgung der Verbindungen) noch nicht.
- Neue Einstellungen der Dialoge (Tabellenkopf fett, Bildausrichtung, Verbindungsoptionen, Sammelschienen-
  Einstellungen, Begrenzungslinien, Schriftfeldhöhe) speichert der Nachbau im eigenen Format (.epj); ihre
  Ablage im .epr ist noch unbekannt.
- Querverweise auf andere Anlagen: Pfeilrichtung aus zwei Beispielen abgeleitet (siehe MECHANICS.md).
- Hilfsprogramme zur Formatanalyse: `cargo run --example dump|vsy|sym|obj -- <datei.epr> …`.
