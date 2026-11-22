//! Prüfung von Blattnummer und Subindex wie im Original (TSBlatt: Blatt_NummerChange 0x4a560c,
//! Prüfung beim Übernehmen 0x4a51e4 und Doppelte-Nummer-Prüfung im OK-Handler):
//! - Die Blattnummer darf nicht kleiner als 0 sein.
//! - Nummer 0 ist mit der Blattart „Schaltplan“ (3) nicht erlaubt.
//! - Nummer 0 mit Subindex 0 gibt es nur für die Blattart „Deckblatt“ (0).
//! - Nummer und Subindex sind je Anlage eindeutig. Beim Anlegen (Blatteinfgen1Click 0x461960) meldet das Original
//!   „Ein Blatt mit dieser Blattnummer …“ bzw. bei einem Deckblatt „Es ist schon ein Deckblatt …“ und lässt den
//!   Dialog offen. Beim Ändern eines vorhandenen Blattes (Objekt-Eigenschaften 0x4427d4 → Blatt umhängen 0x461234)
//!   prüft der OK-Knopf das nicht; erst das Umhängen meldet „Die Blattnummer: … ist schon vorhanden.
//!   Blattverschiebung kann nicht durchgeführt werden.“ – das Blatt behält Nummer und Subindex, die übrigen
//!   Änderungen gelten (`verschiebung_belegt`).

use crate::model::*;

/// Prüft Nummer/Subindex für das Blatt `r` mit der Blattart `art`; liefert (Nummer, Subindex) oder die Meldung.
/// `neu`: Blatt wird gerade angelegt (nur dann prüft der Dialog selbst auf doppelte Nummern).
pub fn pruefen(p: &Project, r: SheetRef, nummer: &str, subindex: &str, art: u8, neu: bool) -> Result<(u32, u8), String> {
    let n: i64 = nummer.trim().parse().map_err(|_| "Die Blattnummer ist keine Zahl!".to_string())?;
    if n < 0 {
        return Err("Die Blattnummer darf nicht kleiner als 0 sein!".into());
    }
    // Subindex über 255: Meldung des Originals (Blatt1SubindexChange)
    if subindex.trim().parse::<i64>().is_ok_and(|v| v > 255) {
        return Err(format!("{}{}", crate::meldungen::SUBINDEX_1, crate::meldungen::SUBINDEX_2));
    }
    let sub: u8 = if subindex.trim().is_empty() { 0 } else { subindex.trim().parse().map_err(|_| "Der Subindex muss zwischen 0 und 255 liegen!".to_string())? };
    if n == 0 && art == 3 {
        return Err("Die Blattnummer '0' kann nicht mit der Blatteinstellung 'Schaltplan'\nvergeben werden!".into());
    }
    if n == 0 && sub == 0 && art != 0 {
        return Err("Die Blattnummer '0' in Verbindung mit dem Subindex '0'\nkann nur mit der Blatteinstellung 'Deckblatt' vergeben werden!".into());
    }
    let n = n as u32;
    if neu && belegt(p, r, n, sub) {
        return Err(if art == 0 { "Es ist schon ein Deckblatt für diese Anlage vorhanden!" } else { "Ein Blatt mit dieser Blattnummer ist bei dieser Anlage schon vorhanden!" }.into());
    }
    Ok((n, sub))
}

/// Hat ein anderes Blatt der Anlage schon Nummer `n` und Subindex `sub`?
fn belegt(p: &Project, r: SheetRef, n: u32, sub: u8) -> bool {
    p.anlagen.get(r.anlage).is_some_and(|a| a.sheets.iter().enumerate().any(|(i, x)| i != r.sheet && x.number == n && x.subindex == sub))
}

/// Vorhandenes Blatt auf Nummer `n`/Subindex `sub` umhängen (0x461234, Aufruf mit Art 0): ist die Stelle schon
/// belegt, die Meldung des Originals (Überschrift „Achtung“); das Blatt behält dann seine Nummer.
pub fn verschiebung_belegt(p: &Project, r: SheetRef, n: u32, sub: u8) -> Option<String> {
    belegt(p, r, n, sub).then(|| format!("Die Blattnummer: {n} mit dem Subindex: {sub}\nist schon vorhanden. Blattverschiebung kann nicht durchgeführt werden."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regeln_wie_im_original() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets.push(Sheet::new(2));
        let r = SheetRef { anlage: 0, sheet: 1 };
        assert_eq!(pruefen(&p, r, "3", "", 3, true), Ok((3, 0)));
        assert!(pruefen(&p, r, "-1", "0", 3, true).unwrap_err().contains("kleiner als 0"));
        assert!(pruefen(&p, r, "0", "1", 3, true).unwrap_err().contains("Schaltplan"));
        assert!(pruefen(&p, r, "0", "0", 1, true).unwrap_err().contains("Deckblatt"));
        assert_eq!(pruefen(&p, r, "0", "0", 0, true), Ok((0, 0)));
        assert_eq!(pruefen(&p, r, "0", "2", 1, true), Ok((0, 2)));
        assert!(pruefen(&p, r, "1", "0", 3, true).unwrap_err().contains("schon vorhanden"));
        // gleiche Nummer, anderer Subindex ist erlaubt
        assert_eq!(pruefen(&p, r, "1", "1", 3, true), Ok((1, 1)));
        // vorhandenes Blatt: der Dialog lässt die Nummer durch, das Umhängen meldet sie
        assert_eq!(pruefen(&p, r, "1", "0", 3, false), Ok((1, 0)));
        assert_eq!(
            verschiebung_belegt(&p, r, 1, 0).as_deref(),
            Some("Die Blattnummer: 1 mit dem Subindex: 0\nist schon vorhanden. Blattverschiebung kann nicht durchgeführt werden.")
        );
        assert_eq!(verschiebung_belegt(&p, r, 1, 1), None);
    }
}
