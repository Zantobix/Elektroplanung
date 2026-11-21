//! Brücken mehrstöckiger Klemmleisten als Ketten (src/klemmplan.rs `ketten`) an W2.epr: Klemmplan -X2 der
//! Heizungssteuerung (zweistöckig). Fehlt W2.epr (EP_SAMPLE), wird übersprungen.

use elektroplanung::{epr, klemmplan};
use std::path::PathBuf;

#[test]
fn bruecken_der_zweistoeckigen_leiste_als_ketten() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let p = epr::import(&path).unwrap();
    let a = p.anlagen.iter().find(|a| a.name.starts_with("Heizungssteuerung")).expect("Anlage Heizungssteuerung");
    let plan = klemmplan::plan(a, "-X2", &p.symdefs);
    assert_eq!(plan.etagen, 2);
    let name = |r: usize| format!("{}{}", plan.zeilen[r].nr, (b'@' + plan.zeilen[r].level) as char);
    let k: Vec<(usize, Vec<String>)> = plan.bruecken.iter().map(|b| (b.lane, b.rows.iter().map(|r| name(*r)).collect())).collect();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    // 7B–8B liegt in einer eigenen Spur neben der A-Kette. Im heutigen W2 verbindet 10B→6B die B-Kette
    // durchgehend; der ältere Ausdruck Seite 39 zeigt ohne diese Brücke 2B–6B, 7A–12A in der ersten und 4A–7A,
    // 7B–8B, 10B–12B in der zweiten Spur (nachgestellt im Unit-Test `bruecken_als_ketten_mehrstoeckig`)
    assert_eq!(
        k,
        vec![
            (0, s(&["1A", "2A"])),
            (0, s(&["2B", "4B", "6B", "10B", "12B"])),
            (1, s(&["4A", "5A", "6A", "7A", "8A", "9A", "10A", "11A", "12A"])),
            (2, s(&["7B", "8B"])),
        ]
    );
}

/// Spuren und Ketten der einstöckigen Leisten wie im Ausdruck „Elektroplanung Wohnung 2“ (Seite 12: -X1 und
/// Seite 13: -X2 der Unterverteilung 1, Seite 22: -X3 der Unterverteilung 2).
#[test]
fn bruecken_wie_im_ausdruck() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let p = epr::import(&path).unwrap();
    let ketten = |anlage: &str, leiste: &str| -> Vec<(usize, Vec<u32>)> {
        let a = p.anlagen.iter().find(|a| a.name == anlage).expect("Anlage");
        let plan = klemmplan::plan(a, leiste, &p.symdefs);
        plan.bruecken.iter().map(|b| (b.lane, b.rows.iter().map(|r| plan.zeilen[*r].nr).collect())).collect()
    };
    assert_eq!(ketten("Unterverteilung 1", "-X1"), vec![(0, vec![3, 4, 5]), (0, vec![12, 14]), (1, vec![13, 15]), (0, vec![16, 18]), (1, vec![17, 19])]);
    assert_eq!(ketten("Unterverteilung 1", "-X2"), vec![(0, vec![1, 2]), (0, vec![3, 4]), (0, vec![5, 6, 7, 8]), (0, vec![10, 11, 12]), (0, vec![13, 14, 15])]);
    // 3–6, 4–5 und 5–6: eine Kette in der ersten Spur (vorher 3, 5, 6 und 4, 5 in zwei Spuren)
    assert_eq!(ketten("Unterverteilung 2", "-X3"), vec![(0, vec![1, 2]), (0, vec![3, 4, 5, 6])]);
    assert!(ketten("Unterverteilung 2", "-X1").is_empty() && ketten("Unterverteilung 2", "-X2").is_empty());
}
