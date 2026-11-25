# Kurzanleitung

Elektroplanung zeichnet Stromlauf- und Installationspläne und liest und schreibt das Dateiformat `.epr` der
Planungssoftware Elektroplanung 6.4.2. Diese Anleitung beschreibt die Bedienung in wenigen Schritten; was bewusst
anders ist als im Original und was noch fehlt, steht im [README](../README.md) unter „Bekannte Grenzen“.

## Projekt öffnen und ansehen

- **Datei › Projekt öffnen** (Strg+O) öffnet `.epr`-Dateien des Originals und das eigene Format `.epj`.
- Blätter wechseln: Reiter unter der Zeichenfläche, Strg+Bild↓ / Strg+Bild↑ oder Projektbaum rechts.
- **Mausrad** vergrößert und verkleinert an der Maus (Optionen › Maus „Mausrad umkehren“ = wie im Original),
  **mittlere Maustaste** gedrückt verschiebt die Ansicht, Strg+Num0 zeigt das ganze Blatt.
- G schaltet das Gitter, der Rasterfang steht in der Werkzeugleiste (z. B. 2,5 mm).

## Symbole setzen

- Rechts den Reiter **Symbole** wählen und ein Symbol auf das Blatt ziehen. Beim Ablegen auf einer Leitung wird das
  Symbol in die Leitung eingebunden; Anschlüsse, die auf eine Leitung oder Schiene zeigen, werden automatisch
  verbunden (A schaltet das ab).
- R dreht, Umschalt+V / Umschalt+H spiegeln, Doppelklick oder Alt+Enter öffnet die Eigenschaften (BMK, Wert,
  Bildvariante, Zusatzoptionen). E öffnet das Symbol im Symboleditor.
- S schaltet die Strompfadausrichtung: Symbole rücken beim Verschieben auf die Mitte des Strompfades.

## Leitungen zeichnen

- Mit gedrückter linker Maustaste **an einem Anschluss** beginnen und ziehen. Die Leitung folgt der Maus mit einem
  Knick; eine Kehre der Maus setzt einen festen Knick, **Leertaste** oder **/** kippt die Knickrichtung.
- Ein Klick auf einen Anschluss ohne Ziehen startet den **Klickbetrieb** wie im Original: jeder Klick setzt einen
  Eckpunkt, Klick auf einen Anschluss, eine Leitung oder Schiene beendet die Leitung.
- An einer Sammelschiene beginnen: auf die Schiene drücken und ziehen. **Umschalt** auf einer Leitung beginnt einen
  Abzweig. Der Zielanschluss ist farbig: grün frei, orange belegt, rot der eigene Anfang.
- Kurze Stücke direkt an einem Anschluss greift **Strg+Klick**.

## Markieren, verschieben, löschen

- Klick markiert ein Objekt, **Strg+Klick** nimmt weitere dazu oder heraus, ein Rahmen markiert alles darin
  (Optionen › Programmverhalten „Markierrichtung wie CAD“: von rechts nach links alles Berührte).
- Auf Leitungen (Option, Standard an): **Doppelklick** markiert die Leitung bis zum nächsten Knoten,
  **Dreifachklick** das ganze Netz ohne die Sammelschienen.
- Verschieben mit der Maus oder den Pfeiltasten: angeschlossene Leitungen folgen. Der **Netz-Wächter** lässt keine
  Bearbeitung zu, die fremde Netze verbinden, trennen oder übereinanderlegen würde – die Statuszeile nennt den Grund.
- **Entf** löscht die Markierung; ein Symbol nimmt wie im Original die Leitungen mit, die an seinen Anschlüssen
  beginnen oder enden (Optionen › Programmverhalten, abschaltbar). Strg+Z nimmt zurück, Strg+W bzw. Strg+Y
  wiederholt.

## Schienen, Klemmen, Verweise

- Werkzeugknopf **Neue Sammelschiene** (zweite Leiste): auf einem Schaltplan entsteht sofort eine Schiene über die
  Blattbreite, gestapelt unter den vorhandenen. Hinzufügen › Sammelschiene legt nur die Schiene im Projekt an.
- Werkzeug **Klemmstelle** (X): Klick auf eine Leitung setzt die Klemme hinein und teilt die Leitung; die Nummer
  zählt in der gewählten Klemmleiste weiter.
- Verweise (Kontextmenü der Fläche › Verweis Start/Ziel) verbinden Blätter; Doppelklick springt zum Ziel.

## Prüfen, speichern, ausgeben

- **Tools › Projekt prüfen** meldet offene Anschlüsse, doppelt vergebene Kennzeichnungen und gleich benannte
  Potentiale (PE, N, Schienenname), die auf einem Blatt nicht verbunden sind.
- **Speichern** (Strg+S) schreibt wieder `.epr`: unveränderte Teile bleiben Byte für Byte erhalten, geänderte
  Objekte werden in ihre Originalblöcke geschrieben. Was das Originalformat nicht aufnehmen kann, nennt das Programm
  beim Speichern.
- PDF-Export, Drucken und Bildexport (BMP, JPG, PNG) unter **Datei**.
- **F12** speichert einen Fehlerbericht (Projektkopie, letzte Eingaben, Bild des Fensters), Hilfe › Fehler melden
  öffnet ein vorausgefülltes Issue.

## Tastenkürzel (Vorgabe, änderbar unter Optionen › Tastenkürzel)

| Taste | Befehl | Taste | Befehl |
|---|---|---|---|
| Strg+N / O / S | Neu, Öffnen, Speichern | Strg+Umschalt+S | Speichern als |
| Strg+Z / W / Y | Rückgängig, Wiederholen | Entf | Löschen |
| Strg+C / X / V | Kopieren, Ausschneiden, Einfügen | Strg+A | Alles markieren |
| Alt+Enter | Eigenschaften | Strg+F | Suchen (alle Blätter) |
| R | Drehen | Umschalt+V / H | Spiegeln |
| Strg+G / Strg+Umschalt+G | Gruppieren, auflösen | E | Symbol bearbeiten |
| S | Strompfadausrichtung | A | Automatische Verbindungen |
| G | Gitter | Esc | Werkzeug Auswahl |
| L / Umschalt+R / K / T / X | Linie, Rechteck, Kreis, Text, Klemmstelle | Strg+Umschalt+N | Neues Blatt |
| Strg+Plus / Minus / Num0 | Vergrößern, Verkleinern, ganzes Blatt | F12 | Fehlerbericht |

## Steuerung über KI (MCP)

`Elektroplanung --mcp` startet statt des Fensters einen MCP-Server, über den ein KI-Werkzeug Projekte öffnen,
Blätter lesen, Symbole, Leitungen, Schienen, Klemmen, Verweise, Tabellen und Bauteile anlegen und im Originalformat
speichern kann (Werkzeugliste im [README](../README.md#mcp-schnittstelle)).
