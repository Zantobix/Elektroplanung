# Änderungen

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/), Versionen nach
[Semantic Versioning](https://semver.org/lang/de/): `MAJOR.MINOR.PATCH`; vor 1.0 können sich Bedienung und
eigenes Dateiformat (`.epj`) noch ändern.

Die Versionsnummer steht nur in `Cargo.toml`. Erreicht eine neue Nummer `main`, erzeugt die Pipeline das Tag
`v<Version>` und ein Release mit dem passenden Abschnitt dieser Datei als Text.

## [Unveröffentlicht]

BMK neuer Symbole wie im Original:
- Neue Symbole speichern die BMK mit Ortsplatzhalter („-E1+%P“ wie in W2), so dass ein später eingetragenes
  Ortskennzeichen erscheint („-E1+W2“); bisher wurde „-E1“ fest geschrieben. Bewusste Abweichung: Bei leerem Ort
  zeigt der Nachbau kein hängendes „+“ (Original: „-E1+“).

Anschlusskreise wie im Original:
- An jedem Symbolanschluss ein kleiner hellgrauer Kreis (Ø 1,6 mm), auch an angeschlossenen – nur auf dem
  Bildschirm, nicht im Druck/PDF/Bildexport (Gegenprobe Original unter Wine und Original-Ausdruck).

Leitung ziehen – Führung folgt von Anfang an der Maus (nach KiCad):
- Das erste Teilstück behält seine Richtung (aus der ersten Bewegung, bei Gleichstand aus der Anschlussrichtung),
  bis es Länge 0 erreicht, also das Ende auf die Höhe bzw. Flucht des Anschlusses zurückkommt; erst dann kippt der
  Knick. Die bisherige Hysterese mit Schwellwerten entfällt, die Vorschau springt bei kleinen Mausbewegungen nicht
  mehr zwischen Führungen. Stich aus dem Anschluss, Sammelschienen, Klickbetrieb und Eckanschlüsse bleiben wie
  bisher.
- Die Leitung verlässt einen Anschluss immer zuerst mit dem Stich in Anschlussrichtung, auch wenn die erste
  Mausbewegung quer dazu geht (vorher z. B. seitlich aus einem oberen Anschluss).
- Bewusste Erweiterung: Leertaste oder „/“ kippt während des Ziehens die Knickrichtung (wie „posture“ in KiCad);
  im Klickbetrieb wechselt die Leertaste weiter wie im Original zum vorigen Blatt.

Brückenbögen an Kreuzungen (bewusste Erweiterung, das Original zeichnet keine):
- Kreuzen sich zwei Leitungen, ohne verbunden zu sein, springt die waagerechtere in einem kleinen Halbkreis über
  die andere (Bildschirm, Druck, PDF, Bildexport). Keine Bögen an Verbindungen (Leitungsende oder Klemmstelle im
  Kreuzungspunkt). Neue Option „Brückenbögen an Kreuzungen“ (Programmverhalten, Standard ein). Die .epr bleibt
  unverändert.

Mehrpolige Symbole wie im Original:
- Symbole mit Versatz je Bild (Element Feld 13/14) werden in der gewählten Variante richtig gezeichnet: Leistungs-
  kontakt, Sicherung, Motorschutz, Thermoschutz 2-/3-polig zeigen alle Pole um den Bezugspunkt verteilt (vorher
  lagen die Pole übereinander), Logikgatter u. a. verschieben Eingänge senkrecht; ebenso im Variantenrad
- Alle Pole sind anschließbar (Anschlussfang, automatische Verbindungen, Klemmplan, Leitungsenden mit Art 1 statt 0);
  Umriss und Treffer neuer Symbole umfassen alle Pole und nur die Elemente der gesetzten Zusatzoptionen
- Der Versatz bleibt beim Bearbeiten und Neuschreiben eines Symbols erhalten

Leitungsenden wie im Original (Maus-Befund M21, Klemmplan im Original leer):
- Beim Speichern bestimmt der Nachbau Art, Ziel und Anschluss jedes Endes neuer und geänderter Verbindungen aus der
  Geometrie: Symbolanschluss (Art 1, VSY-Kennung, Anschlussnummer aus Feld 4 des Anschlusselements, auch bei
  mehrpoligen Symbolen mit Versatz je Option), Sammelschiene (Art 2), Leitung (Art 3), Klemmstelle (Art 5), Verweis
  (Art 6); freie Enden Art 0. Geprüft an allen 1754 Enden in W2 (1751 gleich, 3 begründete Abweichungen)
- Vom Nachbau früher geschriebene Dateien mit Enden ohne Ziel werden beim Speichern berichtigt; unveränderte
  Verbindungen aus Originaldateien bleiben byteidentisch
- Kennungen neuer Objekte (OBJ, VBG, VSY, SMS, SYM) kommen aus einer gemeinsamen Folge und sind eindeutig; die
  Klemmstelle und das erste Leitungsteilstück trugen vorher dieselbe Kennung. Doppelte Kennungen in geladenen
  Dateien werden beim Speichern ersetzt
- Neu geschriebene Projektsymbole tragen Anschlussnummern (Element Feld 4)

Laufzeittexte wie im Original (Verhaltensmetrik M5):
- Speichern über eine temporäre Datei im Zielordner wie das Original; Meldungen „Die Temporäre Datendatei: … kann
  nicht in den gewollten Dateinamen geändert werden!“, „Fehler in den gespeicherten Daten“, „Fehler beim abspeichern
  der Projekt-Daten“
- Laden: Statuszeile „Projekt wird geöffnet“ (Öffnen) bzw. „Projekt wird gelesen“ (Hinzufügen), „Fehlerhaftes
  Projekt“ bei Fehlern; Fehlerliste meldet unlesbare Projektbilder und Klemmleisten mit mehr als 4 Ebenen (werden
  auf 4 begrenzt)
- Eigenschaften mehrerer Objekte: Überschriften „Unterschiedliche Objekte“ und „Gemeinsame Eigenschaften“
- Symboleigenschaften: Tabellenköpfe „Definierte Seiten/Optionen/Stifte/Parameter“, Alternativtexte als Tabelle
  „original Text | Wahl 1 … | Neue Wahl“, Hinweis bei mehr als 32 Alternativtexten bzw. Optionen; Parameter- und
  Textdialog mit den Spaltenköpfen des Originals
- Bibliothek › Symbolgruppen › Hinzufügen: neue Gruppe unter der markierten („Hauptgruppe: …“) oder als „Neue
  Hauptgruppe“, ohne Eingabe „Neue Gruppe“

Behoben (kleinere offene Punkte):
- Einfügen (Strg+V) speichert die eingefügten Objekte mit neuen Kennungen; bisher standen OBJ/VBG/VSY-Kennungen der
  Vorlage doppelt in der .epr-Datei
- Schriftfeld „Geändert“: Blätter ohne Änderungsdatum (z. B. die Klemmpläne in W2) zeigen „---------“ wie das
  Original statt des Projektdatums
- Kennzeichnung (BMK) und Wert der Symbole in der Schriftgröße der Anlage („Titel“/„Wert“ als Bildpunkte bei
  96 dpi: 10 → 2,65 mm, 8 → 2,12 mm) statt fest 2,5/2,2 mm; im Ausdruck des Originals 2,62 mm

Behoben (Maustest 22.11.2026):
- Leitung aufziehen: die Vorschau folgt von Anfang an stetig der Maus; die Knickrichtung wechselt nur nahe am
  Anschluss bei deutlich anderer Bewegung (Hysterese), vom Anschluss weg geht es mindestens einen Stich nach außen,
  kein Sprung mehr auf die Z-Führung beim Überqueren der Anschlusshöhe; Loslassen ergibt die gezeigte Vorschau
- Zeiger wie im Original: über einem Punkt einer Linie, eines Rechtecks, einer Kurve, eines Kreises … Pfeil mit
  vier Pfeilspitzen, Ziehen dort verschiebt nur diesen Punkt (auch unmarkiert); über Klemmstellen das Leitungszeichen
- Leitung ziehen: führt aus dem Anschluss in dessen Richtung heraus und tritt am Ziel von außen ein (Z-Führung
  über die Mitte wie im Original), statt durch das eigene Symbol zu laufen und dabei andere Anschlüsse zu treffen
- Teilstück einer markierten Leitung ziehen verschiebt nur dieses Teilstück quer; die Enden bleiben an ihren
  Anschlüssen (bisher wurde die ganze Leitung verschoben und getrennt)
- Leitungsende auf einer Sammelschiene oder einer anderen Leitung trifft diese senkrecht in der Flucht, statt auf
  den nächsten Rasterpunkt zu springen
- In eine Leitung eingesetzte Klemmstelle lässt sich mit der Maus ziehen; beide Leitungsteile wandern mit (bisher
  begann das Ziehen eine neue Leitung, beim Verschieben mit Strg blieb eine Lücke)
- Neue Sammelschiene (Menü und Werkzeugleiste) wie im Original: nur im Projektbaum anlegen („Schiene n“) und den
  Eigenschaftendialog öffnen; per Ziehen aus dem Baum kommt sie als echtes Sammelschienen-Objekt auf das Blatt
  (oben gestapelt wie in W2). Bisher entstand eine Leitung mit Text, und Ziehen aus dem Baum ging nicht
- Sammelschienen-Dialog: Potential „Keine Zuordnung“ wählbar und Vorgabe; neue Schienen mit Kürzel „L<n>“, ohne Pfeile
  (Vorgaben des Originals)
- Leitung von einer Sammelschiene zu einem Bauteil ziehen: Ziehen auf einer nicht markierten Schiene beginnt eine
  Leitung, der Leiter verlässt die Schiene senkrecht in der Flucht des Ziels (bisher entstand nichts)
- Verbindungsknoten: Punkte auch an Abgriffen von Sammelschienen; Knoten folgen beim Verschieben der Geometrie, und
  beim Speichern trägt das Leitungsende die Anschlussart des Originals (VBG Art 3 auf Leitung bzw. 2 an Schiene mit
  Kennung des Ziels), ein verwaister Knoten wird gelöscht (src/knoten.rs, src/schiene_leitung.rs)
- Doppelklick auf eine Anlage im Projektbaum öffnet ihre Eigenschaften (wie im Original) statt sie zuzuklappen
- Neue Klemmleisten wie im Original: Name „Klemme n“, Titel „Xn“ – eingesetzte Klemmstellen heißen „X1.1“ statt
  „Klemme 1.1“
- Sammelschiene auf dem Blatt mit ihrem Kürzel („L1“) beschriftet statt mit dem Namen („Schiene 1“); Name und Kürzel
  werden wie im Original gespeichert (Name in SMS Byte 0, Kürzel im TIT) – bisher ging das Kürzel beim Speichern
  verloren und das Original beschriftete die Schiene mit dem Namen. Im Projektbaum steht wie im Original der Name
- Textbox-Eigenschaften: Felder Oben/Links schrumpfen nicht mehr („2100“ war abgeschnitten)
- Bewusste Abweichung: Blattreiter mit gleicher Beschriftung aus verschiedenen Anlagen (z. B. zweimal „[1] Blatt 1“)
  tragen die Anlagen-Kurzform bzw. den Anlagennamen davor; das Original unterscheidet sie nicht
- Leitung zu einer Sammelschiene von einem Anschluss, der von der Schiene weg zeigt: der Leiter trifft die Schiene
  senkrecht an der Mausposition, statt um das Symbol herum und auf der Schiene zurück zu laufen (Knoten lag sonst
  auf der Kreuzung mit einer anderen Leitung)
- Hinzufügen › Blatt (auch Datei › Neu › Blatt, Knopf „Neues Blatt“) öffnet wie im Original gleich den
  Eigenschaftendialog des neuen Blattes; Abbruch nimmt es wieder weg
- Leitungsenden werden wie im Original gespeichert (Art, Kennung, Anschlussnummer an Symbol, Schiene, Klemmstelle,
  Leitung); Klemmpläne vom Nachbau gezeichneter Projekte sind im Original nicht mehr leer. Klemmplan nennt Schienen
  „Kürzel (Name)“ wie das Original
- Rahmenauswahl wie im Original: markiert werden Objekte, deren Anfangs-/Einfügepunkt im Rahmen liegt (bisher nur
  ganz enthaltene Objekte)
- Blattrahmen (mit Strompfadleiste, Schriftfeld und Zeichenfläche) kommt aus den Blatträndern der Anlage statt aus
  festen Maßen und folgt einer Änderung in den Anlagen-Eigenschaften sofort; im Querformat ist „Links“ der obere
  Rand (Heftrand). Neue Anlagen und Projekte haben wie im Original Links 20 mm (bisher 10 mm)

## [0.2.0]

Neu:
- `examples/pdf_export.rs`: Projekt ohne Fenster als PDF und jedes Blatt als PNG ausgeben (Vergleich)
- Tabelle wie im Original: Spaltenbreiten (Autobreite mit 3 mm Zuschlag, letzte Spalte bis zum Rand, sonst
  gestaucht), Schrift aus der Zeilenhöhe, „Ab“ als erste angezeigte Zeile; keine doppelten Linien mehr bei leeren
  Tabellen. Neue Tabellen 2 × 2 ohne Überschrift. Dialog mit eigenen Icons und den Hinweisen des Originals; Liste
  laden/speichern als Textdatei mit Tabulatoren, „Zeile löschen“ für die markierte Zeile, Pfeiltasten im Raster
- PDF: Texte mit eigener Schrift betten die installierte TrueType-Schrift als Teilmenge ein (statt Helvetica/
  Courier/Times); Rückfall auf die Standardschriften wie bisher
- Umriss, Treffertest, Markierung und Griffe von Texten passen zur gezeichneten Schrift (Metrik der
  installierten Schrift, Times als Rückfall)
- Symbolbibliothek: Freigaben (Querverweise, Sicherungsübersicht) werden aus Symbole.tfi gelesen und beim
  Setzen übernommen
- `examples/gegenprobe_plaene.rs`: Testdatei für das Original mit Kontaktspiegel, Querverweis-, Artikellisten- und
  Inhaltsverzeichnis-Blättern, Tabellenarten, Symbol-Freigaben, Symbol-Stift und bearbeitetem Schriftfeld
- Texte werden in ihrem eigenen Schriftnamen (sonst der Standardschrift des Projekts) gezeichnet – auf der
  Zeichenfläche, im Bildexport und im PDF (dort als Times/Helvetica/Courier); der Schriftdialog listet die
  installierten Schriften
- Schriftfeld bearbeiten grafisch: Linien, Texte und Logo verschieben, in der Größe ändern, hinzufügen, löschen;
  das Ergebnis wird ins `.epr` geschrieben
- .epr: Freigaben der Projektsymbole (Querverweise, Sicherungsübersicht, SYM Byte 112) werden gelesen und
  geschrieben
- Schriften je Text unter Linux auch aus den XDG-Ordnern ($XDG_DATA_HOME, $XDG_DATA_DIRS)
- Symboleditor: eigene Stifte des Symbols (SIT) anlegen, ändern, löschen und Elementen zuweisen; sie werden ins
  `.epr` geschrieben und beim Zeichnen benutzt (Breite, Art, Farbe)
- Klemmen-Dialog: Feld „Bauteil“ mit Knopf „Bearbeiten“ wie beim Symbol
- Blattmenü › Parameter und › Alternativtext › Benutzerdefiniert: Tabelle wie im Original; Werte werden im
  `.epr` gespeichert (VSY › SPA / ALT)
- Symboleditor: Reiter „Stifte / Parameter“ bearbeitet die Parameter des Symbols (GRD › PRA)
- Symboleigenschaften: „Index“ zeigt den Zähler %C und lässt sich mit „Korrektur“ ändern
- Drucken: „Änderung seit …“ und „seit dem letzten Ausdruck“ wählen die Blätter nach ihrem Änderungsdatum
- Projektgrafiken: Hinzufügen › Grafik legt eine Grafik aus einer Bilddatei an; Eigenschaften (Name, Laden)
  und Löschen im Projektbaum
- Datei › Export › PDF öffnet die Exporteinstellung mit der Projektliste; Vorlagen als `.epv`
- Blattnummer: Regeln des Originals für Nummer 0, Subindex 0 und Deckblatt
- Sammelschienen-Objekt: Knopf „Bearbeiten“ für die Schiene; Leitung: Darstellung über einen Stift
- Kontaktspiegel als verknüpftes Objekt unter der Spule (Dialog wie im Original, Lage der Kontakte als
  „Blatt/Pfad“, führt neue/gelöschte Kontakte nach, folgt der Spule), gelesen und geschrieben im `.epr`
- Blattart Querverweise (1–3 Tabellen) und Tabelle › Daten „Querverweise“; Tabellen „Inhaltsverzeichnis“ und
  „Querverweise“ führen sich selbst nach, die Art wird im `.epr` gespeichert
- Artikelliste mit Zusatz „Summe+BMK“ und „Einzelliste“; Zusatz der Blattarten wird im `.epr` gespeichert;
  fehlende Folgeblätter der Listen werden angelegt
- Tabelle › Daten „Inhaltsverzeichnis“; Bauteil: Herstellerliste und Darstellung (Symbol, Bild)
- .epr: neu angelegte Textboxen, Tabellen (Überschrift/Autobreite), Bilder (Bilddatei wird Projektbild, über
  64 KB geteilt), Verweise samt Ziel und Gegenverweis sowie Sammelschienen (neue Schiene in der Anlage) werden
  geschrieben; neue Verbindungen mit Leitung und Verbindungsbezeichnungen, neue Symbole mit Zähler,
  Alternativtexten und Parametern, neue Zeichenelemente mit „Fixiert“
- .epr: Schriftname und Umbruch von Texten, Bilder auf dem Blatt (Typ 12) und Tabelleneinstellungen werden gelesen
- Zeichenfläche: Vormarkierung wie im Original – das Objekt unter der Maus bekommt ein gestricheltes Rechteck;
  Mauszeiger mit Verschiebekreuz (Objekte) bzw. „A“ (Texte), Größenpfeile über den Griffen, Hand und Hinweis
  „Strg+Klick“ schon beim Überfahren eines Verweises
- Titel und Wert eines Symbols lassen sich direkt (ohne vorheriges Markieren) gegenüber dem Symbol verschieben
- Leitungen lassen sich an die Pfeilspitze eines Verweises anschließen; im .epr verweist die Verbindung auf den
  Verweis (VBG Feld 2/3 bzw. 6/7), Leitungen ziehen beim Verschieben des Verweises mit
- Projektbaum: Plus/Minus-Kästchen vor Knoten mit Kindern wie im Windows-Baum des Originals; Pfeil rechts/links
  klappt auf bzw. zu (und springt zum ersten Kind bzw. zum übergeordneten Knoten)
- Kontaktspiegel: Knopf „Set Referenz“ und Kontextmenü „Setze Referenz“ – der Dialog verschwindet, das danach
  angeklickte Symbol wird Referenz der markierten Zeile; beim Übernehmen wird die Spule sein Eigentümer
- Textbox wie im Original: Text steht oben in der Box und wird am Boxrand abgeschnitten; „Umbruch“ ist wie im
  Original gesperrt und ohne Wirkung (Wert wird gelesen, angezeigt und gespeichert); „Drehung“ als Auswahl 0°–270°
- Bibliotheksverwaltung › Symbolgruppen › Hinzufügen legt eine leere Gruppe ohne Platzhalter-Symbol an (gemerkt in
  `benutzer_gruppen.json`, Symbole.tfi bleibt unverändert); sie erscheint im Symbolbaum unter Benutzerbauteile

Geändert:
- Einheitliche Icons: je Objektart ein Grund-Icon (Klemmleiste, Sammelschiene, Leitung, Potential, Stift, Blatt,
  Anlage, Projekt, Symbol, Bauteil, Platzhalter, Bild, Tabelle, Text, Textbox …), das Werkzeugleisten,
  Projektbaum und Dialoge gemeinsam nutzen; „Neu“-Knöpfe zeigen das Grund-Icon mit grünem Plus, „Bearbeiten“
  mit Stift, „Entfernen“ mit rotem Minus; ein Raster, drei Strichstärken, eine Farbpalette
  (`tools/icons_erzeugen.py --uebersicht` erzeugt eine Übersichtsseite). Anlagen tragen im Projektbaum das
  Schaltschrank-Icon; die Tabellen-Knöpfe Laden/Speichern/Löschen nutzen Icons statt Emoji
- Menü Datei › Export › „PDF“ ohne den Zusatz „(Beta)“ des Originals: Das PDF wurde gegen den Bildexport
  geprüft (alle 39 Blätter von W2 und ein Prüfprojekt mit sieben Schriften in vier Schnitten, Sonderzeichen,
  senkrechtem Text, Bild und Hochformat-Blatt; gerendert mit pdftoppm): Seitengrößen A4 quer/hoch, Linien,
  Texte und Umlaute stimmen überein, Text ist auswählbar und durchsuchbar (pdftotext), eingebettete Schriften
  als Teilmenge mit ToUnicode, Verknüpfungen zeigen auf gültige Seiten. Die dabei gefundenen Fehler sind behoben
  (siehe „Behoben“)

Behoben:
- PDF: Bildobjekte fehlten (nur der Rahmen erschien); sie werden jetzt als Bild eingebettet (auch Bilder
  anderer Blätter und Projektgrafiken, Transparenz als Maske, jedes Bild nur einmal)
- PDF: Zeichen außerhalb von WinAnsi (Ω, ≥, →, ✓ …) erschienen als „?“; sie kommen jetzt aus einer
  eingebetteten Ersatzschrift (Times New Roman, sonst Segoe UI Symbol/DejaVu Sans). Typografische Zeichen wie
  ‚ ‘ ’ • ™ … nutzen die vollständige WinAnsi-Tabelle
- PDF: Ellipsen ohne Strich und Füllung wurden als dünne Linie ausgegeben
- Oberfläche: Symbole wie ✓, die weder Segoe UI noch die egui-Schriften enthalten, erschienen als leeres
  Kästchen; letzter Rückfall ist jetzt Segoe UI Symbol (Linux/macOS: DejaVu Sans, Symbola, Apple Symbols)
Geändert (Vergleich mit dem Original-Ausdruck):
- Textboxen: Text 0,2 mm eingerückt, Zeilenabstand 1,13 Geviert, hochkante Zeilen ab dem linken Boxrand
- Schriftfeld: Schrift 0,88 × Feldhöhe (statt 0,72), „Von“ aus „Erstellt von“ der Anlage; Spaltennummern 12 pt
- Klemmplan: Zeilen 5 mm, Schrift 3,43 mm, Spaltenbreiten nach dem längsten Text, Klemmenkreise und
  Brückenspuren wie im Ausdruck; je Anschlussrichtung höchstens eine Brücke; verbundene Brücken innerhalb einer
  Kette bleiben in deren Spur (Klemmplan -X3: 3–6, 4–5, 5–6 als eine Kette)
- Textboxen und Raumnamen: der Eckgriff ändert wie im Original Breite und Höhe der Box frei, die Schrift bleibt
  gleich; neue Textboxen beginnen wie im Original ohne Text
- Schriftfeld bearbeiten: das Fenster wuchs über den Bildschirm hinaus und die Elementliste lag übereinander
- Neue Textbox wie im Original: Doppelklick und Eigenschaften öffnen den Textbox-Dialog (Abmessung, Rahmen,
  Umbruch, Fixiert, Ausrichtung, Drehung, Schrift, Text) statt nur „Text bearbeiten“; Vorgaben des Originals
  (Rahmen an, Umbruch aus, 8 pt); alle Einstellungen werden im `.epr` gespeichert.
  Textboxen sind jetzt Texte mit Box (eine Art statt zwei); ältere `.epj` werden beim Laden umgewandelt.
  MCP `add_object` kennt `kind: textbox` (mit `frame`, `wrap`), `get_sheet` liefert Box, Rahmen und Umbruch
- Kontaktspiegel wie im Original: steht unmittelbar über dem Schriftfeld (Unterkante der Zeichenfläche minus
  größte Spiegelhöhe des Blattes, bisher mitten im Schriftfeld), Schrift und Schaltzeichen in Kontaktgröße,
  Lage „/Pfad“ für Kontakte auf demselben Blatt (sonst „Blatt/Pfad“), X-Lage folgt der Spule
- Eigentümer (VSY › OWR): Symbole mit Eigentümer zeigen dessen Kennzeichnung und darunter klein seine Lage
  („-K1“ mit „/5“ statt „-Q1+W2“); Kontaktspiegel › Übernehmen und Eigentümer › Setzen schreiben das OWR
  (bisher wurde die Kennzeichnung der Spule in den Kontakt kopiert)
- Querverweisliste (Blattart 2, Tabelle „Querverweise“): Kontakte mit Eigentümer stehen wie im Original mit ihrer
  Fundstelle unter der Kennzeichnung der Spule („-K1“ mit „6/5 6/6“), nicht unter ihrer eigenen
- Klemmplan, Artikelliste, Listen-Export, Suche, Projekt prüfen, Sicherungsübersicht und Beschriftung greifen
  nutzen dieselbe Eigentümer-Quelle wie die Zeichnung: auch im Nachbau gesetzte oder entfernte Eigentümer wirken
  (bisher wertete der Klemmplan nur das gelesene OWR aus); doppelte Kennzeichnung meldet Kontakte mit Eigentümer
  nicht mehr
- Rahmen quer beginnt immer 10 mm unter der Blattkante (oberer Rand der Anlage); BLT Feld 24 ist die Unterkante
  der Zeichenfläche, keine Rahmenhöhe (neue Blätter und Listenblätter lagen bisher bei 20 mm)
- .epr: Anschlussnummern im Kontaktspiegel als STR im ALT (das Original meldete „Fehler beim erzeugen des
  Objekts: STR“); leere Tabellen ohne STR (das Original meldete die ZCN-Datenstruktur als fehlerhaft)
- Listenblätter: Spaltenbreiten, Ränder, Abstand der Tabellen und Zeilenhöhe wie im Original
- Tools › Ausrichtung: Referenzpunkt wie im Original „Mitte“, „Links/Oben“, „Rechts/Unten“ – Bezug ist die
  gemeinsame Umrandung der Auswahl (bisher ein einzelnes markiertes Objekt)
- Projektbaum: Ein einfacher Klick auf eine Gruppe klappte sie schon um, ein Doppelklick daher auf und gleich
  wieder zu. Jetzt markiert ein Klick nur, Doppelklick und Kästchen klappen um
- Verweise wie im Original-Ausdruck der Installationspläne: Pfeil 2,4 mm lang zwischen Lage und Richtung,
  OBJ Feld 15 Bit 0 = Spitze an der Lage, Feld 4 Byte 3 = Text hochkant (bisher als Querverweis gedeutet,
  „=S1+W2“ stand falsch), Textversatz = Mitte der Textoberkante
- Klemmplan: Brücken als Ketten in Spuren wie im Original statt als zusammengefasste Gruppen (mehrstöckige Leisten)
- Kontaktspiegel bleiben wie im Original stehen, wenn die Spule verschoben wird, und werden nicht mitkopiert;
  OBJ Feld 14 = Höhe des Spiegels wie beim Übernehmen im Original
- Elemente mit eigenem Symbolstift (Nummer ab 4) wurden gepunktet gezeichnet und als Stift 3 gespeichert
- Schriftfeld bearbeiten: Elemente lassen sich nicht mehr über den Rand schieben; Zeilenzahl wie im Original
  höchstens 8
- Vergrößerter Text: Markierung, Treffer und Rahmen passten nicht zur gezeichneten Schrift (die Textbox wuchs
  nicht mit; Umriss jetzt aus den Schriftmaßen der Times)
- Drehen eines Verweises dreht nur den Pfeil, der Text bleibt an seiner Stelle; beim Vergrößern eines Symbols
  bleiben Titel und Wert stehen
- Die Rackposition überschrieb beim Speichern die Parameterwerte des Symbols (SPA)
- .epr: Verschieben eines Textes oder einer Tabelle veränderte Schriftgrad bzw. Spalten/Zeilen im Originalblock

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
- Querverweise: Strg+Klick (Hand als Mauszeiger) bzw. Doppelklick springt zum Ziel (auch Querverweistexte von
  Sammelschienen), Verweise lassen sich drehen (der Text bleibt waagerecht), das Ziel wird gespeichert; im PDF
  sind Querverweise anklickbar
- Option „Rasterdarstellung“: Stärke des Rasters (Hell, Mittel, Kräftig oder eigene Farbe)
- Symbole und Texte lassen sich mit dem Rahmen proportional vergrößern/verkleinern (einzeln über Eckgriffe)
- Titel (BMK) und Wert eines markierten Symbols lassen sich einzeln mit der Maus verschieben
- Symbol bearbeiten: Änderung auf alle Symbole dieses Typs oder nur auf das bearbeitete übernehmen (eigene Kopie)

Behoben / geändert wie im Original:
- Variantenrad: Bilder gleichmäßig auf dem Ring, auf die Ringbreite verkleinert und um ihre Mitte zentriert
  (auch bei bis zu 12 Bildern ohne Überlappung); im Loch das gewählte Bild
- Rasterfang: der Gitterknopf schaltet den Rasterfang ein/aus; „Kein“ entfällt in der Rasterauswahl
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
