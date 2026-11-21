# Elektroplanung – Mechanik-Blatt (aus dem Original gelesen)

Quelle: `Elektroplanung.exe` (Borland C++ Builder, VCL), Version 6.4.2 laut Änderung.txt.
Evidence-Ids stehen in Klammern.

## Hauptfenster (DFM-Ressource TFORM1, ev_0b6f30…)
- Fenstertitel neues Projekt: "Elektroplanung Neues Projekt" (String 0x5b9d70, gesetzt in FUN_004055d8).
- Mindestgröße 830 × 600 (Constraints).
- Menü: Datei (Neu ▸ Projekt/Blatt, Projekt öffnen, Projekt hinzufügen, Speichern, Projekt Speichern als,
  Vorlage öffnen/speichern, Schriftfeld speichern, Bitmapexport, DXF Import, Beenden), Projekt (Eigenschaften,
  Drucken, Info), Bearbeiten (Ausschneiden, Kopieren, Einfügen, Löschen), Hinzufügen (Anlage, Blatt,
  Symbol, Bauteil, Potential, Leitung, Klemmleiste, Sammelschine, Schriftfeld, Grafik, Platzhaltergruppe),
  Tools (Markierung als Symbol, Strompfadbezeichnung bearbeiten, Ausrichtung, Textgenerator, Schriftfeld bearbeiten,
  Sammelschienenassistent, Blattnummerierung, Listen Export), Extras (Bibliotheksverwaltung, Option), Hilfe (Info).
- Tastenkürzel: Strg+Z Rückgängig (16474), Strg+W Forwärts (16471), Strg+C (16451), Strg+V (16470),
  Strg+X (16472), Entf Löschen (46). Taste S = Strompfadausrichtung, Taste A = Automatische Verbindungen (Hints).
- Werkzeugleiste oben (ToolBar2, Knöpfe 27 × 26 px): Neues Projekt, Projekt laden, Speichern, Projekt speichern,
  Drucken | Ausschneiden, Kopieren, Einfügen | Rückgängig, Wiederherstellen | "Stift" + Stiftliste (100 px) +
  Neuer Stift | Vergrößern/Verkleinern | "Rasterfang" + Rasterliste (67 px) + Gitter | Strompfadausrichtung |
  Automatische Verbindungen | Rahmen zur Größenänderung | Fixierung aktivieren.
  Startzustand gedrückt: Gitter, Strompfadausrichtung, AutoVerb, SizeRahmen, Fixierung.
- Zweite Leiste (ToolBar3, 29 px Einzug): Neues Blatt, Neues Potential, Neue Sammelschiene | Drehen,
  Vertikal Spiegeln, Horizontal Spiegeln | Gruppieren, Gruppe auflösen | "Leitung" + Leitungsliste + Neue Leitung |
  "Klemmleiste" + Klemmenliste (105 px) + Neue Klemmleiste.
- Linke Leiste (ToolBar1, 27 px breit): Auswahl, Neue Klemmstelle, Neue Linie, Neue Kurve, Neues Rechteck,
  Neues Dreieck, Neuer Kreis, Neuer Text, Neue Textbox, Neue Tabelle, Neues Bild.
- Rechts: PageControl mit Reitern Projekt / Symbole / Bauteile (Baum), darunter Suchfeld (155 px) mit 2 Knöpfen.
- Unten: Blattreiter "[n] Blatt n" (TabPosition unten), Statusleiste mit Feldern 130 / 350 / 200 / Rest px.
- Statusleiste: Feld 1 "X: 65,0  Y: 45,0" (Mauskoordinaten in mm), Feld 3 Projektstatus.

## Startwerte eines neuen Projekts (FUN_004055d8 @ 0x4055d8)
- Schrift: "Times New Roman" (0x5b9d38).
- Potentiale: L1, L2, L3, N, PE, 24VDC, OV (0x5b9d58…).
- Stiftliste: Standard (0), Gestrichelt (1), Strich/Punkt (2), Gepunktet (3); Index 0 aktiv.
- Namen neuer Objekte: "Anlage n", "Blatt n", "Klemme n", "Leitung n", "Potential n", "Schiene n", "Stift n" (0x5b9ccd…).
- Projektstatus-Texte: "Projekt wird bearbeitet" / "Projekt ist abgeschlossen" (0x5b948b).
- Rasterliste: Kein, 1,0mm, 2,5mm, 3,5mm, 5,0mm, 7,0mm, 10,0mm, 14,0mm (0x5b9e0a…); Start 10,0mm (Bild).
- Projektbaum je Anlage (FUN_004031a4): Anlage n ▸ Sammelschienen, Klemmleisten, Blätter (offen).
- Bauteilgruppen: Favoriten (HauptID 100), Benutzerbauteile (200), Standardbauteile (300) (SQL-Strings 0x5bc5cd).

