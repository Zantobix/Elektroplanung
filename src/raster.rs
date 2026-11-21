//! Rasterfang und Rastergitter wie im Original.
//!
//! Im Original (Elektroplanung 6.4.2) gehört das Raster nicht zum Projekt: die Rasterauswahl der Werkzeugleiste
//! (Form1 +0x52c, TAuswahl „Raster“) wird nur aus der Starteinstellung der Optionen gesetzt (Form1 +0x9f0, in der
//! Elektroplanung.INI Byte 0x30; Vorgabe 6 = 10,0mm, Routine 0x41fc20 beim ersten Anzeigen des Fensters) und
//! weder im .epr (PRJ/ANL/BLT) noch beim Wechsel des Blattes gespeichert. RasterChange (0x475c8c) zeichnet nur neu.
//!
//! Fangen (0x41645c, Koordinaten in 0,1 mm): Rasterweite aus dem Index (Kein 0,1 / 1,0 / 2,5 / 3,5 / 5 / 7 / 10 /
//! 14 mm), gerundet wird auf absolute Rasterpunkte `Ursprung + k · Weite` (Rest ≤ 0,49 Weite abrunden, sonst weg
//! vom Ursprung). Der Ursprung ist mit „Rand ist Ursprung“ (Form1 +0xa37, INI Byte 0x77) die linke obere Ecke der
//! Zeichenfläche: quer x = ANL +0xa4 (linker Rand), y = ANL +0xa8 (oberer Rand) + ANL +0xb4 (Höhe der
//! Strompfadleiste); hoch x = ANL +0xa8, y = ANL +0xa0 + ANL +0xb4 (beim Blattwechsel in Form1 +0xb7c/+0xb80).
//! Ohne „Rand ist Ursprung“ liegt er in der Blattecke. Verschieben rechnet mit dem gefangenen Mauspunkt beim
//! Drücken und jetzt, d. h. der Verschiebeweg ist ein Vielfaches der Rasterweite – Bestandsobjekte, die nicht auf
//! dem Raster liegen, bleiben dabei um denselben Rest daneben (kein Springen).
//!
//! Gitter (Zeichenroutine 0x438b94): durchgezogene Linien ab dem Ursprung nach rechts/unten im Abstand der
//! Rasterweite, nur wenn der Abstand auf dem Bildschirm mindestens 5 Pixel beträgt (sonst kein Gitter); Farbe =
//! Blatthintergrund um 1/16 abgedunkelt.
//!
//! Abweichung des Nachbaus: die zuletzt gewählte Rasterweite wird je Projektdatei in den Programmeinstellungen
//! gemerkt und beim Öffnen wieder eingestellt (das .epr bleibt unverändert, das Original kennt dafür kein Feld).

use crate::app::{App, RASTER};
use crate::model::*;

/// Kleinster Abstand der Gitterlinien in Bildpunkten (Original: `5.0 <= Abstand`).
pub const MIN_GITTER_PX: f32 = 5.0;

/// Wie viele Projektdateien sich ihre Rasterweite merken.
const MERKEN_MAX: usize = 50;

/// Rasterursprung (mm) eines Blattes wie im Original (`rand` = Option „Rand ist Ursprung“).
pub fn ursprung(sheet: &Sheet, anl: &AnlSettings, rand: bool) -> P {
    if !rand {
        return P::new(0.0, 0.0);
    }
    // Blattarten ab 'a' (Sonderblätter des Originals): Ursprung in der Blattecke, 'b' nur senkrecht
    match sheet.art {
        b'a' | b'c' => return P::new(0.0, 0.0),
        _ => {}
    }
    let [rechts, links, oben, _unten] = anl.raender;
    let (x, y) = if sheet.portrait { (oben, rechts) } else { (links, oben) };
    let y = if sheet.art == b'b' { 0 } else { y + anl.pfad_hoehe };
    P::new(x as f32 / 10.0, y as f32 / 10.0)
}

/// Eine Koordinate auf das Raster `weite` mit Ursprung `o` fangen (absolute Rasterpunkte).
pub fn fang_wert(v: f32, weite: f32, o: f32) -> f32 {
    if weite <= 0.0 {
        return (v * 10.0).round() / 10.0;
    }
    let r = o + ((v - o) / weite).round() * weite;
    // auf 0,1 mm runden wie die ganzzahligen Koordinaten des Originals
    (r * 10.0).round() / 10.0
}

/// Punkt auf das Raster fangen.
pub fn fang(p: P, weite: f32, o: P) -> P {
    P::new(fang_wert(p.x, weite, o.x), fang_wert(p.y, weite, o.y))
}

/// Lagen der Gitterlinien (mm) einer Achse: ab dem Ursprung (ohne „Rand ist Ursprung“ ab der ersten Weite) bis
/// `ende`; leer, wenn der Abstand auf dem Bildschirm (`weite · zoom`) unter 5 Pixeln liegt.
pub fn gitter_linien(o: f32, weite: f32, zoom: f32, ende: f32, rand: bool) -> Vec<f32> {
    if weite <= 0.0 || weite * zoom < MIN_GITTER_PX {
        return vec![];
    }
    let mut v = vec![];
    let mut i = if rand { 0 } else { 1 };
    loop {
        let x = o + i as f32 * weite;
        if x > ende + 1e-3 {
            break;
        }
        v.push(x);
        i += 1;
    }
    v
}

