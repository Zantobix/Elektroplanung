//! Werkzeugknopf „Neue Sammelschiene“ wie im Original (ToolButton18Click 0x46d9b0; Cloud-Gegenprobe 25.11.2026 mit
//! dem Original unter Wine, Ergebnis gespeichert und ausgelesen): auf einem Schaltplan entsteht sofort ein
//! Schienen-Objekt mit neuer Schiene „L<n>“ („Schiene n“), Auto-Lage am Ende des Stapels; Abbruch nimmt alles zurück.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn w2() -> Option<elektroplanung::app::App> {
    let p = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists())?;
    let mut app = test_app();
    app.open_path(p.into(), false);
    Some(app)
}

fn blatt(app: &elektroplanung::app::App, name: &str) -> SheetRef {
    for (ai, a) in app.project.anlagen.iter().enumerate() {
        if let Some(si) = a.sheets.iter().position(|s| s.name == name) {
            return SheetRef { anlage: ai, sheet: si };
        }
    }
    panic!("Blatt {name} fehlt")
}

#[test]
fn neue_schiene_auf_querformat_wie_original() {
    let Some(mut app) = w2() else { return };
    let r = blatt(&app, "Sicherungen -F10, -F1 und -F2");
    app.open_sheet(r);
    let n = app.project.sheet(r).unwrap().objects.len();
    app.schiene_objekt_neu();
    assert!(app.dlg3.is_some(), "Eigenschaften-Dialog offen");
    app.dlg3_ok();
    let s = app.project.sheet(r).unwrap();
    assert_eq!(s.objects.len(), n + 1);
    // Original: idx 96/97, a = (13, 112), x2 = 271, Name L12, Beschriftung (0, −4), Schiene „Schiene 12“
    let Kind::Busbar { a, x2, name, label, lage, .. } = &s.objects.last().unwrap().kind else { panic!("keine Schiene") };
    assert_eq!((name.as_str(), a.x, *x2, *label, *lage), ("L12", 13.0, 271.0, P::new(0.0, -4.0), 0));
    assert!((a.y - 112.0).abs() < 0.01, "Auto-Lage wie im Original: y = {}", a.y);
    let set = &app.project.anlagen[r.anlage].schienen["L12"];
    assert_eq!((set.name.as_str(), set.quer, set.pfeile), ("Schiene 12", true, false));
}

#[test]
fn neue_schiene_auf_hochformat_und_abbruch() {
    let Some(mut app) = w2() else { return };
    let r = blatt(&app, "Neubau EG");
    app.open_sheet(r);
    let vorher = app.project.clone();
    app.schiene_objekt_neu();
    app.dlg3_abbrechen();
    assert!(app.project == vorher, "Abbruch nimmt Objekt und Schiene zurück");
    app.schiene_objekt_neu();
    app.dlg3_ok();
    // Original: a = (18, 17), x2 = 179, Name L1 („Schiene 1“)
    let Kind::Busbar { a, x2, name, .. } = &app.project.sheet(r).unwrap().objects.last().unwrap().kind else { panic!() };
    assert_eq!((name.as_str(), a.x, a.y, *x2), ("L1", 18.0, 17.0, 179.0));
}