## Standard-Schriftfeld (FUN_00401e40 @ 0x401e40)
Koordinaten normiert 0…10000 auf das Schriftfeld-Rechteck (x nach rechts, y nach unten).
46 Elemente: Typ 1 = Linie, Typ 6 = Textfeld (Text-Nr., Ausrichtung 0 links/1 mittig/2 rechts, Zeilen),
Typ 12 = Logo-Feld. Texte (0x5b91dc): %Projekt, %Erstellt, Erstellt:, %Geändert, Geändert:, %Name, %Von,
%Seite, %Anlage, Blatt, %Next, %Status, %Ort, %Owner, %Kunde, %Prev, %Anzahl, Anzahl:, nach:, vor:, %Max,
Max:, %ZNR, ZNr.:, Stromlaufplan, %Datum, Datum:, Von:, Von:, Gepr.:
Linien: y=0 voll; y=3333 und 6666 (x 0…2536); y=6000 (x 7292…8567); x=2536, 7292, 8571, 1289, 3386, 4261
voll; x=9160 (y 2000…10000); y=2000 (x 7296…10000); y=7333 (x 8571…10000); y=4666 (x 8571…9369);
x=9369 (y 2000…7333). Die Textfelder sind 1:1 in `src/titleblock.rs` übernommen.

## Blatt (gemessen im Bild des Originals, Rasterabstand = 10 mm = 41,5 px → 4,155 px/mm)
- Blattgröße 297 × 210 mm (A4 quer); Hintergrund Ansicht RGB(128,128,128), Blatt RGB(238,221,221),
  Rasterlinien RGB(224,208,208), Rahmen schwarz.
- Rahmen: links 5, rechts 5, oben 20, unten 5 mm. Spaltenleiste oben 5 mm hoch, 8 Strompfade (je 35,875 mm),
  Nummern 1…8 mittig in Times New Roman.
- Schriftfeld: volle Rahmenbreite, 15 mm hoch, am unteren Rahmen.
- Rasterursprung: (5 mm, 5 mm) ab Blattecke.
- Ansicht: Blatt passt mit ca. 5 px Rand in die Höhe und steht mittig.

## Symbolbibliothek (Datenbank\Gruppen.tfi, Symbole.tfi – selbst entschlüsselt)
- Erstes u32 der Datei = Beginn des ersten Datensatzes.
- Gruppen: [u32 Länge=74][01 Name 41 B][Art][01 HauptID i64][01 OwnerID i64][01 ID i64].
  Art 3 = Symbol-Wurzeln (Favoriten 100, Benutzerbauteile 200, Standardbauteile 300), Art 4 = Bauteil-Wurzeln,
  Art 0 = Gruppen (OwnerID = übergeordnete Gruppe, z. B. Lampen → Signaleinrichtung).
- Symbole: [u32 Länge][u32 Länge][01 ID 8 B][01 Name 40 B][01 HauptID][01 GruppeID][01 Geändert f64][01 Favorit]
  [u32 n][01 BMK-Vorlage …][u32 n][01 "Ver1"/"Ver2" Zeichendaten]. Der Symbolkopf (n = 49) trägt ab Byte 45 die
  Freigaben als u32 (Bit 0 Querverweise, Bit 1 Sicherungsübersicht, wie SYM Byte 112 im .epr); Merker 0 = leerer Kopf.
- Zeichendaten: Kopf 0x3c Byte (Breite +0x0c, Höhe +0x10, Elementzahl +0x14), Elemente zu 0x50 Byte
  (gleicher Aufbau wie die Schriftfeld-Elemente in FUN_00401e40): Koordinaten in 0,1 mm, y nach unten;
  Typ +0x3c: 1 Linie, 2 Bézier (4 Punkte), 3 Viereck (4 Punkte), 4 Dreieck (3 Punkte), 5 Ellipse (Rechteck),
  6 Text (Rechteck, Ausrichtung +0x10, Textnummer +0x1c), 7 Anschluss (Punkt + Zuleitung), 9 Titelanker (BMK);
  Stiftart +0x40, Option +0x44, Modus +0x46 (1 = in allen Optionen), Seite +0x48.
  Danach Texttabelle [u32 n][Texte, 0-getrennt; Text 1 = BMK-Vorlage] und Optionsnamen.
- Angezeigt wird Seite 0 / Option 0 (Grundvariante). 104 Symbole, Favoriten wie im Bild.

## Geschätzt (im Code gesucht, aber nicht gefunden/nicht gelesen)
- Weitere Symboloptionen (z. B. Betätigungsarten) und Textmakros %{…} in Symbolen werden noch nicht angezeigt.
- Verhalten "Strompfadausrichtung": Symbol rastet waagerecht auf Anschlusspunkte anderer Symbole ein (±3 mm).
- Verhalten "Automatische Verbindungen": Anschluss auf einer Leitung erzeugt einen Verbindungspunkt.
- Projektdateiformat: eigenes JSON (.epj) statt .epr.

