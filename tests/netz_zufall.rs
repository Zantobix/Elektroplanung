//! Runde 13: Zufällige Bearbeitungen auf W2 (Symbol/Klemme verschieben, Teilstück ziehen) – danach muss jedes
//! Leitungsende, das vorher angeschlossen war, weiter angeschlossen sein, und alle Leitungen bleiben rechtwinklig
//! ohne doppelte Punkte (netz_folgen.rs).
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_anschluss::freies_ende;
use elektroplanung::model::*;

struct Zufall(u64);
impl Zufall {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / ((1u64 << 31) as f32)
    }
}

fn enden(s: &Sheet) -> Vec<(u64, usize, P)> {
    s.objects
        .iter()
        .filter_map(|o| match &o.kind {
            Kind::Wire { pts, .. } if pts.len() >= 2 => Some([(o.id, 0, pts[0]), (o.id, 1, pts[pts.len() - 1])]),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn w2_zufaellige_bearbeitungen_halten_verbindungen() {
    let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    let mut z = Zufall(777);
    let (mut versuche, mut fehler) = (0, vec![]);
    let blaetter = app.tabs.len();
    for b in 0..blaetter {
        app.active = b;
        for _ in 0..30 {
            let Some(s) = app.sheet().cloned() else { break };
            let lib = app.lib.clone();
            let kand: Vec<&Obj> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Wire { .. })).collect();
            if kand.is_empty() {
                break;
            }
            let o = kand[(z.next() * kand.len() as f32) as usize % kand.len()].clone();
            let r = app.raster();
            let d = P::new(((z.next() * 6.0) as i32 - 3) as f32 * r, ((z.next() * 6.0) as i32 - 3) as f32 * r);
            if d.x == 0.0 && d.y == 0.0 {
                continue;
            }
            let vorher_ange: Vec<(u64, usize)> = enden(&s).into_iter().filter(|(id, _, p)| !freies_ende(&s, &lib, *id, *p)).map(|(id, k, _)| (id, k)).collect();
            let was;
            let art = z.next();
            match &o.kind {
                // ganze Leitung markiert verschieben: Enden bleiben an festen Anschlüssen
                Kind::Wire { .. } if art < 0.3 => {
                    app.checkpoint();
                    app.selection = vec![o.id];
                    app.netz_merken();
                    app.netz_verschieben(&[o.id], d);
                    was = format!("ganze Leitung {} um {d:?}", o.id);
                }
                // Symbol drehen
                Kind::Symbol { .. } if art < 0.25 => {
                    app.selection = vec![o.id];
                    app.transform_selection(1);
                    was = format!("Symbol {} gedreht", o.id);
                }
                Kind::Wire { pts, .. } => {
                    let i = (z.next() * (pts.len() - 1) as f32) as usize % (pts.len() - 1);
                    let m = P::new((pts[i].x + pts[i + 1].x) / 2.0, (pts[i].y + pts[i + 1].y) / 2.0);
                    app.selection = vec![o.id];
                    if !app.teilstueck_greifen(o.id, m, 0.3) {
                        continue;
                    }
                    app.teilstueck_ziehen(m.add(d));
                    app.drag = elektroplanung::app::Drag::None;
                    was = format!("Teilstück {i} der Leitung {} um {d:?}", o.id);
                }
                _ => {
                    app.checkpoint();
                    app.selection = vec![o.id];
                    app.netz_merken();
                    app.netz_verschieben(&[o.id], d);
                    was = format!("Objekt {} um {d:?}", o.id);
                }
            }
            app.aufraeumen();
            versuche += 1;
            let n = app.sheet().unwrap().clone();
            for (id, k) in &vorher_ange {
                let Some(Kind::Wire { pts, .. }) = n.objects.iter().find(|x| x.id == *id).map(|x| &x.kind) else { continue };
                let p = if *k == 0 { pts[0] } else { pts[pts.len() - 1] };
                if freies_ende(&n, &lib, *id, p) {
                    fehler.push(format!("Blatt {b}: {was}: Ende {k} der Leitung {id} bei {p:?} jetzt frei"));
                }
            }
            for x in &n.objects {
                if let Kind::Wire { pts, .. } = &x.kind {
                    // nur geänderte Leitungen (W2 enthält schon doppelte Punkte)
                    if s.objects.iter().find(|y| y.id == x.id).is_some_and(|y| matches!(&y.kind, Kind::Wire { pts: q, .. } if q == pts)) {
                        continue;
                    }
                    let war_gerade = s.objects.iter().find(|y| y.id == x.id).is_none_or(|y| matches!(&y.kind, Kind::Wire { pts: q, .. } if q.windows(2).all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01)));
                    if war_gerade && !pts.windows(2).all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01) {
                        fehler.push(format!("Blatt {b}: {was}: Leitung {} schräg {pts:?}", x.id));
                    }
                    if pts.windows(2).any(|w| w[0].dist(w[1]) < 0.001) {
                        fehler.push(format!("Blatt {b}: {was}: Leitung {} doppelter Punkt", x.id));
                    }
                }
            }
            app.do_undo();
        }
    }
    for f in fehler.iter().take(25) {
        println!("{f}");
    }
    println!("{versuche} Bearbeitungen, {} Befunde", fehler.len());
    assert!(fehler.is_empty());
}

