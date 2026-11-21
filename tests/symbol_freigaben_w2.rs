//! Freigaben der Symboldefinitionen (SYM Byte 112, src/symbol_freigaben.rs) aus W2.epr lesen, ändern, speichern
//! und wieder lesen. Fehlt W2.epr (EP_SAMPLE), wird übersprungen.

use elektroplanung::{epr, epr_write};
use std::path::PathBuf;

#[test]
fn freigaben_lesen_aendern_schreiben() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let mut p = epr::import(&path).unwrap();
    let frg = |p: &elektroplanung::model::Project, n: &str| p.symdefs.iter().find(|d| d.name == n).map(|d| d.frg);
    // RCD und Leitungsschutzschalter mit Sicherungsübersicht, die Klemme ohne Querverweise
    assert_eq!(frg(&p, "RCD"), Some((true, true)));
    assert_eq!(frg(&p, "Leitungsschutzsch."), Some((true, true)));
    assert_eq!(frg(&p, "Klemme"), Some((false, false)));
    assert_eq!(frg(&p, "Relais"), Some((true, false)));
    // ändern und zurücklesen
    p.symdefs.iter_mut().find(|d| d.name == "Relais").unwrap().frg = (false, true);
    p.symdefs.iter_mut().find(|d| d.name == "Klemme").unwrap().frg = (true, false);
    let out = std::env::temp_dir().join(format!("ep_symfrg_{}.epr", std::process::id()));
    epr_write::save(&p, &out).unwrap();
    let q = epr::import(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    assert_eq!(frg(&q, "Relais"), Some((false, true)));
    assert_eq!(frg(&q, "Klemme"), Some((true, false)));
    assert_eq!(frg(&q, "RCD"), Some((true, true)));
}
