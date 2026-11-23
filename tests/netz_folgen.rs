//! Runde 13: allgemeines Nachführen des Leitungsnetzes (netz_folgen.rs).
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
    let kind = Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 };
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

fn klemme(id: u64, x: f32, y: f32) -> Obj {
    Obj { id, kind: Kind::Terminal { pos: P::new(x, y), leiste: "X".into(), nr: id as u32, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

fn pts(app: &elektroplanung::app::App, id: u64) -> Vec<P> {
    match &app.sheet().unwrap().objects.iter().find(|o| o.id == id).unwrap().kind {
        Kind::Wire { pts, .. } => pts.clone(),
        _ => panic!(),
    }
}

fn app_mit(objs: Vec<Obj>) -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.with_sheet(|s, _| s.objects.extend(objs));
    app
}

fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

// Raster des Test-Programms: 10 mm, Ursprung (5, 15) → Rasterlinien bei x, y = …5

/// Nutzerbild 23.11.2026: Leitung von -E1 endet als Abzweig auf einer anderen Leitung; deren Teilstück wird
/// verschoben – der Abzweig muss folgen.
#[test]
fn abzweig_folgt_dem_gezogenen_teilstueck() {
    // Leitung 1: Klemme (25,45) → hoch → rechts → runter zur Klemme (125,45); Abzweig 2 von Klemme (75,85) hoch auf 1
    let mut app = app_mit(vec![
        klemme(10, 24.2, 45.0),
        klemme(11, 125.8, 45.0),
        klemme(12, 75.0, 85.8),
        wire(1, &[(25.0, 45.0), (25.0, 25.0), (125.0, 25.0), (125.0, 45.0)]),
        wire(2, &[(75.0, 85.0), (75.0, 25.0)]),
    ]);
    app.selection = vec![1];
    assert!(app.teilstueck_greifen(1, p(55.0, 25.0), 1.0));
    app.teilstueck_ziehen(p(55.0, 15.0));
    let w1 = pts(&app, 1);
    let w2 = pts(&app, 2);
    assert!(w1.windows(2).any(|s| seg_dist(p(75.0, 15.0), s[0], s[1]) < 0.01), "{w1:?}");
    assert_eq!(*w2.last().unwrap(), p(75.0, 15.0), "Abzweig folgt: {w2:?}");
    assert_eq!(w2[0], p(75.0, 85.0));
    // nach unten: Abzweig wird kürzer
    app.teilstueck_ziehen(p(55.0, 35.0));
    assert_eq!(*pts(&app, 2).last().unwrap(), p(75.0, 35.0), "{:?}", pts(&app, 2));
}

#[test]
fn freies_ende_wandert_mit() {
    // gerade Leitung, Anfang frei, Ende an einer Klemme: Teilstück nach unten → Anfang wandert mit, am Ende Querstück
    let mut app = app_mit(vec![klemme(10, 105.8, 25.0), wire(1, &[(25.0, 25.0), (105.0, 25.0)])]);
    app.selection = vec![1];
    assert!(app.teilstueck_greifen(1, p(55.0, 25.0), 1.0));
    app.teilstueck_ziehen(p(55.0, 35.0));
    assert_eq!(pts(&app, 1), vec![p(25.0, 35.0), p(105.0, 35.0), p(105.0, 25.0)]);
    // ganz freie Leitung: wird einfach verschoben
    let mut app = app_mit(vec![wire(1, &[(25.0, 25.0), (105.0, 25.0)])]);
    app.selection = vec![1];
    assert!(app.teilstueck_greifen(1, p(55.0, 25.0), 1.0));
    app.teilstueck_ziehen(p(55.0, 35.0));
    assert_eq!(pts(&app, 1), vec![p(25.0, 35.0), p(105.0, 35.0)]);
}

#[test]
fn abzweig_folgt_der_verschobenen_leitung() {
    // ganze Leitung 1 markiert und verschoben (nicht angeschlossen): Abzweig 2 folgt, sein anderes Ende bleibt
    let mut app = app_mit(vec![klemme(12, 70.0, 80.8), wire(1, &[(20.0, 20.0), (120.0, 20.0)]), wire(2, &[(70.0, 80.0), (70.0, 20.0)])]);
    app.selection = vec![1];
    app.netz_merken();
    app.netz_verschieben(&[1], p(0.0, -10.0));
    assert_eq!(pts(&app, 1), vec![p(20.0, 10.0), p(120.0, 10.0)]);
    assert_eq!(pts(&app, 2), vec![p(70.0, 80.0), p(70.0, 10.0)]);
    // seitlich: Abzweig bleibt senkrecht und wandert mit (gleiche Lage auf dem Zug)
    app.netz_verschieben(&[1], p(10.0, 0.0));
    let w2 = pts(&app, 2);
    assert_eq!(w2[0], p(70.0, 80.0));
    assert!(seg_dist(*w2.last().unwrap(), p(30.0, 20.0), p(130.0, 20.0)) < 0.01, "{w2:?}");
}
