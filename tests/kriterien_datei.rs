//! Prüfung der Leitungskriterien an einer gespeicherten Datei gegen ihre Ausgangslage (Werkzeug für Maustests).
//!
//! Aufruf: `EP_PRUEF_VORHER=<vorher.epr> EP_PRUEF_NACHHER=<nachher.epr> cargo test --test kriterien_datei -- --nocapture`
//! Ohne die Variablen wird der Test übersprungen. Geprüft werden die geänderten Leitungen jedes Blattes:
//! - L1 kein neues freies Leitungsende,
//! - L2 rechtwinklig, keine doppelten Punkte,
//! - L3 kein Weg durch ein fremdes Symbol, über keinen fremden Anschluss,
//! - L4 keine Überdeckung mit Leitungen eines anderen Netzes,
//!
//! und als Kennzahlen L8 (Zahl geänderter Leitungen) und L13 (Knicke vorher/nachher). Harte Verstöße (L1–L4) lassen
//! den Test scheitern; der Bericht steht in der Ausgabe.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_anschluss::freies_ende;
use elektroplanung::model::*;

fn laden(pfad: &str) -> elektroplanung::app::App {
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(pfad), false);
    app
}

fn leitungen(s: &Sheet) -> Vec<(u64, Vec<P>)> {
    s.objects.iter().filter_map(|o| match &o.kind { Kind::Wire { pts, .. } if pts.len() >= 2 => Some((o.id, pts.clone())), _ => None }).collect()
}

fn knicke(w: &[P]) -> usize {
    w.len().saturating_sub(2)
}

fn rechtwinklig(w: &[P]) -> bool {
    w.windows(2).all(|v| (v[0].x - v[1].x).abs() < 0.01 || (v[0].y - v[1].y).abs() < 0.01)
}

/// Überdecken sich zwei achsparallele Strecken auf mehr als einem Punkt?
fn ueberdeckt(a: (P, P), b: (P, P)) -> bool {
    let waag = |s: (P, P)| (s.0.y - s.1.y).abs() < 0.01;
    let senk = |s: (P, P)| (s.0.x - s.1.x).abs() < 0.01;
    let ueber = |a0: f32, a1: f32, b0: f32, b1: f32| a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1)) > 0.05;
    if waag(a) && waag(b) && (a.0.y - b.0.y).abs() < 0.01 {
        return ueber(a.0.x, a.1.x, b.0.x, b.1.x);
    }
    if senk(a) && senk(b) && (a.0.x - b.0.x).abs() < 0.01 {
        return ueber(a.0.y, a.1.y, b.0.y, b.1.y);
    }
    false
}

