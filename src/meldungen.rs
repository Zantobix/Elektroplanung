//! Laufzeittexte des Originals: Meldungen, Rückfragen und Statuszeile (Wortlaut der Original-Exe, nur offensichtliche
//! Tippfehler berichtigt). Die Texte stehen gesammelt hier; die Auslöser rufen die Helfer an der Stelle auf, an der das
//! Original sie zeigt.
//!
//! - Meldungen und Rückfragen zeigt das Original als Windows-MessageBox mit eigener Überschrift: hier ebenso über
//!   `rfd::MessageDialog` (modal). Ja/Nein wie im Original; Nein bzw. Abbrechen bricht die Aktion ab.
//! - Statuszeile (TStatusBar, Feld 1): beim Ziehen „DeltaX: … DeltaY: …“, beim Überfahren der Name des Objekts unter
//!   der Maus, beim Verlassen „Kein Objekt“ (PaintBox1MouseMove 0x40c0c0; Text wechselt nur, wenn ein anderes Objekt
//!   unter die Maus kommt).
//! - Fehler beim Laden (0x426310, 0x498308) und Speichern (0x4228c4) mit denselben Texten und Überschriften.
//!
//! Für Tests lassen sich die Dialoge abschalten (`test_antworten`): Dann werden die Meldungen nur gesammelt und die
//! Rückfragen mit den vorgegebenen Antworten beantwortet.

use crate::hover::Ziel;
use crate::model::*;
use crate::symbols::SymDef;
use std::cell::RefCell;
use std::collections::VecDeque;

// ------------------------------------------------------------------ Überschriften der MessageBoxen

pub const T_ACHTUNG: &str = "Achtung";
pub const T_ACHTUNG_X: &str = "Achtung!";
pub const T_ACHTUNG_FEHLER: &str = "Achtung Fehler!";
pub const T_FEHLER_X: &str = "Fehler!";
pub const T_HINWEIS: &str = "Hinweis!";
pub const T_SICHER: &str = "Sicher";
pub const T_SICHER_F: &str = "Sicher?";
pub const T_GEHT_NICHT: &str = "Geht nicht";
pub const T_SOLL_ICH: &str = "Soll ich?";
pub const T_ANPASSEN: &str = "Anpassen?";
pub const T_GEAENDERT: &str = "Achtung, geänderte Daten!";
pub const T_FRAGE_SYMBOL: &str = "Hallo Frage!";
pub const T_DOPPELT: &str = "Achtung doppelter Name";
pub const T_OEFFNEN: &str = "Fehler beim öffnen des Projekts";
pub const T_TEXTGENERATOR: &str = "Kein Objekt";

// ------------------------------------------------------------------ Statuszeile (PaintBox1MouseMove)

pub const ST_DELTA_X: &str = "DeltaX: ";
pub const ST_DELTA_Y: &str = "  DeltaY: ";
pub const ST_SYMBOL: &str = "Symbol: ";
pub const ST_BAUTEIL: &str = " Bauteil:";
pub const ST_WERT: &str = "Wert";
pub const ST_TITEL_EIGENTUEMER: &str = "Titel(Eigentümer)";
pub const ST_TITEL: &str = "Titel";
pub const ST_KLEMME: &str = "Klemme ";
pub const ST_POSITION: &str = " Position:";
pub const ST_SAMMELSCHIENE: &str = "Sammelschiene";
pub const ST_VERWEIS: &str = "Verweis ";
pub const ST_ZIEL: &str = "Ziel";
pub const ST_START: &str = "Start";
pub const ST_KONTAKTSPIEGEL: &str = "Kontaktspiegel";
pub const ST_VERBINDUNGSFEHLER: &str = "Verbindungsfehler";
pub const ST_MANUELL: &str = "manuelle Verbindung mit ";
pub const ST_AUTOMATISCH: &str = "automatische Verbindung mit ";
pub const ST_EINSPEISUNG: &str = " (Einspeisung)";
pub const ST_KEIN_OBJEKT: &str = "Kein Objekt";

// ------------------------------------------------------------------ Projektexplorer, Bibliothek (Lschen1Click, TBiblio)

pub const WIRKLICH_LOESCHEN: &str = "Wirklich Löschen?\n Dies ist NICHT rückgängig zu machen!";
pub const SYMBOL_IN_BEARBEITUNG: &str = "Das Symbol wird gerade bearbeitet und kann deshalb nicht gelöscht werden";
pub const SCHRIFTFELD_IN_BEARBEITUNG: &str = "Das Schriftfeld wird gerade bearbeitet und kann deshalb nicht geändert werden";
pub const GRUPPE_DOPPELT: &str = "Gruppenname schon vorhanden,\nhinzufügen nicht möglich!";
pub const SYNC_FERTIG: &str = "Synchronisierung abgeschlossen.\nEs wurden:\n";
pub const SYNC_NEUE_VERSION: &str = " neue Symbolversion";
pub const SYNC_HINZUGEFUEGT: &str = "der Symbolbibliothek hinzugefügt!";
pub const SYNC_KEINE_BIBLIOTHEK: &str = "Sie haben keine Vergleichsbibliothek angegeben!";
pub const SYNC_DATENFORMAT: &str = "Fehler im Datenformat der angegebenen Datei!";

// ------------------------------------------------------------------ Bearbeiten, Tools

pub const TITEL_VOM_EIGENTUEMER: &str = "Der Titel wird durch den Eigentümer vorgegeben.\nBitte dessen Titel bearbeiten!";
pub const MARKIERUNG_FRAGE: &str = "Soll das neue Symbol für die markierten Objekte eingesetzt werden?";
pub const MARKIERUNG_LEER: &str = "Es sind keine Objekte markiert die in ein Symbol umgewandelt werden können!";
pub const TEXTGENERATOR_LEER: &str = "Es sind keine Objekte zur Bearbeitung\nmit dem Textgenerator ausgewählt!!";
pub const KEINE_ALTERNATIVTEXTE: &str = "Keine Alternativtexte freigegeben";
pub const ZIELVERWEIS_BELEGT: &str = "Der Zielverweis ist schon anderwertig belegt!\nSoll diese Verbindung wirklich gelöst werden und dafür eine Verbindung zu diesem Verweis eingetragen werden?";
pub const KEINE_SAMMELSCHIENE: &str = "Keine Sammelschiene zum erzeugen ausgewählt!";
pub const DXF_FEHLER: &str = "Fehler beim importieren der DXF Datei";

// ------------------------------------------------------------------ Blätter, Anlagen, Nummerierung

pub const KEINE_ANLAGE_FUER_BLATT: &str = "Es ist keine Anlage für ein neues Blatt vorhanden!\nFügen Sie zuerst eine Anlage hinzu!";
pub const SUBINDEX_1: &str = "Der Subindex darf nicht ";
pub const SUBINDEX_2: &str = "größer als 255 sein!";
pub const NN_FRAGE: &str = "Durch die Neunummerierung wird die Reihenfolge der Blätter geändert!\nSind sie sicher?";
pub const NN_KEINE_ANLAGE: &str = "Es wurde keine Anlage angegeben!";
pub const NN_VON_BIS: &str = "Die Angabe Von ist größer als die Angabe Bis!";
pub const NN_AB_FEHLT: &str = "Es fehlt die Angabe mit welcher Seite die Neunummerierung begonnen werden soll!";
pub const NN_UEBERSCHNEIDUNG: &str = "Die Nummerierung kann nicht durchgeführt werden, da sich Seitenzahlen überschneiden!";
pub const INHALT_FRAGE_1: &str = "Zum anlegen eines vollständigen Inhaltsverzeichnisses";
pub const QUERVERWEIS_FRAGE_1: &str = "Zum Anzeigen einer kompletten Querverweisliste";
pub const FRAGE_SIND_NOCH: &str = " sind noch ";
pub const FRAGE_IST_NOCH_1: &str = " ist noch 1 Blatt";
pub const FB_INHALT: &str = "Es kann kein Folgeblatt für's Inhaltsverzeichnis erstellt werden da die dafür\nbenötigte Seite: ";
pub const FB_VERGEBEN: &str = " schon vergeben ist!\n";
pub const FB_NUMMER: &str = "(Blattnummer: 0     Subindex: ";
pub const SF_KEINE_DATEI: &str = "Keine Datei";
/// Sammelschienenassistent auf einem Blatt, das kein Schaltplan ist (0x4862d4, Überschrift „Achtung“)
pub const SCHIENEN_HIER_NICHT: &str = "Sammelschienen können hier nicht eingefügt werden";
/// Überschrift, wenn die Schriftfelddatei nicht zu öffnen ist (0x4732d4; Text = Systemmeldung)
pub const SF_DATEI_OEFFNEN: &str = "Fehler beim öffnen der Schriftfelddatei";
pub const FRAGE_NOETIG: &str = " nötig.\nSollen alle nötigen Blätter erzeugt werden?";