## Alte Projektdateien (*.epr, Kennung "EP16") – selbst entschlüsselt, Import in src/epr.rs
- Blöcke: 0x99 + Kennung (3 Zeichen) + u16 Länge + i16 (= !Länge) + Daten + u32; Unterblöcke folgen, "0x99END" schließt.
- PRJ (Name 40 B, Erstellt/Geändert als Delphi-Datum) › LTG, POT, SYM (Name 40 B, BMK-Vorlage ab 40, ID 8 B ab 80;
  GRD › GRS Elemente 0x50 B, STR Texte, SEI Optionsnamen), FUS (Projekt-Schriftfeld GRS + STR), ANL (Name 0–55, Kurz 55,
  Von ab 66, Strompfade i32[44]) › BLT (Name, Blattnummer = i32[23]>>8) › ZCN (Zeichenelement, Typ 11 = Textfeld mit STL,
  Schriftgröße i32[4] in pt, i32[5] = 90 hochkant), VSY (Symbol: Titel-Versatz i32[10..11], hochkant Bit 24 von i32[12],
  Lage i32[16..17], Drehung/Spiegelung i32[18] Bits 1/2/4/8/16, Maßstab f32 @76, Option i32[22], Symbol-ID @104, TIT = BMK),
  VBG › VBP (Leitungspunkte), OBJ (Verweis-Text TIT, Lage i32[6..7] + Versatz i32[2..3], Winkel i32[14] in 0,1°).
- Optionssichtbarkeit (aus Lampe/Schließer/Lampensymbole abgeleitet): Modus 0 = nur diese Option, 1 = ab dieser, 2 = bis zu dieser.
- Geprüft gegen "Elektroplanung Wohnung 2.pdf" Seite 3 (Altbau EG): Wände, Räume, Symbole, BMK, Schriftfeld stimmen überein.
- Speichern erfolgt im eigenen Format (.epj neben der .epr); Zurückschreiben ins .epr-Format ist nicht umgesetzt.

## Drucken / PDF / DXF (neu)
- Drucken und "PDF Export": alle Blätter als Vektor-PDF (Times-Roman, Linien, Kurven); Drucken übergibt das PDF an den Windows-Standarddrucker. Geprüft: W2.epr Seite 3 entspricht dem Original-Ausdruck.
- DXF Import wie im Original (Änderung.txt 6.4.2): Linien, Polygone, Kreise, Kreisabschnitte, Splines als Linienzug; zu große Zeichnungen werden in den Rahmen eingepasst (geschätzt).
- Tools: Markierung als Symbol, Ausrichtung (links/rechts/oben/unten/mittig); Symbol-Eigenschaften mit Optionsauswahl.

## Abdeckung der Funktionen des Original-Hauptfensters (Ereignisse aus der Formularressource TFORM1)
Alle 141 Menü-/Knopf-Zuordnungen des Originals sind im Nachbau belegt:
- Datei: Neu Projekt/Blatt, Projekt öffnen/hinzufügen (.epj und alte .epr), Speichern, Speichern als, Vorlage öffnen/speichern,
  Schriftfeld speichern (Vorgabe für neue Projekte), Bitmapexport, PDF Export, DXF Import, zuletzt geöffnete Projekte (Open1–5), Beenden.
- Projekt: Eigenschaften, Drucken (PDF an Standarddrucker), Info.
- Bearbeiten: Ausschneiden, Kopieren, Einfügen, Löschen, Rückgängig (Strg+Z), Forwärts (Strg+W).
- Hinzufügen: Anlage, Blatt, Symbol (nur Projekt) → Symbolbearbeitung, Bauteil, Potential, Leitung, Klemmleiste, Sammelschine,
  Schriftfeld (aus anderem Projekt), Grafik, Platzhaltergruppe.
- Tools: Markierung als Symbol, Strompfadbezeichnung, Ausrichtung, Textgenerator, Schriftfeld bearbeiten, Sammelschienenassistent,
  Blattnummerierung, Listen Export (mit Bauteilen).
- Extras: Bibliotheksverwaltung (Favoriten, Bearbeiten, Benutzerbereich), Option.
- Blatt-Kontextmenü (PopupMenu1): Löschen, Objekt einfügen, Verweis Start/Ziel, Sammelschiene › Nach Unten, Optionen, Alternativtext,
  Eigentümer (Eigenschaften/Setzen/Suchen), Bezeichner setzen, Kontaktspiegel, Bauteilliste (Bearbeiten/Löschen), Symbol bearbeiten,
  Ausrichtung (Links/Mittig/Rechts/Hochkant/Waagerecht), Eigenschaften, Fixieren Ja/Nein, Pfadbezeichnung.
- Explorer-Kontextmenü (PopupMenu2): Eigenschaften, Löschen, In Bibliothek (Standard/Benutzer), Symbol bearbeiten, Blatt schließen,
  Alle Blätter schließen, Bauteil bearbeiten, Favoriten hinzufügen/entfernen, Reihenfolge, Neues Symbol/Grafik/Leitung/Potential/
  Stift/Bauteil/Platzhaltergruppe. Blattreiter-Menü (PopupMenu3) vollständig.
- Symbolbearbeitung (BLeiste): Drehen, Spiegeln, Gruppieren/Auflösen, Stift, Neuer Stift, Neue Option, Optionsliste,
  Option ein/aus für markierte Elemente, Übernehmen, Abbruch.
