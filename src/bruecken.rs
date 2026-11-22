//! Brückenbögen an Kreuzungen von Leitungen – **bewusste Erweiterung gegenüber dem Original** (das Original
//! zeichnet sich kreuzende Leitungen einfach übereinander). Kreuzen sich zwei Leitungen, ohne verbunden zu sein,
//! springt die waagerechtere der beiden in einem kleinen Halbkreis über die andere. Verbindungen (ein Leitungsende
//! auf einer anderen Leitung, Verbindungspunkt) sind keine Kreuzung und bekommen keinen Bogen.
//!
//! Abschaltbar unter Extras › Optionen › Programmverhalten („Brückenbögen an Kreuzungen“). Die Datei bleibt
//! unverändert: Die Bögen entstehen nur beim Zeichnen (Bildschirm, Druck, PDF, Bildexport).

use crate::model::{Kind, Obj, P};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Halbmesser des Bogens in mm.
pub const R: f32 = 1.0;

/// Bogen-Mittelpunkte je Leitung (Objekt-ID → Kreuzungspunkte auf dieser Leitung).
pub type Karte = HashMap<u64, Vec<P>>;

static AN: AtomicBool = AtomicBool::new(true);

/// Schaltet die Bögen ein/aus (aus den Optionen, jedes Bild neu gesetzt).
pub fn setzen(an: bool) {
    AN.store(an, Ordering::Relaxed);
}

pub fn an() -> bool {
    AN.load(Ordering::Relaxed)
}

/// Schnittpunkt zweier Strecken im Innern beider (mit Abstand `rand` zu allen Endpunkten), sonst `None`.
/// Parallele bzw. aufeinanderliegende Strecken kreuzen sich nicht.
fn schnitt(a0: P, a1: P, b0: P, b1: P, rand: f32) -> Option<P> {
    let (dax, day) = (a1.x - a0.x, a1.y - a0.y);
    let (dbx, dby) = (b1.x - b0.x, b1.y - b0.y);
    let den = dax * dby - day * dbx;
    let (la, lb) = ((dax * dax + day * day).sqrt(), (dbx * dbx + dby * dby).sqrt());
    if la < 1e-4 || lb < 1e-4 || den.abs() < 1e-6 * la * lb {
        return None;
    }
    let (ex, ey) = (b0.x - a0.x, b0.y - a0.y);
    let t = (ex * dby - ey * dbx) / den;
    let u = (ex * day - ey * dax) / den;
    let (ra, rb) = (rand / la, rand / lb);
    if t <= ra || t >= 1.0 - ra || u <= rb || u >= 1.0 - rb {
        return None;
    }
    Some(P::new(a0.x + t * dax, a0.y + t * day))
}

/// Waagerechtheit einer Strecke (|dx| / Länge), größer = waagerechter.
fn waagerecht(a: P, b: P) -> f32 {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    dx / (dx * dx + dy * dy).sqrt().max(1e-6)
}

/// Kreuzungen aller Leitungen eines Blattes: Welche Leitung an welchen Stellen einen Bogen zeichnet.
/// Leer, wenn die Bögen abgeschaltet sind.
pub fn karte(objekte: &[Obj]) -> Karte {
    karte_mit(objekte, an())
}

fn karte_mit(objekte: &[Obj], an: bool) -> Karte {
    let mut k = Karte::new();
    if !an {
        return k;
    }
    let wires: Vec<(u64, &[P])> = objekte
        .iter()
        .filter_map(|o| match &o.kind {
            Kind::Wire { pts, .. } if pts.len() >= 2 => Some((o.id, pts.as_slice())),
            _ => None,
        })
        .collect();
    // Leitungsenden und Klemmstellen: Liegt eine davon im Kreuzungspunkt, ist dort eine Verbindung (Knoten)
    let mut punkte: Vec<P> = wires.iter().flat_map(|(_, p)| [p[0], p[p.len() - 1]]).collect();
    punkte.extend(objekte.iter().filter_map(|o| match &o.kind {
        Kind::Terminal { pos, .. } => Some(*pos),
        _ => None,
    }));
    let verbunden = |x: P| punkte.iter().any(|q| (q.x - x.x).abs() < 0.5 && (q.y - x.y).abs() < 0.5);
    for (i, (ia, pa)) in wires.iter().enumerate() {
        for (ib, pb) in &wires[i + 1..] {
            for sa in pa.windows(2) {
                for sb in pb.windows(2) {
                    let Some(x) = schnitt(sa[0], sa[1], sb[0], sb[1], R * 1.2) else { continue };
                    if verbunden(x) {
                        continue;
                    }
                    // die waagerechtere springt; bei Gleichstand die später angelegte (größere ID)
                    let (wa, wb) = (waagerecht(sa[0], sa[1]), waagerecht(sb[0], sb[1]));
                    let id = if (wa - wb).abs() > 1e-3 { if wa > wb { *ia } else { *ib } } else { (*ia).max(*ib) };
                    k.entry(id).or_default().push(x);
                }
            }
        }
    }
    k
}

