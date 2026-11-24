//! NR1 (24.11.2026): Netz-Invariante beim Zeichnen neuer Leitungen. Auf den Blättern von W2 (und der mit der Maus
//! nachgezeichneten Datei) werden zufällig Leitungen gezogen wie in der Oberfläche (Mausweg mit Knie,
//! `leitungsende` → `leitungsweg` → `leitung_anschliessen`): Start an einem Symbolanschluss, Ziel ein fremder
//! Anschluss, ein Punkt auf einer Leitung oder eine freie Stelle. Danach dürfen nur das Netz des Anfangs und das Netz
//! des gewollten Ziels zusammengekommen sein – jede weitere Verbindung (fremder Anschluss, fremde Leitung, Schiene)
//! ist ein Befund, ebenso ein Weg über einen fremden (auch unbelegten) Anschluss.
//! Diagnose: `ZEICHNEN_DETAIL=1 cargo test --release --test zeichnen_netze -- --nocapture`.
use std::collections::{HashMap, HashSet};

use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_fuehrung::Fuehrung;
use elektroplanung::model::*;
use elektroplanung::netz_waechter::knoten;

struct Zufall(u64);
impl Zufall {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / ((1u64 << 31) as f32)
    }
    fn bis(&mut self, n: usize) -> usize {
        (self.next() * n as f32) as usize % n.max(1)
    }
}

/// Leitung mit der Maus über `ecken` ziehen (wie die Oberfläche bei 3,24 px/mm) und anlegen. Liefert den Weg.
fn ziehen(app: &mut App, start: P, ecken: &[P]) -> Option<Vec<P>> {
    let z = 3.24;
    app.drag = Drag::Wire { f: Fuehrung::neu(start) };
    let mut vorher = start;
    for e in ecken {
        let n = (vorher.dist(*e) / 1.2).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            app.leitung_zug_bewegen(P::new(vorher.x + (e.x - vorher.x) * t, vorher.y + (e.y - vorher.y) * t), z);
        }
        vorher = *e;
    }
    let Drag::Wire { f } = std::mem::replace(&mut app.drag, Drag::None) else { return None };
    let ende = app.leitungsende(&f, vorher, z);
    if ende.dist(f.start()) < 0.01 {
        return None;
    }
    let w = app.leitungsweg(&f, ende);
    app.leitung_anschliessen(w.clone())?;
    Some(w)
}

enum Ziel {
    Anschluss(u64, u32, P),
    Leitung(u64, P),
    Frei(P),
}

pub struct Ergebnis {
    pub versuche: usize,
    pub fremd: Vec<String>,
    pub ueber_anschluss: Vec<String>,
    /// übersprungen: Anfang bzw. Ziel liegt im Innern einer Leitung
    pub mehrdeutig: usize,
}

fn pins(s: &Sheet, app: &App) -> Vec<(u64, u32, P)> {
    s.objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }))
        .flat_map(|o| elektroplanung::canvas::obj_pins(o, &app.lib).into_iter().enumerate().map(move |(k, q)| (o.id, k as u32, q)))
        .collect()
}

