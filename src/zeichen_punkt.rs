//! Punkte von Zeichenobjekten unter der Maus (todo 3 „Zeiger“, Metrik-Lücke 3).
//!
//! Original (TForm1.MouseMove): Die Trefferprüfung für Zeichenobjekte (fcn.00455274) sucht zuerst die Punkte des
//! Objekts (Index nach F+0x8e0, sonst 0xff) und erst dann den Körper (F+0x8e4). Die Zeigerwahl (0x415f30) setzt
//! bei Treffer 6 (Zeichenobjekt) **mit** Punktindex den Zeiger 5 = Ressource 2 (Pfeil mit vier Pfeilspitzen um
//! einen Punkt), beim Körper Zeiger 100 = Ressource 3 (Verschiebekreuz). Ziehen an einem solchen Punkt verschiebt
//! nur diesen Punkt (MouseMove 0x958 = 6, Punkt F+0x790 + Index × 8), auch ohne vorherige Markierung.
//!
//! Punkte je Typ (Schleife in 0x455274): Linie (1) und Kreis (5) zwei, Kurve (2) und Rechteck (3) vier,
//! Dreieck (4) drei. Im Nachbau: Linie/Kreis/Dreieck/Tabelle/Bild die beiden Rahmenpunkte, Rechteck die vier
//! Ecken, Kurve Anfang, Kontrollpunkte und Ende. Texte (ZCN 6 und 11) haben zwei Punkte: Feld 0/1 und 2/3, beim
//! Textfeld also linke obere und rechte untere Ecke der Textbox; Ziehen ändert nur die Box (der Schriftgrad
//! bleibt). Gedrehte Texte (Feld 5 ≠ 0) drehen den zweiten Punkt im Original um den ersten (0x4698ec) – im Nachbau
//! haben hochkant stehende Texte deshalb (noch) keine Punkte. Vielecke sind nicht dabei.
//! Ein Klick ohne Bewegung ändert nichts (kein Rasterfang des Punktes, kein Rückgängig-Schritt).

use crate::app::{App, Drag};
use crate::model::{Kind, P};

/// Gegriffener Punkt eines Zeichenobjekts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punkt {
    /// Rahmenpunkt wie beim Ziehen am Eckpunkt (0 = a, 1 = b, 2 = (a.x, b.y), 3 = (b.x, a.y))
    Ecke(usize),
    /// Griff einer Kurve (0 Anfang, 1/2 Kontrollpunkte, 3 Ende), siehe kurve.rs
    Kurve(usize),
    /// Ecke der Textbox eines Textes (0 links oben, 1 rechts unten)
    Text(usize),
}

/// Punkt des Zeichenobjekts `k` in Reichweite `tol` um `m` (der nächstgelegene).
pub fn punkt_bei(k: &Kind, m: P, tol: f32) -> Option<Punkt> {
    let pts: Vec<P> = match k {
        Kind::Curve { .. } => return crate::kurve::griff_bei(k, m, tol).map(Punkt::Kurve),
        Kind::Rect { a, b } => vec![*a, *b, P::new(a.x, b.y), P::new(b.x, a.y)],
        Kind::Line { a, b } | Kind::Circle { a, b } | Kind::Triangle { a, b } | Kind::Table { a, b, .. } | Kind::Image { a, b, .. } => vec![*a, *b],
        Kind::Text { vertical: false, style, .. } => {
            let (mn, mx) = style.rect?;
            return [mn, mx].iter().enumerate().map(|(i, q)| (q.dist(m), i)).filter(|(d, _)| *d <= tol).min_by(|x, y| x.0.total_cmp(&y.0)).map(|(_, i)| Punkt::Text(i));
        }
        _ => return None,
    };
    pts.iter()
        .enumerate()
        .map(|(i, q)| (q.dist(m), i))
        .filter(|(d, _)| *d <= tol)
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map(|(_, i)| Punkt::Ecke(i))
}

/// Punkt `p` des Zeichenobjekts auf `q` setzen.
pub fn punkt_setzen(k: &mut Kind, p: Punkt, q: P) {
    match p {
        Punkt::Kurve(g) => crate::kurve::griff_setzen(k, g, q),
        Punkt::Text(i) => {
            if let Kind::Text { pos, align, vertical, style, .. } = k {
                if let Some((mn, mx)) = style.rect {
                    let (a, b) = if i == 0 { (q, mx) } else { (mn, q) };
                    let (mn, mx) = crate::canvas::norm_rect(a, b);
                    style.rect = Some((mn, mx));
                    *pos = crate::epr::text_anchor(mn, mx, *align, *vertical);
                }
            }
        }
        Punkt::Ecke(c) => {
            if let Some((a, b)) = k.frame_points() {
                let (na, nb) = match c {
                    0 => (q, b),
                    1 => (a, q),
                    2 => (P::new(q.x, a.y), P::new(b.x, q.y)),
                    _ => (P::new(a.x, q.y), P::new(q.x, b.y)),
                };
                k.set_frame_points(na, nb);
            }
        }
    }
}

