# Versionen und Releases

Regeln, wann und wie eine neue Version entsteht. Sie gelten für Menschen und für KI-Helfer (Claude) gleichermaßen.

## Nummern (Semantic Versioning, vor 1.0)

- `0.MINOR.0` – neue sichtbare Funktionen oder geändertes Verhalten (z. B. neues Fenster, neue Option, andere
  Leitungsführung).
- `0.x.PATCH` – nur Fehlerbehebungen ohne neues Verhalten.
- `1.0.0` – erst, wenn die Abnahmekriterien für 1.0 erfüllt sind und der Projektinhaber zustimmt.

Die Nummer steht **nur** in `Cargo.toml`. Erreicht eine neue Nummer `main`, legt die Pipeline Tag `v<Version>` und
das Release an; der Text ist der Abschnitt `## [<Version>]` aus `CHANGELOG.md`. Tags nie von Hand setzen.

## Wann wird ein Release vorgeschlagen?

Ein Release wird **nie ohne ausdrückliche Zustimmung** des Projektinhabers erstellt. Vorgeschlagen wird es am Ende
einer Arbeitsrunde, wenn **alle** Punkte erfüllt sind:

1. Seit dem letzten Release gibt es mindestens eine neue sichtbare Funktion **oder** mindestens fünf behobene Fehler.
2. Alle Tests sind lokal grün (`cargo test --release --no-fail-fast`), die Pipeline auf `main` war beim letzten
   Push grün (Workflow „Bauen“: Linux und Windows; der eigene Workflow „macOS“ darf fehlen).
3. Kein bekannter, in dieser Runde neu entstandener Fehler, der Daten verliert, falsch speichert oder Netze falsch
   verbindet, ist offen.
4. `## [Unveröffentlicht]` im CHANGELOG beschreibt alle Änderungen für Nutzer verständlich.
5. Die Arbeit ist abgeschlossen (keine halbfertige Funktion auf `main`; Unfertiges bleibt auf einem Zweig oder hinter
   einer ausgeschalteten Option).

Ausnahme **Fehlerbehebung**: Ist im letzten Release ein Fehler, der Daten verliert, das Programm abstürzen lässt oder
Dateien für das Original unlesbar macht, wird nach der Behebung sofort ein PATCH-Release vorgeschlagen.

## Aufbau des Release-Textes (Nutzer 24.11.2026)

Der Text eines Releases ist sein Abschnitt in `CHANGELOG.md` (nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/)):

```markdown
## [0.6.0]

**Zusammenfassung:** 2–4 Sätze für Nutzer: das Wichtigste dieser Version in Alltagssprache (bei einer
Wartungsversion auch, was die letzte größere Version brachte).

### Neu
- neue Funktionen (für Nutzer sichtbar)

### Geändert
- geändertes Verhalten, Bedienung, Vorgaben, Pipeline

### Behoben
- Fehler, die vorher auftraten (was war falsch?)
```

Leere Abschnitte entfallen; wichtige Punkte (Datenfehler, neue Grundfunktionen) fett beginnen. Schon beim Sammeln
unter `## [Unveröffentlicht]` gleich in diese drei Abschnitte einsortieren.

## Ablauf

1. Prüfen: Punkte 1–5 oben; `git status` sauber.
2. `Cargo.toml`: Version erhöhen; `cargo build --release` (aktualisiert `Cargo.lock`).
3. `CHANGELOG.md`: Inhalt von `## [Unveröffentlicht]` unter `## [<Version>]` setzen, die **Zusammenfassung**
   davor (siehe „Aufbau des Release-Textes“); darüber einen neuen, leeren Abschnitt `## [Unveröffentlicht]` mit dem Hinweis auf den Entwicklungsstand.
4. Tests erneut laufen lassen, dann **ein** Commit „Release <Version>“ (Versionsnummer, Lock-Datei, CHANGELOG).
5. Push nach Zustimmung; danach Pipeline und Release prüfen (Assets für Linux und Windows sofort, die macOS-.dmg wird nachgereicht, sobald GitHub einen Mac frei hat,
   Text = CHANGELOG-Abschnitt).
6. Im privaten Stand festhalten, welche Version veröffentlicht ist.
