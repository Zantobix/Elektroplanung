//! Verbindungsknoten und Leitungen von Sammelschienen gegen W2 (knoten.rs, schiene_leitung.rs): Knoten
//! verschwinden/entstehen beim Verschieben, die Anschlussart im VBG folgt der Geometrie, unveränderte Leitungen
//! bleiben byteidentisch. Ohne EP_SAMPLE werden die Tests übersprungen.

use elektroplanung::{blatt_tasten::test_app, canvas, epr, epr_write, knoten, model::*, segment_ziehen};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn kennung(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn rundreise(project: &Project, name: &str) -> Project {
    let bytes = epr_write::to_bytes(project).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_knoten_{name}_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let p = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    p
}

fn hat_knoten(s: &Sheet, p: P) -> bool {
    canvas::junctions(s).iter().any(|j| j.dist(p) < 0.05)
}

/// Alle in W2 gespeicherten Knoten (Ende der Art 3) liegen im Innern der Leitung, auf die sie verweisen, und der
/// Nachbau zeichnet dort einen Punkt; beim Speichern bleibt jede dieser Leitungen byteidentisch.
#[test]
fn w2_knoten_stimmen_mit_geometrie() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let mut n = 0;
    for a in &project.anlagen {
        for s in &a.sheets {
            for o in &s.objects {
                let (Kind::Wire { pts, .. }, Some(r)) = (&o.kind, o.raw.as_deref()) else { continue };
                if r.block.data[0x14] != knoten::ART_LEITUNG {
                    continue;
                }
                let e = *pts.last().unwrap();
                let ziel = kennung(&r.block.data, 24);
                let z = s.objects.iter().find(|x| x.raw.as_deref().is_some_and(|r| r.block.is("VBG") && kennung(&r.block.data, 40) == ziel)).expect("Zielleitung");
                let Kind::Wire { pts: zp, .. } = &z.kind else { panic!() };
                assert!(knoten::im_innern(zp, e), "Knoten im Innern der Zielleitung");
                assert!(hat_knoten(s, e));
                assert_eq!(epr_write::object_block(&project, o).unwrap().data, r.block.data);
                n += 1;
            }
        }
    }
    assert_eq!(n, 453);
}

/// Teilstück der Zielleitung wegziehen: der Punkt verschwindet, die abzweigende Leitung verliert beim Speichern
/// ihren Knoten (Art 0, keine Kennung); zurückziehen: Punkt wieder da, Block wieder wie geladen.
#[test]
fn knoten_verschwindet_und_entsteht_beim_verschieben() {
    let Some(path) = sample() else { return };
    let mut project = epr::import(&path).unwrap();
    // Abzweig suchen, dessen Zielteilstück waagerecht ist und in dessen Nähe (10 mm darunter) sonst nichts liegt
    let mut wahl = None;
    'suche: for (ai, a) in project.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                let (Kind::Wire { pts, .. }, Some(r)) = (&o.kind, o.raw.as_deref()) else { continue };
                if r.block.data[0x14] != knoten::ART_LEITUNG {
                    continue;
                }
                let e = *pts.last().unwrap();
                let ziel = kennung(&r.block.data, 24);
                let Some(z) = s.objects.iter().find(|x| x.raw.as_deref().is_some_and(|r| r.block.is("VBG") && kennung(&r.block.data, 40) == ziel)) else { continue };
                let Kind::Wire { pts: zp, .. } = &z.kind else { continue };
                // Zielleitung trägt nur diesen einen Abzweig, Teilstück waagerecht
                let abzweige = s.objects.iter().filter(|x| matches!(&x.kind, Kind::Wire { pts, .. } if x.id != z.id && [pts[0], *pts.last().unwrap()].iter().any(|q| knoten::im_innern(zp, *q)))).count();
                let Some(idx) = zp.windows(2).position(|w| seg_dist(e, w[0], w[1]) < 0.05 && (w[0].y - w[1].y).abs() < 1e-3) else { continue };
                let frei = !s.objects.iter().any(|x| x.id != z.id && x.id != o.id && (canvas::obj_pins(x, &[]).iter().any(|q| q.dist(e) < 0.5) || matches!(&x.kind, Kind::Wire { pts, .. } if pts.windows(2).any(|w| seg_dist(e, w[0], w[1]) < 0.5))));
                if frei && abzweige == 1 && s.objects.iter().filter(|x| matches!(x.kind, Kind::Wire { .. })).count() > 2 {
                    wahl = Some((SheetRef { anlage: ai, sheet: si }, o.id, z.id, idx, e, zp.clone()));
                    break 'suche;
                }
            }
        }
    }
    let (r, bid, zid, idx, e, orig) = wahl.expect("Abzweig auf waagerechtem Teilstück");
    let roh = project.find_obj(bid).unwrap().1.raw.as_ref().unwrap().block.clone();
    let setzen = |p: &mut Project, pts: Vec<P>| {
        let s = p.sheet_mut(r).unwrap();
        if let Some(Kind::Wire { pts: z, .. }) = s.objects.iter_mut().find(|o| o.id == zid).map(|o| &mut o.kind) {
            *z = pts;
        }
    };
    // senkrecht wegziehen (wie Teilstück ziehen mit der Maus): Abzweig hängt frei, kein Punkt mehr
    let weg = segment_ziehen::verschieben(&orig, idx, P::new(0.0, 10.0));
    setzen(&mut project, weg);
    assert!(!hat_knoten(project.sheet(r).unwrap(), e), "Punkt verschwindet mit dem T-Abzweig");
    let b = epr_write::object_block(&project, project.find_obj(bid).unwrap().1).unwrap();
    assert_eq!(b.data[0x14], 0, "Anschlussart frei");
    assert_eq!(kennung(&b.data, 24), 0);
    assert_eq!(b.i32(8), 0);
    // zurück: Punkt wieder da, Block unverändert wie geladen
    setzen(&mut project, orig);
    assert!(hat_knoten(project.sheet(r).unwrap(), e), "Punkt entsteht wieder");
    assert_eq!(epr_write::object_block(&project, project.find_obj(bid).unwrap().1).unwrap().data, roh.data);
}