- Dialoge nach den Originalformularen: TSBlatt (Hochformat/Querformat, Strompfade, Nummer), TStift (Name, Breite, Stiftart, Farbe),
  TLeitung (Name, Querschnitt, Adern, Kennzeichnung, Farbe), TPotential, TSammelS, TSBauteil, TSchriftfeld, TStextliste.
- Hochformat: Rahmen links 10 mm, sonst 5 mm (gemessen im Original-PDF, Seite 1 "Neubau EG"); .epr: BLT i32[24] > 2400.
Bekannte Grenzen: Speichern zurück ins .epr-Format und Schreiben in die Original-Datenbank (Symbole.tfi) sind nicht umgesetzt
(Änderungen an Bibliothekssymbolen werden im Projekt und in einer eigenen Benutzerbibliothek gespeichert); Klemmenplan- und
Sicherungsübersichts-Blätter werden aus .epr als normale Blätter mit ihrem Inhalt übernommen, nicht neu berechnet.

## Nachtrag 18.11.2026 – Originalformat verlustfrei, Ermittlung mit REA/Rizin
Quelle: Elektroplanung.exe über REA (`execute_rizin_command`), Kennungstabelle bei 0x5b9134 (43 Kennungen inkl. DAT),
"EP16" bei 0x5b92e1; Formularressource TFORM1 (RCDATA) ausgelesen.
- Blockprüfsumme (u32 nach den Daten) = Summe der Datenbytes als **vorzeichenbehaftete** Bytes (i8). Geprüft: alle 6557 Blöcke in W2.epr.
- Längenprüfwert i16 = !Länge (auch 0xFFFF / 0x0000).
- Bilder: BLD › JPG. Über 65535 Byte geteilt: Länge 0xFFFF + 4 Byte Teilkopf (Nr., Anzahl, u16 Restlänge), Prüfsumme ohne Teilkopf;
  Folgeteile sind weitere JPG-Köpfe mit Teilkopf; danach END (JPG schließt wie jeder Block mit END).
- Ergebnis: `eprfile.rs` liest und schreibt W2.epr **byteidentisch** (815 515 Byte); `epr_write.rs` schreibt geänderte Projekte zurück.
  Gegenprobe: Das Original öffnet eine vom Nachbau geänderte Datei fehlerfrei und zeigt die Änderungen.
- BLT Feld 24 = Rahmenhöhe in 0,1 mm (hoch 2770, quer 1800…1900); Feld 32 = 7 hoch / 6 quer.
  Quer: Rahmen oben = 205 − Höhe − 5 mm (Altbau EG: 1900 → 10 mm, wie im Original-Ausdruck).
- VSY Feld 23 = Zusatzoptionen (Bitmaske) des Symbols; Symbolelemente mit Feld 0x48 = n sind nur sichtbar, wenn Bit n−1 gesetzt ist.
  Namen in GRD › OPT (Feld 1 = Nummer, STR = Name), z. B. Steckdose: 1 = "Ports" (hochgestellte Anzahl), 2 = "SPD".
- OBJ (Verweis): Feld 6/7 Pfeilspitze, 2/3 Versatz zur **Textmitte** (0,1 mm), 14 Richtung in 0,1° (0 ►, 900 ▲, 1800 ◄, 2700 ▼),
  15 Bit 0 hochkant. Feld 4 = 0x0101FFFF (Querverweis auf andere Anlage): Richtung um 270° versetzt, bei 180° hochkant
  (nur zwei Beispiele: "=S1+W2", "=UV3+W2" – geschätzt).
- Werkzeugleisten-Bilder: eigene Vektor-Icons (resources/icons, erzeugt mit tools/icons_erzeugen.py); Reihenfolge der Knöpfe wie in TFORM1
  (z. B. 0 Auswahl, 14 Neues Projekt, 15 Laden, 38 Gitter, 42/43 Gruppieren/Auflösen).
- Menü Datei des Originals: … Schriftfeld speichern | Export › PDF (Beta), Bitmapexport, DXF Import | Beenden (Menü-IDs über Win32 ausgelesen).
- Startparameter: Das Original öffnet keine Datei aus der Kommandozeile (immer "Neues Projekt").

## Nachtrag 18.11.2026 (2) – Eigenschaften, Kontextmenüs, Dekompilierung
Werkzeuge: Cutter 2.5 bringt rz-ghidra (Ghidra-Dekompiler, ohne Java) mit; damit wurde z. B. der Schrift-Handler
des Textdialogs (0x4a344c) und die Ausrichtungsroutine (0x4567e8) dekompiliert. Die Dialoge folgen den 47 Formularen des
Originals. Feldbelegungen wurden zusätzlich durch Änderungen im Original-Programm
(speichern, Datei vergleichen) bestätigt – siehe `tests/original_edits.rs`.
- Text (ZCN Typ 11): 0–3 Textbox (Links, Oben, Rechts, Unten in 0,1 mm), 4 Schriftgrad (pt), 5 Drehung (Grad),
  6 Byte 0 Ausrichtung (0 Links, 1 Mittig, 2 Rechts) / Bit 16 Rahmen / Bit 24 Fett, 7 Byte 0 Kursiv / Byte 1 Unterstrichen,
  8 Schriftfarbe (TColor), 9 Zeiger auf eigenen Schriftnamen (zur Laufzeit), 10/11 Speicherzeiger ohne Bedeutung,
  15 Byte 0 Typ / Bit 16 Fixiert, 16 Stift.
