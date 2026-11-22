//! Abzweig auf einer mitverschobenen Leitung reißt nicht ab (Maustest Runde 10, 22.11.2026): Wird eine Leitung
//! verschoben, folgt das Ende einer anderen Leitung, das als Abzweig auf ihr liegt.

use elektroplanung::mitziehen::find_followers;
use elektroplanung::model::*;

fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
    let kind = Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 };
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

#[test]
fn abzweig_folgt_verschobener_leitung() {
    let mut s = Sheet::new(1);
    // Reihe: waagerechte Leitung 1; Zuleitung 2 endet als Abzweig auf ihr; Leitung 3 liegt abseits
    s.objects.push(wire(1, &[(100.0, 150.0), (200.0, 150.0)]));
    s.objects.push(wire(2, &[(80.0, 100.0), (80.0, 140.0), (140.0, 140.0), (140.0, 150.0)]));
    s.objects.push(wire(3, &[(80.0, 170.0), (90.0, 170.0)]));
    let f = find_followers(&s, &[], &[1], &[1], false);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].id, 2);
    assert_eq!(f[0].ends, [false, true]);
}