/// Polygonzug mit eingesetzten Halbkreisen an den Kreuzungspunkten `x` (Bogen nach oben, bei senkrechten Stücken
/// nach links). Kreuzungen, deren Bögen sich überschneiden würden, werden zusammengefasst (nur der erste Bogen).
pub fn mit_boegen(pts: &[P], x: &[P]) -> Vec<P> {
    if x.is_empty() || pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            out.push(b);
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        // Bogenrichtung: Normale, die nach oben (−y) bzw. bei senkrechten Stücken nach links (−x) zeigt
        let (mut nx, mut ny) = (uy, -ux);
        if ny > 1e-3 || (ny.abs() <= 1e-3 && nx > 0.0) {
            nx = -nx;
            ny = -ny;
        }
        // Kreuzungen auf diesem Stück, nach Lage entlang des Stücks
        let mut ts: Vec<f32> = x
            .iter()
            .filter(|c| {
                let (cx, cy) = (c.x - a.x, c.y - a.y);
                (cx * uy - cy * ux).abs() < 1e-3 && cx * ux + cy * uy > 0.0 && cx * ux + cy * uy < len
            })
            .map(|c| (c.x - a.x) * ux + (c.y - a.y) * uy)
            .collect();
        ts.sort_by(|p, q| p.total_cmp(q));
        let mut letzt = f32::NEG_INFINITY;
        for t in ts {
            if t - letzt < 2.0 * R + 0.2 {
                continue;
            }
            letzt = t;
            const N: usize = 10;
            for k in 0..=N {
                let w = std::f32::consts::PI * k as f32 / N as f32;
                // von −R über den Scheitel bis +R entlang des Stücks
                let s = t - R * w.cos();
                let h = R * w.sin();
                out.push(P::new(a.x + ux * s + nx * h, a.y + uy * s + ny * h));
            }
        }
        out.push(b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
        let kind = Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 };
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn waagerechte_springt_ueber_senkrechte() {
        let ob = vec![wire(1, &[(0.0, 10.0), (20.0, 10.0)]), wire(2, &[(10.0, 0.0), (10.0, 20.0)])];
        let k = karte_mit(&ob, true);
        assert_eq!(k.get(&1).map(|v| v.len()), Some(1));
        assert!(!k.contains_key(&2));
        let p = mit_boegen(&[P::new(0.0, 10.0), P::new(20.0, 10.0)], &k[&1]);
        // Scheitel des Bogens über der Leitung (y kleiner)
        let top = p.iter().map(|q| q.y).fold(f32::INFINITY, f32::min);
        assert!((top - (10.0 - R)).abs() < 1e-3, "{top}");
        assert_eq!(p.first().unwrap().x, 0.0);
        assert_eq!(p.last().unwrap().x, 20.0);
    }

    #[test]
    fn keine_bruecke_an_t_abzweig_und_knick() {
        // Ende der senkrechten Leitung liegt auf der waagerechten (Verbindung) → kein Bogen
        let t = vec![wire(1, &[(0.0, 10.0), (20.0, 10.0)]), wire(2, &[(10.0, 0.0), (10.0, 10.0)])];
        assert!(karte_mit(&t, true).is_empty());
        // Knickpunkt der einen auf der anderen → kein Bogen (zu nah am Stückende)
        let k = vec![wire(1, &[(0.0, 10.0), (20.0, 10.0)]), wire(2, &[(10.0, 0.0), (10.0, 10.0), (30.0, 10.0)])];
        assert!(karte_mit(&k, true).is_empty());
    }

    #[test]
    fn keine_bruecke_an_verbindungspunkt() {
        // dritte Leitung endet im Kreuzungspunkt → Verbindung, kein Bogen
        let ob = vec![wire(1, &[(0.0, 10.0), (20.0, 10.0)]), wire(2, &[(10.0, 0.0), (10.0, 20.0)]), wire(3, &[(10.0, 10.0), (10.0, 30.0)])];
        assert!(karte_mit(&ob, true).get(&1).is_none());
    }

    #[test]
    fn abschaltbar() {
        let ob = vec![wire(1, &[(0.0, 10.0), (20.0, 10.0)]), wire(2, &[(10.0, 0.0), (10.0, 20.0)])];
        assert!(karte_mit(&ob, false).is_empty());
    }

    #[test]
    fn dicht_liegende_kreuzungen_ein_bogen() {
        let p = mit_boegen(&[P::new(0.0, 0.0), P::new(20.0, 0.0)], &[P::new(10.0, 0.0), P::new(10.5, 0.0)]);
        assert_eq!(p.len(), 2 + 11);
    }
}
