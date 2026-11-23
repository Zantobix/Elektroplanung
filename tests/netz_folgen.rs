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

fn schliesser(id: u64, x: f32, y: f32) -> Obj {
    Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos: P::new(x, y), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

/// Nutzervideo 23.11.2026 16:22: E3 direkt unter E2 geschoben – die Leitung von E3 lief auf der Leitung von E2 und
/// durch E2 hindurch; nach dem Aufräumen hing E3 am unteren Anschluss von E2. Die geführte Leitung darf kein fremdes
/// Symbol durchqueren und auf keiner anderen Leitung liegen.
#[test]
fn video_lampe_unter_andere_lampe() {
    // Schiene (Leitung) H bei y = 45; A bei (105,105) mit Leitung von H; B bei (155,155) mit Leitung von H
    let mut app = app_mit(vec![
        wire(1, &[(15.0, 45.0), (215.0, 45.0)]),
        schliesser(2, 105.0, 105.0),
        wire(3, &[(105.0, 45.0), (105.0, 98.0)]),
        schliesser(4, 155.0, 155.0),
        wire(5, &[(155.0, 45.0), (155.0, 148.0)]),
    ]);
    app.lib = elektroplanung::symbols::builtin();
    let pa = elektroplanung::canvas::obj_pins(&app.sheet().unwrap().objects[1], &app.lib);
    assert_eq!(pa[0], p(105.0, 98.0), "Anschluss oben von A: {pa:?}");
    app.selection = vec![4];
    app.checkpoint();
    app.netz_merken();
    app.netz_verschieben(&[4], p(-50.0, 20.0));
    app.aufraeumen();
    let s = app.sheet().unwrap();
    let wires: Vec<(u64, Vec<P>)> = s.objects.iter().filter_map(|o| match &o.kind { Kind::Wire { pts, .. } => Some((o.id, pts.clone())), _ => None }).collect();
    // A unverändert angeschlossen, Leitung 3 unverändert
    assert!(wires.iter().any(|(id, w)| *id == 3 && *w == vec![p(105.0, 45.0), p(105.0, 98.0)]), "{wires:?}");
    // keine Leitung berührt den unteren Anschluss von A (105,112)
    assert!(!wires.iter().any(|(_, w)| w.windows(2).any(|v| seg_dist(p(105.0, 112.0), v[0], v[1]) < 0.05)), "E3 hängt an A unten: {wires:?}");
    // die Leitung zu B endet an B oben (105,168) und durchquert A nicht (x = 105 zwischen 98 und 112)
    let b_oben = p(105.0, 168.0);
    let wb = wires.iter().find(|(_, w)| w.iter().any(|q| q.dist(b_oben) < 0.05)).map(|(_, w)| w.clone()).expect("Leitung zu B");
    assert!(!wb.windows(2).any(|v| (v[0].x - 105.0).abs() < 0.05 && (v[1].x - 105.0).abs() < 0.05 && v[0].y.min(v[1].y) < 112.0 && v[0].y.max(v[1].y) > 98.0), "durch A: {wb:?}");
}

/// Maustest 23.11.2026 (T5, licht_maus.epr): Lampe senkrecht unter der Klemme nach unten verschoben – die gerade
/// Leitung Klemme → Lampe bekam einen U-Umweg zur Seite, statt einfach länger zu werden (Kriterium L9/L15).
#[test]
fn maustest_gerade_leitung_wird_beim_verschieben_in_flucht_nur_laenger() {
    // Klemme (25,75) → Schließer oben (25,98); Schließer unten (25,112) → runter, rechts, hoch auf N (y = 45)
    let mut app = app_mit(vec![
        wire(1, &[(15.0, 45.0), (215.0, 45.0)]),
        klemme(2, 25.0, 75.0),
        schliesser(3, 25.0, 105.0),
        wire(4, &[(25.0, 75.0), (25.0, 98.0)]),
        wire(5, &[(25.0, 112.0), (25.0, 135.0), (65.0, 135.0), (65.0, 45.0)]),
    ]);
    app.lib = elektroplanung::symbols::builtin();
    app.selection = vec![3];
    app.checkpoint();
    app.netz_merken();
    for dy in [5.0, 10.0, 15.0, 20.0] {
        app.netz_verschieben(&[3], p(0.0, dy));
        assert_eq!(pts(&app, 4), vec![p(25.0, 75.0), p(25.0, 98.0 + dy)], "beim Ziehen um {dy}");
    }
    app.aufraeumen();
    assert_eq!(pts(&app, 4), vec![p(25.0, 75.0), p(25.0, 118.0)]);
}

/// Ladefunktion für die privaten Maustest-Pläne (`EP_FIXTURES`); ohne sie wird der Test übersprungen.
fn licht_maus() -> Option<(elektroplanung::app::App, u64)> {
    let pfad = std::path::PathBuf::from(std::env::var("EP_FIXTURES").ok()?).join("licht_maus.epr");
    if !pfad.exists() {
        return None;
    }
    let mut app = test_app();
    app.open_path(pfad, false);
    let lampe = app.sheet()?.objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { bmk, .. } if bmk.contains("E1")))?.id;
    Some((app, lampe))
}

