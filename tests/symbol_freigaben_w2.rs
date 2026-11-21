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

/// Freigaben der Bibliothekssymbole (Symbole.tfi, Symbolkopf Byte 45) beim Laden der Original-Datenbank
/// (EP_DATENBANK; gesperrt vom laufenden Original: Kopie in %TEMP%\ep_orig_test\Datenbank). Sie stimmen mit den
/// Symbolen gleichen Namens in W2.epr (SYM Byte 112) überein, soweit beide einen gültigen Wert tragen.
#[test]
fn freigaben_aus_der_bibliothek() {
    let kandidaten = [std::env::var("EP_DATENBANK").map(PathBuf::from).unwrap_or_default(), std::env::temp_dir().join("ep_orig_test").join("Datenbank")];
    let Some(lib) = kandidaten.iter().filter(|d| d.join("Symbole.tfi").is_file()).find_map(|d| elektroplanung::symbols::load_db(d).ok()) else {
        eprintln!("Bibliothek nicht lesbar – Test übersprungen");
        return;
    };
    let frg = |n: &str| lib.iter().find(|d| d.name == n && d.haupt == 300).map(|d| d.frg);
    assert_eq!(frg("RCD"), Some((true, true)));
    assert_eq!(frg("Leitungsschutzsch."), Some((true, true)));
    assert_eq!(frg("Klemme"), Some((false, false)));
    assert_eq!(frg("Relais"), Some((true, false)));
    // Textreste im Kopf (Verteilerbox) bzw. leerer Kopf (Motor): Vorgabe
    assert_eq!(frg("Verteilerbox"), Some(elektroplanung::symbol_freigaben::VORGABE));
    assert_eq!(frg("Motor"), Some(elektroplanung::symbol_freigaben::VORGABE));
    // Gegenprobe mit W2.epr
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let p = epr::import(&path).unwrap();
    let mut gleich = 0;
    for d in &p.symdefs {
        let Some(l) = lib.iter().find(|l| l.name == d.name && l.haupt == 300) else { continue };
        if ["RCD", "Leitungsschutzsch.", "Klemme", "Relais", "Schließer", "Lampe"].contains(&d.name.as_str()) {
            assert_eq!(l.frg, d.frg, "{}", d.name);
            gleich += 1;
        }
    }
    assert!(gleich >= 3, "{gleich}");
}