- Symbol (VSY): 10/11 Titel X/Y, 12 Titelausrichtung (Hi-Word) + Bit 24 hochkant, 13/14 Wert X/Y, 15 Wertausrichtung,
  16/17 Lage, 18 Bit 0–2 Drehung / Bit 8 Spiegeln "Vertikal" / Bit 16 Spiegeln "Horizontal", 19 Skalierung (f32),
  20 Freigaben (Bit 0 = keine Querverweise, Bit 24 = Sicherungsübersicht), 21 Alternativtext-Satz, 22 Bild, 23 Optionen.
  Unterblöcke: TIT (Titel), WRT (Wert), FKT (Funktion), SPA (Rackposition), ALT (benutzerdefinierter Alternativtext), OWR (Eigentümer).
- Projektsymbol (SYM): Name 0–40, BMK-Vorlage 40–60, Kennung 80–88 = Erstellungszeitpunkt als Delphi-Datum (f64);
  Byte 112 (u32) Freigaben: Bit 0 Querverweise, Bit 1 Sicherungsübersicht (W2: RCD/LS-Schalter 3, Klemme 0, sonst 1;
  ältere Symbole tragen dort teils Zufallsbytes, Werte > 3 gelten als unbekannt);
  GRD › GRS (Elemente), STR (Texte, Nr. 1 = BMK), SEI (Bilder), OPT (Optionen: Feld 1 = Bit, STR = Name),
  ALT (Alternativtexte: Feld 0 = Textnummer, Feld 1 = Satz, STR = Ersatztext, fehlt = leer).
- Element (0x50 Byte): Typ 0x3c, Stiftart 0x40, Bild/Modus 0x44/0x46, Option 0x48 (0 = immer, n = Bit n−1), 0x4c = !Option.
- Blatt (BLT): Feld 23 Byte 0 Subindex, ab Bit 8 Blattnummer.
- Kontextmenü (PopupMenu1) nach Objektart, aus Bildschirmfotos des Originals: Symbol: Löschen | Optionen ▸ (Häkchen je Option),
  Alternativtext ▸ (Standard, Sätze als 'a';'b';'c', Benutzerdefiniert), Eigentümer ▸, Kontaktspiegel hinzufügen, Bauteilliste ▸ |
  Symbol bearbeiten | Eigenschaften. Text: Löschen | Ausrichtung ▸, Eigenschaften, Fixieren ▸. Fläche: Objekt einfügen ▸
  (Projektsymbole), Verweis einfügen ▸ | Eigenschaften, Pfadbezeichnung.
- Tools › Ausrichtung: Werkzeugfenster (TAusrichtung) mit Referenzpunkt und vier Knöpfen (eigene Vektor-Icons).

## Nachtrag 20.11.2026 – vollständiger Import, Symboltexte (geprüft gegen "Elektroplanung Wohnung 2.pdf")
- OBJ Feld 4 unterscheidet: 0x1FFFF Verweis (ohne TIT = nur Pfeil, z. B. im Schriftfeld; Feld 14 = 900 → ▼,
  1800 → ►), 0x0101FFFF Querverweis auf andere Anlage, **0xFFFF Sammelschiene**, **0x2FFFF Klemmstelle**.
- Sammelschiene (OBJ 0xFFFF): Feld 6/7 Anfang, 10 Ende x, 2/3 Lage des Namens (linke obere Ecke) relativ zum
  Anfang, 8/9 Kennung der SMS (SMS Feld 8/9, Name in SMS › TIT), 15 Byte 0 = 1 strichpunktiert (PE).
  Querverweise "Blatt/Pfad" werden berechnet: am Anfang das vorige, am Ende das nächste Vorkommen derselben
  Schiene in der Anlage (Seite 6: N → "7/1"; Seite 18: -F1.L2 ← "4/10").
- Klemmstelle (OBJ 0x2FFFF): Feld 6/7 Lage, 8 Klemmleiste (= KLM Feld 13), 9/10 Nummer, 11 Byte 0 Etage
  (A, B, C … wenn KLM Feld 15 > 1), 2/3 Textversatz (rechte obere Ecke), TIT = Vorlage "%O%B.%C" → "-X1.8B".