// ------------------------------------------------------------------ Drucken (Drucken1Click)

pub const DRUCK_KEIN_BLATT: &str = "Es ist kein Blatt geöffnet!";
pub const DRUCK_NICHTS: &str = "Es gab keine Blätter die den Vorgaben entsprochen hätten.\nEs wurde nichts gedruckt!";
pub const DRUCK_NIE_GEDRUCKT: &str = "Es wurden noch keine Blätter des Projektes gedruckt\noder das Projekt wurde nach dem Drucken NICHT abgespeichert!\nDiese Funktion ist nicht möglich!";

// ------------------------------------------------------------------ Symboleditor (fcn 0x47689c, 0x477cb0, 0x4795dc …)

pub const SE_SYMBOL_UEBERNEHMEN: &str = "Symbol übernehmen";
pub const SE_EINSTELLUNGEN: &str = "Einstellungen für das Symbol";
pub const SE_STIFT_SYMBOL: &str = "Stift für das Symbol ";
pub const SE_NEUER_KONTAKT: &str = "Neuer Kontakt";
pub const SE_SCHRIFTFELD_UEBERNEHMEN: &str = "Schriftfeld übernehmen";
pub const SE_BAUTEIL_UEBERNEHMEN: &str = "Bauteil übernehmen";
pub const SE_BIBLIOTHEKSSYMBOL: &str = "Bibliothekssymbol '";
pub const SE_PROJEKTSYMBOL: &str = "Projektsymbol '";
pub const SE_BEARBEITEN: &str = "' bearbeiten";
pub const SE_NEU: &str = "Neues Symbol' erstellen";
pub const SE_ERSTE_SEITE: &str = "Die erste Seite ist eine Standardseite\n und kann nicht gelöscht werden!";
pub const SE_HOEHE_FRAGE: &str = "Sollen die Zeichenobjekte an die geänderte Höhe angepasst werden?";
pub const SE_AENDERUNG_1: &str = "Diese Änderung wirkt sich auf alle Symbole des Typs:'";
pub const SE_AENDERUNG_2: &str = "' im gesamten Projekt aus.\nSoll statt dessen ein neues Symbol angelegt werden?";
pub const SE_VERLUST: &str = "Bei mindestens einem Symbol, Schriftfeld oder Bauteil, das gerade bearbeitet wird, gehen alle Änderungen verloren.\nMöchten Sie die Änderungen vorher noch übernehmen?";
pub const SE_PROJEKT_GEAENDERT: &str = "Das Projekt wurde geändert!\nÄnderungen Speichern?";

// ------------------------------------------------------------------ Ablegen aus der Bibliothek (PaintBox1DragDrop 0x440c1a)

pub const BS_BIBLIOTHEK: &str = "Das Symbol aus der Bibliothek ist ";
pub const BS_ALTER: &str = "älter ";
pub const BS_NEUER: &str = " neuer ";
pub const BS_ALS: &str = "als das Symbol im Projekt.\nSoll es trotzdem übernommen werden?\n(Dies wirkt sich auf das ganze Projekt aus)";

// ------------------------------------------------------------------ Kontaktspiegel (fcn 0x4b6674)

pub const KS_OHNE_EIGENTUEMER: &str = "Das Referenz-Symbol hat noch keinen Eigentümer!\nDer Eigentümer des Kontaktspiegels wird übernommen!";
pub const KS_ANDERER_EIGENTUEMER: &str = "Das Referenz-Symbol hat nicht den gleichen Eigentümer wie der Kontaktspiegel!\nDer Eigentümer wird neu gesetzt!";

// ------------------------------------------------------------------ Laden (fcn 0x426310, Objekte 0x498308)

pub const L_VERSION: &str = " (Version:";
pub const L_VOR_600: &str = "Diese Funktion unterstützt keine Projektdateien\n die mit einer Version vor 6.0.0 erzeugt wurden!";
pub const L_VERSION_FALSCH: &str = "Die Version der Projektdatei wird nicht von der Programmversion unterstützt!";
pub const L_VOR_500: &str = "Diese Pogrammversion unterstützt keine Projektdateien\n die mit einer Version vor 5.0.0 abgespeichert wurden!";
pub const L_FEHLER: &str = "Fehler beim laden der Projekt-Daten";
pub const O_PRUEFSUMME: &str = "Prüfsumme ist Fehlerhaft. Objekt: ";
pub const O_STRUKTUR: &str = "Datenstruktur ist Fehlerhaft. Objekt: ";
pub const O_GROESSE: &str = "Fehlerhafte Größe des Objekts: ";
pub const O_KEIN_ENDE: &str = "Strukturfehler (Kein Ende) für das Objekt: ";
pub const O_UNBEKANNT: &str = "Unbekanntes Objekt: ";
pub const K_PROJEKT: &str = "In den Projektdaten:";
pub const K_SCHRIFTFELD: &str = "Im Schriftfeld:";
pub const K_SYMBOL: &str = "In den Symboldaten ";
pub const K_VON: &str = "von \"";
pub const K_BLATT: &str = "  In den Blattdaten ";
pub const K_ANLAGE: &str = " In den Anlagendaten ";
pub const K_BAUTEIL: &str = "In den Bauteildaten ";
pub const K_POTENTIAL: &str = " In den Potentialdaten ";
pub const K_LEITUNG: &str = " In den Leitungsdaten ";
pub const K_PLATZHALTER: &str = " In der Platzhaltergruppe \"";

// ------------------------------------------------------------------ Speichern (fcn 0x4228c4)

pub const S_KEIN_NAME: &str = "Es wurde kein Dateiname angegeben";
pub const S_SCHREIBEN: &str = "Fehler beim schreiben in die Datei";
pub const S_KEIN_BACKUP: &str = "Es konnte keine Backup-Datei angelegt werden,\n das Projekt wurde NICHT gespeichert!";
pub const S_SCHREIBSCHUTZ: &str = "Die Datei kann nicht überschrieben werden!\n(Schreibschutz oder keine Rechte)";
pub const S_NICHT_ANGELEGT: &str = "Die Datei konnte nicht angelegt werden";
pub const S_DATENFEHLER: &str = "Fehler in den gespeicherten Daten";
pub const S_TEMP_1: &str = "Die Temporäre Datendatei:\n";
pub const S_TEMP_2: &str = "\nkann nicht in den gewollten Dateinamen geändert werden!";
pub const S_ALLGEMEIN: &str = "Fehler beim abspeichern der Projekt-Daten";

// ------------------------------------------------------------------ Laden: Statuszeile und Fehlerliste (0x426310, 0x498308)

pub const L_OEFFNEN: &str = "Projekt wird geöffnet";
pub const L_LESEN: &str = "Projekt wird gelesen";
pub const L_FEHLERHAFT: &str = "Fehlerhaftes Projekt";
pub const L_BILDDATEN: &str = "Fehler beim Laden der Bilddaten von: ";
pub const O_EBENEN: &str = "Anzahl Ebenen der Klemmleiste wird auf 4 begrenzt! Klemmleiste: ";

// ------------------------------------------------------------------ Eigenschaften mehrerer Objekte (0x4427d4)

pub const MP_UNTERSCHIEDLICH: &str = "Unterschiedliche Objekte";
pub const MP_GEMEINSAM: &str = "Gemeinsame Eigenschaften";

// ------------------------------------------------------------------ Tabellen der Symboleigenschaften (0x4795dc, 0x479064)

