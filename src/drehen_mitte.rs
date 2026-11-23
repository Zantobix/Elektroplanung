//! Drehen/Spiegeln (ToolButton17Click 0x465994): Das Original bildet die Mitte aus dem umschließenden Rechteck
//! aller markierten Objekte (Zeichenobjekte mit ihren Eckpunkten, Symbole mit ihrem Einfügepunkt, Klemmstellen
//! und Verweise) und dreht bzw. spiegelt alles um diesen einen Punkt. Der Nachbau übernimmt das für jede Auswahl
//! aus mindestens zwei Objekten (Nachzeichnen 24.11.2026: vorher drehten Symbole und Texte einer gemischten
//! Auswahl je für sich, ein Grundriss zerfiel); ein einzelnes Objekt dreht um seine eigene Mitte.

use crate::model::*;
use crate::symbols::SymDef;

/// Gemeinsame Mitte (auf 0,5 mm gerundet) für eine Auswahl aus mindestens zwei Objekten: Zeichenobjekte mit ihrem
/// Umriss, Symbole, Texte, Klemmstellen und Verweise mit ihrem Einfügepunkt (wie das Original); Sammelschienen und
/// Kontaktspiegel zählen nicht. `None` bei weniger als zwei Objekten (dann gilt die Mitte je Objekt).
pub fn gemeinsame_mitte<'a>(objs: impl Iterator<Item = &'a Obj>, lib: &[SymDef]) -> Option<P> {
    let mut n = 0;
    let (mut mn, mut mx) = (P::new(f32::MAX, f32::MAX), P::new(f32::MIN, f32::MIN));
    for o in objs {
        let (a, b) = match &o.kind {
            Kind::Symbol { pos, .. } | Kind::Text { pos, .. } | Kind::Terminal { pos, .. } | Kind::Verweis { pos, .. } => (*pos, *pos),
            Kind::Busbar { .. } | Kind::Spiegel { .. } => continue,
            _ => crate::canvas::obj_bounds(o, lib),
        };
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
        // Symbol mit Einfügepunkt (100, 80) zählt mit
        let s = Obj { id: 3, kind: Kind::Symbol { key: "k".into(), pos: P::new(100.0, 80.0), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
        assert_eq!(gemeinsame_mitte([v[0].clone(), s].iter(), &[]), Some(P::new(50.0, 40.0)));
    }
}
