//! Netz-Wächter (todo N00, netz_waechter.rs): feste Fälle aus dem Zufallsprüfer `netz_zufall.rs` auf W2 – jede
//! Bearbeitung lässt die elektrischen Netze unverändert; was nur mit Kurzschluss ginge, wird abgewiesen.
use elektroplanung::app::App;
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use elektroplanung::netz_waechter::{knoten, verbunden};

fn w2(blatt: usize) -> Option<App> {
    let f = std::env::var_os("EP_SAMPLE")?;
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    app.active = blatt;
    Some(app)
}

fn pts(app: &App, id: u64) -> Vec<P> {
    match &app.sheet().unwrap().objects.iter().find(|o| o.id == id).unwrap().kind {
        Kind::Wire { pts, .. } => pts.clone(),
        _ => panic!(),
    }
}

/// Netze vorher/nachher gleich (ohne Knoten der Objekte `ohne`): weder verbunden noch getrennt.
fn netze_gleich(vorher: &Sheet, app: &App, ohne: &[u64]) {
    let (a, b) = (knoten(vorher, &app.lib, &[]), knoten(app.sheet().unwrap(), &app.lib, &[]));
    assert_eq!(verbunden(&a, &b, ohne), 0, "fremde Netze verbunden");
    assert_eq!(verbunden(&b, &a, ohne), 0, "Netz getrennt");
}

fn teilstueck(app: &mut App, id: u64, i: usize, d: P) {
    let p = pts(app, id);
    let m = P::new((p[i].x + p[i + 1].x) / 2.0, (p[i].y + p[i + 1].y) / 2.0);
    app.selection = vec![id];
    assert!(app.teilstueck_greifen(id, m, 0.3));
    app.teilstueck_ziehen(m.add(d));
    app.drag = elektroplanung::app::Drag::None;
    app.aufraeumen();
}

/// W2 Blatt 15: senkrechtes Endstück an einer Klemme nach links gezogen – das Querstück lief waagerecht aus dem
/// Anschluss durch den Anschluss der Nachbarklemme (Kurzschluss). Jetzt verlässt die Leitung den Anschluss erst
/// senkrecht (Stich), die Bearbeitung wird ausgeführt.
#[test]
fn querstueck_mit_stich_statt_durch_die_nachbarklemme() {
    let Some(mut app) = w2(15) else { return };
    let vorher = app.sheet().unwrap().clone();
    teilstueck(&mut app, 1055, 0, P::new(-5.0, -2.5));
    assert_eq!(app.netz_abgewiesen, 0, "{}", app.info);
    let p = pts(&app, 1055);
    assert_eq!(p[0], P::new(144.1, 155.0));
    assert!((p[1].x - 144.1).abs() < 0.01 && p[1].y < 155.0, "Stich senkrecht aus dem Anschluss: {p:?}");
    netze_gleich(&vorher, &app, &[]);
}

/// W2 Blatt 14: Klemmstelle -X1:3 auf eine fremde Leitung geschoben – abgewiesen, das Blatt bleibt unverändert.
#[test]
fn klemmstelle_auf_fremde_leitung_wird_abgewiesen() {
    let Some(mut app) = w2(14) else { return };
    let vorher = app.sheet().unwrap().clone();
    app.checkpoint();
    app.selection = vec![1018];
    app.netz_merken();
    app.netz_verschieben(&[1018], P::new(5.0, -5.0));
    assert_eq!(app.netz_abgewiesen, 1);
    assert!(app.info.contains("Netze verbinden"), "{}", app.info);
    assert_eq!(app.sheet().unwrap().objects, vorher.objects);
}

/// W2 Blatt 8: Klemme -X5 gedreht – ein Anschluss käme auf die Ecke der Leitung eines anderen Anschlusses.
/// Abgewiesen (zurückgenommen, ohne Wiederholen-Schritt), Markierung bleibt.
#[test]
fn drehen_mit_kurzschluss_wird_zurueckgenommen() {
    let Some(mut app) = w2(8) else { return };
    let vorher = app.sheet().unwrap().clone();
    let undo = app.undo.len();
    app.selection = vec![732];
    app.transform_selection(1);
    assert_eq!(app.netz_abgewiesen, 1);
    assert!(app.info.starts_with("Nicht gedreht"), "{}", app.info);
    assert_eq!(app.sheet().unwrap().objects, vorher.objects);
    assert_eq!(app.undo.len(), undo);
    assert!(app.redo.is_empty());
    assert_eq!(app.selection, vec![732]);
}

