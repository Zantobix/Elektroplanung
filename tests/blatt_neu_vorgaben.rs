//! Neues Blatt übernimmt Strompfade und Ausrichtung der Anlage (Nachzeichnen 24.11.2026, S8/B4): vorher zeigte ein
//! neues Blatt 8 statt der 20 Strompfade der Anlage, bis das Projekt neu geladen wurde, und kam im Querformat,
//! obwohl die übrigen Blätter hochkant waren.

use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

#[test]
fn neues_blatt_wie_anlage() {
    let mut app = test_app();
    app.project = Project::new_default();
    app.project.anlagen[0].set.pfade = 20;
    app.project.anlagen[0].sheets[0].portrait = true;
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.new_sheet_dialog();
    let r = app.cur().unwrap();
    assert_eq!(r.sheet, 1);
    let s = app.project.sheet(r).unwrap();
    assert_eq!((s.columns, s.col_auto), (20, true));
    // Dialog: Strompfade „-1“ (wie Anlage), Ausrichtung wie das vorige Blatt
    if let Some(elektroplanung::props::Props::Sheet(d)) = app.props.as_ref() {
        assert_eq!(d.strompfade, "-1");
        assert!(d.portrait);
    } else {
        panic!("Blattdialog offen");
    }
}

/// Nutzer 24.11.2026: Blatt hinter das letzte gezogen, dann Strg+Z – die Blattfolge kam zurück, die Reiter blieben
/// vertauscht. Nach Rückgängig/Wiederholen folgen die Reiter der Blattfolge.
#[test]
fn reiter_folgen_der_blattfolge_nach_rueckgaengig() {
    let mut app = elektroplanung::blatt_tasten::test_app();
    app.project = elektroplanung::model::Project::new_default();
    app.project.anlagen[0].sheets.push(elektroplanung::model::Sheet::new(2));
    app.project.anlagen[0].sheets.push(elektroplanung::model::Sheet::new(3));
    app.tabs = (0..3).map(|i| elektroplanung::model::SheetRef { anlage: 0, sheet: i }).collect();
    app.active = 1;
    app.move_sheet(0, 1, 2);
    let nr = |app: &elektroplanung::app::App| app.tabs.iter().map(|t| app.project.sheet(*t).unwrap().number).collect::<Vec<_>>();
    assert_eq!(nr(&app), vec![1, 3, 2]);
    app.do_undo();
    assert_eq!(nr(&app), vec![1, 2, 3]);
    app.do_redo();
    assert_eq!(nr(&app), vec![1, 3, 2]);
}
