//! Druckauswahl „Änderung seit dem: …“ und „Änderung seit dem letzten Ausdruck“ (TForm3, AlleClick/DatumChange):
//! gedruckt werden die Blätter, deren Blattdatum „Geändert“ (BLT Byte 120) oder „Erstellt“ (Byte 112) am
//! Stichtag oder danach liegt. Ungespeicherte Änderungen gegenüber der geladenen Datei zählen als heute geändert.

use crate::model::*;
use chrono::NaiveDate;

/// Datum "TT.MM.JJJJ" (auch mit Uhrzeit dahinter) lesen.
pub fn parse_datum(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    let d = t.get(..10).unwrap_or(t);
    NaiveDate::parse_from_str(d, "%d.%m.%Y").ok()
}

/// Inhalt gegenüber dem Originalblock geändert (wie beim Schreiben des .epr: neue, entfernte oder geänderte Objekte)?
fn content_changed(s: &Sheet) -> bool {
    match s.raw.as_deref() {
        None => false,
        Some(r) => r.map.iter().filter(|m| m.is_some()).count() != s.objects.len() || s.objects.iter().any(|o| o.raw.as_ref().map(|x| x.kind != o.kind || x.pen != o.pen || x.fixed != o.fixed).unwrap_or(true)),
    }
}

/// Letztes Änderungsdatum des Blattes (Geändert, sonst Erstellt; ungespeicherte Änderung = heute).
pub fn letzte_aenderung(s: &Sheet) -> Option<NaiveDate> {
    let mut d = [parse_datum(&s.geaendert), parse_datum(&s.erstellt)].into_iter().flatten().max();
    if content_changed(s) {
        d = d.max(parse_datum(&today()));
    }
    d
}

/// Blatt am Stichtag oder danach geändert? Ohne lesbaren Stichtag wird jedes Blatt gedruckt, ohne Blattdatum keines.
pub fn geaendert_seit(s: &Sheet, stichtag: &str) -> bool {
    match parse_datum(stichtag) {
        None => true,
        Some(t) => letzte_aenderung(s).map(|d| d >= t).unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datum_lesen() {
        assert_eq!(parse_datum("05.03.2024"), NaiveDate::from_ymd_opt(2024, 3, 5));
        assert_eq!(parse_datum("05.03.2024 12:00"), NaiveDate::from_ymd_opt(2024, 3, 5));
        assert_eq!(parse_datum("---------"), None);
    }

    #[test]
    fn auswahl_nach_stichtag() {
        let mut s = Sheet::new(1);
        s.erstellt = "01.01.2020".into();
        s.geaendert = "10.06.2023".into();
        assert!(geaendert_seit(&s, "10.06.2023"));
        assert!(geaendert_seit(&s, "01.01.2023"));
        assert!(!geaendert_seit(&s, "11.06.2023"));
        // ohne Geändert zählt Erstellt
        s.geaendert.clear();
        assert!(!geaendert_seit(&s, "02.01.2020"));
        assert!(geaendert_seit(&s, "01.01.2020"));
        // unlesbarer Stichtag (z. B. nie gedruckt): alle Blätter
        assert!(geaendert_seit(&s, ""));
    }

    /// W2: alle Blätter tragen ein Änderungsdatum; ein Stichtag in der Zukunft wählt keins, einer vor dem
    /// ältesten Datum alle.
    #[test]
    fn w2_blattdaten() {
        let Some(p) = std::env::var_os("EP_SAMPLE") else { return };
        let pr = crate::epr::import(std::path::Path::new(&p)).unwrap();
        assert!(pr.total_sheets() > 10);
        let sheets: Vec<&Sheet> = pr.anlagen.iter().flat_map(|a| &a.sheets).collect();
        assert!(sheets.iter().all(|s| letzte_aenderung(s).is_some()));
        assert!(sheets.iter().all(|s| !geaendert_seit(s, "01.01.2999")));
        assert!(sheets.iter().all(|s| geaendert_seit(s, "01.01.1990")));
    }
}
