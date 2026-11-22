//! Leitung an einer eingesetzten Klemmstelle verschieben (Nutzer-Befund Runde 11): Das Ende am Klemmenkreis
//! muss angeschlossen bleiben.
use elektroplanung::app::Drag;
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
    let kind = Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 };
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

fn pts(app: &elektroplanung::app::App, id: u64) -> Vec<P> {
    match &app.sheet().unwrap().objects.iter().find(|o| o.id == id).unwrap().kind {
        Kind::Wire { pts, .. } => pts.clone(),
        _ => panic!(),
    }
}

#[test]
fn teilstueck_an_klemme() {
    for (greif, ziel) in [((100.0, 120.0), (150.0, 120.0)), ((75.0, 150.0), (75.0, 170.0)), ((100.0, 120.0), (100.0, 60.0))] {
        let mut app = test_app();
        app.project = Project::new_default();
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        app.with_sheet(|s, _| s.objects.push(wire(900, &[(100.0, 40.0), (100.0, 150.0), (50.0, 150.0)])));
        app.klemmstelle_setzen(P::new(100.0, 85.0), 1.0);
        let alle: Vec<(u64, Vec<P>)> = app.sheet().unwrap().objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some((o.id, pts.clone())) } else { None }).collect();
        let unten = alle.iter().find(|(_, p)| p.len() == 3).unwrap().0;
        let m = P::new(greif.0, greif.1);
        app.selection = vec![unten];
        assert!(app.teilstueck_greifen(unten, m, 1.0), "{alle:?}");
        app.teilstueck_ziehen(P::new(ziel.0, ziel.1));
        app.drag = Drag::None;
        let p = pts(&app, unten);
        assert!(p[0].dist(P::new(100.0, 85.8)) < 0.01, "{greif:?}→{ziel:?}: {p:?}");
    }
}

/// Testprojekt aus Runde 10 (fixtures/r10_klemmen.epr): Leitung 13 (Spule → Klemme 18) am senkrechten Endstück zur Seite ziehen.
#[test]
fn r10_endstueck_zur_klemme() {
    let Ok(dir) = std::env::var("EP_FIXTURES") else { return };
    let pfad = std::path::Path::new(&dir).join("r10_klemmen.epr");
    if !pfad.exists() { return; }
    for (greif, ziel) in [((225.0, 100.0), (240.0, 100.0)), ((225.0, 100.0), (210.0, 100.0)), ((200.0, 133.0), (200.0, 150.0)), ((225.0, 70.0), (245.0, 70.0))] {
        let mut app = test_app();
        app.open_path(pfad.clone(), false);
        app.active = 0;
        app.selection = vec![13];
        let m = P::new(greif.0, greif.1);
        assert!(app.teilstueck_greifen(13, m, 1.0));
        app.teilstueck_ziehen(P::new(ziel.0, ziel.1));
        app.drag = Drag::None;
        let q = pts(&app, 13);
        assert!(q.last().unwrap().dist(P::new(225.0, 65.8)) < 0.01, "{greif:?}→{ziel:?}: {q:?}");
    }
}

/// Maustest Runde 11: Relais -K1.1 verschoben → Leitung 13 verlor ihr Endstück an der Klemme X1.3 (endete 5 mm
/// darunter).
#[test]
fn relais_verschoben_leitung_bleibt_an_klemme() {
    let Ok(dir) = std::env::var("EP_FIXTURES") else { return };
    let pfad = std::path::Path::new(&dir).join("r10_klemmen.epr");
    if !pfad.exists() { return; }
    for (ri, d) in [(0usize, P::new(-30.0, 20.0)), (1, P::new(-30.0, 20.0)), (2, P::new(-30.0, 20.0)), (3, P::new(-30.0, 20.0)), (3, P::new(0.0, 20.0)), (4, P::new(-30.0, 20.0)), (5, P::new(-30.0, 20.0)), (5, P::new(-20.0, 20.0)), (6, P::new(-30.0, 20.0)), (5, P::new(-36.25, 20.0)), (5, P::new(-35.9, 20.0)), (3, P::new(-36.25, 20.0))] {
        let mut app = test_app();
        app.open_path(pfad.clone(), false);
        app.raster_idx = ri;
        app.active = 0;
        app.selection = vec![9];
        let f = app.wire_followers(&[9]);
        app.with_sheet(|s, _| s.objects.iter_mut().filter(|o| o.id == 9).for_each(|o| o.kind.translate(d)));
        app.apply_followers(&f, d);
        let q = pts(&app, 13);
        assert!(q.last().unwrap().dist(P::new(225.0, 65.8)) < 0.01, "{d:?}: {q:?}");
    }
}

/// Leitung allein markiert und als Ganzes verschoben (z. B. per Rahmen): Enden an der Klemmstelle und am Relais
/// bleiben angeschlossen.
#[test]
fn markierte_leitung_bleibt_an_klemme_und_relais() {
    let Ok(dir) = std::env::var("EP_FIXTURES") else { return };
    let pfad = std::path::Path::new(&dir).join("r10_klemmen.epr");
    if !pfad.exists() {
        return;
    }
    for d in [P::new(30.0, 0.0), P::new(0.0, 10.0), P::new(0.0, -10.0), P::new(20.0, 20.0)] {
        let mut app = test_app();
        app.open_path(pfad.clone(), false);
        app.active = 0;
        app.selection = vec![13];
        let halt = elektroplanung::schiene_halten::finden(app.sheet().unwrap(), &app.lib, &[13]);
        assert_eq!(halt.len(), 1);
        app.with_sheet(|s, _| s.objects.iter_mut().filter(|o| o.id == 13).for_each(|o| o.kind.translate(d)));
        app.schienen_halten(&halt, d);
        let q = pts(&app, 13);
        assert!(q[0].dist(P::new(166.4, 128.0)) < 0.01, "{d:?}: {q:?}");
        assert!(q.last().unwrap().dist(P::new(225.0, 65.8)) < 0.01, "{d:?}: {q:?}");
        // achsengerade
        assert!(q.windows(2).all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01), "{d:?}: {q:?}");
    }
}

#[test]
fn doppelte_leitung_erkannt() {
    use elektroplanung::klemme_leitung::liegt_auf_leitungen;
    let mut s = Sheet::new(1);
    s.objects.push(wire(1, &[(10.0, 10.0), (10.0, 50.0)]));
    s.objects.push(wire(2, &[(10.0, 50.0), (40.0, 50.0)]));
    assert!(liegt_auf_leitungen(&s, &[P::new(10.0, 10.0), P::new(10.0, 50.0)]));
    assert!(liegt_auf_leitungen(&s, &[P::new(10.0, 30.0), P::new(10.0, 50.0), P::new(30.0, 50.0)]));
    assert!(!liegt_auf_leitungen(&s, &[P::new(10.0, 30.0), P::new(20.0, 30.0)]));
    assert!(!liegt_auf_leitungen(&s, &[P::new(10.0, 40.0), P::new(10.0, 60.0)]));
}
