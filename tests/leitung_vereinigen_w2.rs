//! Übereinanderliegende Leitungen gegen W2 (leitung_vereinigen.rs): Bestand messen (nur Bericht), Öffnen/Speichern
//! verändert nichts, Vereinigen nach einer Bearbeitung räumt auf und übersteht die Rundreise. Ohne EP_SAMPLE werden
//! die Tests übersprungen.

use elektroplanung::{epr, epr_write, leitung_vereinigen, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

/// Bericht: wie viele Leitungspaare (und Leitung/Schiene-Paare) sich in W2 überdecken. Ändert nichts.
#[test]
fn w2_ueberdeckungen_zaehlen() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let (mut leitung, mut schiene, mut laenge) = (0, 0, 0.0);
    for a in &project.anlagen {
        for s in &a.sheets {
            for (x, y, l) in leitung_vereinigen::paare(s) {
                let ist_schiene = s.objects.iter().any(|o| o.id == y && matches!(o.kind, Kind::Busbar { .. }));
                if ist_schiene {
                    schiene += 1;
                } else {
                    leitung += 1;
                }
                laenge += l;
                eprintln!("W2 Überdeckung Blatt „{}“: {x} / {y}{} {l:.1} mm", s.name, if ist_schiene { " (Schiene)" } else { "" });
            }
        }
    }
    eprintln!("W2: {leitung} Leitungspaare, {schiene} Leitung/Schiene-Paare überdecken sich, zusammen {laenge:.1} mm");
}

/// Vereinigen auf jedem W2-Blatt (alle Leitungen als bearbeitet): danach keine Überdeckung zwischen Leitungen,
/// keine Verbindung zu Symbolanschlüssen verloren, und die Datei übersteht die Rundreise.
#[test]
fn w2_vereinigen_raeumt_auf() {
    let Some(path) = sample() else { return };
    let mut project = epr::import(&path).unwrap();
    let vorher = epr_write::to_bytes(&project).unwrap();
    assert_eq!(vorher, std::fs::read(&path).unwrap(), "Öffnen/Speichern bleibt byteidentisch");
    let mut next = project.next_id;
    for a in &mut project.anlagen {
        for s in &mut a.sheets {
            let ids: Vec<u64> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).map(|o| o.id).collect();
            let enden = |s: &Sheet| -> Vec<P> { s.objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.clone()) } else { None }).flat_map(|p| [p[0], p[p.len() - 1]]).collect() };
            let alt = enden(s);
            leitung_vereinigen::vereinigen(s, &ids, &[], &mut next);
            assert!(leitung_vereinigen::paare(s).iter().all(|(_, y, _)| s.objects.iter().any(|o| o.id == *y && matches!(o.kind, Kind::Busbar { .. }))), "Blatt {}: {:?}", s.name, leitung_vereinigen::paare(s));
            // jedes frühere Ende liegt noch auf einer Leitung
            let neu: Vec<Vec<P>> = s.objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.clone()) } else { None }).collect();
            for e in alt {
                assert!(neu.iter().any(|p| p.windows(2).any(|w| seg_dist(e, w[0], w[1]) < 0.05)), "Blatt {}: Ende {e:?} hängt frei", s.name);
            }
        }
    }
    project.next_id = next;
    let bytes = epr_write::to_bytes(&project).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_vereinigen_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let p2 = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let zahl = |p: &Project| p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(o.kind, Kind::Wire { .. })).count();
    assert_eq!(zahl(&p2), zahl(&project));
}
