//! Nachzeichnen von W2 mit der Maus (24.11.2026, Blatt „Sicherungen -F10, -F1 und -F2“): vom RCD (4P) zum
//! Leitungsschutzschalter (2P) bei Raster 1 mm. Zuerst L1 auf den linken Pol, dann N auf den rechten Pol mit einem
//! Mausweg über eine eigene Höhe. Gesehen: der Stich war nur ein halbes Raster (0,5 mm), und die N-Leitung lief
//! waagerecht auf Anschlusshöhe über den linken Pol in den rechten – N und L1 verbunden.
//! Ohne EP_DATENBANK übersprungen.

use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_fuehrung::Fuehrung;
use elektroplanung::model::*;
use elektroplanung::symbols;
use std::path::PathBuf;

fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

fn sym(id: u64, key: &str, pos: P, option: u16) -> Obj {
    Obj { id, kind: Kind::Symbol { key: key.into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

fn blatt() -> Option<App> {
    let db = std::env::var("EP_DATENBANK").ok()?;
    let lib = symbols::load_db(&PathBuf::from(db)).ok()?;
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = lib;
    app.raster_idx = 0; // 1,0 mm
    app.grid = true;
    app.with_sheet(|s, _| {
        s.objects.push(sym(1, "db:300:28:RCD", p(69.6, 55.0), 1));
        s.objects.push(sym(2, "db:300:28:Leitungsschutzsch.", p(98.3, 90.0), 1));
    });
    Some(app)
}

/// Leitung mit der Maus über `ecken` ziehen (wie die Oberfläche) und beim Loslassen anlegen; liefert den Weg.
fn ziehen(app: &mut App, ecken: &[P]) -> Vec<P> {
    let z = 4.83;
    app.drag = Drag::Wire { f: Fuehrung::neu(ecken[0]) };
    let mut vorher = ecken[0];
    for e in &ecken[1..] {
        let n = (vorher.dist(*e) / 0.4).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            app.leitung_zug_bewegen(P::new(vorher.x + (e.x - vorher.x) * t, vorher.y + (e.y - vorher.y) * t), z);
        }
        vorher = *e;
    }
    let Drag::Wire { f } = std::mem::replace(&mut app.drag, Drag::None) else { unreachable!() };
    let ende = app.leitungsende(&f, vorher, z);
    let w = app.leitungsweg(&f, ende);
    app.leitung_anschliessen(w.clone());
    w
}

fn pins(app: &App) -> Vec<P> {
    let s = app.sheet().unwrap();
    s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| elektroplanung::canvas::obj_pins(o, &app.lib)).collect()
}

/// Liegt `q` auf dem Linienzug `w` (Endpunkte ausgenommen)?
fn auf_zug(q: P, w: &[P]) -> bool {
    if q.dist(w[0]) < 0.05 || q.dist(w[w.len() - 1]) < 0.05 {
        return false;
    }
    w.windows(2).any(|s| {
        let (a, b) = (s[0], s[1]);
        let t = 0.05;
        q.x >= a.x.min(b.x) - t && q.x <= a.x.max(b.x) + t && q.y >= a.y.min(b.y) - t && q.y <= a.y.max(b.y) + t
    })
}

#[test]
fn rcd_auf_ls_zweiter_pol() {
    let Some(mut app) = blatt() else { return };
    let alle = pins(&app);
    // Anschlüsse: RCD unten (y 67,5), LS oben (y 90)
    let nah = |q: P| *alle.iter().min_by(|a, b| a.dist(q).total_cmp(&b.dist(q))).unwrap();
    let (rcd_n, rcd_l1) = (nah(p(69.6, 67.5)), nah(p(74.6, 67.5)));
    let (ls_l, ls_n) = (nah(p(98.3, 90.0)), nah(p(103.3, 90.0)));
    assert!(rcd_n.dist(p(69.6, 67.5)) < 0.6 && ls_n.dist(p(103.3, 90.0)) < 0.6, "Anschlüsse {alle:?}");

    let w1 = ziehen(&mut app, &[rcd_l1, p(rcd_l1.x, 78.0), p(ls_l.x, 78.0), ls_l]);
    assert_eq!((w1[0], *w1.last().unwrap()), (rcd_l1, ls_l), "L1: {w1:?}");
    let kurz = w1.windows(2).map(|s| s[0].dist(s[1])).fold(f32::MAX, f32::min);
    assert!(kurz >= 2.0, "kein Mini-Stich bei Raster 1 mm: {w1:?}");

    let w2 = ziehen(&mut app, &[rcd_n, p(rcd_n.x, 84.0), p(ls_n.x, 84.0), ls_n]);
    assert_eq!((w2[0], *w2.last().unwrap()), (rcd_n, ls_n), "N: {w2:?}");
    for q in [ls_l, rcd_l1] {
        assert!(!auf_zug(q, &w2), "N läuft über den fremden Anschluss {q:?}: {w2:?}");
    }
    assert!(!elektroplanung::leitung_route::ueberlappt(&{
        let mut s = app.sheet().unwrap().clone();
        s.objects.retain(|o| !matches!(&o.kind, Kind::Wire { pts, .. } if pts == &w2));
        s
    }, &w2), "N liegt auf der L1-Leitung: {w2:?} / {w1:?}");
    // beide Leitungen getrennt: zwei Leitungen auf dem Blatt
    let n = app.sheet().unwrap().objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).count();
    assert_eq!(n, 2);
}