pub fn lauf(datei: std::path::PathBuf, seed: u64, je_blatt: usize) -> Ergebnis {
    let mut app = test_app();
    app.open_path(datei, false);
    let mut z = Zufall(seed);
    let mut erg = Ergebnis { versuche: 0, fremd: vec![], ueber_anschluss: vec![], mehrdeutig: 0 };
    let raster = [1usize, 2, 5]; // 2,5 / 5 / 10 mm
    for b in 0..app.tabs.len() {
        app.active = b;
        for _ in 0..je_blatt {
            let Some(s) = app.sheet().cloned() else { break };
            let alle = pins(&s, &app);
            if alle.len() < 2 {
                break;
            }
            app.raster_idx = raster[z.bis(raster.len())];
            let (sym_a, k_a, a) = alle[z.bis(alle.len())];
            // Anschluss, über den schon eine fremde Leitung hinwegläuft (Datenlage, z. B. nachgezeichnete Datei
            // Blatt 4: Leitung 150 über -X…:1): jede dort beginnende Leitung verbindet beide – nicht messbar
            let innen = |q: P| s.objects.iter().any(|o| matches!(&o.kind, Kind::Wire { pts, .. } if elektroplanung::knoten::im_innern(pts, q)));
            if innen(a) {
                erg.mehrdeutig += 1;
                continue;
            }
            let leitungen: Vec<(u64, Vec<P>)> = s.objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { (pts.len() >= 2).then(|| (o.id, pts.clone())) } else { None }).collect();
            let art = z.next();
            let ziel = if art < 0.45 {
                let (sb, kb, q) = alle[z.bis(alle.len())];
                if sb == sym_a {
                    continue;
                }
                if innen(q) {
                    erg.mehrdeutig += 1;
                    continue;
                }
                Ziel::Anschluss(sb, kb, q)
            } else if art < 0.75 && !leitungen.is_empty() {
                let (wid, pts) = &leitungen[z.bis(leitungen.len())];
                let i = z.bis(pts.len() - 1);
                let (u, v) = (pts[i], pts[i + 1]);
                if u.dist(v) < 6.0 || !((u.x - v.x).abs() < 0.01 || (u.y - v.y).abs() < 0.01) {
                    continue;
                }
                let t = 0.25 + z.next() * 0.5;
                let q = P::new(u.x + (v.x - u.x) * t, u.y + (v.y - u.y) * t);
                // Maus knapp (< 2 mm) neben einer anderen Leitung oder Schiene (meist an einer Kreuzung): nicht zu
                // entscheiden, welche gemeint ist
                let andere = s.objects.iter().any(|o| match &o.kind {
                    Kind::Wire { pts, .. } => o.id != *wid && pts.windows(2).any(|w| seg_dist(q, w[0], w[1]) < 2.0),
                    Kind::Busbar { a, x2, .. } => seg_dist(q, *a, P::new(*x2, a.y)) < 2.0,
                    _ => false,
                });
                if andere {
                    erg.mehrdeutig += 1;
                    continue;
                }
                Ziel::Leitung(*wid, q)
            } else {
                // freie Stelle: mindestens 4 mm von Anschlüssen und Leitungen
                let q = P::new(20.0 + z.next() * 250.0, 30.0 + z.next() * 150.0);
                let q = app.snap(q);
                // auch Schienen und Klemmstellen (sonst fängt sich das Ende gewollt auf eine Schiene daneben)
                let sonst = s.objects.iter().any(|o| match &o.kind {
                    Kind::Busbar { a, x2, .. } => seg_dist(q, *a, P::new(*x2, a.y)) < 4.0,
                    Kind::Terminal { pos, .. } => pos.dist(q) < 4.0,
                    _ => false,
                });
                let nah = sonst || alle.iter().any(|(_, _, p)| p.dist(q) < 4.0) || leitungen.iter().any(|(_, pts)| pts.windows(2).any(|w| seg_dist(q, w[0], w[1]) < 4.0));
                if nah {
                    continue;
                }
                Ziel::Frei(q)
            };
            let ende = match &ziel {
                Ziel::Anschluss(_, _, q) | Ziel::Leitung(_, q) | Ziel::Frei(q) => *q,
            };
            if ende.dist(a) < 3.0 {
                continue;
            }
            // Mausweg: senkrecht/waagerecht mit zufälligem Knie
            let knie = if z.next() < 0.5 { P::new(a.x, a.y + (ende.y - a.y) * z.next()) } else { P::new(a.x + (ende.x - a.x) * z.next(), a.y) };
            let ecken = if z.next() < 0.5 { vec![knie, P::new(ende.x, knie.y), ende] } else { vec![P::new(knie.x, ende.y), ende] };
            let vorher = s.clone();
            let kv = knoten(&vorher, &app.lib, &[]);
            let undo_vor = app.undo.len();
            let Some(weg) = ziehen(&mut app, a, &ecken) else { continue };
            erg.versuche += 1;
            let n = app.sheet().unwrap().clone();
            let kn = knoten(&n, &app.lib, &[]);
            // erlaubte alte Netze: das des Anfangs-Anschlusses und das des gewollten Ziels
            let mut erlaubt: HashSet<usize> = HashSet::new();
            let mut erlaubte_knoten: HashSet<(u64, u32)> = HashSet::from([(sym_a, k_a)]);
            if let Some(x) = kv.get(&(sym_a, k_a)) {
                erlaubt.insert(*x);
            }
            match &ziel {
                Ziel::Anschluss(sb, kb, _) => {
                    erlaubte_knoten.insert((*sb, *kb));
                    if let Some(x) = kv.get(&(*sb, *kb)) {
                        erlaubt.insert(*x);
                    }
                }
                Ziel::Leitung(wid, _) => {
                    let netz = elektroplanung::netz::netz_von(&vorher, &app.lib, *wid);
                    for ((id, k), x) in &kv {
                        let _ = k;
                        if netz.contains(id) {
                            erlaubt.insert(*x);
                        }
                    }
                    // Anschlüsse am Netz der Zielleitung (über ihre Enden)
                    for (sym, q) in elektroplanung::netz::anschluesse(&vorher, &app.lib, &netz) {
                        if let Some((_, k, _)) = pins(&vorher, &app).into_iter().find(|(id, _, p)| *id == sym && p.dist(q) < 0.01) {
                            if let Some(x) = kv.get(&(sym, k)) {
                                erlaubt.insert(*x);
                            }
                        }
                    }
                }
                Ziel::Frei(_) => {}
            }
            // je neuem Netz: alte Netze seiner Knoten bzw. neu angeschlossene Knoten
            let mut gruppen: HashMap<usize, Vec<(u64, u32)>> = HashMap::new();
            for (k, x) in &kn {
                gruppen.entry(*x).or_default().push(*k);
            }
            let ziel_text = match &ziel {
                Ziel::Anschluss(sb, kb, q) => format!("Anschluss {sb}:{kb} {q:?}"),
                Ziel::Leitung(w, q) => format!("Leitung {w} bei {q:?}"),
                Ziel::Frei(q) => format!("frei {q:?}"),
            };
            for g in gruppen.values() {
                let alt: HashSet<usize> = g.iter().filter_map(|k| kv.get(k).copied()).collect();
                let neu_dazu: Vec<&(u64, u32)> = g.iter().filter(|k| !kv.contains_key(k) && !erlaubte_knoten.contains(k)).collect();
                let fremde_alt: Vec<&usize> = alt.iter().filter(|x| !erlaubt.contains(x)).collect();
                let beruehrt_neu = g.iter().any(|k| erlaubte_knoten.contains(k)) || alt.iter().any(|x| erlaubt.contains(x));
                if beruehrt_neu && (!fremde_alt.is_empty() || !neu_dazu.is_empty()) && (alt.len() + neu_dazu.len() > 1) {
                    let fremde_knoten: Vec<&(u64, u32)> = g.iter().filter(|k| kv.get(k).is_some_and(|x| !erlaubt.contains(x))).take(4).collect();
                    erg.fremd.push(format!(
                        "Blatt {b} Raster {}: von {sym_a}:{k_a} {a:?} nach {ziel_text}, Weg {weg:?}: fremde Netze {} {fremde_knoten:?} / neu angeschlossen {:?}",
                        app.raster(),
                        fremde_alt.len(),
                        neu_dazu
                    ));
                    break;
                }
            }
            // Weg über fremden Anschluss (auch unbelegt)
            let ueber: Vec<P> = pins(&n, &app).into_iter().map(|(_, _, q)| q).filter(|q| q.dist(weg[0]) > 0.05 && q.dist(weg[weg.len() - 1]) > 0.05 && weg.windows(2).any(|w| seg_dist(*q, w[0], w[1]) < 0.05)).collect();
            if !ueber.is_empty() {
                erg.ueber_anschluss.push(format!("Blatt {b}: von {a:?} nach {ziel_text}: Weg {weg:?} über {ueber:?}"));
            }
            if app.undo.len() > undo_vor {
                app.do_undo();
            }
            if app.sheet().unwrap().objects != vorher.objects {
                app.with_sheet(|s, _| s.objects = vorher.objects.clone());
            }
        }
    }
    erg
}

