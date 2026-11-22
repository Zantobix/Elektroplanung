//! Hindernisse bei der Neuführung (leitung_hindernis.rs, Maus-Befund M29) gegen W2: Symbole weit verschieben, die
//! angeschlossenen Leitungen mitziehen und prüfen, dass keine neu geführte Leitung durch ein fremdes Symbol läuft,
//! wo es vermeidbar ist. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{epr, leitung_fuehrung, leitung_hindernis as lh, leitung_profil, leitung_route, mitziehen, model::*, symbols};
use std::path::PathBuf;

fn soll_a(w: &mitziehen::Mitzieh, d: P) -> P {
    if w.ends[0] { w.pts[0].add(d) } else { w.pts[0] }
}

fn soll_e(w: &mitziehen::Mitzieh, d: P) -> P {
    let e = w.pts[w.pts.len() - 1];
    if w.ends[1] { e.add(d) } else { e }
}

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

#[test]
fn w2_neufuehrung_umgeht_fremde_symbole() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let mut lib = std::env::var("EP_DATENBANK").ok().and_then(|p| symbols::load_db(&PathBuf::from(p)).ok()).unwrap_or_else(symbols::builtin);
    lib.extend(project.symdefs.iter().filter(|d| symbols::find(&lib, &d.key).is_none()).cloned().collect::<Vec<_>>());
    let raster = |p: P| P::new((p.x / 5.0).round() * 5.0, (p.y / 5.0).round() * 5.0);
    let (mut umgangen, mut unvermeidbar, mut profil_durch, mut fehler) = (0, 0, 0, vec![]);
    // große Verschiebungen (Quadrantenwechsel), auf dem Raster wie beim Ziehen
    for d in [P::new(60.0, 0.0), P::new(-80.0, 40.0), P::new(0.0, -70.0), P::new(45.0, 55.0)] {
        for a in &project.anlagen {
            for s in &a.sheets {
                let ids: Vec<u64> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| o.id).collect();
                for id in ids {
                    let f = mitziehen::find_followers(s, &lib, &[id], &[id], false);
                    if f.is_empty() {
                        continue;
                    }
                    let mut neu = s.clone();
                    neu.objects.iter_mut().filter(|o| o.id == id).for_each(|o| o.kind.translate(d));
                    let umrisse = lh::umrisse(&neu, &lib);
                    for w in &f {
                        let hind = lh::wirksam(&mitziehen::hindernisse(&umrisse, w, d), soll_a(w, d), soll_e(w, d));
                        let standard = mitziehen::folgen(&w.pts, w.ends, d, w.dirs, w.profil, 2.5, &raster);
                        let pts = mitziehen::folgen_um(&w.pts, w.ends, d, w.dirs, w.profil, 2.5, &hind, &raster);
                        let soll = [soll_a(w, d), soll_e(w, d)];
                        assert!(pts[0].dist(soll[0]) < 1e-3 && pts.last().unwrap().dist(soll[1]) < 1e-3, "Blatt {} Leitung {}: {pts:?}", s.number, w.id);
                        if pts == standard {
                            // unverändert: entweder frei oder nicht vermeidbar (dann hätte umgehen etwas gefunden)
                            if lh::kreuzungen(&standard, &hind) > 0 {
                                // war es eine Neuführung (Stich + Z), dann fand umgehen keinen freien Weg
                                let z = |a: P, da, b: P, db| leitung_fuehrung::bereinigen(leitung_route::mit_stichen(a, da, b, db, 2.5));
                                let mut rueck = z(soll_e(w, d), w.dirs[1], soll_a(w, d), w.dirs[0]);
                                rueck.reverse();
                                if standard == z(soll_a(w, d), w.dirs[0], soll_e(w, d), w.dirs[1]) || standard == rueck {
                                    assert_eq!(lh::umgehen(&standard, w.dirs[0], w.dirs[1], &hind, 5.0, false), standard);
                                    unvermeidbar += 1;
                                } else {
                                    profil_durch += 1;
                                }
                            }
                            continue;
                        }
                        // nur Neuführungen ändern sich: frei, rechtwinklig, mit weniger Kreuzungen als vorher
                        if lh::kreuzungen(&pts, &hind) > 0 || lh::kreuzungen(&standard, &hind) == 0 {
                            fehler.push((s.number, w.id, "kreuzt", pts.clone()));
                        }
                        if leitung_profil::profil(&standard).is_some() && leitung_profil::profil(&pts).is_none() {
                            fehler.push((s.number, w.id, "schräg", pts.clone()));
                        }
                        umgangen += 1;
                    }
                }
            }
        }
    }
    // formerhaltende Führungen (Profil) bleiben bewusst, auch wenn sie ein Symbol kreuzen
    println!("{umgangen} Neuführungen um Symbole geführt, {unvermeidbar} Neuführungen unvermeidbar durch ein Symbol, {profil_durch} formerhaltend durch ein Symbol");
    assert!(umgangen > 20, "zu wenige Fälle: {umgangen}");
    assert!(fehler.is_empty(), "{} Fehler: {:?}", fehler.len(), &fehler[..fehler.len().min(5)]);
}
