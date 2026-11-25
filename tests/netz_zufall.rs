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


/// Knoten der elektrischen Netze eines Blattes: Symbolanschlüsse als (Symbol, Anschlussnummer), Klemmstellen,
/// Schienen und Verweise als (Objekt, u32::MAX); je Knoten die Nummer seines Netzes (T2, todo N10).
fn netz_knoten(s: &Sheet, lib: &[elektroplanung::symbols::SymDef]) -> std::collections::HashMap<(u64, u32), usize> {
    let mut out = std::collections::HashMap::new();
    for (ni, netz) in elektroplanung::netz::netze(s, lib).iter().enumerate() {
        for o in s.objects.iter().filter(|o| netz.contains(&o.id) && !matches!(o.kind, Kind::Wire { .. })) {
            out.insert((o.id, u32::MAX), ni);
        }
        for (sym, q) in elektroplanung::netz::anschluesse(s, lib, netz) {
            if let Some(o) = s.objects.iter().find(|o| o.id == sym) {
                if let Some(k) = elektroplanung::canvas::obj_pins(o, lib).iter().position(|a| a.dist(q) < 0.01) {
                    out.insert((sym, k as u32), ni);
                }
            }
        }
    }
    out
}

/// Trennungen und Kurzschlüsse zwischen zwei Netzeinteilungen, ohne Knoten des Objekts `ohne`.
fn netz_aenderungen(vorher: &std::collections::HashMap<(u64, u32), usize>, nachher: &std::collections::HashMap<(u64, u32), usize>, ohne: u64) -> (usize, usize) {
    use std::collections::{HashMap, HashSet};
    let mut v2n: HashMap<usize, HashSet<usize>> = HashMap::new();
    let mut n2v: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (k, a) in vorher {
        if k.0 == ohne {
            continue;
        }
        if let Some(b) = nachher.get(k) {
            v2n.entry(*a).or_default().insert(*b);
            n2v.entry(*b).or_default().insert(*a);
        }
    }
    (v2n.values().filter(|s| s.len() > 1).count(), n2v.values().filter(|s| s.len() > 1).count())
}


/// Diagnose (N00): direkte Berührungen im Blatt `n` zwischen Objekten, die in `s` zu verschiedenen Netzen gehörten
/// (Leitungsende an Leitung/Schiene/Klemme/Verweis bzw. zwei Leitungsenden am selben Symbolanschluss).
fn neue_bruecken(s: &Sheet, n: &Sheet, lib: &[elektroplanung::symbols::SymDef]) -> Vec<String> {
    use elektroplanung::netz::netze;
    let alt: std::collections::HashMap<u64, usize> = netze(s, lib).iter().enumerate().flat_map(|(i, v)| v.iter().map(move |id| (*id, i))).collect();
    let pins: Vec<(u64, P)> = n.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| elektroplanung::canvas::obj_pins(o, lib).into_iter().map(move |q| (o.id, q))).collect();
    let mut out = vec![];
    let enden: Vec<(u64, P)> = n.objects.iter().filter_map(|o| match &o.kind { Kind::Wire { pts, .. } if pts.len() >= 2 => Some([(o.id, pts[0]), (o.id, pts[pts.len() - 1])]), _ => None }).flatten().collect();
    for &(i, e) in &enden {
        let Some(ai) = alt.get(&i) else { continue };
        for o in &n.objects {
            if o.id == i { continue; }
            let Some(aj) = alt.get(&o.id) else { continue };
            if ai == aj { continue; }
            let grund = match &o.kind {
                Kind::Wire { pts, .. } => {
                    if pts[0].dist(e) < 0.05 || pts[pts.len() - 1].dist(e) < 0.05 { Some("Ende an Ende") }
                    else if pts.windows(2).any(|w| seg_dist(e, w[0], w[1]) < 0.05) { Some("Ende im Innern (T)") } else { None }
                }
                Kind::Busbar { a, x2, .. } => ((e.y - a.y).abs() < 0.05 && e.x >= a.x.min(*x2) - 0.05 && e.x <= a.x.max(*x2) + 0.05).then_some("Ende auf Schiene"),
                Kind::Terminal { pos, .. } => (pos.dist(e) <= 0.85).then_some("Ende an Klemmstelle"),
                Kind::Verweis { pos, .. } => (pos.dist(e) < 0.05).then_some("Ende an Verweis"),
                _ => None,
            };
            if let Some(g) = grund {
                out.push(format!("    {g}: Leitung {i} bei {e:?} ↔ {} {}", o.id, match &o.kind { Kind::Wire { pts, .. } => format!("{pts:?}"), _ => String::new() }));
            }
        }
        for (sym, q) in pins.iter().filter(|(_, q)| q.dist(e) <= 0.05) {
            for &(j, f) in &enden {
                if j != i && f.dist(*q) <= 0.05 && alt.get(&j).is_some_and(|aj| aj != ai) && i < j {
                    out.push(format!("    zwei Enden am Anschluss von Symbol {sym} bei {q:?}: Leitung {i} ↔ {j}"));
                }
            }
        }
    }
    out
}

/// Ergebnis eines Zufallslaufs: Bearbeitungen, Befunde (freie Enden, schräg, doppelte Punkte), Netzänderungen,
/// abgewiesene Schritte, mitgeführte Leitungen über fremden Anschlüssen.
struct Lauf {
    fehler: usize,
    netz: usize,
    abgewiesen: usize,
    ueber: usize,
}

