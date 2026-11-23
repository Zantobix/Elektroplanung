//! Drehen/Spiegeln einer gemischten Auswahl (ToolButton17Click 0x465994): Das Original dreht alle markierten
//! Objekte – Zeichenobjekte und Symbole – um die gemeinsame Mitte. Nachzeichnen 24.11.2026: Strg+A und R auf einem
//! Grundriss drehte jedes Objekt um die eigene Mitte, der Grundriss zerfiel.

use elektroplanung::{blatt_tasten::test_app, canvas, model::*, symbols};

fn sym(id: u64, pos: P) -> Obj {
    Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

fn app() -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = symbols::builtin();
    app.with_sheet(|s, _| {
        s.objects.push(Obj { id: 1, kind: Kind::Line { a: P::new(40.0, 40.0), b: P::new(120.0, 40.0) }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(sym(2, P::new(100.0, 80.0)));
        s.objects.push(Obj { id: 3, kind: Kind::Text { pos: P::new(60.0, 70.0), text: "Raum 1".into(), size: 3.5, align: Align::Left, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
    });
    app.selection = vec![1, 2, 3];
    app
}

fn pins(app: &elektroplanung::app::App) -> Vec<P> {
    let s = app.sheet().unwrap();
    canvas::obj_pins(s.objects.iter().find(|o| o.id == 2).unwrap(), &app.lib)
}

fn nah(a: &[P], b: &[P]) -> bool {
    a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| p.dist(*q) < 0.05))
}

#[test]
fn gemischte_auswahl_dreht_um_gemeinsame_mitte() {
    let mut app = app();
    let c = elektroplanung::drehen_mitte::gemeinsame_mitte(app.sheet().unwrap().objects.iter(), &app.lib).expect("Mitte");
    let vorher = pins(&app);
    app.transform_selection(1);
    let dreh = |p: P| P::new(c.x - (p.y - c.y), c.y + (p.x - c.x));
    let soll: Vec<P> = vorher.iter().map(|p| dreh(*p)).collect();
    assert!(nah(&pins(&app), &soll), "Anschlüsse {:?}, erwartet {soll:?}", pins(&app));
    let s = app.sheet().unwrap();
    let Kind::Line { a, b } = s.objects[0].kind else { panic!() };
    assert!(a.dist(dreh(P::new(40.0, 40.0))) < 0.01 && b.dist(dreh(P::new(120.0, 40.0))) < 0.01);
    let Kind::Text { pos, vertical, .. } = s.objects[2].kind else { panic!() };
    assert!(pos.dist(dreh(P::new(60.0, 70.0))) < 0.01 && vertical);
}

#[test]
fn gemischte_auswahl_spiegelt_um_gemeinsame_mitte() {
    for op in [2u8, 4] {
        let mut app = app();
        let c = elektroplanung::drehen_mitte::gemeinsame_mitte(app.sheet().unwrap().objects.iter(), &app.lib).expect("Mitte");
        let vorher = pins(&app);
        app.transform_selection(op);
        let sp = |p: P| if op == 2 { P::new(2.0 * c.x - p.x, p.y) } else { P::new(p.x, 2.0 * c.y - p.y) };
        let soll: Vec<P> = vorher.iter().map(|p| sp(*p)).collect();
        assert!(nah(&pins(&app), &soll), "op {op}: Anschlüsse {:?}, erwartet {soll:?}", pins(&app));
    }
}
