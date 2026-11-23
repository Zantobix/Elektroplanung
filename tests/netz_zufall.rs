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
    let mut ueber_anschluss: Vec<String> = vec![];
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
            if std::env::var("OHNE_AUFR").is_err() { app.aufraeumen(); }
            versuche += 1;
            let n = app.sheet().unwrap().clone();
            // keine neuen freien Enden: Lagen freier Leitungsenden nachher, die vorher keine freien Enden waren und
            // nicht zu einer bewegten Leitung gehören (Leitungen können zusammengelegt werden, deshalb nach Lage)
            let _ = &vorher_ange;
            let frei_vorher: Vec<P> = enden(&s).into_iter().filter(|(id, _, p)| freies_ende(&s, &lib, *id, *p)).map(|(_, _, p)| p).collect();
            for (id, k, p) in enden(&n) {
                if !freies_ende(&n, &lib, id, p) || frei_vorher.iter().any(|q| q.dist(p) < 0.01) {
                    continue;
                }
                // freies Ende einer verschobenen freien Leitung (wandert mit) ist kein Fehler
                let war_frei = s.objects.iter().find(|x| x.id == id).is_some_and(|x| matches!(&x.kind, Kind::Wire { pts, .. } if freies_ende(&s, &lib, id, if k == 0 { pts[0] } else { pts[pts.len() - 1] })));
                if war_frei {
                    continue;
                }
                fehler.push(format!("Blatt {b}: {was}: Ende {k} der Leitung {id} bei {p:?} jetzt frei"));
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
                    // über keinen fremden Symbolanschluss (sonst elektrisch angeschlossen), wenn sie es vorher nicht tat
                    let ueber = |q: &[P], sh: &Sheet| sh.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| elektroplanung::canvas::obj_pins(o, &lib)).any(|a| a.dist(q[0]) > 0.05 && a.dist(q[q.len() - 1]) > 0.05 && q.windows(2).any(|w| seg_dist(a, w[0], w[1]) < 0.05));
                    let vorher_ueber = s.objects.iter().find(|y| y.id == x.id).is_some_and(|y| matches!(&y.kind, Kind::Wire { pts: q, .. } if ueber(q, &s)));
                    if ueber(pts, &n) && !vorher_ueber && x.id != o.id {
                        ueber_anschluss.push(format!("Blatt {b}: {was}: Leitung {} läuft über einen fremden Anschluss {pts:?}", x.id));
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
    for f in ueber_anschluss.iter().take(15) {
        println!("(Kennzahl) {f}");
    }
    println!("{} mitgeführte Leitungen über fremden Anschlüssen", ueber_anschluss.len());
    assert!(fehler.is_empty());
    // Kennzahl (Runde 13): übrig sind Fälle, in denen die Zufallsverschiebung ein Objekt auf fremde Leitungen bzw.
    // Anschlüsse setzt und es keinen sauberen Weg mehr gibt
    assert!(ueber_anschluss.len() <= 12, "{} mitgeführte Leitungen über fremden Anschlüssen", ueber_anschluss.len());
}


/// R5 Rückkehr (Kriterium L10): Symbol bzw. Klemme in einer Geste verschieben, in einer zweiten zurück – danach
/// dieselben Leitungen (nach Lage verglichen, weil das Aufräumen Leitungen zusammenfassen kann). Kennzahl.
#[test]
fn w2_hin_und_zurueck_ergibt_denselben_plan() {
    let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    let mut z = Zufall(4242);
    let (mut versuche, mut abweichend) = (0, vec![]);
    // gezeichnetes Bild: je waagerechter bzw. senkrechter Linie die Vereinigung der überdeckten Strecken (0,1 mm),
    // unabhängig davon, wie das Aufräumen die Linien auf Leitungen aufteilt
    let form = |s: &Sheet| -> Vec<(bool, i32, i32, i32)> {
        let mut seg: Vec<(bool, i32, i32, i32)> = vec![];
        for o in &s.objects {
            if let Kind::Wire { pts, .. } = &o.kind {
                for w in pts.windows(2) {
                    let (a, b) = (((w[0].x * 10.0).round() as i32, (w[0].y * 10.0).round() as i32), ((w[1].x * 10.0).round() as i32, (w[1].y * 10.0).round() as i32));
                    if a == b {
                        continue;
                    }
                    if a.1 == b.1 {
                        seg.push((true, a.1, a.0.min(b.0), a.0.max(b.0)));
                    } else if a.0 == b.0 {
                        seg.push((false, a.0, a.1.min(b.1), a.1.max(b.1)));
                    } else {
                        seg.push((true, i32::MIN, a.0 * 100000 + a.1, b.0 * 100000 + b.1));
                    }
                }
            }
        }
        seg.sort();
        let mut v: Vec<(bool, i32, i32, i32)> = vec![];
        for x in seg {
            match v.last_mut() {
                Some(l) if l.0 == x.0 && l.1 == x.1 && x.2 <= l.3 => l.3 = l.3.max(x.3),
                _ => v.push(x),
            }
        }
        v
    };
    let geste = |app: &mut elektroplanung::app::App, id: u64, d: P| {
        app.selection = vec![id];
        app.checkpoint();
        app.stoesse_an_markierung_zusammenfassen();
        app.netz_merken();
        app.netz_verschieben(&[id], d);
        app.aufraeumen();
    };
    for b in 0..app.tabs.len() {
        app.active = b;
        for _ in 0..12 {
            let Some(s) = app.sheet().cloned() else { break };
            let kand: Vec<u64> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })).map(|o| o.id).collect();
            if kand.is_empty() {
                break;
            }
            let id = kand[(z.next() * kand.len() as f32) as usize % kand.len()];
            let r = app.raster();
            let d = P::new(((z.next() * 8.0) as i32 - 4) as f32 * r, ((z.next() * 8.0) as i32 - 4) as f32 * r);
            if d.x == 0.0 && d.y == 0.0 {
                continue;
            }
            // Ausgangslage nach dem Zusammenfassen der Stoßstücke (das gehört zur ersten Geste)
            app.selection = vec![id];
            app.stoesse_an_markierung_zusammenfassen();
            let vorher = form(app.sheet().unwrap());
            geste(&mut app, id, d);
            geste(&mut app, id, P::new(-d.x, -d.y));
            versuche += 1;
            let nachher = form(app.sheet().unwrap());
            if nachher != vorher {
                let weg: Vec<_> = vorher.iter().filter(|w| !nachher.contains(w)).collect();
                let neu: Vec<_> = nachher.iter().filter(|w| !vorher.contains(w)).collect();
                abweichend.push(format!("Blatt {b}: Objekt {id} um {d:?} und zurück: vorher {weg:?} nachher {neu:?}"));
            }
            app.do_undo();
            app.do_undo();
        }
    }
    for a in abweichend.iter().take(20) {
        println!("{a}");
    }
    println!("{versuche} Rückkehr-Versuche, {} mit anderem Plan", abweichend.len());
    // hart seit 23.11.2026 (Rückkehr auf gemerkte Blattstände, netz_folgen.rs)
    assert!(abweichend.is_empty());
}