impl App {
    /// Punkt eines Zeichenobjekts unter der Maus. Wie im Original zählen Zeichenobjekte erst nach allen anderen
    /// Treffern: liegt ein anderes Objekt obenauf (Symbol, Leitung, Text …), gibt es keinen Punkt.
    pub fn zeichenpunkt_unter(&self, m: P, tol: f32) -> Option<(u64, Punkt)> {
        if self.sym_edit.is_some() {
            return None;
        }
        let s = self.sheet()?;
        if let Some(id) = self.hit_top(m, tol) {
            let o = s.objects.iter().find(|o| o.id == id)?;
            return punkt_bei(&o.kind, m, tol).map(|p| (id, p));
        }
        // Punkt außerhalb des Körpers (z. B. Rahmenecke eines Kreises)
        s.objects.iter().rev().filter(|o| self.selectable(o)).find_map(|o| punkt_bei(&o.kind, m, tol).map(|p| (o.id, p)))
    }

    /// Druck auf einen Punkt eines Zeichenobjekts: Objekt markieren und den Punkt ziehen.
    pub fn zeichenpunkt_greifen(&mut self, m: P, tol: f32) -> bool {
        let Some((id, punkt)) = self.zeichenpunkt_unter(m, tol) else { return false };
        if !self.selection.contains(&id) {
            self.selection = vec![id];
        }
        self.drag = Drag::Zeichenpunkt { id, punkt, ab: Some(m) };
        true
    }

