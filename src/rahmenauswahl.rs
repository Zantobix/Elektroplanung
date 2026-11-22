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

#[cfg(test)]
mod tests {
    use super::*;

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
