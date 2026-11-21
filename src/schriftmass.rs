//! Maße einer installierten Schrift ohne Zeichenfläche (für Umriss, Treffertest und Griffe von Texten in ihrer
//! eigenen Schrift, `textmass::text_umriss`).
//!
//! Gelesen wird dieselbe Schriftdatei, die `schriften::Lader` für egui lädt (`schriften::finden`), mit derselben
//! Bibliothek wie egui (skrifa): Zeichenbreite = Vorschub der Glyphe (hmtx), Zeilenhöhe = Ober- + Unterlänge +
//! Zeilenabstand, beides in Geviert. Zeichen, die der Schrift fehlen, zeichnet egui in Times (Ersatzkette) – sie
//! zählen hier mit der Times-Breite. Ohne Schriftdatei (nicht installiert, Times) gilt Times (`textmass`).

use skrifa::instance::{LocationRef, Size};
use skrifa::MetadataProvider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Metrik einer Schriftdatei (bzw. eines Schnitts in einer Sammlung)
pub struct Metrik {
    daten: Arc<Vec<u8>>,
    index: u32,
    /// Zeilenhöhe in Geviert
    pub zeile: f32,
    /// Zeichenbreiten in Geviert (`None` = fehlt in der Schrift), beim ersten Gebrauch ermittelt
    breiten: Mutex<HashMap<char, Option<f32>>>,
}

impl Metrik {
    /// Metrik aus den Bytes einer Schriftdatei; `None`, wenn sie nicht lesbar ist.
    pub fn aus_daten(daten: Arc<Vec<u8>>, index: u32) -> Option<Metrik> {
        let f = skrifa::FontRef::from_index(&daten, index).ok()?;
        let m = f.metrics(Size::unscaled(), LocationRef::default());
        let upem = m.units_per_em as f32;
        if upem <= 0.0 {
            return None;
        }
        let zeile = (m.ascent - m.descent + m.leading) / upem;
        if !(zeile > 0.1 && zeile < 10.0) {
            return None;
        }
        Some(Metrik { daten, index, zeile, breiten: Mutex::new(HashMap::new()) })
    }

    /// Breite eines Zeichens in Geviert, `None` wenn die Schrift es nicht enthält.
    pub fn zeichen(&self, c: char) -> Option<f32> {
        if let Some(w) = self.breiten.lock().ok().and_then(|b| b.get(&c).copied()) {
            return w;
        }
        let w = (|| {
            let f = skrifa::FontRef::from_index(&self.daten, self.index).ok()?;
            let upem = f.metrics(Size::unscaled(), LocationRef::default()).units_per_em as f32;
            // Tabulator wie egui: vier Leerzeichen
            let (c, n) = if c == '\t' { (' ', 4.0) } else { (c, 1.0) };
            let g = f.charmap().map(c)?;
            let w = f.glyph_metrics(Size::unscaled(), LocationRef::default()).advance_width(g)?;
            Some(w / upem * n)
        })();
        if let Ok(mut b) = self.breiten.lock() {
            b.insert(c, w);
        }
        w
    }
}

/// Metrik der Schrift, mit der ein Text mit Schriftname `name` und Schnitt gezeichnet wird; `None` für Times
/// (bzw. leer) und nicht installierte Schriften. Zwischengespeichert je Name und Schnitt.
pub fn metrik(name: &str, bold: bool, italic: bool) -> Option<Arc<Metrik>> {
    if crate::schriften::ist_times(name) {
        return None;
    }
    type Speicher = Mutex<HashMap<(String, bool, bool), Option<Arc<Metrik>>>>;
    static C: OnceLock<Speicher> = OnceLock::new();
    let key = (name.trim().to_ascii_lowercase(), bold, italic);
    let c = C.get_or_init(Default::default);
    if let Some(m) = c.lock().ok()?.get(&key) {
        return m.clone();
    }
    let m = crate::schriften::finden(name, bold, italic)
        .and_then(|s| crate::schriften::datei_inhalt(&s.pfad).map(|d| (d, s.index)))
        .and_then(|(d, i)| Metrik::aus_daten(d, i))
        .map(Arc::new);
    c.lock().ok()?.insert(key, m.clone());
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Schrift aus den egui-Standardschriften (Hack = dicktengleich, also breiter als Times bei „iiii“)
    fn hack() -> Metrik {
        let defs = eframe::egui::FontDefinitions::default();
        let d = defs.font_data.get("Hack").expect("egui-Schrift Hack");
        Metrik::aus_daten(Arc::new(d.bytes().to_vec()), d.index).unwrap()
    }

    #[test]
    fn breiten_aus_der_schrift() {
        let m = hack();
        let i = m.zeichen('i').unwrap();
        let w = m.zeichen('W').unwrap();
        // dicktengleich: i und W gleich breit, etwa 0,6 Geviert
        assert!((i - w).abs() < 1e-4 && (0.5..0.7).contains(&i), "{i} {w}");
        assert!(m.zeile > 1.0 && m.zeile < 1.5, "{}", m.zeile);
        assert_eq!(m.zeichen('\t'), Some(4.0 * m.zeichen(' ').unwrap()));
        // fehlendes Zeichen
        assert_eq!(m.zeichen('\u{10FFFD}'), None);
        assert!(Metrik::aus_daten(Arc::new(vec![1, 2, 3]), 0).is_none());
    }

    #[test]
    fn times_und_fehlende_schrift_ohne_metrik() {
        assert!(metrik("", false, false).is_none());
        assert!(metrik("Times New Roman", true, false).is_none());
        assert!(metrik("Gibt Es Nicht 4711", false, false).is_none());
    }
}
