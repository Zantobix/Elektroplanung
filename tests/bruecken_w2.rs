//! Brückenbögen an Kreuzungen im Beispielprojekt (W2.epr): Kreuzungen werden gefunden, aber nie an einer Stelle,
//! an der eine Leitung endet (Verbindung). Fehlt die Datei (EP_SAMPLE), wird der Test übersprungen.

use elektroplanung::{bruecken, epr, model::*};
use std::path::PathBuf;

#[test]
fn kreuzungen_in_w2() {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !p.exists() {
        eprintln!("W2.epr nicht gefunden – Test übersprungen");
        return;
    }
    let pr = epr::import(&p).unwrap();
    let mut n = 0;
    for a in &pr.anlagen {
        for s in &a.sheets {
            let k = bruecken::karte(&s.objects);
            let enden: Vec<P> = s
                .objects
                .iter()
                .filter_map(|o| match &o.kind {
                    Kind::Wire { pts, .. } if pts.len() >= 2 => Some([pts[0], pts[pts.len() - 1]]),
                    _ => None,
                })
                .flatten()
                .collect();
            for xs in k.values() {
                for x in xs {
                    assert!(!enden.iter().any(|e| (e.x - x.x).abs() < 0.5 && (e.y - x.y).abs() < 0.5), "Bogen an Verbindung {x:?} Blatt {}", s.number);
                    n += 1;
                }
            }
            if !k.is_empty() {
                eprintln!("Blatt {} ({}): {} Bögen", s.number, a.name, k.values().map(|v| v.len()).sum::<usize>());
            }
        }
    }
    eprintln!("W2: {n} Brückenbögen");
}
