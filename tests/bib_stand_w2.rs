//! Ablegen eines Bibliothekssymbols, das schon im Projekt steht (bib_stand.rs, PaintBox1DragDrop 0x440bbe): gegen
//! W2.epr und die Original-Datenbank. „Sicherung“ ist in der Bibliothek neuer (05.03.2025) als im Projekt
//! (01.03.2025), „Lampe“ hat in beiden denselben Stand. Ohne EP_SAMPLE/EP_DATENBANK übersprungen.

use elektroplanung::blatt_tasten::test_app;
use elektroplanung::meldungen::{self, Antwort};
use elektroplanung::{bib_stand, epr, epr_write, symbols};
use std::path::PathBuf;

fn pfade() -> Option<(PathBuf, PathBuf)> {
    let w2 = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    let db = std::env::var("EP_DATENBANK").map(PathBuf::from).unwrap_or_default();
    if w2.exists() && db.join("Symbole.tfi").exists() {
        Some((w2, db))
    } else {
        eprintln!("W2.epr oder Datenbank nicht gefunden – Test übersprungen");
        None
    }
}

fn app(w2: &PathBuf, db: &PathBuf) -> elektroplanung::app::App {
    let mut app = test_app();
    app.project = epr::import(w2).unwrap();
    let mut lib = symbols::load_db(db).unwrap();
    lib.extend(app.project.symdefs.iter().cloned());
    app.lib = lib;
    app.lib_dir = Some(db.clone());
    app
}

/// Schlüssel des Bibliothekssymbols `name`, das (gleiche Kennung) im Projekt steht.
fn bib_key(app: &elektroplanung::app::App, db: &PathBuf, name: &str) -> String {
    let st = bib_stand::bibliothek(db);
    app.lib
        .iter()
        .filter(|d| d.key.starts_with("db:") && d.name == name)
        .find(|d| st.get(&d.key).is_some_and(|(id, _)| app.project.symdefs.iter().any(|p| p.key == format!("epr:{id:016x}"))))
        .unwrap()
        .key
        .clone()
}

#[test]
fn neuere_bibliothek_fragt_und_uebernimmt() {
    let Some((w2, db)) = pfade() else { return };
    let mut app = app(&w2, &db);
    let key = bib_key(&app, &db, "Sicherung");
    let (id, datum) = bib_stand::bibliothek(&db)[&key];
    meldungen::test_antworten(&[Antwort::Ja]);
    let k = app.bibliothekssymbol_ablegen(key.clone());
    let gezeigt = meldungen::test_gezeigt();
    assert_eq!(k, format!("epr:{id:016x}"), "Projektsymbol wird gesetzt");
    assert_eq!(gezeigt.len(), 1);
    assert_eq!(gezeigt[0].0, "Achtung");
    assert!(gezeigt[0].1.starts_with("Das Symbol aus der Bibliothek ist  neuer als das Symbol im Projekt."), "{}", gezeigt[0].1);
    assert_eq!(app.project.sym_geaendert.get(&id), Some(&datum));
    // Speichern: SYM @104 trägt den Bibliotheksstand, danach keine Frage mehr
    let tmp = std::env::temp_dir().join(format!("ep_bib_stand_{}.epr", std::process::id()));
    epr_write::save(&app.project, &tmp).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    assert!(q.sym_geaendert.is_empty());
    assert_eq!(bib_stand::projekt_datum(&q, id), Some(datum));
}

#[test]
fn nein_behaelt_projektsymbol() {
    let Some((w2, db)) = pfade() else { return };
    let mut app = app(&w2, &db);
    let key = bib_key(&app, &db, "Sicherung");
    let (id, _) = bib_stand::bibliothek(&db)[&key];
    let vorher = app.project.symdefs.clone();
    meldungen::test_antworten(&[Antwort::Nein]);
    let k = app.bibliothekssymbol_ablegen(key);
    assert_eq!(meldungen::test_gezeigt().len(), 1);
    assert_eq!(k, format!("epr:{id:016x}"));
    assert_eq!(app.project.symdefs, vorher);
    assert!(app.project.sym_geaendert.is_empty());
}

#[test]
fn gleicher_stand_ohne_frage() {
    let Some((w2, db)) = pfade() else { return };
    let mut app = app(&w2, &db);
    let key = bib_key(&app, &db, "Lampe");
    let (id, _) = bib_stand::bibliothek(&db)[&key];
    meldungen::test_antworten(&[]);
    let k = app.bibliothekssymbol_ablegen(key);
    assert!(meldungen::test_gezeigt().is_empty());
    assert_eq!(k, format!("epr:{id:016x}"));
}
