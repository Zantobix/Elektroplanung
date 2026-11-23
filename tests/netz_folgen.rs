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

/// Maustest Runde 13: Wirt-Teilstück über den Abzweig hinweg gezogen – der Abzweig kommt von seiner Seite (kein
/// Lauf über den Wirt hinaus und zurück).
#[test]
fn abzweig_wechselt_die_seite_ohne_kreuzung() {
    let mut app = app_mit(vec![
        klemme(10, 24.2, 45.0),
        klemme(11, 125.8, 45.0),
        klemme(12, 95.0, 85.8),
        wire(1, &[(25.0, 45.0), (25.0, 25.0), (125.0, 25.0), (125.0, 45.0)]),
        // Abzweig von unten (95,85) hoch und links auf das senkrechte Stück x = 25
        wire(2, &[(95.0, 85.0), (95.0, 35.0), (25.0, 35.0)]),
    ]);
    app.selection = vec![1];
    assert!(app.teilstueck_greifen(1, p(25.0, 35.0), 1.0) || app.teilstueck_greifen(1, p(25.0, 40.0), 1.0));
    app.teilstueck_ziehen(p(145.0, 40.0));
    let w1 = pts(&app, 1);
    let w2 = pts(&app, 2);
    let e = *w2.last().unwrap();
    assert!(w1.windows(2).any(|s| seg_dist(e, s[0], s[1]) < 0.01), "Ende auf dem Wirt: {w2:?} / {w1:?}");
    // keine Stelle des Abzweigs rechts vom Wirt
    assert!(w2.iter().all(|q| q.x <= e.x + 0.01), "{w2:?}");
}

/// Maustest Runde 13: Lampe mit Abzweig auf die andere Seite des Wirts verschoben – der Abzweig führt ohne Umweg
/// über den Wirt.
#[test]
fn symbol_ueber_den_wirt_hinweg() {
    let mut app = app_mit(vec![klemme(12, 45.0, 85.8), wire(1, &[(85.0, 15.0), (85.0, 105.0)]), wire(2, &[(45.0, 85.0), (45.0, 55.0), (85.0, 55.0)])]);
    app.selection = vec![12];
    app.netz_merken();
    app.netz_verschieben(&[12], p(80.0, 0.0));
    let w2 = pts(&app, 2);
    assert_eq!(w2[0], p(125.0, 85.0), "{w2:?}");
    assert!(seg_dist(*w2.last().unwrap(), p(85.0, 15.0), p(85.0, 105.0)) < 0.01, "{w2:?}");
    // nirgends links vom Wirt
    assert!(w2.iter().all(|q| q.x >= 85.0 - 0.01), "{w2:?}");
}

/// Markierte Leitung an einer Sammelschiene verschoben (vorher schiene_halten.rs): ihr Ende bleibt auf der Schiene,
/// auch wenn sie über die Schiene hinaus bzw. über deren Ende geschoben wird.
#[test]
fn markierte_leitung_bleibt_an_der_schiene() {
    let schiene = Obj { id: 9, kind: Kind::Busbar { a: P::new(10.0, 20.0), x2: 300.0, name: "L1".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 1, versatz: 2.5 }, pen: 0, fixed: false, group: None, raw: None };
    for (d, anfang) in [(p(10.0, 30.0), p(90.0, 20.0)), (p(0.0, -10.0), p(80.0, 20.0)), (p(240.0, 10.0), p(300.0, 20.0))] {
        let mut app = app_mit(vec![schiene.clone(), wire(1, &[(80.0, 20.0), (80.0, 150.0), (60.0, 150.0)])]);
        app.selection = vec![1];
        app.netz_merken();
        app.netz_verschieben(&[1], d);
        let w = pts(&app, 1);
        assert_eq!(w[0], anfang, "{d:?}: {w:?}");
        assert!(w.windows(2).all(|s| (s[0].x - s[1].x).abs() < 0.01 || (s[0].y - s[1].y).abs() < 0.01), "{d:?}: {w:?}");
        assert_eq!(*w.last().unwrap(), p(60.0, 150.0).add(d), "{d:?}: {w:?}");
    }
}

/// Nutzerbild 23.11.2026 (Runde 13): Lampe mit Abzweig nach oben verschoben – das Ende gleitet auf dem Wirt, statt
/// kurz davor eine Ecke zu bilden.
#[test]
fn abzweig_ende_gleitet_auf_dem_wirt() {
    let mut app = app_mit(vec![klemme(12, 45.0, 85.8), wire(1, &[(85.0, 15.0), (85.0, 105.0)]), wire(2, &[(45.0, 85.0), (45.0, 55.0), (85.0, 55.0)])]);
    app.selection = vec![12];
    app.netz_merken();
    app.netz_verschieben(&[12], p(0.0, -40.0));
    let w2 = pts(&app, 2);
    assert_eq!(w2[0], p(45.0, 45.0), "{w2:?}");
    assert!(seg_dist(*w2.last().unwrap(), p(85.0, 15.0), p(85.0, 105.0)) < 0.01, "{w2:?}");
    assert!(w2.len() <= 3, "höchstens ein Knick: {w2:?}");
}