- KLM: Name 0–20, TIT, Feld 13 Kennung, 15 Etagen; KFO Feld 4/5 = Klemmennummer.
- VBG › BZD: Verbindungsbezeichnung (Aderfarbe "sw"/"br"/"gr"), Feld 3/4 Punkt auf der Leitung, 0/1 Versatz.
  VBG Feld 13 = Kennung der Leitung (LTG Feld 9). LTG: Feld 5 Breite (100 = Standard), 6 Farbe (TColor),
  9 Kennung, 10 Strichart (PE = 2 strichpunktiert).
- ZCN Typ 13 = Tabelle: Feld 0–3 Rechteck, 4 Spalten, 5 Zeilen, STR = Zellen zeilenweise (0-getrennt).
- BLT: Byte 51 ff. Ort des Blattes ("W2", ersetzt %P), Feld 22 Byte 0 Blattart (3 Stromlauf, 4 Klemmplan,
  5 Sicherungsübersicht), Feld 32 ab Bit 8 Klemmleiste des Klemmplans.
- VSY: Feld 38 Zähler für %C – die BMK steht als Vorlage im TIT ("-X1.%C+%P", "-F%C") und wird beim Anzeigen
  aufgelöst. SPA = Parameterwert (Feld 0 Nummer, STR Wert) zu GRD › PRA (Feld 0 Nummer, STR Name, STR Vorgabe);
  Symboltexte `%<Typ>` zeigen den Wert ("LS B10"). `%("NO/NC";11;13)` wählt den Text nach dem Bild.
  VSY › ALT = benutzerdefinierter Text der Instanz (Feld 0 Textnummer, STR Text), z. B. "PE" statt "N".
- Symbolelement Typ 6 (Text): Feld 8 Ausrichtung (0 waagerecht, 2 dreht mit dem Symbol – Seite 1 "Motorsymbole"),
  Feld 9 Zeilen ("M"/"~" im Motor; Schrift je Zeile = 0,72 × Höhe / Zeilen, breite Texte auf die Box verkleinert).
  Text 1 ist nur dann die BMK, wenn er eine Vorlage mit % ist ("Verbindung": Text 1 = "N").
  Typ 10 = Wertanker; Feld 10 ≠ 0 bei Viereck/Dreieck/Ellipse = gefüllt (Verbindungspunkt).
- Hochkant-BMK (VSY Feld 12 Bit 24): Text liest von unten nach oben und beginnt bei "Links" am Titelanker
  (Seite 1: "-M2.1" von y = 147,6 bis 156,5 mm bei Anker 157,5 mm); Mittig/Rechts verschieben nach unten.
- PRJ › BLD: Name 0–20, Feld 5 Nummer, JPG = JPEG-Daten (über 64 KB in Teilen).