fn leitungen(app: &elektroplanung::app::App) -> Vec<(u64, Vec<P>)> {
    app.sheet().unwrap().objects.iter().filter_map(|o| match &o.kind { Kind::Wire { pts, .. } => Some((o.id, pts.clone())), _ => None }).collect()
}

/// Eine Ziehgeste wie in der Oberfläche (view.rs, `Drag::Move`): Gesamtverschiebung je Mausbild, mit
/// Strompfadausrichtung des Symbols, dann Aufräumen beim Loslassen.
fn geste(app: &mut elektroplanung::app::App, id: u64, schritte: &[P]) {
    app.selection = vec![id];
    app.checkpoint();
    app.stoesse_an_markierung_zusammenfassen();
    app.netz_merken();
    let p0 = match &app.sheet().unwrap().objects.iter().find(|o| o.id == id).unwrap().kind {
        Kind::Symbol { pos, .. } | Kind::Terminal { pos, .. } => *pos,
        _ => P::default(),
    };
    for d in schritte {
        let mut d = *d;
        d.x = app.align_to_path(p0.add(d)).x - p0.x;
        app.netz_verschieben(&[id], d);
    }
    app.aufraeumen();
}

/// Maustest 23.11.2026 T5 auf dem mit der Maus gezeichneten Plan (privat `licht_maus.epr`): -E1 um 30 mm nach
/// unten – die gerade Leitung von X1.1 bleibt gerade (vorher U-Umweg: die untere Leitung der Lampe wurde in ihrer
/// Ausgangslage als fremdes Hindernis gesehen).
#[test]
fn maustest_licht_lampe_nach_unten() {
    let Some((mut app, lampe)) = licht_maus() else { return };
    let vorher = leitungen(&app);
    geste(&mut app, lampe, &[p(0.0, 10.0), p(0.0, 20.0), p(0.0, 30.0)]);
    for (id, w) in leitungen(&app) {
        let alt = &vorher.iter().find(|(i, _)| *i == id).unwrap().1;
        assert!(w.len() <= alt.len(), "Leitung {id} bekam Knicke: {alt:?} → {w:?}");
    }
}

/// Maustest 23.11.2026 T2 (Kriterium L10 Rückkehr): -E1 in einer Geste nach rechts, in einer zweiten zurück –
/// danach derselbe Plan wie vorher (vorher: Haken in der Leitung von X1.1, später kam der untere Stich mit 5 statt
/// 10 mm zurück – jetzt Formgedächtnis je Quadrant, leitung_gedaechtnis.rs).
#[test]
fn maustest_licht_lampe_hin_und_zurueck() {
    let Some((mut app, lampe)) = licht_maus() else { return };
    let vorher = leitungen(&app);
    geste(&mut app, lampe, &[p(10.0, 0.0), p(20.0, 0.0), p(34.6, 0.0)]);
    geste(&mut app, lampe, &[p(-10.0, 0.0), p(-20.0, 0.0), p(-34.6, 0.0)]);
    for ((id, w), (_, alt)) in leitungen(&app).iter().zip(&vorher) {
        assert_eq!(w.len(), alt.len(), "Leitung {id}: {alt:?} → {w:?}");
        assert!(w.iter().zip(alt).all(|(a, b)| a.dist(*b) < 0.01), "Leitung {id}: {alt:?} → {w:?}");
    }
}