pub const TA_ORIGINAL: &str = "original Text";
pub const TA_ALTERNATIV: &str = "alternativ Text";
pub const TA_WAHL: &str = "Wahl ";
pub const TA_NEUE_WAHL: &str = "Neue Wahl";
/// erste Spalte: eigener Text im Original (0x479116), die weiteren „Wahl n“
pub const TA_WAHL_1: &str = "Wahl 1";
pub const TA_PARAMETER: &str = "Parameter";
pub const TA_WERT: &str = "Wert";
pub const SE_SEITEN: &str = "Definierte Seiten";
pub const SE_OPTIONEN: &str = "Definierte Optionen";
pub const SE_OPTIONSNAME: &str = "Optionsname";
pub const SE_STIFTE: &str = "Definierte Stifte";
pub const SE_PARAMETER: &str = "Definierte Parameter";
pub const SE_PARAMETERNAME: &str = "Parametername";
pub const SE_DEFAULTWERT: &str = "Defaultwert";
pub const SE_ZU_VIELE_TEXTE: &str = "Zu viele Alternativtexte freigegeben!\n Es werden nur 32 Alternativtexte zur Bearbeitung freigegeben!";
pub const SE_ZU_VIELE_OPTIONEN: &str = "Zu viele Optionen definiert!\n Es werden nur 32 Optionen zur Bearbeitung freigegeben!";
/// Höchstzahl der Alternativtexte bzw. Optionen, die der Dialog bearbeitet
pub const SE_HOECHSTENS: usize = 32;

// ------------------------------------------------------------------ Bibliothek: Gruppe hinzufügen (TBiblio.Button2Click)

pub const BG_NEUE_GRUPPE: &str = "Neue Gruppe";
pub const BG_HAUPTGRUPPE: &str = "Hauptgruppe: ";
pub const BG_NEUE_HAUPTGRUPPE: &str = "Neue Hauptgruppe";

// ------------------------------------------------------------------ Export (TFORM1.Export)

pub const EX_ANLEGEN: &str = "Die Datei konnte nicht angelegt werden!";
pub const EX_SCHREIBSCHUTZ: &str = "Die Datei ist Schreibgeschützt!";
pub const EX_PLATZ: &str = "Nicht genug Speicherplatz auf dem Datenträger";

// ------------------------------------------------------------------ Programmeinstellungen (TForm4, 0x4a07ac)

pub const OPT_OHNE_SCHRIFTFELD: &str = "Ohne Schriftfeld";
pub const OPT_VORGABESCHRIFTFELD: &str = "Vorgabeschriftfeld";
pub const OPT_SCHRIFTFELD_DATEI: &str = "Schriftfeld aus Datei";


// ------------------------------------------------------------------ Dialoge

/// Knöpfe einer Meldung (MessageBox-Arten des Originals).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Knoepfe {
    Ok,
    JaNein,
    JaNeinAbbrechen,
    OkAbbrechen,
}

/// Antwort auf eine Meldung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Antwort {
    Ja,
    Nein,
    Abbrechen,
    Ok,
}

/// Gezeigte Meldung (für Tests gesammelt): Überschrift, Text.
pub type Gezeigt = (String, String);

thread_local! {
    static TEST: RefCell<Option<(VecDeque<Antwort>, Vec<Gezeigt>)>> = const { RefCell::new(None) };
}

/// Testbetrieb: keine Fenster, Rückfragen bekommen der Reihe nach diese Antworten (danach „Ja“ bzw. „Ok“).
pub fn test_antworten(antworten: &[Antwort]) {
    TEST.with(|t| *t.borrow_mut() = Some((antworten.iter().copied().collect(), vec![])));
}

/// Im Testbetrieb gezeigte Meldungen (und zurücksetzen).
pub fn test_gezeigt() -> Vec<Gezeigt> {
    TEST.with(|t| t.borrow_mut().as_mut().map(|(_, g)| std::mem::take(g)).unwrap_or_default())
}

/// Meldung zeigen und Antwort abwarten (modal wie die MessageBox des Originals).
pub fn zeigen(titel: &str, text: &str, knoepfe: Knoepfe, warnung: bool) -> Antwort {
    let test = TEST.with(|t| {
        t.borrow_mut().as_mut().map(|(a, g)| {
            g.push((titel.to_string(), text.to_string()));
            a.pop_front()
        })
    });
    if let Some(a) = test {
        return a.unwrap_or(if knoepfe == Knoepfe::Ok { Antwort::Ok } else { Antwort::Ja });
    }
    let buttons = match knoepfe {
        Knoepfe::Ok => rfd::MessageButtons::Ok,
        Knoepfe::JaNein => rfd::MessageButtons::YesNo,
        Knoepfe::JaNeinAbbrechen => rfd::MessageButtons::YesNoCancel,
        Knoepfe::OkAbbrechen => rfd::MessageButtons::OkCancel,
    };
    let level = if warnung { rfd::MessageLevel::Warning } else { rfd::MessageLevel::Info };
    match rfd::MessageDialog::new().set_title(titel).set_description(text).set_buttons(buttons).set_level(level).show() {
        rfd::MessageDialogResult::Yes => Antwort::Ja,
        rfd::MessageDialogResult::No => Antwort::Nein,
        rfd::MessageDialogResult::Ok => Antwort::Ok,
        _ => Antwort::Abbrechen,
    }
}

/// Rückfrage Ja/Nein: true = Ja.
pub fn frage(titel: &str, text: &str) -> bool {
    zeigen(titel, text, Knoepfe::JaNein, false) == Antwort::Ja
}

/// Rückfrage OK/Abbrechen: true = OK.
pub fn frage_ok(titel: &str, text: &str) -> bool {
    zeigen(titel, text, Knoepfe::OkAbbrechen, false) == Antwort::Ok
}

/// Hinweis mit OK.
pub fn hinweis(titel: &str, text: &str) {
    zeigen(titel, text, Knoepfe::Ok, false);
}

/// Warnung (Ausrufezeichen) mit OK.
pub fn warnung(titel: &str, text: &str) {
    zeigen(titel, text, Knoepfe::Ok, true);
}

// ------------------------------------------------------------------ Statuszeile

/// Zahl wie FloatToStr des Originals (0,1 mm genau, Komma, ohne überflüssige Nachkommastellen).
pub fn zahl(v: f32) -> String {
    let z = (v as f64 * 10.0).round() / 10.0;
    let z = if z == 0.0 { 0.0 } else { z };
    format!("{z}").replace('.', ",")
}

/// „DeltaX: 12,5  DeltaY: -5“ beim Verschieben und Aufziehen.
pub fn delta_text(d: P) -> String {
    format!("{ST_DELTA_X}{}{ST_DELTA_Y}{}", zahl(d.x), zahl(d.y))
}

fn symdef<'a>(p: &'a Project, lib: &'a [SymDef], key: &str) -> Option<&'a SymDef> {
    p.symdefs.iter().find(|d| d.key == key).or_else(|| lib.iter().find(|d| d.key == key))
}

/// Langer Name einer Sammelschiene (SMS Name, z. B. „Schiene 4“) aus dem Originalblock der Anlage, sonst der Titel.
fn schienen_name(p: &Project, r: SheetRef, sms: u64, titel: &str) -> String {
    p.anlagen
        .get(r.anlage)
        .and_then(|a| a.raw.as_deref())
        .and_then(|(b, _)| b.kids_of("SMS").find(|s| s.data.get(32..40).map(|x| u64::from_le_bytes(x.try_into().unwrap())) == Some(sms)))
        .map(|s| crate::symbols::cstr(s.data.get(0..20).unwrap_or(&[])))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| titel.to_string())
}

fn bauteil(p: &Project, id: u64) -> String {
    match p.part_of.get(&id).and_then(|i| p.bauteile.get(*i)) {
        Some(b) => format!("{ST_BAUTEIL}{}", b.name),
        None => String::new(),
    }
}

