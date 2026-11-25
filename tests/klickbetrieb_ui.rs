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
