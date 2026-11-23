//! Hilfslinien beim Verschieben von Symbolen (bewusste Erweiterung, Nutzervideo 23.11.2026 mit QElectroTech;
//! nach QElectroTech `Terminal::drawHelpLine`/`Terminal::alignedWithTerminal`, GPL-2.0-or-later, neu geschrieben).
//!
//! Während ein Symbol gezogen wird, zeigt jeder seiner Anschlüsse
//! - eine **graue Linie quer** über das ganze Blatt durch den Anschluss (Anschluss oben/unten → waagerecht, links/rechts
//!   → senkrecht): so lässt sich das Symbol auf die Anschlusshöhe anderer Symbole ausrichten;
//! - an einem **freien** Anschluss (keine Leitung) eine **blaue Flucht** in Anschlussrichtung bis zum Rahmen. Liegt auf
//!   ihr ein freier, entgegengerichteter Anschluss eines anderen Symbols, endet sie dort und wird **grün**: eine
//!   Leitung dorthin wäre gerade.
//!
//! Nur Anzeige; die Datei bleibt unverändert. Abschaltbar unter Optionen › Programmverhalten.

use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

const T: f32 = 0.05;

/// Art einer Hilfslinie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {
    /// quer durch den Anschluss über das Blatt (grau)
    Quer,
    /// Flucht eines freien Anschlusses bis zum Rahmen (blau)
    Flucht,
    /// Flucht bis zu einem freien Gegenanschluss (grün)
    Treffer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Linie {
    pub a: P,
    pub b: P,
    pub art: Art,
}

/// Liegt an `p` ein Leitungspunkt (Ende oder Knick)?
fn belegt(sheet: &Sheet, p: P) -> bool {
    sheet.objects.iter().any(|o| match &o.kind {
        Kind::Wire { pts, .. } => pts.iter().any(|q| q.dist(p) <= T),
        _ => false,
    })
}

/// Hilfslinien für die verschobenen Objekte `bewegt` (nur Symbole) auf `sheet`.
pub fn linien(sheet: &Sheet, lib: &[SymDef], bewegt: &[u64]) -> Vec<Linie> {
    let (x0, y0, x1, y1) = sheet.frame();
    let mut v = vec![];
    // freie Anschlüsse nicht verschobener Symbole mit ihrer Richtung (mögliche Gegenstücke)
    let gegen: Vec<(P, P)> = sheet
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }) && !bewegt.contains(&o.id))
        .flat_map(|o| canvas::obj_pins(o, lib))
        .filter(|q| !belegt(sheet, *q))
        .filter_map(|q| crate::leitung_route::anschluss_richtung(sheet, lib, q).map(|d| (q, d)))
        .collect();
    for o in sheet.objects.iter().filter(|o| bewegt.contains(&o.id) && matches!(o.kind, Kind::Symbol { .. })) {
        for p in canvas::obj_pins(o, lib) {
            let Some(d) = crate::leitung_route::anschluss_richtung(sheet, lib, p) else { continue };
            let waag = d.x.abs() > 0.5;
            v.push(if waag { Linie { a: P::new(p.x, y0), b: P::new(p.x, y1), art: Art::Quer } } else { Linie { a: P::new(x0, p.y), b: P::new(x1, p.y), art: Art::Quer } });
            if belegt(sheet, p) {
                continue;
            }
            // Ende am Rahmen in Anschlussrichtung
            let rand = if waag { P::new(if d.x > 0.0 { x1 } else { x0 }, p.y) } else { P::new(p.x, if d.y > 0.0 { y1 } else { y0 }) };
            // nächster freier, entgegengerichteter Anschluss auf der Flucht
            let treffer = gegen
                .iter()
                .filter(|(q, dq)| {
                    let entgegen = (dq.x + d.x).abs() < 0.1 && (dq.y + d.y).abs() < 0.1;
                    let auf = if waag { (q.y - p.y).abs() <= T && (q.x - p.x) * d.x > T } else { (q.x - p.x).abs() <= T && (q.y - p.y) * d.y > T };
                    entgegen && auf
                })
                .map(|(q, _)| *q)
                .min_by(|a, b| a.dist(p).total_cmp(&b.dist(p)));
            v.push(match treffer {
                Some(q) => Linie { a: p, b: q, art: Art::Treffer },
                None => Linie { a: p, b: rand, art: Art::Flucht },
            });
        }
    }
    v
}

