//! Liste der Klemmleisten-Eigenschaften wie im Original (TKLeiste „Liste“, gefüllt von 0x4830a8 mit Art 1):
//! Spalten „Klemme“ und „Verwendung auf Blatt“; die zweite zeigt für jede gesetzte Klemme
//! Blattbezeichnung + „ (“ + Blattnummer + „)“ (0x4837bc). Die Blattnummer schreibt das Original mit 0x472aec.

use crate::model::*;

/// Spaltenköpfe der Liste (0x4830a8, SetCell der Kopfzeile).
pub const KOPF: [&str; 2] = ["Klemme", "Verwendung auf Blatt"];

/// Blattnummer als Text wie 0x472aec: Nummer, mit Subindex „Nummer.Subindex“; Nummer 0 (Deckblatt, Verzeichnisse)
/// „0“ ohne Subindex, sonst Buchstaben ab „a“ (ab 27 mit vorangestellter Ziffer).
pub fn blattnummer(nummer: u32, sub: u8) -> String {
    if nummer == 0 {
        let mut s = sub as u32;
        if s == 0 {
            return "0".into();
        }
        let mut out = String::new();
        if s / 27 != 0 {
            out.push((b'0' + (s / 27) as u8) as char);
            s += 1;
        }
        out.push((0x60 + (s % 27) as u8) as char);
        return out;
    }
    if sub == 0 {
        nummer.to_string()
    } else {
        format!("{nummer}.{sub}")
    }
}

/// „Verwendung auf Blatt“ der Klemme `nr` der Leiste `leiste` in Anlage `a` (leer, wenn sie auf keinem Blatt steht).
pub fn verwendung(a: &Anlage, leiste: &str, nr: &str) -> String {
    a.sheets
        .iter()
        .find(|s| s.objects.iter().any(|o| matches!(&o.kind, Kind::Terminal { leiste: l, nr: n, .. } if l == leiste && n.to_string() == nr)))
        .map(|s| format!("{} ({})", s.name, blattnummer(s.number, s.subindex)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blattnummer_wie_original() {
        assert_eq!(blattnummer(3, 0), "3");
        assert_eq!(blattnummer(3, 2), "3.2");
        assert_eq!(blattnummer(0, 0), "0");
        assert_eq!(blattnummer(0, 1), "a");
        assert_eq!(blattnummer(0, 26), "z");
    }

    #[test]
    fn verwendung_der_klemme() {
        let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
        let p = crate::epr::import(std::path::Path::new(&f)).unwrap();
        let (a, s, leiste, nr) = p
            .anlagen
            .iter()
            .find_map(|a| a.sheets.iter().find_map(|s| s.objects.iter().find_map(|o| match &o.kind {
                Kind::Terminal { leiste, nr, .. } if !leiste.is_empty() => Some((a, s, leiste.clone(), nr.to_string())),
                _ => None,
            })))
            .expect("W2 hat Klemmen");
        let v = verwendung(a, &leiste, &nr);
        assert!(v.starts_with(&s.name) && v.ends_with(&format!("({})", blattnummer(s.number, s.subindex))), "{v}");
        assert_eq!(verwendung(a, &leiste, "9999"), "");
    }
}
