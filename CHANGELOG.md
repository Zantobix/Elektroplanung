# Änderungen

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/), Versionen nach
[Semantic Versioning](https://semver.org/lang/de/): `MAJOR.MINOR.PATCH`; vor 1.0 können sich Bedienung und
eigenes Dateiformat (`.epj`) noch ändern.

Die Versionsnummer steht nur in `Cargo.toml`. Erreicht eine neue Nummer `main`, erzeugt die Pipeline das Tag
`v<Version>` und ein Release mit dem passenden Abschnitt dieser Datei als Text.

## [Unveröffentlicht]

Neu:
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
- Datei › Export › PDF (Beta) öffnet die Exporteinstellung mit der Projektliste; Vorlagen als `.epv`
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

Behoben:
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