/// Bewertung eines Zielanschlusses beim Zeichnen (nach QElectroTech `Terminal`: grün frei, orange schon belegt,
/// rot nicht erlaubt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ziel {
    Frei,
    Belegt,
    Verboten,
}

/// Ziel `q` für eine Leitung, die bei `start` beginnt: verboten ist der Anfangsanschluss selbst; belegt, wenn dort
/// schon eine Leitung endet (Klemmstellen: am Kreis, daher 1 mm Fangweite).
pub fn ziel(sheet: &Sheet, q: P, start: Option<P>) -> Ziel {
    if start.is_some_and(|a| a.dist(q) <= 1.0) {
        return Ziel::Verboten;
    }
    let belegt = sheet.objects.iter().any(|o| match &o.kind {
        Kind::Wire { pts, .. } if pts.len() >= 2 => [pts[0], pts[pts.len() - 1]].iter().any(|e| e.dist(q) <= 1.0),
        _ => false,
    });
    if belegt {
        Ziel::Belegt
    } else {
        Ziel::Frei
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schliesser(id: u64, x: f32, y: f32) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos: P::new(x, y), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn wire(id: u64, a: P, b: P) -> Obj {
        Obj { id, kind: Kind::Wire { pts: vec![a, b], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn flucht_trifft_freien_gegenanschluss() {
        // A bei (105,105): Anschlüsse oben (105,98), unten (105,112); B genau darunter bei (105,155): oben (105,148)
        let lib = crate::symbols::builtin();
        let mut s = Sheet::new(1);
        s.objects = vec![schliesser(1, 105.0, 105.0), schliesser(2, 105.0, 155.0)];
        let l = linien(&s, &lib, &[1]);
        // je Anschluss eine Querlinie
        assert_eq!(l.iter().filter(|x| x.art == Art::Quer).count(), 2);
        assert!(l.contains(&Linie { a: P::new(105.0, 112.0), b: P::new(105.0, 148.0), art: Art::Treffer }), "{l:?}");
        let (_, y0, _, _) = s.frame();
        assert!(l.contains(&Linie { a: P::new(105.0, 98.0), b: P::new(105.0, y0), art: Art::Flucht }), "{l:?}");
        // B daneben: keine Treffer mehr
        s.objects[1] = schliesser(2, 110.0, 155.0);
        assert!(!linien(&s, &lib, &[1]).iter().any(|x| x.art == Art::Treffer));
    }

    #[test]
    fn zielbewertung() {
        let mut s = Sheet::new(1);
        s.objects = vec![schliesser(1, 105.0, 105.0), wire(2, P::new(105.0, 98.0), P::new(105.0, 60.0))];
        assert_eq!(ziel(&s, P::new(105.0, 98.0), None), Ziel::Belegt);
        assert_eq!(ziel(&s, P::new(105.0, 112.0), None), Ziel::Frei);
        assert_eq!(ziel(&s, P::new(105.0, 112.0), Some(P::new(105.0, 112.0))), Ziel::Verboten);
    }

    #[test]
    fn angeschlossene_anschluesse_ohne_flucht() {
        let lib = crate::symbols::builtin();
        let mut s = Sheet::new(1);
        s.objects = vec![schliesser(1, 105.0, 105.0), wire(2, P::new(105.0, 98.0), P::new(105.0, 60.0)), schliesser(3, 105.0, 155.0)];
        let l = linien(&s, &lib, &[1]);
        assert!(!l.iter().any(|x| x.art != Art::Quer && x.a == P::new(105.0, 98.0)), "{l:?}");
        // der untere bleibt frei und trifft B
        assert!(l.iter().any(|x| x.art == Art::Treffer && x.a == P::new(105.0, 112.0)));
        // ein Gegenanschluss mit Leitung zählt nicht
        s.objects.push(wire(4, P::new(105.0, 148.0), P::new(150.0, 148.0)));
        assert!(!linien(&s, &lib, &[1]).iter().any(|x| x.art == Art::Treffer));
    }
}
