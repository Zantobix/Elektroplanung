//! Einfügen (Strg+V): Versatz nur, wenn die Kopie sonst genau auf den kopierten Objekten läge, und nie über den
//! Zeichnungsrahmen hinaus (Nachzeichnen 24.11.2026: ein Grundriss, auf ein leeres zweites Blatt eingefügt, lag
//! 10 mm versetzt und ragte rechts über den Rahmen).

use elektroplanung::{blatt_tasten::test_app, model::*};

fn app_mit_zwei_blaettern() -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.project.anlagen[0].sheets.push(Sheet::new(2));
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }, SheetRef { anlage: 0, sheet: 1 }];
    app.active = 0;
    app
}

fn linie(app: &mut elektroplanung::app::App, a: P, b: P) -> u64 {
    let id = app.project.alloc_id();
    app.with_sheet(|s, _| s.objects.push(Obj { id, kind: Kind::Line { a, b }, pen: 0, fixed: false, group: None, raw: None }));
    id
}

fn linien(app: &elektroplanung::app::App) -> Vec<(P, P)> {
    app.sheet().unwrap().objects.iter().filter_map(|o| if let Kind::Line { a, b } = o.kind { Some((a, b)) } else { None }).collect()
}

#[test]
fn auf_anderes_blatt_ohne_versatz() {
    let mut app = app_mit_zwei_blaettern();
    let (x0, y0, x1, y1) = app.sheet().unwrap().frame();
    let (rechts, oben) = (x0.max(x1), y0.min(y1));
    let a = P::new(30.0, oben + 15.0);
    let b = P::new(rechts - 5.0, oben + 15.0);
    let id = linie(&mut app, a, b);
    app.selection = vec![id];
    app.copy();
    app.active = 1;
    app.paste();
    assert_eq!(linien(&app), vec![(a, b)], "leeres Blatt: Kopie an derselben Stelle");
}

#[test]
fn auf_dasselbe_blatt_versetzt_aber_im_rahmen() {
    let mut app = app_mit_zwei_blaettern();
    let (x0, y0, x1, y1) = app.sheet().unwrap().frame();
    let (rechts, oben, unten) = (x0.max(x1), y0.min(y1), y0.max(y1));
    // Linie in der Mitte: um 10 mm versetzt
    let a = P::new(50.0, oben + 40.0);
    let b = P::new(100.0, oben + 40.0);
    let id = linie(&mut app, a, b);
    app.selection = vec![id];
    app.copy();
    app.paste();
    let l = linien(&app);
    assert_eq!(l.len(), 2);
    assert!(l[1] != (a, b), "Kopie liegt nicht auf dem Original: {l:?}");
    // Linie bis an den rechten Rahmen: Kopie bleibt im Rahmen
    let c = P::new(60.0, unten - 30.0);
    let d = P::new(rechts, unten - 30.0);
    let id = linie(&mut app, c, d);
    app.selection = vec![id];
    app.copy();
    app.paste();
    let l = linien(&app);
    let (p, q) = l[3];
    assert!(p.x.max(q.x) <= rechts + 1e-3, "Kopie ragt über den Rahmen: {p:?} {q:?} (Rahmen {rechts})");
    assert!((p, q) != (c, d), "Kopie liegt nicht auf dem Original");
}