fn bericht(name: &str, e: &Ergebnis) {
    let detail = std::env::var("ZEICHNEN_DETAIL").is_ok();
    for f in e.fremd.iter().take(if detail { 1000 } else { 10 }) {
        println!("(fremd) {f}");
    }
    for f in e.ueber_anschluss.iter().take(if detail { 1000 } else { 10 }) {
        println!("(über) {f}");
    }
    println!("{name}: {} Leitungen gezeichnet, {} verbinden fremde Netze, {} laufen über fremde Anschlüsse ({} Anfänge/Ziele auf einer Leitung übersprungen)", e.versuche, e.fremd.len(), e.ueber_anschluss.len(), e.mehrdeutig);
}

#[test]
fn w2_gezeichnete_leitungen_verbinden_nur_ihr_ziel() {
    let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
    let seed = std::env::var("ZEICHNEN_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(2411);
    let e = lauf(std::path::PathBuf::from(f), seed, 12);
    bericht("W2", &e);
    // erste Messung 24.11.2026: 16 von 207 verbinden fremde Netze, 12 über fremde Anschlüsse; seit `beruehrt_fremd`,
    // Rastersuche (leitung_suche.rs) und eindeutigem Ende hart 0
    assert!(e.fremd.is_empty(), "{} verbinden fremde Netze", e.fremd.len());
    assert!(e.ueber_anschluss.is_empty(), "{} über fremde Anschlüsse", e.ueber_anschluss.len());
}

#[test]
fn nachgezeichnet_gezeichnete_leitungen_verbinden_nur_ihr_ziel() {
    let Some(dir) = std::env::var_os("EP_FIXTURES") else { return };
    let f = std::path::PathBuf::from(dir).join("nachzeichnen/nachgezeichnet_2026-11-24c.epr");
    if !f.exists() {
        return;
    }
    let seed = std::env::var("ZEICHNEN_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(99);
    let e = lauf(f, seed, 25);
    bericht("nachgezeichnet", &e);
    // erste Messung 24.11.2026: 3 von 63 bzw. 1 über fremden Anschluss; jetzt hart 0
    assert!(e.fremd.is_empty() && e.ueber_anschluss.is_empty());
}
