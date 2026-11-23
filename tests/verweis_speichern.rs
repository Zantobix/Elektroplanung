//! Verweis einfügen (Kontextmenü › Verweis einfügen › Start/Ziel) und speichern (Nachzeichnen 24.11.2026): Der
//! Nachbau legte ein loses Dreieck (Vieleck) und einen Text als Gruppe an; beides war so klein, dass die
//! Skalierungsgriffe das Objekt überdeckten (Ziehen schrumpfte das Dreieck auf 0,2 mm), und ein leerer Text fehlte
//! nach dem Laden („ZCN-Block ohne Darstellung“). Jetzt entsteht ein Verweisobjekt wie im Original.

use elektroplanung::{blatt_tasten::test_app, epr, epr_write, model::*};

fn rundreise(start: bool, text: &str) -> (Kind, Vec<Obj>) {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.add_reference(P::new(259.0, 24.0), start, text.into());
    let objs = app.sheet().unwrap().objects.clone();
    assert_eq!(objs.len(), 1, "ein Verweisobjekt: {objs:?}");
    assert!(app.sel_frame().is_none(), "Verweis ohne Skalierungsgriffe");
    let bytes = epr_write::to_bytes(&app.project).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_verweis_{}_{}.epr", std::process::id(), start as u8));
    std::fs::write(&tmp, &bytes).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    (objs[0].kind.clone(), q.sheet(SheetRef { anlage: 0, sheet: 0 }).unwrap().objects.clone())
}

#[test]
fn verweis_start_ueberlebt_speichern() {
    let (vorher, nachher) = rundreise(true, "/3.1");
    let Kind::Verweis { pos, text_pos, spitze, .. } = vorher else { panic!("{vorher:?}") };
    assert!(!spitze && text_pos.x > pos.x, "Start: Pfeil nach rechts, Text rechts");
    let v = nachher.iter().find_map(|o| if let Kind::Verweis { pos, text, .. } = &o.kind { Some((*pos, text.clone())) } else { None });
    let (p, t) = v.expect("Verweis nach dem Laden");
    assert!(p.dist(P::new(259.0, 24.0)) < 0.05 && t == "/3.1", "{p:?} {t:?}");
}

#[test]
fn verweis_ziel_ohne_text_ueberlebt_speichern() {
    let (vorher, nachher) = rundreise(false, "");
    let Kind::Verweis { pos, text_pos, spitze, .. } = vorher else { panic!("{vorher:?}") };
    assert!(spitze && text_pos.x < pos.x, "Ziel: Spitze an der Lage, Text links");
    assert!(nachher.iter().any(|o| matches!(o.kind, Kind::Verweis { .. })), "Verweis nach dem Laden: {nachher:?}");
}
