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
