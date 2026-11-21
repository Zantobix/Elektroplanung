//! Freigaben einer Symboldefinition (Original: TSymEigenschaften › „Freigabe für Querverweise“ / „Freigabe für
//! Sicherungsübersicht“), Vorgabe für neu gesetzte Symbole.
//!
//! Ablage im SYM-Block des Projekts: u32 bei Byte 112 (nach Name 0–40, BMK 40–60, Kennung 80–88, Datum 104–112),
//! Bit 0 = Querverweise, Bit 1 = Sicherungsübersicht. Ermittelt an W2.epr: RCD und Leitungsschutzschalter tragen 3
//! (ihre Instanzen haben fast alle VSY Feld 20 Bit 24 = Sicherungsübersicht), die Klemme 0, alle übrigen 1. Ältere
//! Symbole haben dort teils Zufallsbytes (z. B. Text "5626"); solche Werte gelten als unbekannt (Vorgabe
//! Querverweise an, Sicherungsübersicht aus) und bleiben beim Schreiben unverändert, solange die Freigaben gleich
//! der Vorgabe sind.
//!
//! Bibliothek (Symbole.tfi): Der Symbolkopf eines Datensatzes (49 Byte: Merker 01, BMK-Vorlage 20 Byte, Lagen …)
//! trägt denselben Wert als u32 ab Byte 45 (ermittelt an der Original-Datenbank: RCD, Leitungsschutzschalter und
//! Sicherung 3, Klemme, Pfeil, Motorschutz u. a. 0, die übrigen 1; Textreste dort oder ein leerer Kopf mit
//! Merker 0 = unbekannt). Beim Laden aus der Bibliothek wird er übernommen (`aus_tfi_kopf`).

/// Byte-Lage der Freigaben im SYM-Block
pub const LAGE: usize = 112;

/// Vorgabe, wenn die Datei keinen gültigen Wert enthält (Querverweise an, Sicherungsübersicht aus)
pub const VORGABE: (bool, bool) = (true, false);

/// Freigaben (Querverweise, Sicherungsübersicht) aus den Daten eines SYM-Blocks; `None` bei fehlendem oder
/// ungültigem Wert.
pub fn aus_sym(data: &[u8]) -> Option<(bool, bool)> {
    let b = data.get(LAGE..LAGE + 4)?;
    let v = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    (v <= 3).then_some((v & 1 != 0, v & 2 != 0))
}

/// Byte-Lage der Freigaben im Symbolkopf eines Symbole.tfi-Datensatzes (nach dem Merker 01)
pub const LAGE_TFI: usize = 45;

/// Freigaben aus dem Symbolkopf eines Bibliothekssymbols (Symbole.tfi); `None` bei leerem Kopf (Merker 0) oder
/// ungültigem Wert.
pub fn aus_tfi_kopf(kopf: &[u8]) -> Option<(bool, bool)> {
    if kopf.first() != Some(&1) {
        return None;
    }
    let b = kopf.get(LAGE_TFI..LAGE_TFI + 4)?;
    let v = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    (v <= 3).then_some((v & 1 != 0, v & 2 != 0))
}

/// Freigaben in die Daten eines SYM-Blocks schreiben (nur wenn sie sich vom gelesenen Wert unterscheiden).
pub fn setzen(data: &mut Vec<u8>, frg: (bool, bool)) {
    let alt = aus_sym(data);
    if alt == Some(frg) || (alt.is_none() && data.len() >= LAGE + 4 && frg == VORGABE) {
        return;
    }
    if data.len() < LAGE + 4 {
        data.resize(LAGE + 4, 0);
    }
    let v = frg.0 as u32 | (frg.1 as u32) << 1;
    data[LAGE..LAGE + 4].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesen_und_schreiben() {
        let mut d = vec![0u8; 116];
        d[LAGE] = 3;
        assert_eq!(aus_sym(&d), Some((true, true)));
        d[LAGE] = 0;
        assert_eq!(aus_sym(&d), Some((false, false)));
        setzen(&mut d, (true, false));
        assert_eq!(&d[LAGE..], &[1, 0, 0, 0]);
        setzen(&mut d, (false, true));
        assert_eq!(&d[LAGE..], &[2, 0, 0, 0]);
        // Zufallsbytes älterer Symbole: unbekannt, bleiben bei der Vorgabe stehen
        d[LAGE..].copy_from_slice(b"5626");
        assert_eq!(aus_sym(&d), None);
        setzen(&mut d, VORGABE);
        assert_eq!(&d[LAGE..], b"5626");
        setzen(&mut d, (true, true));
        assert_eq!(&d[LAGE..], &[3, 0, 0, 0]);
        // zu kurzer Block wird verlängert
        let mut k = vec![0u8; 88];
        setzen(&mut k, (true, true));
        assert_eq!(k.len(), 116);
        assert_eq!(aus_sym(&k), Some((true, true)));
        assert_eq!(aus_sym(&[0u8; 100]), None);
    }

    #[test]
    fn bibliothekskopf() {
        let mut k = vec![0u8; 49];
        assert_eq!(aus_tfi_kopf(&k), None, "leerer Kopf");
        k[0] = 1;
        k[LAGE_TFI] = 3;
        assert_eq!(aus_tfi_kopf(&k), Some((true, true)));
        k[LAGE_TFI] = 0;
        assert_eq!(aus_tfi_kopf(&k), Some((false, false)));
        k[LAGE_TFI..].copy_from_slice(b"5161");
        assert_eq!(aus_tfi_kopf(&k), None, "Textreste");
        assert_eq!(aus_tfi_kopf(&k[..40]), None);
    }
}