    /// Ziehen am Punkt: erst nach 3 Bildpunkten Bewegung (Klick ändert nichts), dann auf den Rasterpunkt der Maus.
    pub fn zeichenpunkt_ziehen(&mut self, m: P, z: f32) {
        let Drag::Zeichenpunkt { id, punkt, ab } = self.drag.clone() else { return };
        if let Some(a) = ab {
            if a.dist(m) * z <= 3.0 {
                return;
            }
            self.checkpoint();
            self.drag = Drag::Zeichenpunkt { id, punkt, ab: None };
        }
        let q = self.snap(m);
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                punkt_setzen(&mut o.kind, punkt, q);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn punkte_je_zeichenobjekt() {
        let linie = Kind::Line { a: p(0.0, 0.0), b: p(20.0, 0.0) };
        assert_eq!(punkt_bei(&linie, p(19.0, 0.5), 1.5), Some(Punkt::Ecke(1)));
        // Mitte der Linie: kein Punkt (Körper → Verschiebekreuz)
        assert_eq!(punkt_bei(&linie, p(10.0, 0.0), 1.5), None);
        let rechteck = Kind::Rect { a: p(0.0, 0.0), b: p(20.0, 10.0) };
        assert_eq!(punkt_bei(&rechteck, p(0.5, 9.5), 1.5), Some(Punkt::Ecke(2)));
        assert_eq!(punkt_bei(&rechteck, p(19.5, 0.5), 1.5), Some(Punkt::Ecke(3)));
        let kurve = Kind::Curve { a: p(0.0, 0.0), c1: p(0.0, 10.0), c2: p(10.0, 20.0), b: p(20.0, 20.0) };
        assert_eq!(punkt_bei(&kurve, p(0.3, 9.8), 1.5), Some(Punkt::Kurve(1)));
        // Text (Textbox): linke obere und rechte untere Ecke; hochkant und ohne Box keine
        let text = crate::textbox::neu(p(10.0, 10.0), p(40.0, 20.0), "A");
        assert_eq!(punkt_bei(&text, p(10.5, 10.4), 1.5), Some(Punkt::Text(0)));
        assert_eq!(punkt_bei(&text, p(39.5, 19.4), 1.5), Some(Punkt::Text(1)));
        assert_eq!(punkt_bei(&text, p(40.0, 10.0), 1.5), None);
        let mut hoch = text.clone();
        if let Kind::Text { vertical, .. } = &mut hoch {
            *vertical = true;
        }
        assert_eq!(punkt_bei(&hoch, p(10.0, 10.0), 1.5), None);
        // Symbole und Leitungen haben keine solchen Punkte
        let leitung = Kind::Wire { pts: vec![p(0.0, 0.0), p(20.0, 0.0)], leitung: 0, bez: vec![], opts: 0 };
        assert_eq!(punkt_bei(&leitung, p(0.0, 0.0), 1.5), None);
    }

    #[test]
    fn linie_am_ende_ziehen_wie_original() {
        use crate::hover::{Objektart, Ziel};
        use crate::model::{Obj, Project, SheetRef};
        let mut app = crate::blatt_tasten::test_app();
        app.project = Project::new_default();
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        let a = p(40.0, 40.0);
        app.with_sheet(|s, _| s.objects.push(Obj { id: 7, kind: Kind::Line { a, b: p(80.0, 40.0) }, pen: 0, fixed: false, group: None, raw: None }));
        let tol = 1.5;
        // Zeiger: am Ende Punkt (vier Pfeilspitzen), in der Mitte Verschiebekreuz
        assert_eq!(app.vormarkierung(p(40.3, 40.2), tol, 2.0), Some(Ziel::Ecke));
        assert!(matches!(app.vormarkierung(p(60.0, 40.2), tol, 2.0), Some(Ziel::Objekt { art: Objektart::Verschiebbar, .. })));
        // Druck am Ende: Linie markiert, kleiner Ruck ändert nichts
        assert!(app.zeichenpunkt_greifen(p(40.3, 40.2), tol));
        assert_eq!(app.selection, vec![7]);
        app.zeichenpunkt_ziehen(p(40.8, 40.4), 2.0);
        assert_eq!(app.sheet().unwrap().objects[0].kind.frame_points(), Some((a, p(80.0, 40.0))));
        // gezogen: nur der Anfang wandert (auf den Rasterpunkt)
        app.zeichenpunkt_ziehen(p(50.1, 60.2), 2.0);
        let (na, nb) = app.sheet().unwrap().objects[0].kind.frame_points().unwrap();
        assert_eq!(nb, p(80.0, 40.0));
        assert_eq!(na, app.snap(p(50.1, 60.2)));
        app.do_undo();
        assert_eq!(app.sheet().unwrap().objects[0].kind.frame_points(), Some((a, p(80.0, 40.0))));
        // in der Mitte: kein Punkt
        assert!(!app.zeichenpunkt_greifen(p(60.0, 40.0), tol));
    }

    #[test]
    fn punkt_setzen_wie_eckpunkt_ziehen() {
        let mut r = Kind::Rect { a: p(0.0, 0.0), b: p(20.0, 10.0) };
        punkt_setzen(&mut r, Punkt::Ecke(2), p(-5.0, 15.0));
        assert_eq!(r.frame_points(), Some((p(-5.0, 0.0), p(20.0, 15.0))));
        let mut l = Kind::Line { a: p(0.0, 0.0), b: p(20.0, 0.0) };
        punkt_setzen(&mut l, Punkt::Ecke(0), p(5.0, 5.0));
        assert_eq!(l.frame_points(), Some((p(5.0, 5.0), p(20.0, 0.0))));
        let mut k = Kind::Curve { a: p(0.0, 0.0), c1: p(0.0, 10.0), c2: p(10.0, 20.0), b: p(20.0, 20.0) };
        punkt_setzen(&mut k, Punkt::Kurve(3), p(30.0, 25.0));
        assert!(matches!(k, Kind::Curve { b, .. } if b == p(30.0, 25.0)));
        // Textbox: nur die Box ändert sich, der Ankerpunkt folgt der Ausrichtung, der Schriftgrad bleibt
        let mut t = crate::textbox::neu(p(10.0, 10.0), p(40.0, 20.0), "A");
        punkt_setzen(&mut t, Punkt::Text(1), p(50.0, 30.0));
        let Kind::Text { pos, style, size, .. } = &t else { unreachable!() };
        assert_eq!(style.rect, Some((p(10.0, 10.0), p(50.0, 30.0))));
        assert_eq!(*pos, crate::epr::text_anchor(p(10.0, 10.0), p(50.0, 30.0), crate::model::Align::Left, false));
        assert_eq!(*size, crate::textbox::VORGABE_PT * 0.3528);
        punkt_setzen(&mut t, Punkt::Text(0), p(60.0, 5.0));
        let Kind::Text { style, .. } = &t else { unreachable!() };
        assert_eq!(style.rect, Some((p(50.0, 5.0), p(60.0, 30.0))));
    }
}