/// Text der Statuszeile für das Objekt unter der Maus (Original: Feld 1 der Statuszeile, nur bei Wechsel des Objekts).
/// None = kein Objekt im Sinne des Originals (dann „Kein Objekt“), Some("") = Objekt ohne eigenen Text
/// (Zeichenobjekte, Anschlüsse).
pub fn objekt_text(p: &Project, lib: &[SymDef], r: SheetRef, ziel: &Ziel) -> Option<String> {
    let sheet = p.sheet(r)?;
    let obj = |id: u64| sheet.objects.iter().find(|o| o.id == id);
    match *ziel {
        Ziel::Griff(_) | Ziel::Ecke => None,
        Ziel::Anschluss => Some(String::new()),
        Ziel::Beschriftung { id, wert, .. } => {
            let o = obj(id)?;
            let Kind::Symbol { pos, title, ext, .. } = &o.kind else { return None };
            let off = if wert { ext.value_off.unwrap_or_default() } else { title.map(|t| t.0).unwrap_or_default() };
            let lage = format!(" X:{} Y:{}", zahl(pos.x + off.x), zahl(pos.y + off.y));
            let art = if wert {
                ST_WERT
            } else if ext.eigentuemer != 0 {
                ST_TITEL_EIGENTUEMER
            } else {
                ST_TITEL
            };
            Some(format!("{art}{lage}"))
        }
        Ziel::Verweis { id, .. } | Ziel::Text { id, .. } | Ziel::Objekt { id, .. } | Ziel::Leitung { id } => {
            let o = obj(id)?;
            Some(match &o.kind {
                Kind::Symbol { key, pos, option, .. } => {
                    let def = symdef(p, lib, key);
                    let mut t = format!("{ST_SYMBOL}{}", def.map(|d| d.name.as_str()).unwrap_or(""));
                    if let Some(opt) = def.filter(|d| !d.options.is_empty()).and_then(|d| d.options.get(*option as usize)) {
                        t.push('/');
                        t.push_str(opt);
                    }
                    t.push_str(&format!("  X: {}  Y: {}", zahl(pos.x), zahl(pos.y)));
                    t + &bauteil(p, o.id)
                }
                Kind::Terminal { pos, nr, opt, .. } => {
                    let position = if opt.position == 0 { *nr } else { opt.position };
                    format!("{ST_KLEMME} X:{} Y:{}{ST_POSITION}{position}{}", zahl(pos.x), zahl(pos.y), bauteil(p, o.id))
                }
                Kind::Busbar { name, sms, .. } => format!("{ST_SAMMELSCHIENE}: {}({name})", schienen_name(p, r, *sms, name)),
                Kind::Verweis { spitze, .. } => format!("{ST_VERWEIS}{}", if *spitze { ST_ZIEL } else { ST_START }),
                Kind::Spiegel { .. } => ST_KONTAKTSPIEGEL.to_string(),
                Kind::Wire { leitung, .. } => {
                    // Art der Verbindung: VBG Feld 12 Byte 0 (2 = manuell, 4 = Fehler, sonst automatisch); neu
                    // gezeichnete Leitungen sind manuell
                    let raw = o.raw.as_deref().map(|r| &r.block);
                    let art = raw.and_then(|b| b.data.get(0x30).copied()).unwrap_or(2);
                    if art == 4 {
                        return Some(ST_VERBINDUNGSFEHLER.to_string());
                    }
                    let name = p.leitungen.get(*leitung).map(|l| l.name.as_str()).unwrap_or("");
                    let mut t = format!("{}{name}", if art == 2 { ST_MANUELL } else { ST_AUTOMATISCH });
                    // Einspeisung: Anfang (Feld 0 Byte 0 = 2, Feld 4 = 1) oder Ende (Feld 5 Byte 0 = 2, Feld 8 = 1)
                    if let Some(b) = raw.filter(|b| b.data.len() >= 0x24) {
                        if (b.data[0] == 2 && b.i32(4) == 1) || (b.data[0x14] == 2 && b.i32(8) == 1) {
                            t.push_str(ST_EINSPEISUNG);
                        }
                    }
                    t
                }
                _ => String::new(),
            })
        }
    }
}

/// Schlüssel des Objekts unter der Maus (Wechsel erkennen wie 0x5b92d4 im Original).
fn objekt_schluessel(z: &Ziel) -> Option<(u64, u8)> {
    match *z {
        Ziel::Griff(_) | Ziel::Ecke => None,
        Ziel::Anschluss => Some((0, 1)),
        Ziel::Beschriftung { id, wert, .. } => Some((id, 2 + wert as u8)),
        Ziel::Verweis { id, .. } | Ziel::Text { id, .. } | Ziel::Objekt { id, .. } | Ziel::Leitung { id } => Some((id, 0)),
    }
}

/// Statuszeile ohne Objekt unter der Maus: „Kein Objekt“ wie im Original, bei einer Markierung deren Umfang.
pub fn markierung_text(n: usize) -> String {
    match n {
        0 => ST_KEIN_OBJEKT.to_string(),
        1 => "1 Objekt markiert".to_string(),
        n => format!("{n} Objekte markiert"),
    }
}

#[derive(Clone, Default)]
struct StatusMerker {
    /// zuletzt gemeldetes Objekt (None = keins)
    zuletzt: Option<(u64, u8)>,
    /// Mauslage beim Beginn des Ziehens
    start: Option<P>,
    /// gehaltene Meldung und Zeitpunkt, seit dem sie steht
    halten: Option<(String, f64)>,
    /// beim Verschieben: Gesamtverschiebung, bei der die letzte Abweisung kam (danach angenommen = Meldung weg)
    abgewiesen_bei: Option<P>,
}

/// So lange (s) bleibt eine Meldung des Netz-Wächters stehen, bevor Überfahr- bzw. Zieh-Texte sie ersetzen
/// (Nachzeichnen 24.11.2026, NR4: die Meldung war im nächsten Bild schon überschrieben).
pub const MELDUNG_HALTEN: f64 = 4.0;

/// Meldungen, die gehalten werden (Abweisungen des Netz-Wächters und ähnliche).
pub fn wird_gehalten(t: &str) -> bool {
    t.starts_with("Nicht ausgeführt") || t.starts_with("Nicht gedreht") || t.starts_with("Nicht eingefügt")
        // Stufe des Mehrfachklicks auf Leitungen (mehrfachklick.rs, NR12)
        || ["Teilstück", "Bis zum Knoten:", "Netz ohne Sammelschiene:", "Ganzes Potential:", "Ende frei"].iter().any(|p| t.starts_with(p))
}

impl crate::app::App {
    /// Statuszeile nach der Mauseingabe dieses Bildes nachführen (nur über der Zeichenfläche wie PaintBox1MouseMove).
    pub fn statuszeile(&mut self, ctx: &eframe::egui::Context, ueber_flaeche: bool) {
        use crate::app::Drag;
        let id = eframe::egui::Id::unique("ep_statuszeile");
        let mut m: StatusMerker = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
        let jetzt = ctx.input(|i| i.time);
        // Beim Verschieben zählt die Lage beim Loslassen: Abweisungen einer Zwischenlage stehen nur, solange die Lage
        // abgewiesen bleibt, und werden nicht gehalten (Nachzeichnen 25.11.2026 #96: nach zulässigem Ablegen stand
        // noch „würde ein Symbol auf ein anderes legen“); gehalten wird erst nach dem Loslassen
        // während eines Ziehens (Verschieben, Teilstück, Knick …) nichts halten: sonst verdeckte z. B. der gehaltene
        // Mehrfachklick-Text „Teilstück …“ die Meldungen beim Ziehen
        if !matches!(self.drag, Drag::None) {
            m.halten = None;
        }
        if matches!(self.drag, Drag::None) && wird_gehalten(&self.info) && m.halten.as_ref().is_none_or(|(t, _)| *t != self.info) {
            m.halten = Some((self.info.clone(), jetzt));
        }
        if let Some((t, seit)) = m.halten.clone() {
            if jetzt - seit < MELDUNG_HALTEN {
                self.info = t;
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
                ctx.data_mut(|d| d.insert_temp(id, m));
                return;
            }
            m.halten = None;
            // danach wieder normal: Objekt unter der Maus neu melden
            m.zuletzt = Some((u64::MAX, 0));
        }
        match &self.drag {
            Drag::None | Drag::Pan { .. } | Drag::Rubber { .. } => {
                m.start = None;
                m.abgewiesen_bei = None;
            }
            Drag::Move { applied, .. } => {
                let abgewiesen = wird_gehalten(&self.info) && match m.abgewiesen_bei {
                    None => {
                        m.abgewiesen_bei = Some(*applied);
                        true
                    }
                    Some(a) => a.dist(*applied) < 0.001,
                };
                if !abgewiesen {
                    m.abgewiesen_bei = None;
                    if ueber_flaeche {
                        self.info = delta_text(*applied);
                    }
                }
                m.start.get_or_insert(self.mouse_mm);
            }
            _ => {
                let start = *m.start.get_or_insert(self.mouse_mm);
                if ueber_flaeche {
                    self.info = delta_text(self.mouse_mm.sub(start));
                }
            }
        }
        if matches!(self.drag, Drag::None) && ueber_flaeche && self.sym_edit.is_none() && self.tool == crate::app::Tool::Select && self.placing.is_none() {
            let ziel = crate::hover::gemerkt(ctx);
            let schluessel = ziel.as_ref().and_then(objekt_schluessel);
            if schluessel != m.zuletzt {
                let text = match (ziel, self.cur()) {
                    (Some(z), Some(r)) => objekt_text(&self.project, &self.lib, r, &z),
                    _ => None,
                };
                match text {
                    // Zeichenobjekte leeren das Feld (Original: Leertext), Anschlüsse lassen es stehen
                    Some(t) => {
                        if !matches!(ziel, Some(Ziel::Anschluss)) {
                            self.info = t;
                        }
                    }
                    // mit Markierung deren Umfang statt „Kein Objekt“ (Nachzeichnen 24.11.2026, S5)
                    None if m.zuletzt.is_some() => self.info = markierung_text(self.selection.len()),
                    None => {}
                }
                m.zuletzt = schluessel;
            }
        }
        ctx.data_mut(|d| d.insert_temp(id, m));
    }
}

