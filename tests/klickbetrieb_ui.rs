//! Leitung im Klickbetrieb über die echte Oberfläche (egui ohne Fenster, Mausereignisse wie vom System):
//! Klick auf einen Anschluss, Klicks auf freie Fläche setzen Eckpunkte, Klick auf einen Anschluss beendet.
//! Nachzeichnen 25.11.2026 #97: der erste Eckpunkt-Klick brach die Leitung ab (keine Leitung entstand).
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

struct Ui {
    ctx: egui::Context,
    frame: eframe::Frame,
    t: f64,
}

impl Ui {
    fn lauf(&mut self, app: &mut App, events: Vec<Event>) {
        self.t += 0.05;
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 950.0))), time: Some(self.t), events, ..Default::default() };
        let _ = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, &mut self.frame));
    }
    fn px(app: &App, p: P) -> Pos2 {
        let o = app.canvas_area.min + app.view.offset;
        Pos2::new(o.x + p.x * app.view.zoom, o.y + p.y * app.view.zoom)
    }
    /// Drücken und Loslassen im selben Bild (wie xdotool click unter X11)
    fn klick_kurz(&mut self, app: &mut App, p: P) {
        let q = Self::px(app, p);
        self.lauf(app, vec![Event::PointerMoved(q)]);
        self.lauf(app, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }, Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.t += 0.6;
        self.lauf(app, vec![]);
    }
    fn klick(&mut self, app: &mut App, p: P) {
        let q = Self::px(app, p);
        self.lauf(app, vec![Event::PointerMoved(q)]);
        self.lauf(app, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        self.lauf(app, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        // Abstand zum nächsten Klick (kein Doppelklick)
        self.t += 0.6;
        self.lauf(app, vec![]);
    }
}

#[test]
fn klickbetrieb_mit_eckpunkten() {
    klickbetrieb(false);
    // Drücken und Loslassen im selben Bild: vorher blieb die Leitung im Ziehmodus und der nächste Klick verwarf sie
    klickbetrieb(true);
}

fn klickbetrieb(kurz: bool) {
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    // zwei Klemmstellen als Anschlüsse
    for (id, x) in [(1u64, 50.0f32), (2, 120.0)] {
        app.project.anlagen[0].sheets[0].objects.push(Obj { id, pen: 0, fixed: false, group: None, raw: None, kind: Kind::Terminal { pos: P::new(x, 100.0), leiste: "X1".into(), nr: id as u32, level: 0, text_off: None, tpl: String::new(), opt: Default::default() } });
    }
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    let a = app.pin_near(P::new(50.0, 100.0), 2.0, &[]).expect("Anschluss der Klemmstelle");
    let b = app.pin_near(P::new(120.0, 100.0), 2.0, &[]).expect("Anschluss der Klemmstelle");
    let klick = |ui: &mut Ui, app: &mut App, p: P| if kurz { ui.klick_kurz(app, p) } else { ui.klick(app, p) };
    klick(&mut ui, &mut app, a);
    assert!(matches!(&app.drag, Drag::Wire { f } if f.klickbetrieb), "Klickbetrieb nach Klick auf den Anschluss");
    klick(&mut ui, &mut app, P::new(a.x, 80.0));
    assert!(matches!(&app.drag, Drag::Wire { f } if f.klickbetrieb && f.pts.len() == 2), "Eckpunkt 1 gesetzt: {:?}", match &app.drag { Drag::Wire { f } => format!("{:?}", f.pts), d => format!("{}", std::mem::discriminant(d) == std::mem::discriminant(&Drag::None)) });
    klick(&mut ui, &mut app, P::new(b.x, 80.0));
    assert!(matches!(&app.drag, Drag::Wire { f } if f.pts.len() == 3), "Eckpunkt 2 gesetzt");
    klick(&mut ui, &mut app, b);
    assert!(matches!(app.drag, Drag::None), "auf dem Anschluss beendet");
    let w: Vec<&Vec<P>> = app.project.anlagen[0].sheets[0].objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None }).collect();
    assert_eq!(w.len(), 1, "eine Leitung");
    assert_eq!((w[0].first().copied(), w[0].last().copied()), (Some(a), Some(b)));
    // die geklickten Ecken (auf das Raster gefangen) liegen auf der Leitung
    let y = app.snap(P::new(a.x, 80.0)).y;
    assert!(w[0].iter().filter(|p| (p.y - y).abs() < 0.01).count() >= 2, "Ecken auf y {y} fehlen: {:?}", w[0]);
}

