//! Letztes Blatt einer Anlage löschen wie im Original (Cloud-Gegenprobe 25.11.2026: Original löscht es, die Anlage
//! bleibt ohne Blatt, 0 BLT-Blöcke; Nutzer 25.11.2026 „letztes Blatt kann gelöscht werden“).
use elektroplanung::app::{TAction, TNode};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use elektroplanung::{epr, epr_write};

#[test]
fn letztes_blatt_loeschen_speichern_laden() {
    let Some(p) = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists()) else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let mut app = test_app();
    app.open_path(p.into(), false);
    let ai = 0;
    let n = app.project.anlagen[ai].sheets.len();
    assert!(n >= 1);
    let anlagen = app.project.anlagen.len();
    for _ in 0..n {
        let r = SheetRef { anlage: ai, sheet: 0 };
        app.open_sheet(r);
        let node = TNode { id: String::new(), label: String::new(), bold: false, icon: 3, action: TAction::Sheet(r), children: vec![] };
        app.tree_context(&node, "Löschen");
    }
    assert!(app.project.anlagen[ai].sheets.is_empty());
    assert_eq!(app.project.anlagen.len(), anlagen);
    assert!(app.tabs.iter().all(|t| t.anlage != ai));
    let out = std::env::temp_dir().join("ep_letztes_blatt.epr");
    epr_write::save(&app.project, &out).unwrap();
    let b = epr::import(&out).unwrap();
    assert_eq!(b.anlagen.len(), anlagen);
    assert!(b.anlagen[ai].sheets.is_empty(), "{:?}", b.anlagen.iter().map(|a| (a.name.clone(), a.sheets.iter().map(|s| s.name.clone()).collect::<Vec<_>>())).collect::<Vec<_>>());
    let andere: usize = app.project.anlagen.iter().map(|a| a.sheets.len()).sum();
    assert_eq!(b.anlagen.iter().map(|a| a.sheets.len()).sum::<usize>(), andere);
    // rückgängig: alle Blätter wieder da
    for _ in 0..n {
        app.do_undo();
    }
    assert_eq!(app.project.anlagen[ai].sheets.len(), n);
}