impl crate::app::App {
    /// Löschen im Explorer bzw. in der Bibliothek (Lschen1Click): Ein Symbol, das gerade im Symboleditor offen ist,
    /// lässt sich nicht löschen; sonst Rückfrage „Wirklich Löschen?“ (Nein = Abbruch).
    pub fn loeschen_bestaetigt(&self, symbol: Option<&str>) -> bool {
        if let (Some(k), Some(e)) = (symbol, &self.sym_edit) {
            if e.key == k {
                warnung(T_ACHTUNG, SYMBOL_IN_BEARBEITUNG);
                return false;
            }
        }
        frage(T_SICHER, WIRKLICH_LOESCHEN)
    }

    /// Vor Neu und Beenden (0x47d2e4): ist ein Symbol- oder Schriftfeldeditor offen, fragen, ob dessen Änderungen
    /// übernommen werden sollen (Ja = übernehmen, Nein = verwerfen). false = Abbrechen.
    pub fn editoren_abschliessen(&mut self) -> bool {
        let schriftfeld = matches!(self.xdlg, Some(crate::extra::XDialog::Title { .. }));
        if self.sym_edit.is_none() && !schriftfeld {
            return true;
        }
        match zeigen(T_GEAENDERT, SE_VERLUST, Knoepfe::JaNeinAbbrechen, false) {
            Antwort::Ja => {
                if self.sym_edit.is_some() {
                    self.symbol_editor_commit();
                }
                if let Some(crate::extra::XDialog::Title { elems }) = self.xdlg.take() {
                    self.checkpoint();
                    self.project.titleblock = elems;
                    self.dirty = true;
                }
                true
            }
            Antwort::Nein => {
                self.symbol_editor_cancel();
                if schriftfeld {
                    self.xdlg = None;
                }
                true
            }
            _ => false,
        }
    }
}

// ------------------------------------------------------------------ Laden

/// Fehler beim Öffnen einer .epr wie das Original (Überschrift, Text) – vor dem eigentlichen Einlesen geprüft:
/// Datei nicht lesbar, Kennung „EP“ und Versionsnummer (zwei Ziffern; 5–9 = Version 5.x, 10–16 = 6.x).
/// `hinzufuegen`: Projekt hinzufügen (nimmt nur Dateien ab 6.0.0).
pub fn ladefehler_vorab(d: &std::io::Result<Vec<u8>>, hinzufuegen: bool) -> Option<(&'static str, String)> {
    let d = match d {
        Ok(d) => d,
        Err(e) => return Some((T_OEFFNEN, e.to_string())),
    };
    if d.len() < 4 || &d[0..2] != b"EP" {
        return Some((T_ACHTUNG_FEHLER, L_FEHLER.into()));
    }
    let ziffer = |b: u8| (b as i32) - b'0' as i32;
    let n = ziffer(d[2]) * 10 + ziffer(d[3]);
    if n < 5 {
        return Some((T_ACHTUNG_X, L_VOR_500.into()));
    }
    if n <= 9 && hinzufuegen {
        return Some((T_ACHTUNG_FEHLER, L_VOR_600.into()));
    }
    if n > 16 {
        return Some((T_ACHTUNG_FEHLER, L_VERSION_FALSCH.into()));
    }
    None
}

/// Kontextzeile der Fehlerliste für einen Block (wie 0x498308: je Ebene einmal).
fn kontext(tag: &[u8; 3], data: &[u8]) -> Option<String> {
    let name = || crate::symbols::cstr(data.get(0..40).unwrap_or(&[]));
    Some(match tag {
        b"PRJ" => K_PROJEKT.to_string(),
        b"FUS" => K_SCHRIFTFELD.to_string(),
        b"SYM" => format!("{K_SYMBOL}{K_VON}{}\"", name()),
        b"ANL" => format!("{K_ANLAGE}{K_VON}{}\"", name()),
        b"BLT" => format!("{K_BLATT}{K_VON}{}\"", name()),
        b"BTL" => format!("{K_BAUTEIL}{K_VON}{}\"", name()),
        b"POT" => format!("{K_POTENTIAL}{K_VON}{}\"", name()),
        b"LTG" => format!("{K_LEITUNG}{K_VON}{}\"", name()),
        b"ERS" => format!("{K_PLATZHALTER}{}\"", name()),
        _ => return None,
    })
}

/// Fehlerliste (TErrliste) des Originals für eine .epr: Prüfsummen, Größenangaben, unbekannte Objekte, fehlendes
/// Ende – mit den Kontextzeilen „In den Projektdaten:“, „ In den Anlagendaten von "…"“ usw.
pub fn ladefehler_liste(d: &[u8]) -> Vec<String> {
    let mut out = vec![];
    if d.len() < 4 {
        return out;
    }
    // offene Blöcke: (Kennung, Kontextzeile)
    let mut stapel: Vec<([u8; 3], Option<String>)> = vec![];
    let mut gemeldet: Vec<String> = vec![];
    let mut melden = |stapel: &Vec<([u8; 3], Option<String>)>, text: String, out: &mut Vec<String>| {
        let ketten: Vec<String> = stapel.iter().filter_map(|(_, k)| k.clone()).collect();
        let gleich = gemeldet.iter().zip(&ketten).take_while(|(a, b)| a == b).count();
        out.extend(ketten[gleich..].iter().cloned());
        gemeldet = ketten;
        out.push(format!("    {text}"));
    };
    let mut p = 4;
    let mut jpg_teil: Option<(u8, u8)> = None;
    while p + 4 <= d.len() {
        if d[p] != 0x99 {
            break;
        }
        let tag = [d[p + 1], d[p + 2], d[p + 3]];
        let name = String::from_utf8_lossy(&tag).to_string();
        if &tag == b"END" {
            stapel.pop();
            p += 4;
            continue;
        }
        if !crate::eprfile::TAGS.iter().any(|t| t.as_bytes() == tag) {
            melden(&stapel, format!("{O_UNBEKANNT}{name}"), &mut out);
            break;
        }
        if p + 8 > d.len() {
            break;
        }
        let ln = u16::from_le_bytes([d[p + 4], d[p + 5]]) as usize;
        let chk = i16::from_le_bytes([d[p + 6], d[p + 7]]);
        if chk != !(ln as u16 as i16) {
            melden(&stapel, format!("{O_GROESSE}{name}"), &mut out);
            break;
        }
        let mut q = p + 8;
        // geteilte Bilder: Teilkopf (Teil, Anzahl) vor den Daten; Folgeteile ohne eigenes Ende (siehe eprfile)
        let folgeteil = &tag == b"JPG" && stapel.last().is_some_and(|s| &s.0 == b"JPG") && matches!(jpg_teil, Some((t, n)) if t < n);
        if &tag == b"JPG" && (ln == 0xffff || folgeteil) {
            jpg_teil = d.get(q..q + 2).map(|h| (h[0], h[1]));
            q += 4;
        } else if &tag == b"JPG" {
            jpg_teil = None;
        }
        let Some(data) = d.get(q..q + ln) else {
            melden(&stapel, format!("{O_GROESSE}{name}"), &mut out);
            break;
        };
        let Some(sum) = d.get(q + ln..q + ln + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())) else {
            melden(&stapel, format!("{O_STRUKTUR}{name}"), &mut out);
            break;
        };
        if !folgeteil {
            stapel.push((tag, kontext(&tag, data)));
        }
        if sum != crate::eprfile::checksum(data) {
            melden(&stapel, format!("{O_PRUEFSUMME}{name}"), &mut out);
        }
        // Klemmleiste mit mehr als 4 Ebenen (KLM Feld 15): das Original begrenzt sie beim Laden (klemmen.rs)
        if &tag == b"KLM" && data.get(60..64).is_some_and(|e| i32::from_le_bytes(e.try_into().unwrap()) > 4) {
            melden(&stapel, format!("{O_EBENEN}{}", crate::symbols::cstr(&data[..44])), &mut out);
        }
        p = q + ln + 4;
    }
    if let Some((tag, _)) = stapel.last().cloned() {
        if p >= d.len() {
            let name = String::from_utf8_lossy(&tag).to_string();
            melden(&stapel, format!("{O_KEIN_ENDE}{name}"), &mut out);
        }
    }
    out
}

