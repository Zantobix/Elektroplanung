//! Auswahlbefehle und Auswahlfilter (NR13, auswahl.rs) auf W2 „Sicherungen -F10, -F1 und -F2“ (56 Leitungen).
use elektroplanung::auswahl::Art;
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

#[test]
fn befehle_und_filter_w2() {
    let Some(p) = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists()) else { return };
    let mut app = test_app();
    app.open_path(p.into(), false);
    let ai = 1;
    let si = app.project.anlagen[ai].sheets.iter().position(|s| s.name.starts_with("Sicherungen -F10")).unwrap();
    app.open_sheet(SheetRef { anlage: ai, sheet: si });
    let zahl = |app: &elektroplanung::app::App, f: &dyn Fn(&Kind) -> bool| app.sheet().unwrap().objects.iter().filter(|o| app.selection.contains(&o.id) && f(&o.kind)).count();
    assert!(app.auswahl_befehl("ausw_leitungen"));
    assert_eq!(app.selection.len(), 56);
    assert_eq!(app.info, "56 Leitungen markiert");
    let alle = app.sheet().unwrap().objects.len();
    assert!(app.auswahl_befehl("ausw_umkehren"));
    assert_eq!(app.selection.len(), alle - 56);
    assert_eq!(zahl(&app, &|k| matches!(k, Kind::Wire { .. })), 0);
    assert!(app.auswahl_befehl("ausw_symbole"));
    let n_sym = app.selection.len();
    assert!(n_sym > 0 && zahl(&app, &|k| matches!(k, Kind::Symbol { .. })) == n_sym);
    // Filter ohne Leitungen: Strg+A und Klick treffen keine Leitung, der ausdrückliche Befehl schon
    app.zusatz.filter.umschalten(Art::Leitung);
    app.run_befehl("alles");
    assert_eq!(app.selection.len(), alle - 56);
    let w = app.sheet().unwrap().objects.iter().find_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some((o.id, P::new((pts[0].x + pts[1].x) / 2.0, (pts[0].y + pts[1].y) / 2.0))) } else { None }).unwrap();
    assert_ne!(app.hit_top(w.1, 0.3), Some(w.0));
    assert!(app.auswahl_befehl("ausw_leitungen"));
    assert_eq!(app.selection.len(), 56);
    app.zusatz.filter = Default::default();
    assert_eq!(app.hit_top(w.1, 0.3), Some(w.0));
    // verbundene Leitungen: von einer Leitung aus wächst die Markierung über ihr Netz
    app.selection = vec![w.0];
    assert!(app.auswahl_befehl("ausw_verbunden"));
    assert!(app.selection.len() > 1);
    assert!(!app.auswahl_befehl("kein_befehl"));
}
