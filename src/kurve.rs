//! Werkzeug „Neue Kurve“ und Griffe der Kontrollpunkte wie im Original.
//!
//! Original (TForm1, Werkzeug +0x78e = 2, Schrittzähler +0x78c): die Kurve entsteht aus vier Mausklicks –
//! 1. Anfangspunkt, 2. Endpunkt (bis dahin folgt eine gerade Linie der Maus), 3. erster Kontrollpunkt (in der
//! Vorschau liegen beide Kontrollpunkte auf der Maus), 4. zweiter Kontrollpunkt. Beim Anlegen ordnet das Original die
//! Klickfolge [1, 2, 3, 4] in die gespeicherte Bézierfolge [Anfang, Kontrolle 1, Kontrolle 2, Ende] = [1, 3, 4, 2] um.
//! Eine markierte Kurve zeigt kleine graue Quadrate (7 px) an Anfang und Ende und grüne gefüllte Kreise (7 px) an
//! den Kontrollpunkten, je mit einer grünen Hilfslinie zum zugehörigen Endpunkt; alle vier Punkte lassen sich mit
//! der Maus verschieben (Zieh-Index +0x8e0).

use crate::canvas::Xf;
use crate::model::{Kind, P};
use eframe::egui::{self, Color32, Rect, Stroke, StrokeKind, Vec2};

/// Grün der Kontrollpunkte und Hilfslinien (Bildschirmfoto des Originals).
pub const GRIFF_GRUEN: Color32 = Color32::from_rgb(0, 112, 11);
/// Grau der Quadrate an Anfang und Ende.
pub const GRIFF_GRAU: Color32 = Color32::from_rgb(112, 112, 123);
/// Kantenlänge der Griffe in Bildschirmpunkten (Original: Rechteck −3 … +4 px).
pub const GRIFF_PX: f32 = 7.0;

/// Eine im Entstehen begriffene Kurve (zwischen den Klicks).
#[derive(Clone, Debug, PartialEq)]
pub struct NeueKurve {
    pub a: P,
    pub b: Option<P>,
    pub c1: Option<P>,
}

/// Was die Vorschau beim Aufziehen zeigt.
#[derive(Clone, Debug, PartialEq)]
pub enum Vorschau {
    /// Noch kein Endpunkt: gerade Linie vom Anfang zur Maus
    Linie(P, P),
    /// Endpunkt gesetzt: Kurve (Kontrollpunkte auf der Maus bzw. erster schon gesetzt)
    Kurve(Kind),
}

impl NeueKurve {
    pub fn neu(a: P) -> Self {
        NeueKurve { a, b: None, c1: None }
    }

    /// Nummer des nächsten Klicks (2 = Endpunkt, 3 = erster, 4 = zweiter Kontrollpunkt).
    pub fn schritt(&self) -> u8 {
        match (self.b, self.c1) {
            (None, _) => 2,
            (Some(_), None) => 3,
            _ => 4,
        }
    }

    /// Hinweis für die Statuszeile.
    pub fn hinweis(&self) -> &'static str {
        match self.schritt() {
            2 => "Neue Kurve: Endpunkt anklicken (Rechtsklick/Esc bricht ab)",
            3 => "Neue Kurve: ersten Kontrollpunkt anklicken",
            _ => "Neue Kurve: zweiten Kontrollpunkt anklicken",
        }
    }

    /// Nächster Klick bei `p`. Liefert nach dem vierten Klick die fertige Kurve. Ein Endpunkt auf dem Anfang
    /// wird übergangen (keine Kurve der Länge null).
    pub fn klick(&mut self, p: P) -> Option<Kind> {
        match (self.b, self.c1) {
            (None, _) => {
                if p.dist(self.a) >= 0.01 {
                    self.b = Some(p);
                }
                None
            }
            (Some(_), None) => {
                self.c1 = Some(p);
                None
            }
            (Some(b), Some(c1)) => Some(Kind::Curve { a: self.a, c1, c2: p, b }),
        }
    }

    /// Vorschau mit der Maus bei `m` (wie das Original: vor dem Endpunkt eine Linie, danach die Kurve mit
    /// beiden bzw. dem zweiten Kontrollpunkt auf der Maus).
    pub fn vorschau(&self, m: P) -> Vorschau {
        match (self.b, self.c1) {
            (None, _) => Vorschau::Linie(self.a, m),
            (Some(b), None) => Vorschau::Kurve(Kind::Curve { a: self.a, c1: m, c2: m, b }),
            (Some(b), Some(c1)) => Vorschau::Kurve(Kind::Curve { a: self.a, c1, c2: m, b }),
        }
    }
}

/// Die vier Griffpunkte einer Kurve in der Reihenfolge Anfang, Kontrolle 1, Kontrolle 2, Ende.
pub fn punkte(k: &Kind) -> Option<[P; 4]> {
    match k {
        Kind::Curve { a, c1, c2, b } => Some([*a, *c1, *c2, *b]),
        _ => None,
    }
}