/// Fehlerliste: Projektbilder (PRJ › BLD › JPG), deren Bilddaten sich nicht lesen lassen (wie 0x426310 mit Namen).
pub fn bildfehler(p: &Project) -> Vec<String> {
    p.bilder.iter().filter(|b| image::load_from_memory(&b.jpeg).is_err()).map(|b| format!("{L_BILDDATEN}{}", b.name)).collect()
}

/// Bibliothek › Symbolgruppen › Hinzufügen (TBiblio.Button2Click): Beschriftung („Hauptgruppe: <markierte>“ bzw.
/// „Neue Hauptgruppe“) und der volle Name der neuen Gruppe aus der Eingabe (leer = „Neue Gruppe“).
pub fn neue_gruppe_ziel(markiert: &str) -> (String, impl Fn(&str) -> String + '_) {
    let haupt = markiert.trim();
    let text = if haupt.is_empty() { BG_NEUE_HAUPTGRUPPE.to_string() } else { format!("{BG_HAUPTGRUPPE}{haupt}") };
    (text, move |eingabe: &str| {
        let name = if eingabe.trim().is_empty() { BG_NEUE_GRUPPE } else { eingabe.trim() };
        if haupt.is_empty() { name.to_string() } else { format!("{haupt} / {name}") }
    })
}

// ------------------------------------------------------------------ Speichern

/// Meldung zu einem fehlgeschlagenen Speichern (Fehlerart wie im Original aus dem Zustand der Zieldatei).
pub fn speicherfehler(pfad: &std::path::Path) -> &'static str {
    if pfad.as_os_str().is_empty() {
        return S_KEIN_NAME;
    }
    match std::fs::metadata(pfad) {
        Ok(m) if m.permissions().readonly() => S_SCHREIBSCHUTZ,
        Ok(_) => S_SCHREIBEN,
        Err(_) => S_NICHT_ANGELEGT,
    }
}

/// Projektdatei schreiben wie das Original (0x4228c4): erst in eine temporäre Datei im Zielordner, dann – nach der
/// Sicherung `backup` (falls angegeben) – die Zieldatei ersetzen. Fehler als Meldungstext des Originals; scheitert
/// erst das Umbenennen, bleibt die temporäre Datei stehen und die Meldung nennt sie.
pub fn speichern_ueber_temp(pfad: &std::path::Path, daten: &[u8], backup: Option<&std::path::Path>) -> Result<(), String> {
    use std::io::Write;
    if pfad.as_os_str().is_empty() {
        return Err(S_KEIN_NAME.into());
    }
    // Name der temporären Datei: Zeitstempel hexadezimal (das Original nimmt die 8 Byte der aktuellen Zeit)
    let zeit = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let temp = pfad.with_file_name(format!("{zeit:016X}.tmp"));
    let mut f = std::fs::File::create(&temp).map_err(|_| S_NICHT_ANGELEGT.to_string())?;
    if f.write_all(daten).and_then(|_| f.sync_all()).is_err() {
        drop(f);
        let _ = std::fs::remove_file(&temp);
        return Err(S_SCHREIBEN.into());
    }
    drop(f);
    let weg = |t: &std::path::Path, m: &str| {
        let _ = std::fs::remove_file(t);
        Err(m.to_string())
    };
    if let Ok(m) = std::fs::metadata(pfad) {
        if let Some(b) = backup {
            if std::fs::copy(pfad, b).is_err() {
                return weg(&temp, S_KEIN_BACKUP);
            }
        }
        if m.permissions().readonly() {
            return weg(&temp, S_SCHREIBSCHUTZ);
        }
    }
    std::fs::rename(&temp, pfad).map_err(|_| format!("{S_TEMP_1}{}{S_TEMP_2}", temp.display()))
}

/// Meldung zu einer nicht schreibbaren Exportdatei (TFORM1.Export: Fehlercodes der Export.dll).
pub fn exportfehler(pfad: &std::path::Path, e: &std::io::Error) -> &'static str {
    let schreibgeschuetzt = std::fs::metadata(pfad).map(|m| m.permissions().readonly()).unwrap_or(false);
    match e.kind() {
        _ if schreibgeschuetzt => EX_SCHREIBSCHUTZ,
        std::io::ErrorKind::StorageFull => EX_PLATZ,
        _ => EX_ANLEGEN,
    }
}

// ------------------------------------------------------------------ Blattnummerierung (TBlattIndex, 0x49c834)

/// Neue Blattnummern für die Blätter `von..=bis` (Lage in der Anlage, 1-basiert) ab `ab` in Schritten `plus`
/// prüfen wie das Original. Ok((neue Nummern aller Blätter, Reihenfolge ändert sich)), Err(Meldung).
pub fn neunummerierung(nummern: &[(u32, u8)], von: &str, bis: &str, ab: &str, plus: &str) -> Result<(Vec<u32>, bool), &'static str> {
    if ab.trim().is_empty() {
        return Err(NN_AB_FEHLT);
    }
    let n = nummern.len();
    let v = von.trim().parse::<usize>().unwrap_or(1).max(1);
    let b = bis.trim().parse::<usize>().unwrap_or(n).min(n);
    if v > b {
        return Err(NN_VON_BIS);
    }
    let start = ab.trim().parse::<u32>().unwrap_or(1).max(1);
    let step = plus.trim().parse::<u32>().unwrap_or(1).max(1);
    let mut neu: Vec<u32> = nummern.iter().map(|x| x.0).collect();
    for (k, x) in neu.iter_mut().enumerate().filter(|(k, _)| *k + 1 >= v && *k + 1 <= b) {
        *x = start + (k + 1 - v) as u32 * step;
    }
    // Überschneidung: gleiche (Nummer, Subindex) wie ein anderes Blatt
    for i in 0..n {
        for j in i + 1..n {
            if (neu[i], nummern[i].1) == (neu[j], nummern[j].1) {
                return Err(NN_UEBERSCHNEIDUNG);
            }
        }
    }
    let reihe = |nr: &dyn Fn(usize) -> u32| {
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by_key(|i| (nr(*i), nummern[*i].1));
        idx
    };
    let vorher = reihe(&|i| nummern[i].0);
    let nachher = reihe(&|i| neu[i]);
    Ok((neu, vorher != nachher))
}

/// Text der Rückfrage nach fehlenden Blättern (Inhaltsverzeichnis bzw. Querverweisliste) beim Einfügen.
pub fn fehlende_blaetter(inhalt: bool, anzahl: usize) -> String {
    let anfang = if inhalt { INHALT_FRAGE_1 } else { QUERVERWEIS_FRAGE_1 };
    if anzahl == 1 {
        format!("{anfang}{FRAGE_IST_NOCH_1}{FRAGE_NOETIG}")
    } else {
        format!("{anfang}{FRAGE_SIND_NOCH}{anzahl} Blätter{FRAGE_NOETIG}")
    }
}

/// Vor dem Anlegen der Folgeblätter eines Inhaltsverzeichnisses bzw. einer Querverweisliste fragen wie das Original
/// (Blatteinfgen1Click, Überschrift „Frage“); true = anlegen (auch wenn nichts fehlt oder nicht gefragt wird).
pub fn folgeblaetter_frage(p: &Project, r: SheetRef) -> bool {
    let Some(a) = p.anlagen.get(r.anlage) else { return true };
    let Some(s) = a.sheets.get(r.sheet) else { return true };
    let art = s.blattart();
    if !matches!(art, 1 | 2) {
        return true;
    }
    let erstes = r.sheet - crate::klemmplan::folge_index(a, s).min(r.sheet);
    let noetig = crate::berechnet::blaetter_noetig(p, &a.sheets[erstes], a);
    let vorhanden = a.sheets[erstes..].iter().take_while(|x| x.blattart() == art).count();
    if noetig <= vorhanden {
        return true;
    }
    frage("Frage", &fehlende_blaetter(art == 1, noetig - vorhanden))
}

