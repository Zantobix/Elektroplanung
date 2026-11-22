//! Einspeisungskreis an Sammelschienen (todo „Einspeisungskreis an Sammelschienen“).
//!
//! Blattzeichnung 0x438b94, Zweig Sonderobjekt Art 1 (Sammelschiene), 0x43cab0–0x43cbea: Nach den Kreisen an den
//! Abgriffen zeichnet das Original einen Kreis (Ellipse 0x548c94, Radius 0,8 mm = ±8 in 0,1 mm um die Lage
//! +0x18/+0x1c der Schiene, also um ihren **Anfang**), wenn in den Einstellungen der Schiene (SMS)
//! - „Klemmen“ gesetzt ist (Byte 0x31),
//! - die Kennung der Einspeisung (Byte 0x38, 8 Byte) nicht 0 ist und
//! - die Blattkennung bei Byte 0x40 (8 Byte) die Kennung des gezeichneten Blattes ist (BLT Byte 72).
//!
//! Die Ellipse füllt der Pinsel des Zeichenkontexts; vorher setzt die Schleife über die Abgriffe Farbe 0 und
//! Stil 0 (schwarz, voll) – der Nachbau zeichnet den Kreis deshalb gefüllt in der Linienfarbe.

use crate::model::{SchieneSet, Sheet};

/// Radius des Kreises in mm.
pub const RADIUS: f32 = 0.8;

/// Kennung eines aus der Datei gelesenen Blattes (BLT Byte 72); neue Blätter haben noch keine.
pub fn blatt_kennung(s: &Sheet) -> Option<u64> {
    s.raw.as_deref().and_then(|r| r.block.data.get(72..80)).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

/// Zeichnet das Original auf diesem Blatt den Einspeisungskreis am Anfang der Schiene?
pub fn kreis(set: &SchieneSet, blatt: Option<u64>) -> bool {
    set.klemmen && set.einspeisung && set.einspeisung_blatt != 0 && blatt == Some(set.einspeisung_blatt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nur_mit_klemmen_auf_dem_blatt_der_einspeisung() {
        let mut set = SchieneSet { klemmen: true, einspeisung: true, einspeisung_blatt: 7, ..Default::default() };
        assert!(kreis(&set, Some(7)));
        assert!(!kreis(&set, Some(8)));
        assert!(!kreis(&set, None));
        set.klemmen = false;
        assert!(!kreis(&set, Some(7)));
        set.klemmen = true;
        set.einspeisung = false;
        assert!(!kreis(&set, Some(7)));
    }

    /// W2: jede Schiene mit Einspeisung trägt die Kennung eines Blattes des Projekts (in kopierten Anlagen auch
    /// eines Blattes der Anlage, aus der kopiert wurde – dort erscheint der Kreis dann nicht).
    #[test]
    fn w2_einspeisung_mit_blattkennung() {
        let Some(p) = std::env::var_os("EP_SAMPLE") else { return };
        let pr = crate::epr::import(std::path::Path::new(&p)).unwrap();
        let alle: Vec<u64> = pr.anlagen.iter().flat_map(|x| &x.sheets).filter_map(blatt_kennung).collect();
        let mit: Vec<&SchieneSet> = pr.anlagen.iter().flat_map(|a| a.schienen.values()).filter(|s| s.einspeisung).collect();
        assert!(!mit.is_empty());
        assert!(mit.iter().all(|s| alle.contains(&s.einspeisung_blatt)));
    }
}
