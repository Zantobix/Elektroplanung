//! Anschlusskreise wie im Original: An jedem Anschluss eines Symbols zeichnet das Original auf dem Bildschirm einen
//! kleinen hellgrauen Kreis (Ø ≈ 1,6 mm), auch an angeschlossenen Anschlüssen. Im Ausdruck erscheinen sie nicht
//! (Gegenprobe Original unter Wine und Original-Ausdruck, 22.11.2026) – deshalb nur in der Bildschirmansicht.

use crate::canvas::{obj_pins, Xf};
use crate::model::{Kind, Sheet};
use crate::symbols::SymDef;
use eframe::egui::{Color32, Painter, Stroke};

/// Halbmesser in mm.
pub const R: f32 = 0.8;
/// Farbe (hellgrau wie im Original)
pub const FARBE: Color32 = Color32::from_rgb(192, 192, 192);

/// Mittelpunkte aller Anschlusskreise eines Blattes (nur Symbole).
pub fn punkte(sheet: &Sheet, lib: &[SymDef]) -> Vec<crate::model::P> {
    sheet.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| obj_pins(o, lib)).collect()
}

/// Zeichnet die Anschlusskreise (Bildschirm, nicht Druck/Export).
pub fn zeichnen(p: &Painter, xf: Xf, sheet: &Sheet, lib: &[SymDef]) {
    let st = Stroke::new((0.12 * xf.z).max(1.0), FARBE);
    for q in punkte(sheet, lib) {
        p.circle_stroke(xf.s(q), R * xf.z, st);
    }
}
