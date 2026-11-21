//! Prüfung von Blattnummer und Subindex wie im Original (TSBlatt: Blatt_NummerChange 0x4a560c,
//! Prüfung beim Übernehmen 0x4a51e4 und Doppelte-Nummer-Prüfung im OK-Handler):
//! - Die Blattnummer darf nicht kleiner als 0 sein.
//! - Nummer 0 ist mit der Blattart „Schaltplan“ (3) nicht erlaubt.
//! - Nummer 0 mit Subindex 0 gibt es nur für die Blattart „Deckblatt“ (0).
//! - Nummer und Subindex sind je Anlage eindeutig; ist das andere Blatt ein Deckblatt, meldet das Original
//!   „Es ist schon ein Deckblatt für diese Anlage vorhanden!“.

use crate::model::*;

/// Prüft Nummer/Subindex für das Blatt `r` mit der Blattart `art`; liefert (Nummer, Subindex) oder die Meldung.
pub fn pruefen(p: &Project, r: SheetRef, nummer: &str, subindex: &str, art: u8) -> Result<(u32, u8), String> {
    let n: i64 = nummer.trim().parse().map_err(|_| "Die Blattnummer ist keine Zahl!".to_string())?;
    if n < 0 {
        return Err("Die Blattnummer darf nicht kleiner als 0 sein!".into());
    }
    let sub: u8 = if subindex.trim().is_empty() { 0 } else { subindex.trim().parse().map_err(|_| "Der Subindex muss zwischen 0 und 255 liegen!".to_string())? };
    if n == 0 && art == 3 {
        return Err("Die Blattnummer '0' kann nicht mit der Blatteinstellung 'Schaltplan'\nvergeben werden!".into());
    }
    if n == 0 && sub == 0 && art != 0 {
        return Err("Die Blattnummer '0' in Verbindung mit dem Subindex '0'\nkann nur mit der Blatteinstellung 'Deckblatt' vergeben werden!".into());
    }
    let n = n as u32;
    if let Some(a) = p.anlagen.get(r.anlage) {
        if a.sheets.iter().enumerate().any(|(i, x)| i != r.sheet && x.number == n && x.subindex == sub) {
            return Err(if art == 0 { "Es ist schon ein Deckblatt für diese Anlage vorhanden!" } else { "Ein Blatt mit dieser Blattnummer ist bei dieser Anlage schon vorhanden!" }.into());
        }
    }
    Ok((n, sub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regeln_wie_im_original() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets.push(Sheet::new(2));
        let r = SheetRef { anlage: 0, sheet: 1 };
        assert_eq!(pruefen(&p, r, "3", "", 3), Ok((3, 0)));
        assert!(pruefen(&p, r, "-1", "0", 3).unwrap_err().contains("kleiner als 0"));
        assert!(pruefen(&p, r, "0", "1", 3).unwrap_err().contains("Schaltplan"));
        assert!(pruefen(&p, r, "0", "0", 1).unwrap_err().contains("Deckblatt"));
        assert_eq!(pruefen(&p, r, "0", "0", 0), Ok((0, 0)));
        assert_eq!(pruefen(&p, r, "0", "2", 1), Ok((0, 2)));
        assert!(pruefen(&p, r, "1", "0", 3).unwrap_err().contains("schon vorhanden"));
        // gleiche Nummer, anderer Subindex ist erlaubt
        assert_eq!(pruefen(&p, r, "1", "1", 3), Ok((1, 1)));
    }
}