fn zufall_lauf(f: std::ffi::OsString, seed: u64) -> Lauf {
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    let mut z = Zufall(seed);
    let (mut versuche, mut fehler) = (0, vec![]);
    let mut ueber_anschluss: Vec<String> = vec![];
    let mut netzfehler: Vec<String> = vec![];
    let mut abgewiesen: std::collections::BTreeMap<String, usize> = Default::default();
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
            let netz_vorher = netz_knoten(&s, &lib);
            let abgew_vorher = app.netz_abgewiesen;
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
            if app.netz_abgewiesen > abgew_vorher {
                *abgewiesen.entry(format!("{} – {}", was.split(' ').next().unwrap_or(""), app.info)).or_insert(0) += 1;
                if std::env::var("NETZ_DETAIL").is_ok() {
                    println!("(abgewiesen-einzeln) Blatt {b}: {was}: {}", app.info);
                }
            }
            let n = app.sheet().unwrap().clone();
            // T2 Netz-Invariante: außerhalb des bearbeiteten Objekts dieselben elektrischen Netze
            let (getrennt, verbunden) = netz_aenderungen(&netz_vorher, &netz_knoten(&n, &lib), o.id);
            if getrennt + verbunden > 0 {
                let mut t = format!("Blatt {b}: {was}: {getrennt} Netze getrennt, {verbunden} Netze verbunden");
                if std::env::var("NETZ_DETAIL").is_ok() {
                    for z in neue_bruecken(&s, &n, &lib) {
                        t.push('\n');
                        t.push_str(&z);
                    }
                }
                netzfehler.push(t);
            }
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
    for f in netzfehler.iter().take(if std::env::var("NETZ_DETAIL").is_ok() { 1000 } else { 20 }) {
        println!("(Netz) {f}");
    }
    println!("{} Bearbeitungen mit geänderten Netzen, {} Schritte vom Netz-Wächter abgewiesen", netzfehler.len(), app.netz_abgewiesen);
    for (k, v) in &abgewiesen {
        println!("(abgewiesen) {v} × {k}");
    }
    // Symbole übereinander (S02, gewollte Abweisung) getrennt zählen: die Kennzahl betrifft nur Netze
    let auf_symbol: usize = abgewiesen.iter().filter(|(k, _)| k.contains("Symbol auf ein anderes")).map(|(_, v)| *v).sum();
    println!("davon {auf_symbol} wegen Symbol auf Symbol (S02)");
    Lauf { fehler: fehler.len(), netz: netzfehler.len(), abgewiesen: app.netz_abgewiesen - auf_symbol, ueber: ueber_anschluss.len() }
}

#[test]
fn w2_zufaellige_bearbeitungen_halten_verbindungen() {
    let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
    let l = zufall_lauf(f, 777);
    // T2 Netz-Invariante, hart seit 24.11.2026 (erste Messung 150 von 733 verbanden fremde Netze; Netz-Wächter
    // netz_waechter.rs, Vereinigen nur im selben Netz, Gleis-Wandern nur auf dem eigenen Gleis, Querstück mit Stich)
    assert_eq!(l.netz, 0, "Bearbeitungen mit geänderten Netzen");
    // Kennzahl: vom Netz-Wächter abgewiesene Schritte (meist Zufallsziel auf einer fremden Leitung) – darf nur sinken;
    // ohne die Abweisungen „Symbol auf Symbol“ (S02, seit 24.11.2026 abends, gewollt)
    // 25.11.2026: 123 → 117 (ganze Leitung: Ausweichform mit Stich am festen Anschluss, NR5)
    assert!(l.abgewiesen <= 117, "{} Schritte abgewiesen", l.abgewiesen);
    assert_eq!(l.fehler, 0);
    // Kennzahl (Runde 13): übrig sind Fälle, in denen die Zufallsverschiebung ein Objekt auf fremde Leitungen bzw.
    // Anschlüsse setzt und es keinen sauberen Weg mehr gibt
    assert!(l.ueber <= 1, "{} mitgeführte Leitungen über fremden Anschlüssen", l.ueber);
}

/// Dasselbe auf dem mit der Maus nachgezeichneten Plan (todo N0: Netz-Invariante je nachgezeichnetem Blatt);
/// private Datei `EP_FIXTURES/nachzeichnen/nachgezeichnet_2026-11-24c.epr`, ohne sie übersprungen.
#[test]
fn nachgezeichnet_zufaellige_bearbeitungen_halten_netze() {
    let Some(dir) = std::env::var_os("EP_FIXTURES") else { return };
    let f = std::path::PathBuf::from(dir).join("nachzeichnen/nachgezeichnet_2026-11-24c.epr");
    if !f.exists() {
        return;
    }
    let l = zufall_lauf(f.into_os_string(), 4711);
    assert_eq!(l.netz, 0, "Bearbeitungen mit geänderten Netzen");
    assert_eq!(l.fehler, 0);
    println!("nachgezeichnet: {} abgewiesen, {} über fremden Anschlüssen", l.abgewiesen, l.ueber);
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
            let abgew = app.netz_abgewiesen;
            geste(&mut app, id, d);
            if app.netz_abgewiesen > abgew {
                // Hinweg vom Netz-Wächter abgewiesen (netz_waechter.rs): das Objekt steht noch, kein Rückweg
                app.do_undo();
                continue;
            }
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
