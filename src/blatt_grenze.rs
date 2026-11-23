//! Verschieben auf dem Blatt halten (Nutzerfrage 23.11.2026: „werden Bauteile richtig behandelt, die außerhalb der
//! Zeichenfläche gezogen werden?“ – eine Lampe, unter den Rahmen gezogen, lag unterhalb des Papiers).
//!
//! Die Gesamtverschiebung einer Geste wird je Achse so begrenzt, dass der gemeinsame Umriss der bewegten Objekte
//! innerhalb des Zeichnungsrahmens bleibt (dann bleiben auch die Stiche angeschlossener Leitungen auf dem Papier).
//! War er schon vorher teilweise draußen, darf er nicht weiter hinaus.

use crate::model::*;
use crate::symbols::SymDef;

/// Gesamtverschiebung `total` der Objekte `bewegt` (aus `objs`, jetzt um `angewandt` verschoben) begrenzen.
pub fn begrenzen(sheet: &Sheet, lib: &[SymDef], objs: &[Obj], bewegt: &[u64], angewandt: P, total: P) -> P {
    let (x0, y0, x1, y1) = sheet.frame();
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    for o in objs.iter().filter(|o| bewegt.contains(&o.id)) {
        let (a, b) = crate::canvas::obj_bounds(o, lib);
        // Umriss in der Lage vor der Geste
        let (a, b) = (a.sub(angewandt), b.sub(angewandt));
        mn = P::new(mn.x.min(a.x.min(b.x)), mn.y.min(a.y.min(b.y)));
        mx = P::new(mx.x.max(a.x.max(b.x)), mx.y.max(a.y.max(b.y)));
    }
    if mn.x > mx.x {
        return total;
    }
    let achse = |t: f32, lo: f32, hi: f32, g0: f32, g1: f32| -> f32 {
        // erlaubter Bereich der Verschiebung: lo + t ≥ g0 und hi + t ≤ g1 (schon draußen: nicht weiter hinaus)
        let min_t = (g0 - lo).min(0.0);
        let max_t = (g1 - hi).max(0.0);
        t.clamp(min_t.min(max_t), max_t.max(min_t))
    };
    P::new(achse(total.x, mn.x, mx.x, x0.min(x1), x0.max(x1)), achse(total.y, mn.y, mx.y, y0.min(y1), y0.max(y1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bleibt_auf_dem_blatt() {
        let lib = crate::symbols::builtin();
        let o = Obj { id: 1, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos: P::new(100.0, 100.0), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
        let s = Sheet::new(1);
        let (fx0, fy0, fx1, fy1) = s.frame();
        let (w, h) = (fx0.max(fx1), fy0.max(fy1));
        let (a, b) = crate::canvas::obj_bounds(&o, &lib);
        // weit nach unten: Unterkante endet am Blattrand
        let t = begrenzen(&s, &lib, std::slice::from_ref(&o), &[1], P::default(), P::new(0.0, 500.0));
        assert!((b.y.max(a.y) + t.y - h).abs() < 1e-3, "{t:?}");
        // weit nach links: linke Kante am Rahmen
        let t = begrenzen(&s, &lib, std::slice::from_ref(&o), &[1], P::default(), P::new(-500.0, 0.0));
        assert!((a.x.min(b.x) + t.x - fx0.min(fx1)).abs() < 1e-3, "{t:?}");
        // innerhalb: unverändert
        assert_eq!(begrenzen(&s, &lib, std::slice::from_ref(&o), &[1], P::default(), P::new(10.0, -10.0)), P::new(10.0, -10.0));
        let _ = w;
    }
}
