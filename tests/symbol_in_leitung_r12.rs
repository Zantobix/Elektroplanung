//! Symbol auf eine Leitung setzen (symbol_in_leitung.rs): passt kein Anschluss zur durchquerenden Leitung,
//! erscheint die Meldung des Originals (0x47bf14); senkrecht auf eine senkrechte Leitung wird eingebunden.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use elektroplanung::{canvas, meldungen, symbol_in_leitung, symbols};

const AT: P = P { x: 100.0, y: 100.0 };

fn leer() -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = symbols::builtin();
    app.auto_conn = false;
    app.path_align = false;
    app
}

/// Lage der Anschlüsse und Umriss des Symbols `key`, wenn es bei `AT` gesetzt wird.
fn probe(key: &str) -> (Vec<P>, (P, P)) {
    let mut app = leer();
    app.place_symbol(key.into(), AT);
    let o = app.sheet().unwrap().objects.iter().find(|o| matches!(o.kind, Kind::Symbol { .. })).unwrap().clone();
    (canvas::obj_pins(&o, &app.lib), canvas::obj_bounds(&o, &app.lib))
}

fn mit_leitung(a: P, b: P) -> elektroplanung::app::App {
    let mut app = leer();
    app.with_sheet(|s, _| {
        s.objects.push(Obj { id: 900, kind: Kind::Wire { pts: vec![a, b], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
    });
    app
}

#[test]
fn lampe_auf_waagerechter_leitung_meldet() {
    // Anschlüsse oben und unten in der Mitte: zu einer waagerechten Leitung passt keiner
    let (_, (mn, mx)) = probe("builtin:lampe");
    let y = (mn.y + mx.y) / 2.0;
    let mut app = mit_leitung(P::new(mn.x - 40.0, y), P::new(mx.x + 40.0, y));
    meldungen::test_antworten(&[]);
    app.place_symbol("builtin:lampe".into(), AT);
    assert_eq!(meldungen::test_gezeigt(), vec![(meldungen::T_ACHTUNG_X.to_string(), symbol_in_leitung::MELDUNG.to_string())]);
    // Symbol liegt, Leitung unverändert
    let s = app.sheet().unwrap();
    assert!(s.objects.iter().any(|o| matches!(o.kind, Kind::Symbol { .. })));
    assert!(s.objects.iter().any(|o| o.id == 900));
}

#[test]
fn schliesser_auf_senkrechter_leitung_ohne_meldung() {
    let (pins, (mn, mx)) = probe("builtin:schliesser");
    let x = pins[0].x;
    let mut app = mit_leitung(P::new(x, mn.y - 40.0), P::new(x, mx.y + 40.0));
    meldungen::test_antworten(&[]);
    app.place_symbol("builtin:schliesser".into(), AT);
    assert!(meldungen::test_gezeigt().is_empty());
    // eingebunden: die Leitung ist am Symbol aufgetrennt
    let s = app.sheet().unwrap();
    assert_eq!(s.objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).count(), 2);
}
