//! Netz-Index (netz.rs) und Teilstück ziehen (segment_ziehen.rs) gegen W2. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{blatt_tasten::test_app, epr, knoten, leitung_fuehrung, model::*, netz, schiene_halten, segment_ziehen};
use std::path::PathBuf;

fn sample() -> Option<Project> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then(|| epr::import(&p).unwrap())
}

fn blaetter(p: &Project) -> impl Iterator<Item = (SheetRef, &Sheet)> {
    p.anlagen.iter().enumerate().flat_map(|(ai, a)| a.sheets.iter().enumerate().map(move |(si, s)| (SheetRef { anlage: ai, sheet: si }, s)))
}

/// Jede Leitung liegt in genau einem Netz; jedes Leitungsende, das nach der Geometrie an einer anderen Leitung,
/// einer Schiene oder Klemmstelle hängt (`knoten::ziel`, daraus schreibt der Nachbau die VBG-Enden), liegt im Netz
/// seines Ziels.
#[test]
fn w2_netze_folgen_den_enden() {
    let Some(project) = sample() else { return };
    let (mut enden, mut netze_gesamt) = (0, 0);
    for (_, s) in blaetter(&project) {
        let n = netz::netze(s, &[]);
        netze_gesamt += n.len();
        let wo = |id: u64| n.iter().position(|v| v.contains(&id));
        for o in &s.objects {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            assert_eq!(n.iter().filter(|v| v.contains(&o.id)).count(), 1, "Leitung in genau einem Netz");
            for e in [pts[0], *pts.last().unwrap()] {
                if let Some((_, z)) = knoten::ziel(s, o.id, e) {
                    assert_eq!(wo(o.id), wo(z.id), "Ende und Ziel im selben Netz");
                    enden += 1;
                }
            }
        }
    }
    // W2: 453 Enden der Art 3 und dazu die Abgriffe an Schienen und Klemmstellen
    assert!(enden > 453, "{enden}");
    assert!(netze_gesamt > 0);
}

/// Kontextmenü „Netz markieren“ auf einer W2-Leitung mit Abzweig: das ganze Netz ist markiert.
#[test]
fn w2_netz_markieren() {
    let Some(project) = sample() else { return };
    let (r, id, netz) = blaetter(&project)
        .find_map(|(r, s)| {
            netz::netze(s, &[]).into_iter().map(|n| netz::leitungen(s, &n)).find(|l| l.len() >= 3).map(|l| (r, l[0], l))
        })
        .expect("Netz mit mehreren Leitungen");
    let mut app = test_app();
    app.project = project;
    app.tabs = vec![r];
    app.active = 0;
    app.netz_markieren(id);
    let mut sel = app.selection.clone();
    sel.sort_unstable();
    assert_eq!(sel, netz);
    assert!(app.info.starts_with("Netz:"), "{}", app.info);
}

fn sauber(v: &[P]) -> bool {
    v.windows(2).all(|w| w[0].dist(w[1]) > 0.01) && v.windows(3).all(|w| {
        let (a, b) = (w[1].sub(w[0]), w[2].sub(w[1]));
        (a.x * b.y - a.y * b.x).abs() > 1e-3
    })
}

/// Jedes achsgerade Teilstück jeder W2-Leitung 10 mm quer ziehen: Enden bleiben (bzw. der Abgriff bleibt auf
/// seiner Schiene), keine doppelten Punkte, keine zurücklaufenden Stücke; zurück in die Ausgangslage ergibt wieder
/// die bereinigte Ausgangsleitung.
#[test]
fn w2_teilstueck_ziehen_hin_und_zurueck() {
    let Some(project) = sample() else { return };
    let mut n = 0;
    for (_, s) in blaetter(&project) {
        let sch = schiene_halten::schienen(s, &[]);
        for o in &s.objects {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            let orig = leitung_fuehrung::bereinigen(pts.clone());
            if orig.len() < 2 || !sauber(&orig) {
                continue;
            }
            for idx in 0..orig.len() - 1 {
                let (a, b) = (orig[idx], orig[idx + 1]);
                if (a.x - b.x).abs() > 1e-3 && (a.y - b.y).abs() > 1e-3 {
                    continue;
                }
                for d in [-10.0f32, 10.0] {
                    let neu = schiene_halten::abgreifen(&segment_ziehen::verschieben(&orig, idx, segment_ziehen::quer(a, b, P::new(d, d))), &sch);
                    assert!(sauber(&neu), "{neu:?}");
                    for (alt, jetzt) in [(orig[0], neu[0]), (*orig.last().unwrap(), *neu.last().unwrap())] {
                        let s0 = schiene_halten::schiene_bei(alt, &sch);
                        assert!(alt.dist(jetzt) < 1e-4 || (s0.is_some() && schiene_halten::schiene_bei(jetzt, &sch) == s0), "Ende bleibt angeschlossen: {orig:?} → {neu:?}");
                    }
                    n += 1;
                }
                assert_eq!(segment_ziehen::verschieben(&orig, idx, P::new(0.0, 0.0)), orig);
            }
        }
    }
    assert!(n > 1000, "{n}");
}
