//! Mehrfachklick auf Leitungen in Stufen (NR12, src/mehrfachklick.rs) auf W2 „Sicherungen -F10, -F1 und -F2“:
//! 1× Teilstück, 2× bis zum Knoten, 3× Netz ohne Sammelschiene, 4× ganzes Potential mit Schiene; Entf bei
//! markiertem Teilstück löscht nur dieses Stück.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

fn w2() -> Option<(elektroplanung::app::App, SheetRef)> {
    let p = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists())?;
    let mut app = test_app();
    app.open_path(p.into(), false);
    let si = app.project.anlagen[1].sheets.iter().position(|s| s.name.starts_with("Sicherungen -F10"))?;
    let r = SheetRef { anlage: 1, sheet: si };
    app.open_sheet(r);
    Some((app, r))
}

#[test]
fn stufen_auf_w2() {
    let Some((mut app, r)) = w2() else { return };
    let s = app.project.sheet(r).unwrap().clone();
    let art = |id: &u64| s.objects.iter().find(|o| o.id == *id).map(|o| std::mem::discriminant(&o.kind)).unwrap();
    let schiene = std::mem::discriminant(&Kind::Busbar { a: P::default(), x2: 0.0, name: String::new(), label: P::default(), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 });
    // Leitung 426: von -F10 (37,5 | 162,7) über zwei Ecken zur PE-Schiene (20 | 183)
    let id = 426;
    let m = P::new(28.0, 155.0);
    app.mehrfachklick(id, m, 0.5, 1);
    assert_eq!(app.selection, vec![id]);
    assert_eq!(app.teil_auswahl(), Some((id, 1)), "Teilstück 37,5|155 – 20|155");
    app.mehrfachklick(id, m, 0.5, 2);
    let kette = app.selection.clone();
    assert!(kette.contains(&id) && app.teil_auswahl().is_none());
    app.mehrfachklick(id, m, 0.5, 3);
    let netz = app.selection.clone();
    assert!(netz.iter().all(|i| art(i) != schiene), "Stufe 3 ohne Schiene");
    app.mehrfachklick(id, m, 0.5, 4);
    let pot = app.selection.clone();
    assert!(pot.iter().any(|i| art(i) == schiene), "Stufe 4 mit Schiene");
    assert!(kette.len() <= netz.len() && netz.len() < pot.len(), "{} ≤ {} < {}", kette.len(), netz.len(), pot.len());
    // Stufe 4 geht über die PE-Schiene hinaus: mehr Leitungen als Stufe 3
    let pot_leitungen = pot.iter().filter(|i| art(i) != schiene).count();
    assert!(pot_leitungen > netz.len(), "über die Schiene weiter: {pot_leitungen} > {}", netz.len());
}

#[test]
fn teilstueck_loeschen_auf_w2() {
    let Some((mut app, r)) = w2() else { return };
    let n = app.project.sheet(r).unwrap().objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).count();
    app.mehrfachklick(426, P::new(28.0, 155.0), 0.5, 1);
    app.delete_selection();
    let s = app.project.sheet(r).unwrap();
    let w: Vec<&Vec<P>> = s.objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None }).collect();
    // aus einer Leitung werden zwei: 37,5|162,7 – 37,5|155 und 20|155 – 20|183
    assert_eq!(w.len(), n + 1);
    assert!(w.iter().any(|p| **p == vec![P::new(37.5, 162.7), P::new(37.5, 155.0)]));
    assert!(w.iter().any(|p| **p == vec![P::new(20.0, 155.0), P::new(20.0, 183.0)]));
    app.do_undo();
    assert_eq!(app.project.sheet(r).unwrap().objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).count(), n);
}
