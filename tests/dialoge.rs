//! Reine Hilfsfunktionen der Dialoge (dialoge3.rs): Titelgenerator, Reihenfolge, Listen laden/speichern,
//! Fehlerliste nach dem Laden.

use elektroplanung::{dialoge3, epr, model::*};
use std::path::PathBuf;

#[test]
fn titelgenerator_formel() {
    assert_eq!(dialoge3::titel_formel("-S@x", 3, 7, 2, "UV1", ""), "-S3");
    assert_eq!(dialoge3::titel_formel("@A/@S.@P (@T)", 1, 4, 10, "UV2", "alt"), "UV2/4.10 (alt)");
}

#[test]
fn reihenfolge_sortiert_wie_gewaehlt() {
    let pts = [P::new(30.0, 10.0), P::new(10.0, 20.0), P::new(20.0, 5.0)];
    let sorted = |dir: usize| {
        let mut v = pts.to_vec();
        v.sort_by(|a, b| dialoge3::order_key(*a, dir).partial_cmp(&dialoge3::order_key(*b, dir)).unwrap());
        v.iter().map(|p| p.x as i32).collect::<Vec<_>>()
    };
    assert_eq!(sorted(0), [10, 20, 30]); // links → rechts
    assert_eq!(sorted(1), [30, 20, 10]); // rechts → links
    assert_eq!(sorted(2), [20, 30, 10]); // oben → unten
    assert_eq!(sorted(3), [10, 30, 20]); // unten → oben
}

#[test]
fn liste_laden_speichern() {
    let rows = vec![vec!["Port".to_string(), "Gerät".to_string()], vec!["1".into(), "-PP1.3".into()]];
    let txt = dialoge3::list_to_text(&rows);
    assert_eq!(dialoge3::list_from_text(&txt, 2), rows);
    // fehlende Spalten werden aufgefüllt
    assert_eq!(dialoge3::list_from_text("a", 3), vec![vec!["a".to_string(), String::new(), String::new()]]);
}

#[test]
fn w2_laedt_ohne_fehlerliste() {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !p.exists() {
        return;
    }
    let prj = epr::import(&p).unwrap();
    assert!(epr::load_warnings(&prj).is_empty(), "{:?}", epr::load_warnings(&prj));
}

#[test]
fn schienen_einstellungen_und_schriftfeldhoehe() {
    let mut p = Project::new_default();
    assert_eq!(p.title_h(), TITLE_H);
    p.tb_hoehe = 20.0;
    assert_eq!(p.title_h(), 20.0);
    let set = SchieneSet::default();
    assert!(set.quer && set.pfeile && !set.einspeisung);
    // ältere Projektdateien ohne die neuen Felder bleiben lesbar
    let mut v = serde_json::to_value(&p).unwrap();
    v.as_object_mut().unwrap().remove("tb_hoehe");
    let back: Project = serde_json::from_value(v).unwrap();
    assert_eq!(back.title_h(), TITLE_H);
}
