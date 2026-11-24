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
- Icons (`resources/icons/*.svg`) und Programmsymbol sind eigene Vektorgrafiken dieses Projekts, erzeugt mit
  `tools/icons_erzeugen.py` bzw. `tools/symbol_erzeugen.py`; es werden keine Grafiken des Originals verwendet.
- `resources/template.epr`: leeres Projekt im Originalformat als Vorlage für neue Projekte (ohne persönliche Daten).

## Herunterladen

Fertige Programme gibt es unter [Releases](https://github.com/Zantobix/Elektroplanung/releases), jeweils als
einzelne Datei ohne Installation. Neben den Versionen gibt es das Vorab-Release
[Entwicklungsstand](https://github.com/Zantobix/Elektroplanung/releases/tag/entwicklung) mit dem jeweils neuesten
Stand von `main` (wird bei jeder Änderung ersetzt).

| System | Datei | Hinweis |
|---|---|---|
| Windows | `Elektroplanung-<Version>-windows-x86_64.exe` | direkt starten. Die Exe ist nicht signiert: Windows SmartScreen warnt beim ersten Start → „Weitere Informationen“ → „Trotzdem ausführen“. |
| Linux | `Elektroplanung-<Version>-x86_64.AppImage` | ausführbar machen (`chmod +x …`) und starten |
| macOS (Apple Silicon) | `Elektroplanung-<Version>-macos-arm64.dmg` | App in „Programme“ ziehen. Nicht notarisiert: beim ersten Start Rechtsklick → „Öffnen“ (bzw. Systemeinstellungen › Datenschutz & Sicherheit › „Trotzdem öffnen“). |

Einstellungen und Benutzerbibliothek liegen in `%APPDATA%\Elektroplanung` bzw. `~/.config/elektroplanung`.

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
  Tags werden nicht von Hand gesetzt. Ohne neue Nummer ersetzt jeder Push auf `main` nur das Vorab-Release
  `entwicklung`; veröffentlichte Versionen bleiben unverändert.
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
| Import-Vollständigkeit (`tests/import_vollstaendig.rs`) | Sammelschienen und Querverweise wie Ausdruck Seite 6/18, Klemmstellen, Aderfarben, Tabellen, Bilder, BMK-Vorlagen, Sicherungsübersicht Seite 5, Klemmplan Seite 12 mit allen Zielen und Brücken |
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
  (Captions, Hinweise, Listeneinträge) gegen den Nachbau. Aktueller Stand: **587 von 587 = 100 %**,
  je Formular in [docs/abdeckung.txt](docs/abdeckung.txt). Nicht gezählt werden Komponentennamen von
  Werkzeugleisten und Bildknöpfen ohne Anzeige (ToolBar1–3, OB_Fang, SizeRahmen …) und die 10 bewusst nicht
  nachgebauten Beschriftungen (Info-Box des Autors, An-/Abmelden am Datenbankserver, S7-Import); das Werkzeug
  listet sie getrennt auf. Nach der früheren Zählung (alle 604) sind es 589 = 97,5 %.

## Bekannte Grenzen

- Neue Klemmstellen und Klemmleisten werden geschrieben (OBJ, KLM mit KFO-Einträgen, `src/klemmen.rs`). Neue Textboxen, Tabellen, Bilder (samt
  Projektbild), Verweise und Sammelschienen werden geschrieben (`src/epr_neu.rs`); ihr Aufbau folgt der
  Speicherroutine des Originals, im Original geöffnet ist das Ergebnis noch nicht geprüft
  (`cargo run --example neu_objekte` erzeugt eine Testdatei).
- Schriftname je Text (STR nach dem STL, sonst die Standardschrift des Projekts): Zeichenfläche und Bildexport
  zeichnen mit der installierten Schrift (gesucht über die Namenstabelle der Schriftdateien in den Schriftordnern
  des Systems – Windows: Fonts und Benutzerschriften; Linux: XDG-Ordner wie /usr/share/fonts und
  ~/.local/share/fonts, ohne fontconfig, mit metrisch gleichen Ersatzschriften wie Liberation; macOS:
  /System/Library/Fonts, /Library/Fonts, ~/Library/Fonts). Fehlt die Schrift, wird Times gezeichnet. Umriss,
  Treffertest, Markierung und Griffe rechnen mit der Metrik dieser Schrift (Vorschübe, Zeilenhöhe), sonst mit
  Times. Das PDF bettet die Schrift als Teilmenge ein (TrueType, Type0/Identity-H mit ToUnicode, Glyphenabstände
  wie auf dem Bildschirm); Times bleibt Standardschrift. Nicht einbettbare Schriften (nicht installiert,
  CFF-Umrisse, Lizenzbits) und der Schrift fehlende Zeichen kommen wie bisher als Times, Helvetica (Arial,
  Segoe UI, Tahoma …) oder Courier (Courier New, Consolas …).
- Symboleditor: Stifte des Symbols (SIT, Nummer ab 4) lassen sich anlegen, ändern, löschen und Elementen
  zuweisen und werden ins `.epr` geschrieben. Die Freigaben eines Symbols (Querverweise, Sicherungsübersicht,
  SYM Byte 112, Bit 0/1, an W2.epr ermittelt) werden gelesen und geschrieben und gelten als Vorgabe für neu
  gesetzte Symbole. In der Symbolbibliothek (Symbole.tfi) stehen sie als u32 ab Byte 45 des Symbolkopfs und
  werden beim Laden übernommen (leerer Kopf oder Textreste: Querverweise an, Sicherungsübersicht aus).
- Schriftfeld bearbeiten: Linien, Texte (bis 8 Zeilen) und Logo werden grafisch bearbeitet und ins `.epr`
  geschrieben (FUS › GRS/STR); unbekannte Elementarten des Originals bleiben angehängt erhalten.
- Tabelle › Daten „Inhaltsverzeichnis“/„Querverweise“: der Inhalt wird bei jedem Zeichnen aus dem Projekt
  berechnet (Zeilenzahl bleibt die der Tabelle); Spaltenbreiten geschätzt.
- Optionen › Datenbank: Serverangaben werden nur gespeichert; der Nachbau liest die Bibliothek direkt aus Symbole.tfi.
- Info-Dialog bewusst nicht 1:1 (Copyright, E-Mail und Registrierungsschlüssel des Original-Autors).
- Bewusste Abweichungen in der Bedienung: Einfügen (Strg+V) versetzt die Kopie um Rasterfelder (mindestens 10 mm)
  und vergibt eine neue BMK (das Original legt sie verdeckt auf das Original mit gleicher BMK); das Mausrad zoomt feiner
  (≈ 5 % je Rastung statt ≈ 1,8-fach).
- Bewusste Erweiterungen gegenüber dem Original (bleiben, auch wenn das Original sie nicht hat):
  - Textgenerator: Zähler wie im Original (`@` + ein Zeichen, Start 0 oder Wert in Klammern), dazu die Werte
    `@{Blatt}`, `@{Pfad}`, `@{Anlage}` und `@{Text}` (bisheriger Text).
  - Rasterweite je Projekt: Die gewählte Weite wird je Datei gemerkt; ist keine gemerkt, stellt das Öffnen die
    Weite ein, in der das Projekt gezeichnet ist (`src/raster.rs`, `raster_erkennen`).
  - Leitungsnetz: Jede Bearbeitung (Verschieben, Teilstück/Knick ziehen, Skalieren, Drehen, Pfeiltasten) führt
    alle angeschlossenen Leitungen nach demselben Verfahren nach, auch Abzweige auf einer gezogenen Leitung
    (`src/netz_folgen.rs`); eine markierte und als Ganzes verschobene Leitung bleibt an nicht verschobenen Schienen,
    Klemmstellen, Anschlüssen und Leitungen hängen; ein freies Leitungsende wandert mit seinem Teilstück.
  - Leitungsführung: Beim Ziehen folgt die Leitung der Maus als einfacher Knick, ein fester Knick entsteht erst bei
    einer Kehre der Maus (`leitung_bahn.rs`); die Vorschau hält ihre Form, solange sie nicht deutlich schlechter ist
    (kein Umspringen); beim Mitziehen ersetzt ein einfacher Weg mit weniger Knicken die formerhaltende Führung
    (`leitung_kurz.rs`); übereinanderliegende Leitungen werden vereinigt (`leitung_vereinigen.rs`), aneinanderstoßende
    ohne Abzweig zu einer Leitung zusammengefasst (`leitung_zusammen.rs`); eine Leitung kann überall an einer anderen
    enden – auf dem Raster, senkrecht darauf –, ein freier Überstand wird gekürzt (`leitung_anschluss.rs`).
  - Nach dem Verschieben verbindet der Nachbau keine Anschlüsse automatisch (Nutzerwunsch); ein Abzweig-Ende
    gleitet beim Nachführen auf seinem Wirt, wenn das Knicke spart.
  - Fang: Leitungsenden und Klemmstellen fangen je Achse auf die Fluchten von Anschlüssen und Leitungsenden, ein
    kleines Kreuz zeigt die tatsächliche Lage (`src/fang.rs`); verschobene Objekte und gezogene Teilstücke rücken auf
    das Raster bzw. auf die Flucht anderer Leitungen.
  - Schriftfeld „Geändert“: ein in dieser Sitzung geändertes Blatt zeigt gleich das heutige Datum (das Original
    zeigt bis zum Speichern „Heute“); Alternativtext-Sätze erscheinen lesbar („L | N | PE“ statt `'L';'N';'PE'`).
  - Ziehen am Rand der Zeichenfläche schiebt die Ansicht mit (nach kurzer Verweilzeit, `src/rand_schieben.rs`); ein
    Leitungsende bleibt im Zeichnungsrahmen. Eine Leitung, die auf einer Schiene oder Leitung endet und sonst durch ein
    Symbol liefe, rutscht mit ihrem Ende auf der Schiene in die günstigste freie Lage.
  - Absturzschutz: bei ungespeicherten Änderungen regelmäßig eine Sicherung im Einstellungsordner
    (`wiederherstellung`), Wiederherstellen beim nächsten Start (`src/wiederherstellung.rs`).
  - Statuszeile ohne Objekt unter der Maus: bei einer Markierung deren Umfang („3 Objekte markiert“) statt „Kein
    Objekt“; Speichern eines neuen Projekts schlägt den Projektnamen im Ordner des zuletzt geöffneten Projekts vor.
  - Markierrichtung wie CAD (Option, Programmverhalten): Rahmen von links nach rechts markiert nur ganz
    eingeschlossene Objekte (Rahmen durchgezogen), von rechts nach links alles Berührte (gestrichelt); ohne Option wie
    im Original (Bezugspunkt im Rahmen).
  - Fehlerbericht (F12 bzw. Hilfe › Fehlerbericht speichern): Projektkopie, Protokoll der letzten Eingaben und Bild
    des Fensters in einen Ordner unter `fehlerberichte` in den Einstellungen (`src/fehlerbericht.rs`).
  - DXF-Import mit eigenem Fenster: Ebenen an/aus (Farbe, Anzahl), Maßstab nach der Einheit der Datei oder fest,
    Einpassen und Gruppieren wählbar, Ebenenfarben als Stifte, Vorschau; zusätzlich Blöcke (INSERT), MTEXT und
    ELLIPSE (`src/dxf.rs`, `src/dxf_dialog.rs`).
  - Meldung „Fehler in den Daten“: Blöcke ohne Darstellung lassen sich dort entfernen.
  - Neue Projekte bekommen das Schriftfeld der Vorlage (Aufbau eines echten Projekts: Erstellt/Geändert/Datum, Von,
    Anlage/Ort, Projekt und Blattname, ZNr., Blatt x / y, Status) statt des eingebauten Standards; ein Logofeld
    bleibt ohne Logobild leer wie im Original-Ausdruck.
  - Suche nach neuer Version: Hilfe › Nach neuer Version suchen und (abschaltbar unter Optionen ›
    Programmverhalten) einmal beim Start; fragt mit dem `curl` des Systems das neueste Release auf GitHub ab und zeigt
    bei einer neueren Version einen Hinweis mit Link, lädt aber nichts herunter (`src/update_pruefung.rs`).
  - Hilfslinien beim Verschieben (nach QElectroTech): graue Linie quer durch jeden Anschluss des gezogenen Symbols,
    an freien Anschlüssen die Flucht in Anschlussrichtung (blau), grün bis zu einem freien Gegenanschluss in Flucht
    (`src/hilfslinien.rs`, abschaltbar unter Optionen › Programmverhalten).
  - Leitungsform-Gedächtnis (nach QElectroTech `ConductorProfilesGroup`): Form je Quadrant und genaue Lage werden zu
    Beginn jeder Geste gemerkt; Symbol hin und zurück ergibt denselben Plan (`src/leitung_gedaechtnis.rs`,
    `netz_folgen.rs` Rückkehr). Zielanschluss beim Zeichnen farbig (grün frei, orange belegt, rot eigener Anfang).
  - Verschobenes Symbol in Leitung einbinden (nach QElectroTech `autoBreakConductors`; das Original bindet nur beim
    Neusetzen ein): ein freies Symbol, auf eine Leitung abgelegt, teilt sie (abschaltbar, `src/symbol_in_leitung.rs`).
  - Netz-Wächter: Verschieben, Teilstück ziehen, Drehen und Pfeiltasten verbinden nie ungewollt fremde Netze –
    was nur mit Kurzschluss, Trennung oder zwei Potentialen übereinander ginge, wird nicht ausgeführt (Meldung in der
    Statuszeile); ein freies Leitungsende darf gezielt verbinden (`src/netz_waechter.rs`, abschaltbar).
  - Brückenbögen an Kreuzungen: Kreuzen sich zwei Leitungen ohne Verbindung, springt die waagerechtere in einem
    Halbkreis (Ø 2 mm) über die andere – auf dem Bildschirm, im Druck, PDF und Bildexport; die Datei bleibt
    unverändert (`src/bruecken.rs`, abschaltbar unter Optionen › Programmverhalten).
  - BMK mit leerem Ortskennzeichen ohne hängendes „+“ („-E1“ statt „-E1+“); gespeichert wird wie im Original
    „-E1+%P“.
  - Leitung verschieben: Das Ziehen an einem Teilstück ändert die Führung, die Enden bleiben angeschlossen (das
    Original verhindert das Verschieben angeschlossener Leitungen).
  - Symbol verschieben: Angeschlossene Leitungen folgen formerhaltend (Profil nach QElectroTech), auch solche mit
    freiem anderen Ende (das Original zieht diese nicht mit).
  - Leitung ziehen: Der Knick folgt der Maus ohne Springen (erstes Teilstück behält seine Richtung, bis es Länge 0
    erreicht, nach KiCad); **Leertaste** oder **„/“** kippt die Knickrichtung während des Ziehens
    (`src/leitung_knick.rs`). Im Klickbetrieb gilt die Regel des Originals.
  - Sammelschiene: Beim Verschieben wandert der Abgriff einer Leitung auf der Schiene mit, so dass keine Leitung
    auf der Schiene entlangläuft; markierte Leitungen reißen beim Verschieben nicht von der Schiene ab
    (`src/schiene_halten.rs`).
  - Bitmapexport: vor dem Speichern ein kleiner Dialog mit Auflösung (150/300/480/600 dpi, Vorgabe 480 wie das
    Original bei 96 dpi), allen Blättern der Anlage und Kantenglättung; neben BMP und JPG des Originals auch PNG
    (`src/bildexport.rs`).
  - Vormarkierung (gestricheltes Rechteck beim Überfahren) nur bei Objekten, nicht bei Leitungen.
  - Kontextmenü einer Leitung › „Netz markieren“: markiert alle elektrisch verbundenen Leitungen des Blattes
    (Netz-Index zur Laufzeit nach QElectroTech, `src/netz.rs`; nicht in der Datei).
  - Hilfe › „Fehler melden…“ / „Wunsch äußern…“: kleines Fenster, öffnet ein ausgefülltes GitHub-Issue im Browser
    (Systemangaben sichtbar und abwählbar, nie Projektinhalt) oder kopiert den Text (`src/fehlermeldung.rs`).
- Klemmplan: "Von Ziel"/"Nach Ziel" nach den vier Anschlussrichtungen wie im Original, mit "Ziel Wechsel",
  Eigentümer-Kennung und Funktion (Regeln in `src/klemmplan.rs`); Brücken als Ketten in Spuren wie in den
  Original-Ausdrucken (Seiten 12, 13, 20–22, 39; je Anschlussrichtung höchstens eine Brücke), die
  Ordnungsroutine des Originals ist nur teilweise entschlüsselt (Seite 39 zeigt eine Brücke 10B–6B nicht).
  Tabelle wie im Ausdruck: Zeilen 5 mm (auf die Höhe verteilt), Schrift 3,43 mm, Kennung-Spalten so breit wie der
  längste Text, Klemmenkreise und Brückenspuren im Abstand 2,45 mm.
  "Alter Klemmenplan" zeigt die
  8 Spalten des Originals (Kennung, Anschluß, Pfad, Klemme), ohne Brücken. Folgeblätter: direkt folgende
  Klemmplan-Blätter derselben Leiste (bzw. Sicherungsübersichten, Listenblätter) setzen die Liste fort.
- Inhaltsverzeichnis, Querverweise (1–3 Tabellen) und Artikelliste (Summenliste, Summe+BMK, Einzelliste) als
  Blattart: Spaltenköpfe, Spaltenbreiten, Ränder und Zeilenhöhe wie in Bildschirmaufnahmen des Originals; der
  Rahmen beginnt wie bei allen Querblättern 10 mm unter der Blattkante (oberer Rand der Anlage).
- Kontaktspiegel: verknüpftes Objekt unter der Spule (Lage der Kontakte, neue/gelöschte Kontakte). Wie im
  Original (Dekompilat) steht er unmittelbar über dem Schriftfeld (Unterkante der Zeichenfläche minus größte
  Spiegelhöhe, Feld 14), folgt der X-Lage der Spule, bleibt beim Löschen der Spule erhalten und wird nicht
  mitkopiert. Die Schaltzeichen sind eigene Zeichnung (das Original nimmt drei Bibliothekssymbole aus den
  Programmeinstellungen) in den an einer Aufnahme des Originals gemessenen Maßen; Lage „/Pfad“ auf demselben
  Blatt, sonst „Blatt/Pfad“ (`cargo run --example gegenprobe_plaene`).
- Eigentümer (VSY › OWR): ein Kontakt mit Eigentümer zeigt dessen Kennzeichnung und darunter klein die Lage der
  Spule (Ausdruck Seite 10: „-K1“ mit „/5“); ob die Freigabe „Querverweise“ den Verweis abschaltet, ist nicht
  belegt (der Nachbau richtet sich nach der Freigabe des Kontakts). Die Querverweisliste (Blattart 2) und der
  Klemmplan für im Nachbau gesetzte Eigentümer nutzen noch die eigene Kennzeichnung bzw. nur das gelesene OWR.
- Textboxen (Raumnamen der Installationspläne) stehen wie im Original-Ausdruck: oben links in der Box, 0,2 mm
  eingerückt, Zeilenabstand 1,13 Geviert, hochkant ab dem linken Rand (`src/schriftlage.rs`, Abweichung
  ≤ 0,1 mm). Schriftfeld-Texte 0,88 × Feldhöhe, Spaltennummern 12 pt, „Von“ aus der Anlage. Noch abweichend:
  Kennzeichnungen 2,5 statt 2,62 mm, einige Symboltexte (SPD, M, Dusch/WC) im Original 5–20 % größer.
- Projekt- und Anlagenebene im .epr (`src/epr_projekt.rs`): Stifte, Leitungsdaten, Potentialwerte, Eigentümer/
  Kunde/Standardschrift, Schriftfeld samt Höhe, Zeichnungsnummer, neue Sammelschienen (Potential, Stift), Blattart
  (Klemmleisten und Blattart/Leiste der Klemmpläne: `src/klemmen.rs`), Ort, Strompfade, Pfadbeschreibungen und Begrenzungslinien werden geschrieben. Weitere Angaben (`src/epr_zusatz.rs`): Bauteile samt
  Zuordnung zu Symbolen und Klemmen (BTL/IDL), Platzhaltergruppen (ERS/TLI), Projektstatus, "Alter Klemmenplan",
  Schriftfeld "Absolut", Schalter der Sammelschienen, Verbindungsoptionen, Rackposition sowie Umbenennen,
  Ersetzen und Löschen von Projektgrafiken. Nur im eigenen Format (.epj) bleiben der Ort auf Projektebene (das Original kennt ihn nur je Blatt), Änderungen an
  den fest eingebauten Stiften, ein zweites Kürzel einer Sammelschiene und das Setzen einer Einspeisung; zu lange
  Texte werden gekürzt. Der Nachbau nennt all das beim Speichern als .epr.
  Testdatei für die Gegenprobe im Original: `cargo run --release --example gegenprobe_epr3`.
- Querverweise auf andere Anlagen: Pfeilrichtung aus zwei Beispielen abgeleitet (siehe MECHANICS.md).
- Hilfsprogramme zur Formatanalyse: `cargo run --example dump|vsy|sym|obj -- <datei.epr> …`.