/// Beschriftung des Symboleditors (0x477cb0): „Bibliothekssymbol 'Name' bearbeiten“, „Projektsymbol 'Name'
/// bearbeiten“ bzw. für ein neues Symbol „Projektsymbol 'Neues Symbol' erstellen“. `symbol` = (Schlüssel, Name).
pub fn editor_titel(symbol: Option<(&str, &str)>) -> String {
    match symbol {
        Some((key, name)) => {
            let projekt = key.starts_with("proj:") || key.starts_with("epr:");
            format!("{}{name}{SE_BEARBEITEN}", if projekt { SE_PROJEKTSYMBOL } else { SE_BIBLIOTHEKSSYMBOL })
        }
        None => format!("{SE_PROJEKTSYMBOL}{SE_NEU}"),
    }
}

/// Spaltenkopf der Alternativtexte (0x479064): Spalte `k` (ab 0) = „Wahl 1“, „Wahl 2“ …
pub fn wahl_kopf(k: usize) -> String {
    if k == 0 {
        TA_WAHL_1.to_string()
    } else {
        format!("{TA_WAHL}{}", k + 1)
    }
}

/// Abschlussmeldung der Synchronisierung (TBiblio.Button7Click, Überschrift „Summe“).
pub fn sync_summe(gruppen: usize, versionen: usize, symbole: usize, bauteile: usize) -> String {
    let e = |n: usize, eins: &'static str, mehr: &'static str| if n == 1 { eins } else { mehr };
    format!(
        "{SYNC_FERTIG}{gruppen} Gruppenname{}\n{versionen}{SYNC_NEUE_VERSION}{} und\n{symbole} neue{} Symbol{} und\n{bauteile} neue{} Bauteil{}\n{SYNC_HINZUGEFUEGT}",
        e(gruppen, "", "n"),
        e(versionen, "", "en"),
        e(symbole, "s", ""),
        e(symbole, "", "e"),
        e(bauteile, "s", ""),
        e(bauteile, "", "e"),
    )
}

/// Ist der Gegenverweis `gegen` schon mit einem anderen Verweis als `id` verbunden? (Rückfrage „Der Zielverweis ist
/// schon anderwertig belegt!“ beim Ziel setzen)
pub fn zielverweis_belegt(p: &Project, id: u64, gegen: Option<u64>) -> bool {
    let Some(g) = gegen.filter(|g| *g != id) else { return false };
    match p.find_obj(g).map(|(_, o)| &o.kind) {
        Some(Kind::Verweis { ziel: Some(z), .. }) => z.obj != 0 && z.obj != id && z.obj != g,
        _ => false,
    }
}

