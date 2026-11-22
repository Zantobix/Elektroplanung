//! Drehen/Spiegeln (ToolButton17Click 0x465994): Das Original bildet die Mitte aus dem umschließenden Rechteck
//! aller markierten Objekte (Zeichenobjekte mit ihren Eckpunkten, Symbole mit ihrem Einfügepunkt, Klemmstellen
//! und Verweise) und dreht bzw. spiegelt alles um diesen einen Punkt. Der Nachbau übernimmt das für Auswahlen aus
//! mehreren Zeichenobjekten; Symbole und Sonderobjekte drehen weiter einzeln um ihren Einfügepunkt (angeschlossene
//! Leitungen folgen dort nicht).

use crate::model::*;
use crate::symbols::SymDef;

/// Gemeinsame Mitte (auf 0,5 mm gerundet) für eine Auswahl aus mindestens zwei Zeichenobjekten; `None`, wenn die
/// Auswahl Symbole, Sonderobjekte oder nur ein Objekt enthält (dann gilt die Mitte je Objekt).
pub fn gemeinsame_mitte<'a>(objs: impl Iterator<Item = &'a Obj>, lib: &[SymDef]) -> Option<P> {
    let mut n = 0;
    let (mut mn, mut mx) = (P::new(f32::MAX, f32::MAX), P::new(f32::MIN, f32::MIN));
    for o in objs {
        match o.kind {
            Kind::Line { .. } | Kind::Rect { .. } | Kind::Triangle { .. } | Kind::Circle { .. } | Kind::Curve { .. } | Kind::Poly { .. } | Kind::Wire { .. } => {}
            _ => return None,
        }
        let (a, b) = crate::canvas::obj_bounds(o, lib);
        mn = P::new(mn.x.min(a.x), mn.y.min(a.y));
        mx = P::new(mx.x.max(b.x), mx.y.max(b.y));
        n += 1;
    }
    (n >= 2).then(|| P::new(((mn.x + mx.x) / 2.0 / 0.5).round() * 0.5, ((mn.y + mx.y) / 2.0 / 0.5).round() * 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linie(id: u64, a: P, b: P) -> Obj {
        Obj { id, kind: Kind::Line { a, b }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn mitte_der_auswahl() {
        let v = [linie(1, P::new(0.0, 0.0), P::new(10.0, 0.0)), linie(2, P::new(20.0, 10.0), P::new(30.0, 10.0))];
        assert_eq!(gemeinsame_mitte(v.iter(), &[]), Some(P::new(15.0, 5.0)));
        assert_eq!(gemeinsame_mitte(v[..1].iter(), &[]), None);
    }
}