/// Nutzervideo 23.11.2026 19:04: Klemmstelle X1.1 auf ihrer eigenen Leitung nach oben gezogen – die untere Leitung
/// lief in einer U-Schleife neben der oberen hoch, statt einfach länger zu werden (die obere wird kürzer).
#[test]
fn video_klemmstelle_auf_der_leitung_nach_oben() {
    let Some((mut app, _)) = licht_maus() else { return };
    let klemme = app.sheet().unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Terminal { pos, .. } if (pos.x - 22.9).abs() < 0.1 && (pos.y - 115.0).abs() < 0.1)).expect("X1.1").id;
    let vorher = leitungen(&app);
    for dy in [-10.0, -20.0, -30.0] {
        let mut a2 = licht_maus().unwrap().0;
        geste(&mut a2, klemme, &[p(0.0, dy / 3.0), p(0.0, dy * 2.0 / 3.0), p(0.0, dy)]);
        for (id, w) in leitungen(&a2) {
            let alt = vorher.iter().find(|(i, _)| *i == id).map(|(_, w)| w.clone()).unwrap_or_default();
            assert!(w.len() <= alt.len().max(2), "dy {dy}: Leitung {id} bekam Knicke: {alt:?} → {w:?}");
            assert!(w.iter().all(|q| (q.x - 22.9).abs() < 0.05) || alt.iter().any(|q| (q.x - 22.9).abs() > 0.05), "dy {dy}: Leitung {id} verlässt die Flucht: {alt:?} → {w:?}");
        }
    }
    let _ = &mut app;
}


/// Dasselbe Video, zweiter Teil: -E1 danach nach unten – der Stich am unteren Anschluss verschwand, die Leitung lief
/// seitlich aus dem Anschluss (Kriterium L14).
#[test]
fn video_lampe_nach_unten_stich_bleibt() {
    let Some((mut app, lampe)) = licht_maus() else { return };
    geste(&mut app, lampe, &[p(0.0, 10.0), p(0.0, 20.0)]);
    let w = leitungen(&app).into_iter().find(|(_, w)| w[0].dist(p(22.9, 155.0)) < 0.05).expect("untere Leitung").1;
    assert!((w[1].x - 22.9).abs() < 0.05 && w[1].y > 155.0 + 2.0, "Stich nach unten fehlt: {w:?}");
}

/// Rückkehr (netz_folgen.rs `rueckkehr_anwenden`) darf eine danach gezeichnete Leitung nie zurücknehmen: Lampe hin
/// und zurück, dann eine neue Leitung – sie bleibt, auch nach einer weiteren Geste hin und zurück.
#[test]
fn rueckkehr_behaelt_neue_leitung() {
    let Some((mut app, lampe)) = licht_maus() else { return };
    geste(&mut app, lampe, &[p(35.0, 0.0)]);
    geste(&mut app, lampe, &[p(-35.0, 0.0)]);
    // „gezeichnet“: neue freie Leitung mit eigenem Rückgängig-Schritt und Aufräumen
    app.checkpoint();
    app.with_sheet(|s, _| s.objects.push(wire(999, &[(100.0, 60.0), (150.0, 60.0)])));
    app.aufraeumen();
    assert!(leitungen(&app).iter().any(|(id, _)| *id == 999));
    geste(&mut app, lampe, &[p(35.0, 0.0)]);
    geste(&mut app, lampe, &[p(-35.0, 0.0)]);
    assert!(leitungen(&app).iter().any(|(id, _)| *id == 999), "neue Leitung verschwand bei der Rückkehr");
}