/// Bericht für ein Blatt: (harte Verstöße, Hinweise).
pub fn pruefen(vorher: &Sheet, nachher: &Sheet, lib: &[elektroplanung::symbols::SymDef]) -> (Vec<String>, Vec<String>) {
    let (mut hart, mut info) = (vec![], vec![]);
    let alt = leitungen(vorher);
    let neu = leitungen(nachher);
    // nach Lage verglichen (die IDs vergibt das Laden neu; Richtung egal)
    let gleich = |a: &[P], b: &[P]| a.len() == b.len() && (a.iter().zip(b).all(|(x, y)| x.dist(*y) < 0.01) || a.iter().zip(b.iter().rev()).all(|(x, y)| x.dist(*y) < 0.01));
    let geaendert: Vec<&(u64, Vec<P>)> = neu.iter().filter(|(_, w)| !alt.iter().any(|(_, q)| gleich(q, w))).collect();
    let weg_l: Vec<&(u64, Vec<P>)> = alt.iter().filter(|(_, w)| !neu.iter().any(|(_, q)| gleich(q, w))).collect();
    let weg = weg_l.len();
    let (k0, k1): (usize, usize) = (weg_l.iter().map(|(_, w)| knicke(w)).sum(), geaendert.iter().map(|(_, w)| knicke(w)).sum());
    info.push(format!("L8: {} Leitungen neu/geändert, {weg} alte nicht mehr da; L13: Knicke {k0} → {k1}", geaendert.len()));
    // L1: freie Enden nach Lage
    let frei = |s: &Sheet| -> Vec<P> {
        leitungen(s).iter().flat_map(|(id, w)| [(*id, w[0]), (*id, w[w.len() - 1])]).filter(|(id, p)| freies_ende(s, lib, *id, *p)).map(|(_, p)| p).collect()
    };
    let (f0, f1) = (frei(vorher), frei(nachher));
    if f1.len() > f0.len() {
        let neu_frei: Vec<&P> = f1.iter().filter(|p| !f0.iter().any(|q| q.dist(**p) < 0.01)).collect();
        hart.push(format!("L1: {} freie Enden vorher, {} nachher; neu: {neu_frei:?}", f0.len(), f1.len()));
    }
    for (_, w) in &weg_l {
        info.push(format!("  weg: {w:?}"));
    }
    for (id, w) in &geaendert {
        info.push(format!("  neu {id}: {w:?}"));
    }
    let umr = elektroplanung::leitung_hindernis::umrisse(nachher, lib);
    let netze = elektroplanung::netz::netze(nachher, lib);
    let netz_von = |id: u64| netze.iter().position(|n| n.contains(&id));
    for (id, w) in &geaendert {
        // L2
        if !rechtwinklig(w) {
            hart.push(format!("L2: Leitung {id} schräg {w:?}"));
        }
        if w.windows(2).any(|v| v[0].dist(v[1]) < 0.001) {
            hart.push(format!("L2: Leitung {id} doppelter Punkt {w:?}"));
        }
        // L3: durch fremde Symbole (ohne die an den eigenen Enden) bzw. über fremde Anschlüsse
        let enden = [w[0], w[w.len() - 1]];
        let hind = elektroplanung::leitung_hindernis::hindernisse(&umr, &[], &enden);
        if elektroplanung::leitung_hindernis::kreuzungen(w, &hind) > 0 {
            hart.push(format!("L3: Leitung {id} durch ein fremdes Symbol {w:?}"));
        }
        for u in &umr {
            for q in &u.pins {
                if enden.iter().all(|e| e.dist(*q) > 0.05) && w.windows(2).any(|v| seg_dist(*q, v[0], v[1]) < 0.05) {
                    hart.push(format!("L3: Leitung {id} über fremden Anschluss {q:?} (Symbol {})", u.id));
                }
            }
        }
        // L4: Überdeckung mit Leitungen anderer Netze
        for (j, v) in &neu {
            if *j == *id || netz_von(*j).is_some() && netz_von(*j) == netz_von(*id) {
                continue;
            }
            if w.windows(2).any(|a| v.windows(2).any(|b| ueberdeckt((a[0], a[1]), (b[0], b[1])))) {
                hart.push(format!("L4: Leitung {id} überdeckt Leitung {j} (anderes Netz)"));
            }
        }
    }
    (hart, info)
}

#[test]
fn kriterien_der_gespeicherten_datei() {
    let (Ok(v), Ok(n)) = (std::env::var("EP_PRUEF_VORHER"), std::env::var("EP_PRUEF_NACHHER")) else { return };
    let a = laden(&v);
    let b = laden(&n);
    let mut alle = vec![];
    for (si, s1) in b.project.anlagen.iter().flat_map(|an| an.sheets.iter()).enumerate() {
        let Some(s0) = a.project.anlagen.iter().flat_map(|an| an.sheets.iter()).find(|s| s.number == s1.number) else { continue };
        let (hart, info) = pruefen(s0, s1, &b.lib);
        for i in &info {
            println!("Blatt {} ({si}): {i}", s1.number);
        }
        for h in &hart {
            println!("Blatt {}: VERSTOSS {h}", s1.number);
        }
        alle.extend(hart);
    }
    println!("{} Verstöße", alle.len());
    assert!(alle.is_empty());
}