/// Neue Leitung auf das Innere einer anderen: Knoten (Art 3, Kennung der Leitung, Anschluss −1) nach der Rundreise;
/// neue Leitung von einer W2-Sammelschiene zu einem freien Punkt: senkrecht, Anfang Art 2 mit Kennung der Schiene.
#[test]
fn neue_leitung_auf_leitung_und_von_schiene() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    // Blatt mit Sammelschiene
    let (r, schiene) = project
        .anlagen
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| a.sheets.iter().enumerate().map(move |(si, s)| (SheetRef { anlage: ai, sheet: si }, s)))
        .find_map(|(r, s)| s.objects.iter().find(|o| matches!(o.kind, Kind::Busbar { .. })).map(|o| (r, o.clone())))
        .expect("Blatt mit Sammelschiene");
    let Kind::Busbar { a, x2, .. } = schiene.kind else { unreachable!() };
    let mut app = test_app();
    app.project = project;
    app.tabs = vec![r];
    app.active = 0;

    // Schiene → Ziel 40 mm darunter: Druck auf der Schiene beginnt eine Leitung
    let x = ((a.x + x2) / 2.0 / 5.0).round() * 5.0 + 0.3;
    assert!(app.leitung_an_schiene_beginnen(P::new(x - 2.0, a.y + 0.2), 1.0));
    let elektroplanung::app::Drag::Wire { f } = app.drag.clone() else { panic!("Leitung begonnen") };
    let ziel = P::new(x, a.y + 40.0);
    let pts = app.leitungsweg(&f, ziel);
    assert_eq!(pts, vec![P::new(x, a.y), ziel], "senkrecht von der Schiene ab");
    app.drag = elektroplanung::app::Drag::None;
    let w1 = app.add_obj(Kind::Wire { pts: pts.clone(), leitung: 0, bez: vec![], opts: 0 });
    assert!(hat_knoten(app.sheet().unwrap(), P::new(x, a.y)), "Punkt am Abgriff");
    // zweite Leitung von rechts waagerecht auf die Mitte der ersten
    let t = P::new(x, a.y + 20.0);
    let w2 = app.add_obj(Kind::Wire { pts: vec![P::new(x + 30.0, a.y + 20.0), t], leitung: 0, bez: vec![], opts: 0 });
    assert!(hat_knoten(app.sheet().unwrap(), t), "T-Abzweig hat einen Punkt");

    let p2 = rundreise(&app.project, "neu");
    let s2 = p2.sheet(r).unwrap();
    let finde = |anf: P, ende: P| s2.objects.iter().find(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts[0].dist(anf) < 0.05 && pts.last().unwrap().dist(ende) < 0.05)).and_then(|o| o.raw.as_deref()).map(|r| r.block.clone());
    let b1 = finde(P::new(x, a.y), ziel).expect("Leitung von der Schiene");
    let b2 = finde(P::new(x + 30.0, a.y + 20.0), t).expect("Abzweig");
    let sk = kennung(&s2.objects.iter().find(|o| matches!(o.kind, Kind::Busbar { .. }) && o.raw.as_deref().is_some_and(|r| r.kind == schiene.kind)).unwrap().raw.as_ref().unwrap().block.data, 0);
    assert_eq!(b1.data[0], knoten::ART_SCHIENE, "Anfang an der Schiene");
    assert_eq!(kennung(&b1.data, 8), sk);
    assert_eq!(b2.data[0x14], knoten::ART_LEITUNG, "Ende auf der Leitung");
    assert_eq!(kennung(&b2.data, 24), kennung(&b1.data, 40), "Kennung der getroffenen Leitung");
    assert_eq!(b2.i32(8), -1);
    let _ = (w1, w2);
}