/// Nutzervideo 23.11.2026: Abzweig von E2 und eine zweite Leitung enden am selben Punkt auf N; das waagerechte Stück
/// der zweiten wird nach unten gezogen. E2s Abzweig bleibt, der Anfang der gezogenen Leitung wandert auf N mit
/// (kein Querstück auf N).
#[test]
fn video_gezogene_leitung_mit_gemeinsamem_punkt_auf_n() {
    let mut app = app_mit(vec![
        klemme(10, 25.0, 205.8),
        wire(1, &[(25.0, 5.0), (25.0, 205.0)]),
        // E2-Abzweig von links auf N bei y = 45
        klemme(11, 0.0, 85.8),
        wire(2, &[(-15.0, 85.0), (-15.0, 45.0), (25.0, 45.0)]),
        // obere Leitung von N (25,45) nach rechts und runter auf die Ecke der E3-Leitung (105,125)
        wire(3, &[(25.0, 45.0), (105.0, 45.0), (105.0, 125.0)]),
        // E3-Leitung von N (25,125) nach rechts und runter zur Klemme
        klemme(12, 105.0, 155.8),
        wire(4, &[(25.0, 125.0), (105.0, 125.0), (105.0, 155.0)]),
    ]);
    let w2 = pts(&app, 2);
    app.selection = vec![3];
    assert!(app.teilstueck_greifen(3, p(65.0, 45.0), 1.0));
    app.teilstueck_ziehen(p(65.0, 85.0));
    assert_eq!(pts(&app, 2), w2, "E2-Abzweig bleibt");
    let w3 = pts(&app, 3);
    assert_eq!(w3, vec![p(25.0, 85.0), p(105.0, 85.0), p(105.0, 125.0)], "Anfang wandert auf N");
}

/// Nutzervideo 23.11.2026 15:50: senkrechtes Stück der roten Leitung (Abgriff an der Schiene mit kleinem Versatz)
/// auf die Ecke der schwarzen Leitung gezogen. Danach: Abgriff rückt auf der Schiene nach (kein kleines Stück oben),
/// rote und schwarze Leitung werden zu einem durchgehenden Strang zusammengelegt, das Stück von E2 bleibt Abzweig.
#[test]
fn video_knoten_auf_knoten_und_abgriff_nachziehen() {
    let schiene = Obj { id: 9, kind: Kind::Busbar { a: P::new(5.0, 25.0), x2: 295.0, name: "L1".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 1, versatz: 2.5 }, pen: 0, fixed: false, group: None, raw: None };
    let mut app = app_mit(vec![
        schiene,
        klemme(10, 84.2, 145.0),
        // schwarz: von E2 (85,145) nach rechts zur Ecke (115,145), dann runter
        wire(2, &[(85.0, 145.0), (115.0, 145.0), (115.0, 205.0)]),
        // rot: Abgriff (85,25) kurz runter, rechts, runter auf schwarz (95,145)
        wire(1, &[(85.0, 25.0), (85.0, 35.0), (95.0, 35.0), (95.0, 145.0)]),
        // E3-Abzweig auf rot
        klemme(11, 14.2, 65.0),
        wire(3, &[(15.0, 65.0), (95.0, 65.0)]),
    ]);
    app.selection = vec![1];
    assert!(app.teilstueck_greifen(1, p(95.0, 100.0), 1.0));
    app.teilstueck_ziehen(p(115.0, 100.0));
    app.drag = elektroplanung::app::Drag::None;
    app.aufraeumen();
    let s = app.sheet().unwrap();
    let wires: Vec<(u64, Vec<P>)> = s.objects.iter().filter_map(|o| match &o.kind { Kind::Wire { pts, .. } => Some((o.id, pts.clone())), _ => None }).collect();
    // ein durchgehender senkrechter Strang von der Schiene bis unten
    let strang = wires.iter().find(|(_, w)| w.first() == Some(&p(115.0, 25.0)) || w.last() == Some(&p(115.0, 25.0)));
    assert!(strang.is_some_and(|(_, w)| w.len() == 2 && w.contains(&p(115.0, 205.0))), "{wires:?}");
    // E2-Stück endet als Abzweig am Strang
    assert!(wires.iter().any(|(_, w)| w.len() == 2 && w.contains(&p(85.0, 145.0)) && w.contains(&p(115.0, 145.0))), "{wires:?}");
    // E3-Abzweig endet auf dem Strang
    assert!(wires.iter().any(|(_, w)| *w == vec![p(15.0, 65.0), p(115.0, 65.0)]), "{wires:?}");
}
