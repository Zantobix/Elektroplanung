//! Beschriftung einer neuen Klemmstelle (Maustest M4, 24.11.2026): in einer senkrechten Leitung wie im Original
//! (−1 | −2 mm, links über der Klemme), in einer waagerechten höher (−1 | −4 mm), sonst läge sie auf der Linie.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn beschriftung(a: P, b: P, m: P) -> Option<P> {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.add_obj(Kind::Wire { pts: vec![a, b], leitung: 0, bez: vec![], opts: 0 });
    let id = app.klemmstelle_setzen(m, 1.0);
    app.sheet().unwrap().objects.iter().find(|o| o.id == id).and_then(|o| if let Kind::Terminal { text_off, .. } = &o.kind { *text_off } else { None })
}

#[test]
fn beschriftung_nicht_auf_der_leitung() {
    assert_eq!(beschriftung(P::new(50.0, 40.0), P::new(50.0, 100.0), P::new(50.0, 70.0)), Some(P::new(-1.0, -2.0)));
    assert_eq!(beschriftung(P::new(40.0, 70.0), P::new(120.0, 70.0), P::new(80.0, 70.0)), Some(P::new(-1.0, -4.0)));
}