impl App {
    /// Rasterursprung des aktiven Blattes (Symboleditor: wie bisher 5 mm).
    pub fn raster_ursprung(&self) -> P {
        if self.sym_edit.is_some() {
            return P::new(GRID_ORIGIN, GRID_ORIGIN);
        }
        let Some(r) = self.cur() else { return P::new(GRID_ORIGIN, GRID_ORIGIN) };
        match (self.project.anlagen.get(r.anlage), self.project.sheet(r)) {
            (Some(a), Some(s)) => ursprung(s, &a.set, self.opt.rand_ursprung),
            _ => P::new(GRID_ORIGIN, GRID_ORIGIN),
        }
    }

    /// Gewählte Rasterweite für die aktuelle Projektdatei merken (Abweichung des Nachbaus, siehe Modulkopf).
    pub fn raster_merken(&mut self) {
        let Some(p) = &self.path else { return };
        let k = p.to_string_lossy().to_string();
        let l = &mut self.opt.raster_je_projekt;
        l.retain(|(n, _)| n != &k);
        l.insert(0, (k, self.raster_idx));
        l.truncate(MERKEN_MAX);
    }

    /// Beim Öffnen: gemerkte Rasterweite der Projektdatei einstellen (sonst bleibt die aktuelle Auswahl).
    pub fn raster_laden(&mut self) {
        let Some(p) = &self.path else { return };
        let k = p.to_string_lossy().to_string();
        if let Some((_, i)) = self.opt.raster_je_projekt.iter().find(|(n, _)| n == &k) {
            self.raster_idx = (*i).min(RASTER.len() - 1);
        }
    }

    /// Schrittweite der Pfeiltasten wie im Original (0x43f054): Rasterweite, mit Umschalt 0,1 mm; der
    /// ausgeschaltete Rasterfang (im Original „Kein“) ebenfalls 0,1 mm.
    pub fn pfeil_schritt(&self, shift: bool) -> f32 {
        if shift || self.fang() <= 0.0 {
            0.1
        } else {
            self.fang()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blatt(hoch: bool) -> Sheet {
        Sheet { portrait: hoch, ..Sheet::new(1) }
    }

    #[test]
    fn ursprung_wie_original() {
        let a = AnlSettings::default(); // Ränder 50/50/100/50, Strompfadleiste 5 mm (wie W2)
        assert_eq!(ursprung(&blatt(false), &a, true), P::new(5.0, 15.0));
        assert_eq!(ursprung(&blatt(true), &a, true), P::new(10.0, 10.0));
        assert_eq!(ursprung(&blatt(false), &a, false), P::new(0.0, 0.0));
        let mut s = blatt(false);
        s.art = b'b';
        assert_eq!(ursprung(&s, &a, true), P::new(5.0, 0.0));
        s.art = b'a';
        assert_eq!(ursprung(&s, &a, true), P::new(0.0, 0.0));
    }

    #[test]
    fn fangen_absolut_auf_rasterpunkte() {
        let o = P::new(5.0, 15.0);
        assert_eq!(fang(P::new(12.0, 21.0), 10.0, o), P::new(15.0, 25.0));
        assert_eq!(fang(P::new(9.9, 19.9), 10.0, o), P::new(5.0, 15.0));
        // 7 mm: Raster ab dem Rand, nicht ab der Blattecke
        assert_eq!(fang(P::new(20.0, 20.0), 7.0, o), P::new(19.0, 22.0));
        // hoch: 10-mm-Raster liegt auf vollen Zentimetern
        assert_eq!(fang(P::new(14.0, 26.0), 10.0, P::new(10.0, 10.0)), P::new(10.0, 30.0));
        // ohne Fang: 0,1 mm
        assert_eq!(fang(P::new(14.04, 26.06), 0.0, o), P::new(14.0, 26.1));
    }

    #[test]
    fn verschieben_in_rasterschritten_laesst_rest_stehen() {
        // Symbol eines Bestandsblattes neben dem Raster (x = 42,5 bei 10 mm): Verschiebeweg = Differenz der
        // gefangenen Mauspunkte, die Lage behält ihren Rest von 7,5 mm
        let o = P::new(5.0, 15.0);
        let pos = P::new(42.5, 57.3);
        let start = fang(P::new(43.0, 58.0), 10.0, o);
        let jetzt = fang(P::new(64.0, 79.0), 10.0, o);
        let neu = pos.add(jetzt.sub(start));
        assert!((neu.x - 62.5).abs() < 1e-4 && (neu.y - 77.3).abs() < 1e-4, "{neu:?}");
    }

    #[test]
    fn gitter_ab_ursprung_und_erst_ab_5_pixeln() {
        assert!(gitter_linien(5.0, 10.0, 0.4, 297.0, true).is_empty());
        let v = gitter_linien(5.0, 10.0, 4.0, 30.0, true);
        assert_eq!(v, vec![5.0, 15.0, 25.0]);
        let v = gitter_linien(0.0, 10.0, 4.0, 30.0, false);
        assert_eq!(v, vec![10.0, 20.0, 30.0]);
        // 1 mm bei 4 px/mm: unter 5 Pixeln, kein Gitter (das Original vergröbert nicht)
        assert!(gitter_linien(5.0, 1.0, 4.0, 297.0, true).is_empty());
        assert_eq!(gitter_linien(5.0, 1.0, 5.0, 7.0, true).len(), 3);
    }
}
