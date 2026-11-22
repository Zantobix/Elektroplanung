//! Mitziehen mit Profil (mitziehen.rs, leitung_profil.rs) gegen W2: jedes Symbol und jede Klemmstelle einzeln
//! verschieben, die angeschlossenen Leitungen nachführen und prüfen: Enden sitzen weiter am Anschluss (VBG-Enden aus
//! der Geometrie wie vorher), rechtwinklige Leitungen bleiben rechtwinklig. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{epr, leitung_profil, mitziehen, model::*, symbols, vbg_enden};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

#[test]
fn w2_verschieben_enden_und_rechtwinkligkeit_bleiben() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    // Bibliothek wie im Programm: Datenbank, dazu die Projektsymbole
    let mut lib = std::env::var("EP_DATENBANK").ok().and_then(|p| symbols::load_db(&PathBuf::from(p)).ok()).unwrap_or_else(symbols::builtin);
    lib.extend(project.symdefs.iter().filter(|d| symbols::find(&lib, &d.key).is_none()).cloned().collect::<Vec<_>>());
    let (mut leitungen, mut gleiche_form, mut enden_falsch, mut schief) = (0, 0, vec![], vec![]);
    // krumme Verschiebungen: das bewegte Objekt landet nicht zufällig auf einem fremden Anschluss
    for d in [P::new(3.1, 4.3), P::new(-7.7, 0.0), P::new(0.0, 6.9), P::new(-41.3, -23.9)] {
        for a in &project.anlagen {
            for s in &a.sheets {
                let bewegbar: Vec<u64> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })).map(|o| o.id).collect();
                for id in bewegbar {
                    let f = mitziehen::find_followers(s, &lib, &[id], &[id], false);
                    if f.is_empty() {
                        continue;
                    }
                    let mut neu = s.clone();
                    neu.objects.iter_mut().filter(|o| o.id == id).for_each(|o| o.kind.translate(d));
                    for w in &f {
                        let pts = mitziehen::folgen(&w.pts, w.ends, d, w.dirs, w.profil, 2.5, &|p| p);
                        // bewegtes Ende genau mitgewandert, festes Ende geblieben
                        let n = w.pts.len();
                        let soll = [if w.ends[0] { w.pts[0].add(d) } else { w.pts[0] }, if w.ends[1] { w.pts[n - 1].add(d) } else { w.pts[n - 1] }];
                        assert!(pts[0].dist(soll[0]) < 1e-3 && pts.last().unwrap().dist(soll[1]) < 1e-3, "Blatt {} Leitung {}: {:?} → {pts:?}", s.number, w.id, w.pts);
                        leitungen += 1;
                        if pts.len() == n {
                            gleiche_form += 1;
                        }
                        if leitung_profil::profil(&w.pts).is_some() && leitung_profil::profil(&pts).is_none() {
                            schief.push((s.number, w.id));
                        }
                        if let Some(Obj { kind: Kind::Wire { pts: p, .. }, .. }) = neu.objects.iter_mut().find(|o| o.id == w.id) {
                            *p = pts;
                        }
                    }
                    // Leitungsenden aus der Geometrie: vor und nach dem Verschieben gleich
                    for w in &f {
                        let Some(Obj { kind: Kind::Wire { pts, .. }, .. }) = neu.objects.iter().find(|o| o.id == w.id) else { continue };
                        for (i, (vorher, nachher)) in [(w.pts[0], pts[0]), (*w.pts.last().unwrap(), *pts.last().unwrap())].into_iter().enumerate() {
                            let alt = vbg_enden::bestimmen(&project, s, w.id, vorher, i == 1);
                            let jetzt = vbg_enden::bestimmen(&project, &neu, w.id, nachher, i == 1);
                            if alt != jetzt {
                                enden_falsch.push((s.number, w.id, i, alt, jetzt));
                            }
                        }
                    }
                }
            }
        }
    }
    println!("{leitungen} Leitungen nachgeführt, {gleiche_form} mit gleicher Punktzahl");
    assert!(leitungen > 500, "{leitungen}");
    assert!(schief.is_empty(), "rechtwinklige Leitungen schräg geworden: {schief:?}");
    assert!(enden_falsch.is_empty(), "{} Enden geändert: {:?}", enden_falsch.len(), &enden_falsch[..enden_falsch.len().min(10)]);
}
