//! Nachzeichnen von W2 mit der Maus, Fortsetzung 24.11.2026 (Blatt „Sicherungen -F10, -F1 und -F2“): PE von der
//! Steckdose -X1 (Anschlüsse oben) senkrecht nach unten zur PE-Schiene gezogen. Gesehen: die Leitung lief senkrecht
//! **durch die eigene Steckdose** (Stich nach oben, dann x = 125 durch den Umriss). Erwartet: Umweg um das Symbol.
//! Plan: private Nachzeichen-Datei (`EP_FIXTURES/nachzeichnen`), ohne sie übersprungen.

use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_fuehrung::Fuehrung;
use elektroplanung::model::*;

fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

/// Blatt 2 der Anlage UV1 aus der Nachzeichen-Datei; die kaputten Reste der vorigen Runde (Leitung bis unter das
/// Blatt, Leitung mit freiem Ende, PE-Pin von -X1 auf der L-Leitung) sind entfernt.
fn blatt() -> Option<App> {
    let pfad = std::path::PathBuf::from(std::env::var("EP_FIXTURES").ok()?).join("nachzeichnen/nachgezeichnet_2026-11-24.epr");
    if !pfad.exists() {
        return None;
    }
    let mut app = test_app();
    app.open_path(pfad, false);
    app.tabs = vec![SheetRef { anlage: 1, sheet: 1 }];
    app.active = 0;
    app.raster_idx = 5; // 10 mm wie beim Nachzeichnen
    app.grid = true;
    let weg: Vec<u64> = app
        .sheet()?
        .objects
        .iter()
        .filter(|o| match &o.kind {
            Kind::Wire { pts, .. } => pts.iter().any(|q| q.y > 200.0) || pts.first() == Some(&p(119.0, 180.0)) || (pts.len() == 2 && pts[0] == p(127.0, 130.0)),
            _ => false,
        })
        .map(|o| o.id)
        .collect();
    assert_eq!(weg.len(), 3, "Reste der vorigen Runde");
    app.with_sheet(|s, _| s.objects.retain(|o| !weg.contains(&o.id)));
    Some(app)
}

/// Leitung mit der Maus über `ecken` ziehen (wie die Oberfläche) und anlegen; liefert den Weg.
fn ziehen(app: &mut App, ecken: &[P]) -> Vec<P> {
    let z = 3.24;
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

/// Umriss der Steckdose mit der BMK `bmk`.
fn umriss(app: &App, bmk: &str) -> (P, P) {
    let s = app.sheet().unwrap();
    let o = s.objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { bmk: b, .. } if b.starts_with(bmk))).unwrap();
    elektroplanung::canvas::obj_bounds(o, &app.lib)
}

/// Läuft ein Teilstück von `w` mehr als 0,3 mm in das Innere von `r`?
fn durch(w: &[P], r: (P, P)) -> bool {
    let rand = 0.3;
    w.windows(2).any(|s| {
        let (a, b) = (s[0], s[1]);
        (0..=40).any(|i| {
            let t = i as f32 / 40.0;
            let q = P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            q.x > r.0.x + rand && q.x < r.1.x - rand && q.y > r.0.y + rand && q.y < r.1.y - rand
        })
    })
}

#[test]
fn pe_vom_pin_oben_nach_unten_nicht_durch_die_steckdose() {
    let Some(mut app) = blatt() else { return };
    let r = umriss(&app, "-X1");
    let w = ziehen(&mut app, &[p(127.0, 130.0), p(127.0, 150.0), p(127.0, 183.0)]);
    eprintln!("Weg {w:?}");
    assert!(!durch(&w, r), "durch -X1 {r:?}: {w:?}");
    assert!(w.len() <= 5, "höchstens drei Knicke: {w:?}");
    assert!(w.last().unwrap().y > 182.9, "endet auf PE: {w:?}");
}

/// Wie in der Oberfläche: Mausweg auf ganze Bildpunkte gerundet (Zoom 3,2425 px/mm, Blattursprung 103,2/84,0 px),
/// Schritte von 4 px (werkzeuge/linux/nz.sh `pathdr`).
fn ziehen_px(app: &mut App, ecken: &[P]) -> Vec<P> {
    let (s, ox, oy) = (3.2425f32, 103.2f32, 84.0f32);
    let px = |q: P| ((ox + s * q.x).round(), (oy + s * q.y).round());
    let mm = |x: f32, y: f32| P::new((x - ox) / s, (y - oy) / s);
    let (x0, y0) = px(ecken[0]);
    let start = app.pin_near(mm(x0, y0), 6.0 / s, &[]).expect("Anschluss unter der Maus");
    app.drag = Drag::Wire { f: Fuehrung::neu(start) };
    let (mut x, mut y) = (x0, y0);
    for e in &ecken[1..] {
        let (x2, y2) = px(*e);
        let n = (((x2 - x).hypot(y2 - y)) / 4.0).max(8.0) as i32;
        for j in 1..=n {
            let (xi, yi) = ((x + (x2 - x) * j as f32 / n as f32).trunc(), (y + (y2 - y) * j as f32 / n as f32).trunc());
            app.leitung_zug_bewegen(mm(xi, yi), s);
        }
        (x, y) = (x2, y2);
    }
    let Drag::Wire { f } = std::mem::replace(&mut app.drag, Drag::None) else { unreachable!() };
    let ende = app.leitungsende(&f, mm(x, y), s);
    let w = app.leitungsweg(&f, ende);
    app.leitung_anschliessen(w.clone());
    w
}

/// Ist ein Leitungsende (nicht der Anschluss des Symbols) bei `q`? Dann wäre dort ein Knoten.
fn knoten_bei(app: &App, q: P) -> bool {
    app.sheet().unwrap().objects.iter().any(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts.first().is_some_and(|a| a.dist(q) < 0.05) || pts.last().is_some_and(|a| a.dist(q) < 0.05)))
}

#[test]
fn pe_vom_pin_oben_mit_bildpunkten_wie_in_der_oberflaeche() {
    let Some(mut app) = blatt() else { return };
    let r = umriss(&app, "-X1");
    let w = ziehen_px(&mut app, &[p(127.0, 130.0), p(127.0, 150.0), p(127.0, 183.0)]);
    eprintln!("Weg {w:?}");
    assert!(!durch(&w, r), "durch -X1 {r:?}: {w:?}");
    // PE kreuzt die N- (y 124) und L-Leitung (y 127) über der Anschlussreihe, verbunden wird nichts
    assert!(!knoten_bei(&app, p(127.0, 127.0)) && !knoten_bei(&app, p(127.0, 124.0)), "PE an L/N angeschlossen: {w:?}");
}

/// PE von -X7 (untere Reihe, Anschlüsse oben bei y 165) zur PE-Schiene 18 mm darunter: kurzer Umweg neben der
/// Steckdose, nicht 34 mm an der Schiene entlang (erste freie Lage rechts statt der günstigsten).
#[test]
fn pe_untere_reihe_kurzer_umweg() {
    let Some(mut app) = blatt() else { return };
    let r = umriss(&app, "-X7");
    let w = ziehen_px(&mut app, &[p(141.3, 165.0), p(141.3, 175.0), p(141.3, 183.0)]);
    eprintln!("Weg {w:?}");
    assert!(!durch(&w, r), "durch -X7 {r:?}: {w:?}");
    let laenge: f32 = w.windows(2).map(|v| v[0].dist(v[1])).sum();
    assert!(w.len() <= 4 && laenge <= 45.0, "Umweg zu lang ({laenge} mm): {w:?}");
}