/// Griff einer Kurve in Reichweite `tol` (mm) um `m`: 0 Anfang, 1 Kontrolle 1, 2 Kontrolle 2, 3 Ende.
/// Bei Überdeckung gewinnt der nächstgelegene, bei gleichem Abstand ein Kontrollpunkt (liegt er auf einem
/// Endpunkt, bliebe er sonst unerreichbar).
pub fn griff_bei(k: &Kind, m: P, tol: f32) -> Option<usize> {
    let p = punkte(k)?;
    let mut best: Option<(f32, usize)> = None;
    for i in [1, 2, 0, 3] {
        let d = p[i].dist(m);
        if d <= tol && best.map(|(bd, _)| d < bd - 1e-4).unwrap_or(true) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

/// Griff `i` einer Kurve auf `p` setzen.
pub fn griff_setzen(k: &mut Kind, i: usize, p: P) {
    if let Kind::Curve { a, c1, c2, b } = k {
        match i {
            0 => *a = p,
            1 => *c1 = p,
            2 => *c2 = p,
            _ => *b = p,
        }
    }
}

/// Griffe einer markierten Kurve zeichnen: Hilfslinien Anfang–Kontrolle 1 und Ende–Kontrolle 2, grüne Kreise an
/// den Kontrollpunkten, graue Quadrate an den Endpunkten.
pub fn griffe_zeichnen(p: &egui::Painter, xf: Xf, k: &Kind) {
    let Some([a, c1, c2, b]) = punkte(k) else { return };
    let gruen = Stroke::new(1.0, GRIFF_GRUEN);
    p.line_segment([xf.s(a), xf.s(c1)], gruen);
    p.line_segment([xf.s(b), xf.s(c2)], gruen);
    for c in [c1, c2] {
        p.circle_filled(xf.s(c), GRIFF_PX / 2.0, GRIFF_GRUEN);
    }
    for e in [a, b] {
        p.rect_stroke(Rect::from_center_size(xf.s(e), Vec2::splat(GRIFF_PX)), 0.0, Stroke::new(1.0, GRIFF_GRAU), StrokeKind::Inside);
    }
}

impl crate::app::App {
    /// Griff einer markierten, nicht fixierten Kurve unter der Maus (Objekt, Griffnummer).
    pub fn kurven_griff_bei(&self, m: P, tol: f32) -> Option<(u64, usize)> {
        self.selected_objs()
            .iter()
            .rev()
            .filter(|o| !(o.fixed && self.fix_active))
            .find_map(|o| griff_bei(&o.kind, m, tol).map(|g| (o.id, g)))
    }

    /// Kurvengriff beim Ziehen auf `p` setzen.
    pub fn kurven_griff_ziehen(&mut self, id: u64, griff: usize, p: P) {
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                griff_setzen(&mut o.kind, griff, p);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vier_klicks_wie_original() {
        // Bild kurve_soll: Anfang oben rechts, Ende unten links, Kontrollpunkte senkrecht unter dem Anfang
        // bzw. waagerecht neben dem Ende
        let mut k = NeueKurve::neu(P::new(100.0, 20.0));
        assert_eq!(k.schritt(), 2);
        assert!(k.klick(P::new(40.0, 80.0)).is_none());
        assert_eq!(k.schritt(), 3);
        assert!(k.klick(P::new(100.0, 60.0)).is_none());
        assert_eq!(k.schritt(), 4);
        let fertig = k.klick(P::new(80.0, 80.0)).expect("Kurve nach dem vierten Klick");
        // Klickfolge [1,2,3,4] → Bézier [1,3,4,2]
        assert_eq!(fertig, Kind::Curve { a: P::new(100.0, 20.0), c1: P::new(100.0, 60.0), c2: P::new(80.0, 80.0), b: P::new(40.0, 80.0) });
    }

    #[test]
    fn vorschau_je_schritt() {
        let a = P::new(0.0, 0.0);
        let m = P::new(5.0, 7.0);
        let mut k = NeueKurve::neu(a);
        assert_eq!(k.vorschau(m), Vorschau::Linie(a, m));
        k.klick(P::new(10.0, 10.0));
        assert_eq!(k.vorschau(m), Vorschau::Kurve(Kind::Curve { a, c1: m, c2: m, b: P::new(10.0, 10.0) }));
        k.klick(P::new(0.0, 10.0));
        assert_eq!(k.vorschau(m), Vorschau::Kurve(Kind::Curve { a, c1: P::new(0.0, 10.0), c2: m, b: P::new(10.0, 10.0) }));
    }

    #[test]
    fn endpunkt_auf_anfang_wird_uebergangen() {
        let mut k = NeueKurve::neu(P::new(3.0, 3.0));
        assert!(k.klick(P::new(3.0, 3.0)).is_none());
        assert_eq!(k.schritt(), 2);
    }

    #[test]
    fn griffe_finden_und_setzen() {
        let mut c = Kind::Curve { a: P::new(0.0, 0.0), c1: P::new(0.0, 10.0), c2: P::new(10.0, 20.0), b: P::new(20.0, 20.0) };
        assert_eq!(griff_bei(&c, P::new(0.5, 9.5), 1.0), Some(1));
        assert_eq!(griff_bei(&c, P::new(19.6, 20.2), 1.0), Some(3));
        assert_eq!(griff_bei(&c, P::new(5.0, 5.0), 1.0), None);
        // Kontrollpunkt auf dem Endpunkt: der Kontrollpunkt gewinnt
        let d = Kind::Curve { a: P::new(0.0, 0.0), c1: P::new(0.0, 0.0), c2: P::new(1.0, 1.0), b: P::new(1.0, 1.0) };
        assert_eq!(griff_bei(&d, P::new(0.0, 0.0), 0.5), Some(1));
        assert_eq!(griff_bei(&d, P::new(1.0, 1.0), 0.5), Some(2));
        griff_setzen(&mut c, 2, P::new(15.0, 20.0));
        assert_eq!(punkte(&c).unwrap()[2], P::new(15.0, 20.0));
        griff_setzen(&mut c, 0, P::new(1.0, 1.0));
        assert_eq!(punkte(&c).unwrap()[0], P::new(1.0, 1.0));
    }
}
