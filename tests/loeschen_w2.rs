//! Löschen wie im Original (Cloud-Gegenprobe 25.11.2026, beide Fälle im Original unter Wine gelöscht, gespeichert und
//! ausgelesen): W2 „Sicherungen -F10, -F1 und -F2“.
//! 1. Entf auf −F2 (4 Anschlüsse): 56 → 52 Leitungen, Schienen −F2.L/−F2.N auf 13…284 (im .epr Länge 271).
//! 2. Entf nur auf die Einspeiseleitung von −F2.L: 56 → 55 Leitungen, nur −F2.L auf 13…284, −F2.N bleibt.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn w2() -> Option<(elektroplanung::app::App, SheetRef)> {
    let p = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists())?;
    let mut app = test_app();
    app.open_path(p.into(), false);
    let ai = 1;
    let si = app.project.anlagen[ai].sheets.iter().position(|s| s.name.starts_with("Sicherungen -F10"))?;
    let r = SheetRef { anlage: ai, sheet: si };
    app.open_sheet(r);
    Some((app, r))
}

fn leitungen(s: &Sheet) -> usize {
    s.objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).count()
}

fn schiene(s: &Sheet, n: &str) -> (f32, f32, f32) {
    s.objects.iter().find_map(|o| if let Kind::Busbar { a, x2, name, .. } = &o.kind { (name == n).then_some((a.x, a.y, *x2)) } else { None }).unwrap()
}

#[test]
fn symbol_loeschen_wie_original() {
    let Some((mut app, r)) = w2() else { return };
    let f2 = app.project.sheet(r).unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { pos, .. } if pos.dist(P::new(97.5, 85.0)) < 0.01)).unwrap().id;
    app.selection = vec![f2];
    app.delete_selection();
    let s = app.project.sheet(r).unwrap();
    assert_eq!(leitungen(s), 52);
    assert_eq!(schiene(s, "-F2.L"), (13.0, 102.0, 284.0));
    assert_eq!(schiene(s, "-F2.N"), (13.0, 107.0, 284.0));
    // Rückgängig: alles wieder da
    app.do_undo();
    assert_eq!(leitungen(app.project.sheet(r).unwrap()), 56);
}

#[test]
fn einspeiseleitung_loeschen_wie_original() {
    let Some((mut app, r)) = w2() else { return };
    let w = app.project.sheet(r).unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts.last().is_some_and(|p| p.dist(P::new(102.5, 102.0)) < 0.01))).unwrap().id;
    app.selection = vec![w];
    app.delete_selection();
    let s = app.project.sheet(r).unwrap();
    assert_eq!(leitungen(s), 55);
    assert_eq!(schiene(s, "-F2.L"), (13.0, 102.0, 284.0));
    assert_eq!(schiene(s, "-F2.N"), (97.5, 107.0, 284.0));
}

#[test]
fn option_aus_nur_markiertes() {
    let Some((mut app, r)) = w2() else { return };
    app.opt.loeschen_mit_leitungen = false;
    let f2 = app.project.sheet(r).unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { pos, .. } if pos.dist(P::new(97.5, 85.0)) < 0.01)).unwrap().id;
    app.selection = vec![f2];
    app.delete_selection();
    let s = app.project.sheet(r).unwrap();
    assert_eq!(leitungen(s), 56);
    assert_eq!(schiene(s, "-F2.L"), (102.5, 102.0, 284.0));
}
