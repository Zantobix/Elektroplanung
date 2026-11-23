//! „Auto+“ des Blattdialogs (BLT Feld 32 Bit 5, src/blatt_neu.rs): in W2.epr hat nur „Klemmplan -X1“ das Bit;
//! Setzen und Löschen kommt nach Speichern und Lesen an, auch zusammen mit einem Wechsel der Ausrichtung.

use elektroplanung::{epr, epr_write};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

#[test]
fn auto_plus_lesen_und_schreiben() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let mit: Vec<&str> = p.anlagen.iter().flat_map(|a| &a.sheets).filter(|s| s.auto_plus).map(|s| s.name.as_str()).collect();
    assert_eq!(mit, vec!["Klemmplan -X1"]);
    let (ai, si) = p.anlagen.iter().enumerate().find_map(|(ai, a)| a.sheets.iter().position(|s| s.auto_plus).map(|si| (ai, si))).unwrap();
    p.anlagen[ai].sheets[si].auto_plus = false;
    let s0 = &mut p.anlagen[0].sheets[0];
    s0.auto_plus = true;
    s0.portrait = !s0.portrait;
    let (name0, hoch0) = (s0.name.clone(), s0.portrait);
    let out = std::env::temp_dir().join("ep_blatt_auto.epr");
    epr_write::save(&p, &out).unwrap();
    let b = epr::import(&out).unwrap();
    let s = &b.anlagen[0].sheets[0];
    assert_eq!((s.name.as_str(), s.auto_plus, s.portrait), (name0.as_str(), true, hoch0));
    assert!(!b.anlagen[ai].sheets[si].auto_plus);
    assert_eq!(b.anlagen.iter().flat_map(|a| &a.sheets).filter(|s| s.auto_plus).count(), 1);
}