/// Steckdose (N/PE/L oben) unter einer waagerechten L-Leitung, die genau einen Stich über der Anschlussreihe liegt
/// (so legt der Nachbau Leitungen zu einer Anschlussreihe selbst). PE vom Pin nach unten zur PE-Schiene ziehen:
/// Gesehen (Nachzeichnen 24.11.2026): der Stich endete auf der L-Leitung, der Weg lief auf ihr entlang und wurde mit
/// ihr vereinigt – PE und L verbunden.
#[test]
fn pe_nach_unten_nicht_auf_l_leitung() {
    let Some(mut app) = blatt() else { return };
    app.with_sheet(|s, _| {
        s.objects.retain(|o| o.id > 2);
        s.objects.push(sym(5, "db:300:2769841913:Steckdose", p(127.0, 130.0), 0));
        s.objects.push(Obj { id: 6, kind: Kind::Wire { pts: vec![p(98.3, 102.5), p(98.3, 127.0), p(261.1, 127.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        // N-Leitung 3 mm darüber und die Abzweige von L/N der Steckdose wie im Blatt
        s.objects.push(Obj { id: 8, kind: Kind::Wire { pts: vec![p(103.3, 102.5), p(103.3, 124.0), p(251.1, 124.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj { id: 9, kind: Kind::Wire { pts: vec![p(132.0, 130.0), p(132.0, 127.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj { id: 10, kind: Kind::Wire { pts: vec![p(122.0, 130.0), p(122.0, 124.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        // Steckdose der zweiten Reihe rechts unterhalb (wie -X7)
        s.objects.push(sym(11, "db:300:2769841913:Steckdose", p(141.3, 165.0), 0));
        s.objects.push(sym(12, "db:300:2769841913:Steckdose", p(155.7, 130.0), 0));
        s.objects.push(sym(13, "db:300:2769841913:Steckdose", p(155.7, 165.0), 0));
        s.objects.push(Obj { id: 7, kind: Kind::Busbar { a: p(13.0, 183.0), x2: 271.0, name: "PE".into(), label: p(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 }, pen: 0, fixed: false, group: None, raw: None });
    });
    let alle = pins(&app);
    let pe = *alle.iter().min_by(|a, b| a.dist(p(127.0, 130.0)).total_cmp(&b.dist(p(127.0, 130.0)))).unwrap();
    assert!(pe.dist(p(127.0, 130.0)) < 0.6, "PE-Pin {alle:?}");
    let l_vorher = app.sheet().unwrap().objects.iter().find(|o| o.id == 6).unwrap().kind.clone();
    let w = ziehen(&mut app, &[pe, p(127.0, 160.0), p(127.0, 183.0)]);
    let s = app.sheet().unwrap();
    let neu: Vec<Vec<P>> = s.objects.iter().filter(|o| ![6, 8, 9, 10].contains(&o.id)).filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.clone()) } else { None }).collect();
    assert_eq!(neu, vec![w.clone()], "beim Anlegen unverändert eine Leitung (nicht an Kreuzungen aufgetrennt)");
    assert_eq!(s.objects.iter().find(|o| o.id == 6).map(|o| o.kind.clone()), Some(l_vorher), "L-Leitung unverändert; PE-Weg {w:?}");
    let mut ohne = s.clone();
    ohne.objects.retain(|o| !matches!(&o.kind, Kind::Wire { pts, .. } if *pts == w));
    assert!(!elektroplanung::leitung_route::ueberlappt(&ohne, &w), "PE-Weg liegt auf der L-Leitung: {w:?}");
    assert!(w.iter().all(|q| (q.y - 127.0).abs() > 0.01), "PE-Weg berührt die L-Höhe: {w:?}");
    assert!(!elektroplanung::leitung_route::ueber_fremdem_anschluss(&ohne, &app.lib, &w), "PE-Weg über fremdem Anschluss: {w:?}");
}