## Nachtrag 21.11.2026 – Verweisziele, Doppelklick, Drehen, PDF-Verknüpfungen
- OBJ (Verweis) Feld 0/1 = eigene Kennung, 8/9 Kennung der Zielanlage (ANL Byte 0x88), 10/11 Zielblatt (BLT Byte 0x48),
  12/13 Gegenverweis (dessen Feld 0/1), Feld 15 Byte 1 = Ziel gesetzt. Belegt durch W2.epr ("Raum 10" Neubau EG ↔
  "Raum 7" Neubeu DG zeigen gegenseitig aufeinander) und die Routine 0x48056c ("Der Zielverweis ist schon anderwertig
  belegt …"), die genau diese Felder vergleicht, den alten Partner löst und den Gegenverweis zurückverknüpft.
- Anklicken: Das Original springt per **Doppelklick** zum Ziel – Option (TForm4) "Doppelklick auf Verweise =" Ziel
  anzeigen / Eigenschaften. Der Nachbau springt ebenso bei Doppelklick auf die Querverweistexte der Sammelschienen
  ("Blatt/Pfad", vorige/nächste Stelle der Schiene) – dafür gibt es im Original keinen Beleg.
- Querverweis (Feld 4 = 0x0101FFFF): hochkant = Bit 0 XOR (Feld 14 = 1800); ohne Text gilt Bit 0 unverändert.
  Drehen (R) um die Pfeilspitze: Richtung −90°, der Text bleibt an seiner Stelle (siehe Nachtrag (3)).
- PDF: Verweise und Sammelschienen-Querverweise sind Verknüpfungen (/Link, /Dest auf die Zielseite).

## Nachtrag 21.11.2026 – Klemmstellen, Klemmleisten, Klemmplan
- OBJ Byte 0x14 = Objektart: 0 Klemmstelle, 1 Sammelschiene, 2 Verweis. Klemmstelle: Byte 0x12 Ausrichtung des
  Titels (0/1/2, daher Feld 4 = 0x2FFFF bei „Rechts“), Feld 9 Nummer, 10 Position, Byte 0x2C Ebene, Byte 0x2D
  Bit 0 Automatisch / Bit 1 Korrektur Nummer / Bit 2 Korrektur Position / Bit 3 Ziel Wechsel, Byte 0x2E
  Gruppenautomatik; Unterblock FKT = Funktion (vor TIT). Neue Klemme im Original: 0x2D = 1, 0x2E = 1, 0x2C = 1.
- KLM: Byte 0–43 Name, Feld 10/11 Titelposition, 13 Kennung, 15 Ebenen, 18/19 nächste Nummer/Position;
  KFO = Kennung der Klemmstelle, Kennung ihres Blattes (BLT @72), Nummer, Position. Der Klemmplan geht die KFO durch.
- Gruppenautomatik: nächste Klemmstelle links auf gleicher Höhe < 2 cm und gleiche Leiste → Titel ab %C ("2B").
- Klemmplan-Richtungen (Routine 0x4830a8): Klemmenende → Nachbarpunkt der Verbindung: 0 oben, 1 links, 2 rechts,
  3 unten (Ziel Wechsel: 3 − Richtung); erste Verbindung je Richtung; Von = erste Richtung mit Ziel, Nach = unten
  falls Von oben, sonst die nächste. Klemmen derselben Leiste sind nur über „Verbindung ist Brücke“ (VBG Feld 14
  Bit 0x100) Brücken, sonst Ziele ("-X2 2"); Ziel auf Klemmen ohne Etagenbuchstaben. VSY › OWR Byte 0–7 =
  Kennung des Eigentümers (VSY Byte 96) – der Klemmplan zeigt dessen BMK ("-K1").
- BLT Feld 32 ab Bit 8 = Klemmleiste des Klemmplans; Zusätze der Blattarten (Texte der Exe): Deckblatt mit/ohne
  Rahmen, Querverweise 1–3 Tabellen, Klemmenplan Klemmleiste, Artikelliste Summenliste/Summe+BMK/Einzelliste.
- Offen: epr_neu deutet Byte 0x12 als Objektart (0/1/2), der Klemmen-Dialog (0x4543xx, Maske 0x10) vergleicht
  bei Klemmstellen Byte 0x12 mit dem Index der Ausrichtung; in W2 stehen nur Klemmstellen mit 2 („Rechts“).
  Erkannt werden Klemmstellen über Byte 0x14 = 0.
- Listen Export (TListExport): Stückliste, Betriebsmittelkennzeichnung, Klemmenbezeichnung; Tabulator, ';', '#'.

## Nachtrag 21.11.2026 (2) – neue Blattobjekte im .epr (aus der Speicherroutine 0x424b8f ff. und dem Eigenschaftendialog)
- Speichern je Blattelement: ZCN (immer 0x50 Byte), dann Typ 11: STL (Text als TStringList, Zeilen mit CR LF) und –
  falls ein eigener Schriftname gesetzt ist – **STR = Schriftname** (Feld 9 = Zeiger darauf zur Laufzeit);
  Typ 13: STR (Zellen, je 0-terminiert); Typ 6: STR (Text). **Typ 12 = Bild** ohne Unterblöcke. Jeder Unterblock mit END.
- Werkzeuge (Knopf-Tag = Elementtyp): Textbox = 11 (2 Klicks), Bild = 12, Tabelle = 13; im Symboleditor wird aus
  Klemmstelle 7 und Textbox 8 (Anschluss, Text).
- Text (Typ 11): Feld 6 Byte 1 = Umbruch (Dialog TTextbox: Rahmen +0x1a, Umbruch +0x19, Ausrichtung +0x18, Drehung +0x14).
- Bild (Typ 12): Feld 0–3 Rechteck, Feld 4 = Nummer des Projektbildes (BLD Feld 5), Feld 5 Byte 0 Rahmen,
  Byte 1 Ausrichtung (0 Zentriert, 1 Eingepasst, 2 Gestreckt), Feld 15 Bit 16 Fixiert.
- Tabelle (Typ 13): Feld 6 „Ab“, Feld 9 Byte 0 Überschrift, Byte 1 Autobreite, Byte 2 Art (0 Benutzerdaten,
  1 Inhaltsverzeichnis, 2 Querverweise); berechnete Tabellen zeichnet der Nachbau stets mit dem aktuellen Inhalt.
- Kontaktspiegel (OBJ, Byte 0x14 = 5): Feld 6 X in 0,1 mm, Feld 8/9 Kennung der Spule (VSY Byte 96), Byte 0x2c
  Bit 0 = Zeilen vorhanden; je Zeile STB (0x1c Byte: Kennung Kontakt, Blatt (BLT Byte 72), Anlage (ANL Byte 0x88),
  Byte 0x18 Art 0 Schließer/1 Öffner/2 Wechsler) mit ALT (Feld 0 = Nummer 1–3) + STR je Anschlussnummer.
  Y = Rahmenhöhe des Blattes, Zeilen 3 mm (Wechsler +2 mm davor, +0,5 mm danach). Details in `src/kontaktspiegel.rs`.
- BLT Feld 32 Byte 1 (0x81) = Zusatz: Deckblatt 0 mit/1 ohne Rahmen, Querverweise 1–3 Tabellen, Artikelliste
  0 Summenliste/1 Summe+BMK/2 Einzelliste (Klemmplan: ab Bit 8 die Leiste).
- OBJ (0x40 Byte): Feld 4 Wort 0 = 0xFFFF, Byte 2 Art (0 Sammelschiene, 1 Verweis, 2 Klemmstelle), Byte 3 = 1 Ziel in
  anderer Anlage; Feld 5 Byte 0 (1 Sammelschiene, 2 Verweis); Verweis-Vorgabe Textversatz (0, −6 mm). Unterblock TIT = Text.
- VBG: Feld 2/3 und 6/7 = Kennungen der verbundenen Objekte, 10/11 eigene Kennung (fortlaufende Zeitstempel), 13 Leitung.
  BZD (44 Byte): 0/1 Versatz, 3/4 und 5/6 Punkt, 9/10 meist 80/2 (Bedeutung offen).
- Neue Kennungen (OBJ, VBG, SMS, ANL Byte 0x88) vergibt der Nachbau als Delphi-Zeitstempel im Millisekundenabstand.

## Nachtrag 21.11.2026 (3) – Vormarkierung, Zeiger, Textmaße, Leitungen an Verweisen
- VBG Feld 2/3 (Anfang) und 6/7 (Ende) = Kennung des verbundenen Objekts: VSY Byte 96 bei Symbolen, OBJ Feld 0/1
  bei Klemmstellen und Sammelschienen; Feld 4 bzw. 8 = Anschlussnummer (Klemmstelle 0), −1 = Ende auf einer anderen
  Leitung (W2: 364 Verbindungen an OBJ-Blöcken). In W2 hängt keine Leitung an einem Verweis; der Nachbau schreibt
  für Enden an der Pfeilspitze eines Verweises dessen OBJ-Kennung und Anschluss 0 (`epr_neu::verweis_enden`).
- Verweis drehen: nur die Richtung (Feld 14) ändert sich, der Text bleibt an seiner Stelle (Feld 2/3 relativ zur
  Pfeilspitze, die beim Drehen stehen bleibt).
- Symbol vergrößern (VSY Feld 19): Titel (10/11) und Wert (13/14) bleiben auf dem Blatt stehen, ihr Versatz zur
  Lage wird angepasst. Text vergrößern: Schriftgrad (ZCN Feld 4) und Textbox (Feld 0–3) wachsen gemeinsam.
- Textmaße (`textmass.rs`): egui 0.37 setzt die Schriftgröße = Geviert, Zeilenhöhe = (Ober- + Unterlänge +
  Zeilenabstand) ≈ 1,15 Geviert bei Times New Roman; Breiten aus der Times-Metrik. Umriss = Textbox ∪ Schrift.
- Vormarkierung (`hover.rs`): gestricheltes Rechteck um das Objekt unter der Maus, Zeiger Pfeil + Verschiebekreuz
  bzw. Pfeil + „A“ (Bildschirmfotos des Originals); Reihenfolge wie beim Drücken (Griffe, Ecken, Titel/Wert,
  Anschluss, Sammelschienen-Querverweis, oberstes Objekt).

## Weitere Projektangaben im .epr (src/epr_zusatz.rs, aus Speicher- und Laderoutine; Gegenprobe im Original offen)

- BTL (Bauteil, 232 Byte, ohne Unterblöcke, im PRJ vor ERS): 0 Kennung (Zeitstempel), 8 Bauteilgruppe und 0x10 Hersteller
  (Kennungen aus Gruppen.tfi Art 1 bzw. 2, 0 = „Keine Zuordnung“), Feld 8/9/10 Breite/Höhe/Tiefe in mm (Vorgabe
  500/800/700), Texte je 40 Byte: 0x2C Name, 0x54 Typ, 0x7C Artikel-Nr., 0xA4 interne Nr.; 0xD0 Symbol, 0xD8 Bild,
  0xE0 Erstellungszeit. Zuordnung: VSY › IDL bzw. OBJ (Klemmstelle) › IDL vor TIT, 8 Byte = Kennung des Bauteils.
- ERS (Platzhaltergruppe, 12 Byte Name) › TLI (8 Byte Suchtext) › STR (Ersetzungstext, fehlt bei leerem Text).
- PRJ Byte 0x90 = 1 Projekt abgeschlossen; Byte 0x70 Bit 0 = 0 „Alter Klemmenplan“. FUS Feld 1 = 2 Maßangaben
  relativ (GRS-Koordinaten 0..10000), 0 absolut (0,1 mm: x · 2870/10000, y · Höhe/10000, gerundet – so rechnet
  das Original beim Umschalten um, geprüft an einer vom Original umgeschalteten Datei).
- SMS Byte 0x30 Querverweise, 0x31 Klemmen, 0x33 Durchschleifung, 0x34 mit Pfeilen, 0x35 Autoende; Kennung bei 0x38
  ≠ 0 = Einspeisung (setzt das Original).
- VBG Feld 14 Bit 0x40 Zielführung links/oben, 0x80 rechts/unten, 0x100 Brücke; BZD Byte 0x0B Bit 0 Hochkant,
  Feld 7 Bit 0 Markierung, Bit 1 Unterstrichen, Bit 2 Durch Markierung.
- VSY Feld 37 = Rackposition (−1 = keine).
