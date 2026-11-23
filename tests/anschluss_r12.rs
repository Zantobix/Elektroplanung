//! Prüfliste Runde 12: Leitungen folgen beim Skalieren/Drehen/Spiegeln verdrahteter Symbole (C1/C2) und neue
//! Leitungen enden bzw. beginnen auf anderen Leitungen, freie Überstände werden gekürzt (A5–A8).
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use elektroplanung::{canvas, symbols};

fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
    let kind = Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 };
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

fn sym(id: u64, pos: P) -> Obj {
    let kind = Kind::Symbol { key: "builtin:schliesser".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: "-S1".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() };
    Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
}

fn klemme(id: u64, pos: P) -> Obj {
    Obj { id, kind: Kind::Terminal { pos, leiste: "X".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

fn pts(app: &elektroplanung::app::App, id: u64) -> Vec<P> {
    match &app.sheet().unwrap().objects.iter().find(|o| o.id == id).unwrap().kind {
        Kind::Wire { pts, .. } => pts.clone(),
        _ => panic!(),
    }
}

fn rechtwinklig(p: &[P]) -> bool {
    p.windows(2).all(|w| (w[0].x - w[1].x).abs() < 1e-3 || (w[0].y - w[1].y).abs() < 1e-3)
}

/// Schließer bei (100, 100) (Anschlüsse (100, 93) oben, (100, 107) unten), Leitung 2 von oben nach rechts zur
/// Klemme, Leitung 3 von unten nach links zur Klemme.
fn app_mit_schliesser() -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = symbols::builtin();
    app.with_sheet(|s, _| {
        s.objects.push(sym(1, P::new(100.0, 100.0)));
        s.objects.push(wire(2, &[(100.0, 93.0), (100.0, 60.0), (150.0, 60.0)]));
        s.objects.push(klemme(4, P::new(150.8, 60.0)));
        s.objects.push(wire(3, &[(100.0, 107.0), (100.0, 140.0), (40.0, 140.0)]));
        s.objects.push(klemme(5, P::new(39.2, 140.0)));
    });
    app
}

fn pins(app: &elektroplanung::app::App) -> Vec<P> {
    let o = app.sheet().unwrap().objects.iter().find(|o| o.id == 1).unwrap().clone();
    canvas::obj_pins(&o, &app.lib)
}

fn pruefe_angeschlossen(app: &elektroplanung::app::App, was: &str) {
    let pn = pins(app);
    let (a, b) = (pts(app, 2), pts(app, 3));
    assert!(a[0].dist(pn[0]) < 0.01, "{was}: Leitung 2 {a:?}, Anschlüsse {pn:?}");
    assert!(b[0].dist(pn[1]) < 0.01, "{was}: Leitung 3 {b:?}, Anschlüsse {pn:?}");
    // die fernen Enden bleiben an den Klemmen
    assert_eq!(*a.last().unwrap(), P::new(150.0, 60.0), "{was}");
    assert_eq!(*b.last().unwrap(), P::new(40.0, 140.0), "{was}");
    assert!(rechtwinklig(&a) && rechtwinklig(&b), "{was}: {a:?} {b:?}");
}

#[test]
fn skalieren_leitungen_folgen() {
    for (griff, faktor) in [(4usize, 2.0f32), (0, 2.0), (4, 0.5), (2, 1.5)] {
        let mut app = app_mit_schliesser();
        app.selection = vec![1];
        let (mn, mx) = app.sel_frame().unwrap();
        app.checkpoint();
        let orig = app.selected_objs();
        app.netz_merken();
        // Eckgriff 4 = rechts unten (Festpunkt links oben), 0 = links oben, 2 = rechts oben
        let fest = match griff {
            0 => mx,
            2 => P::new(mn.x, mx.y),
            _ => mn,
        };
        let ecke = match griff {
            0 => mn,
            2 => P::new(mx.x, mn.y),
            _ => mx,
        };
        let cur = P::new(fest.x + (ecke.x - fest.x) * faktor, fest.y + (ecke.y - fest.y) * faktor);
        // mehrere Bilder des Ziehens: jedes aus dem Zustand beim Drücken
        for k in 1..=4 {
            let t = k as f32 / 4.0;
            let c = P::new(ecke.x + (cur.x - ecke.x) * t, ecke.y + (cur.y - ecke.y) * t);
            app.rahmen_ziehen(griff, (mn, mx), &orig, c);
        }
        let pn = pins(&app);
        assert!(pn[0].dist(P::new(100.0, 93.0)) > 0.5 || pn[1].dist(P::new(100.0, 107.0)) > 0.5, "Anschlüsse unverändert");
        pruefe_angeschlossen(&app, &format!("Griff {griff} × {faktor}"));
        // ein Rückgängig-Schritt stellt alles her
        app.do_undo();
        assert_eq!(pts(&app, 2), vec![P::new(100.0, 93.0), P::new(100.0, 60.0), P::new(150.0, 60.0)]);
    }
}

#[test]
fn drehen_und_spiegeln_leitungen_folgen() {
    for ops in [vec![1u8], vec![1, 1], vec![1, 1, 1], vec![2], vec![4], vec![1, 2]] {
        let mut app = app_mit_schliesser();
        app.selection = vec![1];
        for &op in &ops {
            app.transform_selection(op);
            pruefe_angeschlossen(&app, &format!("{ops:?}"));
        }
        // keine Leitung läuft durch das Symbol (Umriss ohne Rand)
        let o = app.sheet().unwrap().objects.iter().find(|o| o.id == 1).unwrap().clone();
        let (mn, mx) = canvas::obj_bounds(&o, &app.lib);
        for id in [2, 3] {
            let p = pts(&app, id);
            for w in p.windows(2).skip(1) {
                let m = P::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
                assert!(!(m.x > mn.x + 0.5 && m.x < mx.x - 0.5 && m.y > mn.y + 0.5 && m.y < mx.y - 0.5), "{ops:?}: Leitung {id} {p:?} im Symbol {mn:?}–{mx:?}");
            }
        }
    }
}

fn app_leer() -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = symbols::builtin();
    app
}

#[test]
fn ende_auf_leitung_kuerzt_freien_ueberstand() {
    let mut app = app_leer();
    app.with_sheet(|s, _| {
        s.objects.push(klemme(4, P::new(19.2, 50.0)));
        s.objects.push(wire(10, &[(20.0, 50.0), (120.0, 50.0)]));
    });
    let n0 = app.undo.len();
    let id = app.leitung_anschliessen(vec![P::new(80.0, 100.0), P::new(80.0, 50.0)]).unwrap();
    assert_eq!(app.undo.len(), n0 + 1, "ein Rückgängig-Schritt");
    // gekürzt bis zum Anschlusspunkt; beide stoßen dort ohne Abzweig an → eine Leitung (Runde 13, leitung_zusammen.rs)
    assert_eq!(id, 10);
    assert_eq!(pts(&app, 10), vec![P::new(20.0, 50.0), P::new(80.0, 50.0), P::new(80.0, 100.0)]);
    // Ecke: kein Knoten
    assert!(elektroplanung::knoten::punkte(app.sheet().unwrap()).is_empty());
    app.do_undo();
    assert_eq!(pts(&app, 10), vec![P::new(20.0, 50.0), P::new(120.0, 50.0)]);
    assert_eq!(app.sheet().unwrap().objects.len(), 2);
}

#[test]
fn abzweig_mit_knoten_und_anfang_auf_leitung() {
    let mut app = app_leer();
    app.with_sheet(|s, _| {
        s.objects.push(klemme(4, P::new(19.2, 50.0)));
        s.objects.push(wire(10, &[(20.0, 50.0), (120.0, 50.0)]));
        s.objects.push(klemme(5, P::new(120.8, 50.0)));
    });
    // A5: beide Enden angeschlossen → Abzweig, Knoten
    let id = app.leitung_anschliessen(vec![P::new(80.0, 100.0), P::new(80.0, 50.0)]).unwrap();
    assert_eq!(pts(&app, 10).len(), 2);
    assert_eq!(elektroplanung::knoten::punkte(app.sheet().unwrap()), vec![P::new(80.0, 50.0)]);
    // A8: Umschalt + Ziehen auf der Leitung beginnt dort (Rasterpunkt in der Flucht der Leitung)
    assert!(app.leitung_an_leitung_beginnen(P::new(50.3, 50.4), 1.0));
    let elektroplanung::app::Drag::Wire { f } = app.drag.clone() else { panic!() };
    let x = app.snap(P::new(50.3, 50.4)).x;
    assert_eq!(f.start(), P::new(x, 50.0));
    app.drag = elektroplanung::app::Drag::None;
    let id2 = app.leitung_anschliessen(vec![P::new(x, 50.0), P::new(x, 20.0)]).unwrap();
    let k = elektroplanung::knoten::punkte(app.sheet().unwrap());
    assert!(k.contains(&P::new(x, 50.0)) && k.contains(&P::new(80.0, 50.0)), "{k:?}");
    assert!(id2 != id);
    // A5 beim Zeichnen: Fang auf das Innere der Leitung
    let e = elektroplanung::leitung_route::ende_auf_leitung(app.sheet().unwrap(), P::new(100.0, 90.0), P::new(99.0, 50.6), 1.0);
    assert_eq!(e, Some(P::new(100.0, 50.0)));
}

/// W2: verdrahtete Symbole drehen – alle angeschlossenen Leitungsenden sitzen danach auf den neuen Anschlüssen;
/// geschrieben und wieder gelesen bleibt das so. Die unveränderte Datei bleibt byteidentisch.
#[test]
fn w2_drehen_rundreise() {
    let p = std::env::var("EP_SAMPLE").map(std::path::PathBuf::from).unwrap_or_default();
    if !p.exists() {
        return;
    }
    let mut app = test_app();
    app.open_path(p.clone(), false);
    assert_eq!(elektroplanung::epr_write::to_bytes(&app.project).unwrap(), std::fs::read(&p).unwrap());
    // erstes Blatt mit einem Symbol, an dem zwei Leitungen hängen
    let mut gefunden = None;
    'aussen: for (ai, a) in app.project.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })) {
                let f = elektroplanung::mitziehen::find_followers(s, &app.lib, &[o.id], &[], false);
                if f.len() >= 2 && f.iter().all(|w| w.wirt.is_none()) {
                    gefunden = Some((ai, si, o.id, f.iter().map(|w| w.id).collect::<Vec<_>>()));
                    break 'aussen;
                }
            }
        }
    }
    let (ai, si, sid, wids) = gefunden.expect("Symbol mit Leitungen");
    app.tabs = vec![SheetRef { anlage: ai, sheet: si }];
    app.active = 0;
    app.selection = vec![sid];
    app.transform_selection(1);
    let enden = |app: &elektroplanung::app::App| {
        let s = app.sheet().unwrap();
        let o = s.objects.iter().find(|o| o.id == sid).unwrap();
        let pn = canvas::obj_pins(o, &app.lib);
        wids.iter().all(|w| {
            let Kind::Wire { pts, .. } = &s.objects.iter().find(|o| o.id == *w).unwrap().kind else { return false };
            pn.iter().any(|q| q.dist(pts[0]) < 0.05 || q.dist(*pts.last().unwrap()) < 0.05)
        })
    };
    assert!(enden(&app), "Leitungen nach dem Drehen nicht angeschlossen");
    let bytes = elektroplanung::epr_write::to_bytes(&app.project).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_anschluss_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let mut back = test_app();
    back.open_path(tmp.clone(), false);
    let _ = std::fs::remove_file(&tmp);
    back.tabs = vec![SheetRef { anlage: ai, sheet: si }];
    back.active = 0;
    assert!(enden(&back), "nach der Rundreise nicht angeschlossen");
}