/// W2 Blatt 10: nach dem Ziehen glättete das Aufräumen die Leitung zu einer Geraden durch den Anschluss der
/// Nachbarklemme. Der Aufräumschritt wird zurückgenommen, die Netze bleiben.
#[test]
fn glaetten_durch_fremden_anschluss_unterbleibt() {
    let Some(mut app) = w2(10) else { return };
    let vorher = app.sheet().unwrap().clone();
    teilstueck(&mut app, 887, 0, P::new(-2.5, 0.0));
    netze_gleich(&vorher, &app, &[]);
    let p = pts(&app, 887);
    assert!(p.windows(2).all(|w| seg_dist(P::new(109.3, 157.5), w[0], w[1]) > 0.1), "durch den Anschluss: {p:?}");
}

/// W2 Blatt 5: Teilstück auf eine fremde Leitung gezogen (gleiche Höhe) – abgewiesen statt beim Aufräumen zu einem
/// Abzweig vereinigt.
#[test]
fn teilstueck_auf_fremde_leitung_wird_abgewiesen() {
    let Some(mut app) = w2(5) else { return };
    let vorher = app.sheet().unwrap().clone();
    teilstueck(&mut app, 462, 1, P::new(2.5, -5.0));
    assert_eq!(app.netz_abgewiesen, 1, "{}", app.info);
    netze_gleich(&vorher, &app, &[]);
}

fn wire(id: u64, p: &[(f32, f32)]) -> Obj {
    Obj { id, kind: Kind::Wire { pts: p.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
}

fn schiene(id: u64, y: f32, x1: f32, x2: f32, name: &str) -> Obj {
    let kind: Kind = serde_json::from_value(serde_json::json!({"Busbar": {"a": {"x": x1, "y": y}, "x2": x2, "name": name, "label": {"x": x1, "y": y}}})).unwrap();
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

/// Gewollt: ein **freies** Ende einer verschobenen Leitung auf eine andere Leitung gesetzt verbindet (kein Abweisen),
/// obwohl dadurch die Netze L1 und N zusammenkommen.
#[test]
fn freies_ende_darf_verbinden() {
    let mut app = test_app();
    app.with_sheet(|s, _| {
        s.objects.push(schiene(1, 20.0, 0.0, 100.0, "L1"));
        s.objects.push(schiene(2, 80.0, 0.0, 100.0, "N"));
        // von L1 nach unten und rechts, freies Ende bei (65, 50)
        s.objects.push(wire(3, &[(30.0, 20.0), (30.0, 50.0), (65.0, 50.0)]));
        // von N nach oben
        s.objects.push(wire(4, &[(70.0, 80.0), (70.0, 30.0)]));
    });
    app.checkpoint();
    app.selection = vec![3];
    app.netz_merken();
    app.netz_verschieben(&[3], P::new(5.0, 0.0));
    assert_eq!(app.netz_abgewiesen, 0, "{}", app.info);
    let p = pts(&app, 3);
    assert_eq!(p[p.len() - 1], P::new(70.0, 50.0));
    // ohne freies Ende (an PE angeschlossen) wäre derselbe Schritt ein Kurzschluss mit N
    app.do_undo();
    app.with_sheet(|s, _| {
        s.objects.push(schiene(5, 90.0, 50.0, 100.0, "PE"));
        if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == 3).map(|o| &mut o.kind) {
            *pts = vec![P::new(30.0, 20.0), P::new(30.0, 50.0), P::new(65.0, 50.0), P::new(65.0, 90.0)];
        }
    });
    app.checkpoint();
    app.selection = vec![3];
    app.netz_merken();
    app.netz_verschieben(&[3], P::new(5.0, 0.0));
    assert_eq!(app.netz_abgewiesen, 1, "{}", app.info);
}