/// Ablauf aus dem Nachzeichnen (Arbeitskopie über EP_NZ): N-Leitung von -X14 über zwei Ecken zu -X16.
#[test]
fn klickbetrieb_nachzeichnen() {
    let Some(p) = std::env::var("EP_NZ").ok() else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(p.into(), false);
    app.open_sheet(SheetRef { anlage: 1, sheet: 3 });
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    for (i, q) in [P::new(150.7, 165.0), P::new(150.7, 158.0), P::new(210.7, 158.0), P::new(210.7, 165.0)].into_iter().enumerate() {
        if std::env::var("EP_KURZ").is_ok() { ui.klick_kurz(&mut app, q) } else { ui.klick(&mut app, q) }
        let d = match &app.drag { Drag::Wire { f } => format!("Wire {:?} klick {}", f.pts, f.klickbetrieb), Drag::None => "None".into(), _ => "anderes".into() };
        eprintln!("Klick {i} {q:?}: {d} | {}", app.info);
    }
}

/// Erkundung (EP_NZ): L-Leitung von X1.3 an ihrem waagerechten Teilstück langsam nach oben/unten ziehen und die Form
/// je Bild ausgeben (Nutzervideo 25.11.2026: Ende springt zwischen X1.3 und der Leitung über X1.2 hin und her).
#[test]
fn teilstueck_ziehen_erkunden() {
    let Some(p) = std::env::var("EP_NZ").ok() else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(p.into(), false);
    app.open_sheet(SheetRef { anlage: 1, sheet: 3 });
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    let id: u64 = std::env::var("EP_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(218);
    let x: f32 = std::env::var("EP_X").ok().and_then(|v| v.parse().ok()).unwrap_or(70.0);
    let y0: f32 = std::env::var("EP_Y0").ok().and_then(|v| v.parse().ok()).unwrap_or(95.0);
    let y1: f32 = std::env::var("EP_Y1").ok().and_then(|v| v.parse().ok()).unwrap_or(75.0);
    let senkrecht = std::env::var("EP_SENKRECHT").is_ok();
    ui.klick(&mut app, P::new(x, y0));
    let pk = |p: P| Ui::px(&app_ref(&app), p);
    fn app_ref(a: &App) -> &App {
        a
    }
    let q0 = pk(P::new(x, y0));
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    let n = 60;
    let mut alt = String::new();
    for k in 0..=n {
        let v = y0 + (y1 - y0) * k as f32 / n as f32;
        let q = if senkrecht { Ui::px(&app, P::new(v, x)) } else { Ui::px(&app, P::new(x, v)) };
        ui.lauf(&mut app, vec![Event::PointerMoved(q)]);
        let s = app.project.sheet(SheetRef { anlage: 1, sheet: 3 }).unwrap();
        let form: Vec<String> = s.objects.iter().filter(|o| o.id == id || std::env::var("EP_ALLE").is_ok()).filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(format!("{}:{:?}", o.id, pts.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>())) } else { None }).collect();
        let f = form.join(" ");
        if f != alt {
            eprintln!("Maus {v:6.2}: {f} | {}", app.info);
            alt = f;
        }
    }
}

/// Nutzervideo 25.11.2026 („Wackeln“): die L-Leitung von X1.3 auf -F3 (Nachzeichen-Stand 25.11.) an ihrem
/// waagerechten Teilstück gleichmäßig nach oben gezogen – die Lage darf nur in Zugrichtung wandern, nie zurückspringen.
#[test]
fn teilstueck_wackelt_nicht() {
    let Some(f) = std::env::var("EP_FIXTURES").ok().map(|d| std::path::Path::new(&d).join("nachzeichnen/nachgezeichnet_2026-11-25.epr")).filter(|p| p.exists()) else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(f, false);
    let r = SheetRef { anlage: 1, sheet: 3 };
    app.open_sheet(r);
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    let y_von = |app: &App| app.project.sheet(r).unwrap().objects.iter().find(|o| o.id == 218).and_then(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts[2].y) } else { None }).unwrap();
    assert_eq!(y_von(&app), 95.0);
    ui.klick(&mut app, P::new(50.0, 95.0));
    let q0 = Ui::px(&app, P::new(50.0, 95.0));
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    let mut zuletzt = 95.0;
    for k in 0..=60 {
        let q = Ui::px(&app, P::new(50.0, 95.0 - 20.0 * k as f32 / 60.0));
        ui.lauf(&mut app, vec![Event::PointerMoved(q)]);
        let y = y_von(&app);
        assert!(y <= zuletzt + 0.01, "Schritt {k}: zurückgesprungen von {zuletzt} auf {y}");
        zuletzt = y;
    }
    assert!(zuletzt < 80.0, "gezogen bis {zuletzt}");
}

