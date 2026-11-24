//! Rahmenauswahl wie im Original (Gegenprobe Wine 22.11.2026, notizen/runde7_bilder/orig_09–11): markiert wird jedes
//! Objekt, dessen Bezugspunkt (Anfangspunkt bzw. Einfügepunkt, die Lage im Objektblock) im aufgezogenen Rahmen liegt –
//! weder „ganz enthalten“ noch „berührt“. Eine Linie, deren Anfang im Rahmen liegt, wird markiert, auch wenn sie
//! weit hinausragt; liegen nur Mitte oder Ende im Rahmen, bleibt sie unmarkiert.

use crate::model::{Kind, P};

/// Bezugspunkt eines Objekts: erster Punkt bzw. Einfügepunkt.
pub fn bezugspunkt(k: &Kind) -> Option<P> {
    Some(match k {
        Kind::Line { a, .. } | Kind::Curve { a, .. } | Kind::Rect { a, .. } | Kind::Triangle { a, .. } | Kind::Circle { a, .. } => *a,
        Kind::Table { a, .. } | Kind::Image { a, .. } | Kind::Busbar { a, .. } => *a,
        Kind::Text { pos, .. } | Kind::Symbol { pos, .. } | Kind::Terminal { pos, .. } | Kind::Verweis { pos, .. } | Kind::Spiegel { pos, .. } => *pos,
        Kind::Poly { pts } | Kind::Wire { pts, .. } => *pts.first()?,
        #[allow(unreachable_patterns)]
        _ => return None,
    })
}

/// Liegt der Bezugspunkt im Rahmen `mn`..`mx` (Ränder eingeschlossen)?
pub fn im_rahmen(k: &Kind, mn: P, mx: P) -> bool {
    bezugspunkt(k).is_some_and(|p| p.x >= mn.x && p.y >= mn.y && p.x <= mx.x && p.y <= mx.y)
}

/// Markieren wie im CAD (Option, Nutzerwunsch 23.11.2026, todo N4): von links nach rechts aufgezogen zählt nur, was
/// ganz im Rahmen liegt; von rechts nach links alles, was der Rahmen berührt. `start` = Punkt beim Drücken.
pub fn cad_treffer(o: &crate::model::Obj, lib: &[crate::symbols::SymDef], start: P, ende: P) -> bool {
    let (mn, mx) = crate::canvas::norm_rect(start, ende);
    let (a, b) = crate::canvas::obj_bounds(o, lib);
    if ende.x >= start.x {
        return a.x >= mn.x && a.y >= mn.y && b.x <= mx.x && b.y <= mx.y;
    }
    // berührend: Umrisse überlappen; Linienzüge nur, wenn ein Teilstück den Rahmen schneidet
    if b.x < mn.x || a.x > mx.x || b.y < mn.y || a.y > mx.y {
        return false;
    }
    let zug: Option<Vec<P>> = match &o.kind {
        Kind::Wire { pts, .. } | Kind::Poly { pts } => Some(pts.clone()),
        Kind::Line { a, b } => Some(vec![*a, *b]),
        _ => None,
    };
    match zug {
        Some(z) => z.windows(2).any(|s| strecke_im_rahmen(s[0], s[1], mn, mx)),
        None => true,
    }
}

/// Schneidet oder berührt die Strecke `p`–`q` das Rechteck `mn`..`mx`? (Liang-Barsky)
fn strecke_im_rahmen(p: P, q: P, mn: P, mx: P) -> bool {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (pk, qk) in [(-dx, p.x - mn.x), (dx, mx.x - p.x), (-dy, p.y - mn.y), (dy, mx.y - p.y)] {
        if pk.abs() < 1e-9 {
            if qk < 0.0 {
                return false;
            }
        } else {
            let r = qk / pk;
            if pk < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(kind: Kind) -> crate::model::Obj {
        crate::model::Obj { id: 1, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn cad_eingeschlossen_und_beruehrt() {
        // L-förmige Leitung (10,10)–(90,10)–(90,90)
        let w = obj(Kind::Wire { pts: vec![P::new(10.0, 10.0), P::new(90.0, 10.0), P::new(90.0, 90.0)], leitung: 0, bez: vec![], opts: 0 });
        // links → rechts: nur ganz eingeschlossen
        assert!(cad_treffer(&w, &[], P::new(0.0, 0.0), P::new(100.0, 100.0)));
        assert!(!cad_treffer(&w, &[], P::new(0.0, 0.0), P::new(50.0, 100.0)));
        // rechts → links: berührt reicht
        assert!(cad_treffer(&w, &[], P::new(50.0, 100.0), P::new(0.0, 0.0)));
        // Rahmen in der Ecke innerhalb des Umrisses, aber ohne die Leitung: nicht berührt
        assert!(!cad_treffer(&w, &[], P::new(60.0, 80.0), P::new(20.0, 30.0)));
        // Linie nur mit dem Ende im Rahmen
        let l = obj(Kind::Line { a: P::new(100.0, 50.0), b: P::new(200.0, 50.0) });
        assert!(cad_treffer(&l, &[], P::new(210.0, 40.0), P::new(180.0, 60.0)));
        assert!(!cad_treffer(&l, &[], P::new(180.0, 40.0), P::new(210.0, 60.0)));
    }

    fn linie(x0: f32, x1: f32) -> Kind {
        Kind::Line { a: P::new(x0, 50.0), b: P::new(x1, 50.0) }
    }

    #[test]
    fn linie_wie_original() {
        // Original: Rahmen um den Anfang markiert, Rahmen um Mitte oder Ende nicht
        let l = linie(100.0, 200.0);
        assert!(im_rahmen(&l, P::new(90.0, 40.0), P::new(130.0, 60.0)));
        assert!(!im_rahmen(&l, P::new(130.0, 40.0), P::new(170.0, 60.0)));
        assert!(!im_rahmen(&l, P::new(180.0, 40.0), P::new(220.0, 60.0)));
    }

    #[test]
    fn leitung_nach_erstem_punkt() {
        let w = Kind::Poly { pts: vec![P::new(10.0, 10.0), P::new(90.0, 10.0)] };
        assert!(im_rahmen(&w, P::new(0.0, 0.0), P::new(20.0, 20.0)));
        assert!(!im_rahmen(&w, P::new(80.0, 0.0), P::new(100.0, 20.0)));
        assert!(!im_rahmen(&Kind::Poly { pts: vec![] }, P::new(0.0, 0.0), P::new(1e6, 1e6)));
    }
}
