//! Kleine Sperr- und Eingaberegeln der Dialoge, aus den Handlern des Originals übernommen (Runde 5, M1):
//! - Drucken (TForm3): AlleClick 0x4adc78 sperrt die Anlagenwahl bei „Aktuelles Blatt“ (Feld +0x310 = Blatt)
//!   oder wenn das Projekt weniger als zwei Anlagen hat.
//! - Numerische Eingabefelder (TIn mit Numeric=True und Kommastellen): Anzeige mit Komma und fester Zahl von
//!   Nachkommastellen, z. B. Querschnitt der Leitung „1,50“.

/// Anlagenwahl im Druckdialog frei? `art` 1 = „Aktuelles Blatt“.
pub fn druck_anlage_frei(art: usize, anlagen: usize) -> bool {
    art != 1 && anlagen >= 2
}

/// Vorwahl der Anlagenliste im Druckdialog (0x49c75c): Eintrag 0 „Alle“ bei keiner oder mehreren Anlagen, bei
/// genau einer Anlage diese (Eintrag 1).
pub fn druck_anlage_vorwahl(anlagen: usize) -> usize {
    usize::from(anlagen == 1)
}

/// Zahl wie ein numerisches TIn anzeigen (Komma, `stellen` Nachkommastellen); ungültige Eingaben bleiben stehen.
pub fn zahl_text(s: &str, stellen: usize) -> String {
    match s.trim().replace(',', ".").parse::<f64>() {
        Ok(v) if v.is_finite() => format!("{v:.stellen$}").replace('.', ","),
        _ => s.to_string(),
    }
}

/// Wie `zahl_text`, leere Eingabe als 0 (TIn.FSetZahl zeigt 0 mit Kommastellen als „0,000“).
pub fn zahl_text_leer0(s: &str, stellen: usize) -> String {
    zahl_text(if s.trim().is_empty() { "0" } else { s }, stellen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potential_anzeige() {
        assert_eq!(zahl_text_leer0("230", 3), "230,000");
        assert_eq!(zahl_text_leer0("", 3), "0,000");
        assert_eq!(zahl_text_leer0("0,5", 3), "0,500");
    }

    #[test]
    fn druck_anlage_gesperrt() {
        assert!(druck_anlage_frei(0, 2));
        assert!(!druck_anlage_frei(1, 3), "Aktuelles Blatt");
        assert!(!druck_anlage_frei(2, 1), "nur eine Anlage");
    }

    #[test]
    fn zahlen_wie_tin() {
        assert_eq!(zahl_text("1.5", 2), "1,50");
        assert_eq!(zahl_text(" 3 ", 0), "3");
        assert_eq!(zahl_text("2,456", 2), "2,46");
        assert_eq!(zahl_text("abc", 2), "abc");
    }
}