/// Erkundung (EP_NZ, EP_VON=x,y, EP_NACH=x,y): Objekt unter EP_VON greifen und schrittweise nach EP_NACH ziehen;
/// danach alle Leitungen ausgeben, die sich geändert haben.
#[test]
fn verschieben_erkunden() {
    let (Some(p), Some(von), Some(nach)) = (std::env::var("EP_NZ").ok(), std::env::var("EP_VON").ok(), std::env::var("EP_NACH").ok()) else { return };
    let xy = |s: &str| {
        let v: Vec<f32> = s.split(',').map(|t| t.trim().parse().unwrap()).collect();
        P::new(v[0], v[1])
    };
    let (von, nach) = (xy(&von), xy(&nach));
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(p.into(), false);
    let r = SheetRef { anlage: 1, sheet: 3 };
    app.open_sheet(r);
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    let vorher = app.project.sheet(r).unwrap().objects.clone();
    let q0 = Ui::px(&app, von);
    ui.lauf(&mut app, vec![Event::PointerMoved(q0)]);
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    for k in 1..=40 {
        let q = Ui::px(&app, P::new(von.x + (nach.x - von.x) * k as f32 / 40.0, von.y + (nach.y - von.y) * k as f32 / 40.0));
        ui.lauf(&mut app, vec![Event::PointerMoved(q)]);
    }
    let q = Ui::px(&app, nach);
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    ui.lauf(&mut app, vec![]);
    eprintln!("Markierung: {:?} | {}", app.selection, app.info);
    let nachher = &app.project.sheet(r).unwrap().objects;
    for o in nachher {
        let alt = vorher.iter().find(|v| v.id == o.id);
        if alt.map(|a| format!("{:?}", a.kind)) != Some(format!("{:?}", o.kind)) {
            let f = |k: &Kind| match k {
                Kind::Wire { pts, .. } => format!("{:?}", pts.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()),
                Kind::Symbol { pos, bmk, .. } => format!("{bmk} {:?}", (pos.x, pos.y)),
                k => format!("{:?}", std::mem::discriminant(k)),
            };
            eprintln!("{}: {} -> {}", o.id, alt.map(|a| f(&a.kind)).unwrap_or("neu".into()), f(&o.kind));
        }
    }
    for v in &vorher {
        if !nachher.iter().any(|o| o.id == v.id) {
            eprintln!("{}: entfernt", v.id);
        }
    }
    let s = app.project.sheet(r).unwrap();
    let mut alt_blatt = s.clone();
    alt_blatt.objects = vorher.clone();
    eprintln!("Wächter: {:?}", elektroplanung::netz_waechter::pruefen(&vorher, s, &app.lib, &app.selection));
    for (a, b) in [(202u64, 204u64), (202, 218), (204, 218)] {
        let n = elektroplanung::netz::netz_von(s, &app.lib, a);
        eprintln!("{a} und {b} im selben Netz: vorher {} nachher {}", elektroplanung::netz::netz_von(&alt_blatt, &app.lib, a).contains(&b), n.contains(&b));
    }
}

/// Nutzerbild 2 (25.11.2026): -E3 auf -F3 mit der Maus nach links an -E2 vorbeigezogen – ihre Zuleitung läuft danach
/// als kurzer Stich auf die Leitung darüber statt im Zickzack unter den Lampen entlang; kein Netz ändert sich.
#[test]
fn lampe_vorbeiziehen_ohne_umweg() {
    let Some(f) = std::env::var("EP_FIXTURES").ok().map(|d| std::path::Path::new(&d).join("nachzeichnen/nachgezeichnet_2026-11-25.epr")).filter(|p| p.exists()) else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(f, false);
    let r = SheetRef { anlage: 1, sheet: 3 };
    app.open_sheet(r);
    let mut ui = Ui { ctx: egui::Context::default(), frame: eframe::Frame::_new_kittest(), t: 1.0 };
    for _ in 0..3 {
        ui.lauf(&mut app, vec![]);
    }
    let vorher = app.project.sheet(r).unwrap().clone();
    let (von, nach) = (P::new(127.0, 149.4), P::new(80.0, 149.4));
    let q0 = Ui::px(&app, von);
    ui.lauf(&mut app, vec![Event::PointerMoved(q0)]);
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    for k in 1..=40 {
        let q = Ui::px(&app, P::new(von.x + (nach.x - von.x) * k as f32 / 40.0, von.y));
        ui.lauf(&mut app, vec![Event::PointerMoved(q)]);
    }
    let q = Ui::px(&app, nach);
    ui.lauf(&mut app, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    ui.lauf(&mut app, vec![]);
    let s = app.project.sheet(r).unwrap();
    assert_eq!(elektroplanung::netz_waechter::pruefen(&vorher.objects, s, &app.lib, &[200]), None);
    let w203 = s.objects.iter().find(|o| o.id == 203).and_then(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.clone()) } else { None }).unwrap();
    assert!(w203.len() <= 3, "Zuleitung von -E3 ohne Umweg: {w203:?}");
}
