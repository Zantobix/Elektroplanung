//! Nachzeichnen von W2 mit der Maus (24.11.2026 nachmittags, Blatt „Sicherung -F3“, Raster 10 mm): -F3 (LS 2P)
//! oben, drei Steckdosen -X11…-X13 in einer Reihe darunter (Anschlüsse N, PE, L auf y = 115), L schon verdrahtet
//! (Hauptleitung auf y = 100 mit Abzweigen). Gesehen: die N-Leitung von -F3 zur hintersten Steckdose, Mausweg mit
//! Knie bei y = 95, lief senkrecht bis auf die Anschlusshöhe und dann waagerecht **durch** die Anschlüsse N und L von
//! -X11/-X12 – L und N verbunden.
//! Plan: private Nachzeichen-Datei (`EP_FIXTURES/nachzeichnen/f3_steckdosen.epr`), ohne sie übersprungen.

use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_fuehrung::Fuehrung;
use elektroplanung::model::*;

fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

fn blatt() -> Option<App> {
    let pfad = std::path::PathBuf::from(std::env::var("EP_FIXTURES").ok()?).join("nachzeichnen/f3_steckdosen.epr");
    if !pfad.exists() {
        return None;
    }
    let mut app = test_app();
    app.open_path(pfad, false);
    app.tabs = vec![SheetRef { anlage: 1, sheet: 3 }];
    app.active = 0;
    app.raster_idx = 5; // 10 mm wie beim Nachzeichnen
    app.grid = true;
    Some(app)
}

/// Leitung mit der Maus über `ecken` ziehen (wie die Oberfläche, 3,24 px/mm) und beim Loslassen anlegen.
fn ziehen(app: &mut App, ecken: &[P]) -> Vec<P> {
    let z = 3.24;
    let start = app.snap(ecken[0]);
    let pins: Vec<P> = app.sheet().unwrap().objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| elektroplanung::canvas::obj_pins(o, &app.lib)).collect();
    let start = pins.into_iter().find(|q| q.dist(ecken[0]) < 1.5).unwrap_or(start);
    app.drag = Drag::Wire { f: Fuehrung::neu(start) };
    let mut vorher = ecken[0];
    for e in &ecken[1..] {
        let n = (vorher.dist(*e) / 1.2).ceil().max(1.0) as usize;
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

/// Fremde Anschlüsse im Innern des Weges.
fn ueber_anschluessen(app: &App, w: &[P]) -> Vec<P> {
    let s = app.sheet().unwrap();
    s.objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }))
        .flat_map(|o| elektroplanung::canvas::obj_pins(o, &app.lib))
        .filter(|q| q.dist(w[0]) > 0.05 && q.dist(w[w.len() - 1]) > 0.05 && w.windows(2).any(|s| seg_dist(*q, s[0], s[1]) < 0.05))
        .collect()
}

#[test]
fn n_leitung_nicht_durch_die_anschlussreihe() {
    let Some(mut app) = blatt() else { return };
    let w = ziehen(&mut app, &[p(31.5, 67.5), p(31.5, 80.0), p(31.5, 95.0), p(80.0, 95.0), p(115.0, 95.0), p(115.0, 108.0), p(115.0, 115.0)]);
    println!("{w:?}");
    assert_eq!(w[w.len() - 1], p(115.0, 115.0), "endet an -X13 N");
    assert!(ueber_anschluessen(&app, &w).is_empty(), "über fremde Anschlüsse: {:?} – Weg {w:?}", ueber_anschluessen(&app, &w));
    // verlässt -F3 nach unten und erreicht -X13 von oben (Anschlussrichtungen)
    assert!((w[1].x - w[0].x).abs() < 0.01 && w[1].y > w[0].y, "{w:?}");
    let n = w.len();
    assert!((w[n - 2].x - w[n - 1].x).abs() < 0.01 && w[n - 2].y < w[n - 1].y, "{w:?}");
}

/// Motor rechts abgelegt: die automatische Verbindung (Strahl wie im Original) lief vom Anschluss N 145 mm nach links
/// durch den Taster -Q2 bis zur PE-Leitung. Ein fremdes Symbol auf dem Strahl beendet ihn ohne Verbindung.
#[test]
fn auto_verbindung_nicht_durch_fremde_symbole() {
    let Some(dir) = std::env::var("EP_FIXTURES").ok() else { return };
    let pfad = std::path::PathBuf::from(dir).join("nachzeichnen/f3_motor.epr");
    if !pfad.exists() {
        return;
    }
    let mut app = test_app();
    app.open_path(pfad, false);
    app.tabs = vec![SheetRef { anlage: 1, sheet: 3 }];
    app.active = 0;
    let s = app.sheet().unwrap().clone();
    // der Motor (im Nachbau ohne BMK abgelegt) hat Anschluss N bei (182,5 | 132,5)
    let motor = s.objects.iter().find(|o| elektroplanung::canvas::obj_pins(o, &app.lib).iter().any(|q| q.dist(p(182.5, 132.5)) < 0.05)).expect("Motor").id;
    let umrisse = elektroplanung::leitung_hindernis::umrisse(&s, &app.lib);
    let wires = elektroplanung::app::auto_wires(&s, &app.lib, &[motor], elektroplanung::app::AUTO_CONN_MAX);
    for (a, b) in &wires {
        for u in umrisse.iter().filter(|u| u.id != motor) {
            let (mn, mx) = (P::new(u.r.0.x.min(u.r.1.x) + 0.2, u.r.0.y.min(u.r.1.y) + 0.2), P::new(u.r.0.x.max(u.r.1.x) - 0.2, u.r.0.y.max(u.r.1.y) - 0.2));
            let quer = if (a.y - b.y).abs() < 0.01 { a.y > mn.y && a.y < mx.y && a.x.max(b.x) > mn.x && a.x.min(b.x) < mx.x } else { a.x > mn.x && a.x < mx.x && a.y.max(b.y) > mn.y && a.y.min(b.y) < mx.y };
            assert!(!quer, "automatische Leitung {a:?}–{b:?} durch Symbol {}", u.id);
        }
    }
}
