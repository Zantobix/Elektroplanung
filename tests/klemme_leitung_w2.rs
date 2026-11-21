//! Werkzeug „Neue Klemmstelle“ auf einer Leitung (klemme_leitung.rs) gegen W2: Klemme in der Leitung, Nummer =
//! nächste der Klemmleiste, Rundreise .epr mit Verweisen der beiden Leitungsteile auf die Klemmstelle.
//! Ohne EP_SAMPLE werden die Tests übersprungen.

use elektroplanung::{blatt_tasten::test_app, epr, epr_write, klemme_leitung::KLEMME_R, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn kennung(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

#[test]
fn klemmstelle_in_leitung_und_rundreise() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    // Blatt mit Klemmstellen der Leiste -X2 und einer langen waagerechten Leitung ohne Klemme in der Mitte
    let mut wahl = None;
    'suche: for (ai, a) in project.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            if !s.objects.iter().any(|o| matches!(&o.kind, Kind::Terminal { leiste, .. } if leiste == "-X2")) {
                continue;
            }
            for o in &s.objects {
                let Kind::Wire { pts, .. } = &o.kind else { continue };
                if o.raw.is_none() {
                    continue;
                }
                for w in pts.windows(2) {
                    if (w[0].y - w[1].y).abs() < 1e-3 && (w[0].x - w[1].x).abs() >= 30.0 {
                        let mid = P::new(((w[0].x + w[1].x) / 2.0 / 5.0).round() * 5.0, w[0].y);
                        // keine andere Leitung in der Nähe (Treffer eindeutig)
                        let andere = s.objects.iter().filter(|x| x.id != o.id).any(|x| match &x.kind {
                            Kind::Wire { pts, .. } => pts.windows(2).any(|v| seg_dist(mid, v[0], v[1]) < 3.0),
                            _ => elektroplanung::canvas::obj_pins(x, &[]).iter().any(|q| q.dist(mid) < 3.0),
                        });
                        if !andere {
                            wahl = Some((SheetRef { anlage: ai, sheet: si }, o.id, mid));
                            break 'suche;
                        }
                    }
                }
            }
        }
    }
    let (r, wid, mid) = wahl.expect("Leitung auf einem Blatt mit -X2");
    // Zähler der Klemmleiste (KLM Feld 18) wie im Original: -X2 in Unterverteilung 1 → „-X2.15“
    assert_eq!(project.anlagen[r.anlage].name, "Unterverteilung 1");
    let soll_nr = 15;

    let mut app = test_app();
    app.project = project;
    app.tabs = vec![r];
    app.active = 0;
    app.klemme = app.klemmleisten().iter().position(|k| k == "-X2").expect("Klemmleiste -X2");
    let n_vorher = app.sheet().unwrap().objects.len();
    let klick = P::new(mid.x + 0.3, mid.y + 0.4);
    let soll = P::new(app.snap(klick).x, mid.y);
    let tid = app.klemmstelle_setzen(klick, 1.0);
    let s = app.sheet().unwrap();
    assert_eq!(s.objects.len(), n_vorher + 2, "Klemme und zweiter Leitungsteil");
    let t = s.objects.iter().find(|o| o.id == tid).unwrap();
    let Kind::Terminal { pos, nr, leiste, .. } = &t.kind else { panic!() };
    assert_eq!((leiste.as_str(), *nr), ("-X2", soll_nr));
    assert!(pos.dist(soll) < 0.01, "Klemme auf der Leitung am Rasterpunkt: {pos:?} {soll:?}");
    let enden: Vec<(u64, P, P)> = s.objects.iter().filter_map(|o| match &o.kind {
        Kind::Wire { pts, .. } => Some((o.id, pts[0], *pts.last().unwrap())),
        _ => None,
    }).collect();
    let erster = enden.iter().find(|(id, ..)| *id == wid).unwrap();
    assert!((erster.2.dist(*pos) - KLEMME_R).abs() < 0.01, "erster Teil endet am Kreis");
    let zweiter = enden.iter().find(|(id, a, _)| *id != wid && (a.dist(*pos) - KLEMME_R).abs() < 0.01).expect("zweiter Teil beginnt am Kreis");
    assert_ne!(zweiter.0, tid);
    // die nächste Klemmstelle derselben Leiste zählt weiter
    assert_eq!(app.klemmen_zaehler("-X2"), soll_nr + 1);
    // ein Rückgängig-Schritt
    let nach = app.project.clone();
    app.do_undo();
    assert_eq!(app.sheet().unwrap().objects.len(), n_vorher);
    app.project = nach;

    // Rundreise .epr
    let bytes = epr_write::to_bytes(&app.project).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_klemme_leitung_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let p2 = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let s2 = p2.sheet(r).unwrap();
    let t2 = s2.objects.iter().find(|o| matches!(&o.kind, Kind::Terminal { leiste, nr, pos, .. } if leiste == "-X2" && *nr == soll_nr && pos.dist(soll) < 0.01)).expect("neue Klemme nach dem Laden");
    let tk = kennung(&t2.raw.as_ref().unwrap().block.data, 0);
    assert_ne!(tk, 0);
    let Kind::Terminal { pos: p2pos, .. } = &t2.kind else { panic!() };
    let teile: Vec<&Obj> = s2.objects.iter().filter(|o| match &o.kind {
        Kind::Wire { pts, .. } => [pts[0], *pts.last().unwrap()].iter().any(|q| (q.dist(*p2pos) - KLEMME_R).abs() < 0.01),
        _ => false,
    }).collect();
    assert_eq!(teile.len(), 2, "zwei Leitungen an der neuen Klemme");
    let mut eigene = vec![];
    for w in teile {
        let b = &w.raw.as_ref().unwrap().block.data;
        let Kind::Wire { pts, .. } = &w.kind else { unreachable!() };
        let feld = if (pts[0].dist(*p2pos) - KLEMME_R).abs() < 0.01 { 2 } else { 6 };
        assert_eq!(kennung(b, feld * 4), tk, "Ende verweist auf die Klemmstelle");
        eigene.push(kennung(b, 40));
    }
    assert_ne!(eigene[0], eigene[1], "jede Verbindung mit eigener Kennung");
}