/// Fenstertitel wie das Original: „Elektroplanung <Datei> (Version:EP16)“ für .epr, sonst nur der Name.
pub fn fenstertitel(name: &str, pfad: Option<&std::path::Path>, kennung: Option<[u8; 4]>, vorlage: bool) -> String {
    let v = if vorlage { "Vorlage " } else { "" };
    match (pfad, kennung) {
        (Some(p), Some(k)) => format!("Elektroplanung {v}{}{L_VERSION}{})", p.display(), String::from_utf8_lossy(&k)),
        _ => format!("Elektroplanung {v}{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaltenkoepfe_alternativtexte() {
        // 0x479116: erste Spalte „Wahl 1“, danach „Wahl n“
        assert_eq!(wahl_kopf(0), "Wahl 1");
        assert_eq!(wahl_kopf(3), "Wahl 4");
    }

    #[test]
    fn schriftfeld_auswahl_wie_tform4() {
        let t: Vec<&str> = crate::optionen::SCHRIFTFELD_REIHENFOLGE.iter().map(|v| crate::optionen::schriftfeld_text(*v)).collect();
        assert_eq!(t, ["Ohne Schriftfeld", "Vorgabeschriftfeld", "Schriftfeld aus Datei"]);
    }

    #[test]
    fn folgeblatt_inhaltsverzeichnis_wortlaut() {
        // 0x5caa8e + Seite + 0x5caaee
        let t = format!("{FB_INHALT}2.1{FB_VERGEBEN}");
        assert!(t.starts_with("Es kann kein Folgeblatt für's Inhaltsverzeichnis erstellt werden da die dafür\nbenötigte Seite: 2.1 schon vergeben ist!"));
    }

    #[test]
    fn zahlen_wie_floattostr() {
        assert_eq!(zahl(12.5), "12,5");
        assert_eq!(zahl(12.0), "12");
        assert_eq!(zahl(-0.04), "0");
        assert_eq!(zahl(-5.0), "-5");
        assert_eq!(delta_text(P::new(2.5, -10.0)), "DeltaX: 2,5  DeltaY: -10");
    }

    #[test]
    fn testbetrieb_sammelt_und_antwortet() {
        test_antworten(&[Antwort::Nein]);
        assert!(!frage(T_SICHER, WIRKLICH_LOESCHEN));
        assert!(frage(T_SICHER, WIRKLICH_LOESCHEN));
        hinweis(T_HINWEIS, DRUCK_KEIN_BLATT);
        let g = test_gezeigt();
        assert_eq!(g.len(), 3);
        assert_eq!(g[2], (T_HINWEIS.to_string(), DRUCK_KEIN_BLATT.to_string()));
    }

    #[test]
    fn ladefehler_vorab_wie_original() {
        let ok = |s: &[u8]| Ok::<Vec<u8>, std::io::Error>(s.to_vec());
        assert!(ladefehler_vorab(&ok(b"EP16xxxx"), false).is_none());
        assert_eq!(ladefehler_vorab(&ok(b"EP04xxxx"), false).unwrap().1, L_VOR_500);
        assert_eq!(ladefehler_vorab(&ok(b"EP07xxxx"), true).unwrap().1, L_VOR_600);
        assert!(ladefehler_vorab(&ok(b"EP07xxxx"), false).is_none());
        assert_eq!(ladefehler_vorab(&ok(b"EP17xxxx"), false).unwrap().1, L_VERSION_FALSCH);
        assert_eq!(ladefehler_vorab(&ok(b"XX16"), false).unwrap(), (T_ACHTUNG_FEHLER, L_FEHLER.to_string()));
        let e = std::fs::read("/gibt/es/nicht.epr");
        assert_eq!(ladefehler_vorab(&e, false).unwrap().0, T_OEFFNEN);
    }

    fn block(tag: &str, data: &[u8]) -> Vec<u8> {
        let mut v = vec![0x99];
        v.extend_from_slice(tag.as_bytes());
        let ln = data.len() as u16;
        v.extend_from_slice(&ln.to_le_bytes());
        v.extend_from_slice(&(!(ln as i16)).to_le_bytes());
        v.extend_from_slice(data);
        v.extend_from_slice(&crate::eprfile::checksum(data).to_le_bytes());
        v
    }

    #[test]
    fn fehlerliste_mit_kontext() {
        let mut name = b"Anlage 1".to_vec();
        name.resize(56, 0);
        let mut blatt = b"Blatt 3".to_vec();
        blatt.resize(40, 0);
        let mut d = b"EP16".to_vec();
        d.extend(block("PRJ", &[0; 8]));
        d.extend(block("ANL", &name));
        d.extend(block("BLT", &blatt));
        let mut vsy = block("VSY", &[1, 2, 3, 4]);
        let n = vsy.len();
        vsy[n - 4] ^= 1;
        d.extend(vsy);
        d.extend_from_slice(b"\x99END\x99END\x99END\x99END");
        let l = ladefehler_liste(&d);
        assert_eq!(
            l,
            vec![
                "In den Projektdaten:".to_string(),
                " In den Anlagendaten von \"Anlage 1\"".into(),
                "  In den Blattdaten von \"Blatt 3\"".into(),
                "    Prüfsumme ist Fehlerhaft. Objekt: VSY".into(),
            ]
        );
        // fehlendes Ende und unbekanntes Objekt
        let mut e = b"EP16".to_vec();
        e.extend(block("PRJ", &[0; 8]));
        assert_eq!(ladefehler_liste(&e).last().unwrap(), "    Strukturfehler (Kein Ende) für das Objekt: PRJ");
        e.extend_from_slice(b"\x99XYZ\x01\x00\xfe\xff");
        assert_eq!(ladefehler_liste(&e).last().unwrap(), "    Unbekanntes Objekt: XYZ");
    }

    #[test]
    fn neunummerierung_prueft_wie_original() {
        let nr = [(1, 0), (2, 0), (3, 0), (10, 0)];
        assert_eq!(neunummerierung(&nr, "1", "4", "", "1"), Err(NN_AB_FEHLT));
        assert_eq!(neunummerierung(&nr, "3", "2", "1", "1"), Err(NN_VON_BIS));
        // Blätter 1–2 ab 10: überschneidet sich mit Blatt 10
        assert_eq!(neunummerierung(&nr, "1", "2", "9", "1"), Err(NN_UEBERSCHNEIDUNG));
        assert_eq!(neunummerierung(&nr, "1", "4", "1", "1"), Ok((vec![1, 2, 3, 4], false)));
        // Blätter 1–2 ab 20: Reihenfolge ändert sich
        assert_eq!(neunummerierung(&nr, "1", "2", "20", "1"), Ok((vec![20, 21, 3, 10], true)));
    }

    #[test]
    fn rueckfrage_fehlende_blaetter() {
        assert_eq!(fehlende_blaetter(true, 1), "Zum anlegen eines vollständigen Inhaltsverzeichnisses ist noch 1 Blatt nötig.\nSollen alle nötigen Blätter erzeugt werden?");
        assert!(fehlende_blaetter(false, 3).contains(" sind noch 3 Blätter nötig."));
    }

    #[test]
    fn weitere_texte() {
        let p = Project::new_default();
        let r = SheetRef { anlage: 0, sheet: 0 };
        assert_eq!(crate::blattnummer::pruefen(&p, r, "2", "300", 3, false), Err("Der Subindex darf nicht größer als 255 sein!".to_string()));
        assert_eq!(editor_titel(Some(("db:300:1:Lampe", "Lampe"))), "Bibliothekssymbol 'Lampe' bearbeiten");
        assert_eq!(editor_titel(Some(("epr:00ff", "Taster"))), "Projektsymbol 'Taster' bearbeiten");
        assert_eq!(editor_titel(None), "Projektsymbol 'Neues Symbol' erstellen");
        assert!(sync_summe(0, 2, 1, 0).contains("2 neue Symbolversionen und\n1 neues Symbol und\n0 neue Bauteile\nder Symbolbibliothek"));
        let t = fenstertitel("x", Some(std::path::Path::new("C:/a/W2.epr")), Some(*b"EP16"), false);
        assert_eq!(t, "Elektroplanung C:/a/W2.epr (Version:EP16)");
        assert_eq!(speicherfehler(std::path::Path::new("")), S_KEIN_NAME);
        assert_eq!(speicherfehler(std::path::Path::new("/gibt/es/nicht/x.epr")), S_NICHT_ANGELEGT);
    }

    #[test]
    fn speichern_ueber_temporaere_datei() {
        let dir = std::env::temp_dir().join(format!("ep_meldungen2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ziel = dir.join("a.epr");
        let bak = dir.join("a.epr.bak");
        // neue Datei, dann Überschreiben mit Sicherung der alten Fassung
        speichern_ueber_temp(&ziel, b"eins", None).unwrap();
        speichern_ueber_temp(&ziel, b"zwei", Some(&bak)).unwrap();
        assert_eq!(std::fs::read(&ziel).unwrap(), b"zwei");
        assert_eq!(std::fs::read(&bak).unwrap(), b"eins");
        // keine temporäre Datei bleibt liegen
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(speichern_ueber_temp(std::path::Path::new(""), b"x", None), Err(S_KEIN_NAME.to_string()));
        assert_eq!(speichern_ueber_temp(&dir.join("fehlt/b.epr"), b"x", None), Err(S_NICHT_ANGELEGT.to_string()));
        // schreibgeschützte Zieldatei
        let mut perm = std::fs::metadata(&ziel).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&ziel, perm.clone()).unwrap();
        assert_eq!(speichern_ueber_temp(&ziel, b"drei", None), Err(S_SCHREIBSCHUTZ.to_string()));
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        std::fs::set_permissions(&ziel, perm).unwrap();
        // Umbenennen scheitert (Ziel ist ein Ordner): Meldung nennt die temporäre Datei, die stehen bleibt
        let ordner = dir.join("ordner.epr");
        std::fs::create_dir_all(ordner.join("x")).unwrap();
        let m = speichern_ueber_temp(&ordner, b"vier", None).unwrap_err();
        assert!(m.starts_with(S_TEMP_1) && m.ends_with(S_TEMP_2), "{m}");
        let temp = m.trim_start_matches(S_TEMP_1).trim_end_matches(S_TEMP_2);
        assert_eq!(std::fs::read(temp).unwrap(), b"vier");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ladefehler_klemmleiste_und_bilder() {
        let mut klm = b"Leiste X1".to_vec();
        klm.resize(80, 0);
        klm[60..64].copy_from_slice(&6i32.to_le_bytes());
        let mut name = b"Anlage 1".to_vec();
        name.resize(56, 0);
        let mut d = b"EP16".to_vec();
        d.extend(block("PRJ", &[0; 8]));
        d.extend(block("ANL", &name));
        d.extend(block("KLM", &klm));
        d.extend_from_slice(b"\x99END\x99END\x99END");
        let l = ladefehler_liste(&d);
        assert_eq!(l.last().unwrap(), "    Anzahl Ebenen der Klemmleiste wird auf 4 begrenzt! Klemmleiste: Leiste X1");
        // beim Lesen begrenzt
        let k = crate::eprfile::Block::new("KLM", klm);
        assert_eq!(crate::klemmen::klemm_info(&k).etagen, 4);
        let mut p = Project::new_default();
        p.bilder = vec![Bild { name: "Logo".into(), nr: 1, jpeg: vec![1, 2, 3] }];
        assert_eq!(bildfehler(&p), vec!["Fehler beim Laden der Bilddaten von: Logo".to_string()]);
    }

    #[test]
    fn neue_bibliotheksgruppe() {
        let (t, name) = neue_gruppe_ziel("");
        assert_eq!(t, "Neue Hauptgruppe");
        assert_eq!(name(""), "Neue Gruppe");
        assert_eq!(name(" Taster "), "Taster");
        let (t, name) = neue_gruppe_ziel("Schalter");
        assert_eq!(t, "Hauptgruppe: Schalter");
        assert_eq!(name(""), "Schalter / Neue Gruppe");
    }

    #[test]
    fn statuszeile_objekttexte() {
        let mut p = Project::new_default();
        p.leitungen = vec![Leitung { name: "NYM".into(), color: [0; 3], width_mm: 0.25, quer: 1.5, adern: 3, kennz: 0, style: 0, id: 0 }];
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.clear();
        let mk = |id, kind| Obj { id, kind, pen: 0, fixed: false, group: None, raw: None };
        s.objects.push(mk(1, Kind::Wire { pts: vec![P::new(0.0, 0.0), P::new(10.0, 0.0)], leitung: 0, bez: vec![], opts: 0 }));
        s.objects.push(mk(2, Kind::Verweis { pos: P::default(), text_pos: P::default(), text: "x".into(), dir: 0, vertical: false, spitze: true, ziel: None }));
        s.objects.push(mk(3, Kind::Terminal { pos: P::new(12.5, 30.0), leiste: "X1".into(), nr: 4, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }));
        s.objects.push(mk(4, Kind::Line { a: P::default(), b: P::new(1.0, 1.0) }));
        let r = SheetRef { anlage: 0, sheet: 0 };
        let z = |id| Ziel::Objekt { id, mn: P::default(), mx: P::default(), art: crate::hover::Objektart::Verschiebbar };
        assert_eq!(objekt_text(&p, &[], r, &z(1)).unwrap(), "manuelle Verbindung mit NYM");
        assert_eq!(objekt_text(&p, &[], r, &z(2)).unwrap(), "Verweis Ziel");
        assert_eq!(objekt_text(&p, &[], r, &z(3)).unwrap(), "Klemme  X:12,5 Y:30 Position:4");
        assert_eq!(objekt_text(&p, &[], r, &z(4)).unwrap(), "");
        assert_eq!(objekt_text(&p, &[], r, &Ziel::Griff(0)), None);
    }
}
