//! Symbole als Hindernisse bei der Neuführung von Leitungen (Maus-Befund M29, 22.11.2026).
//!
//! Nach einem großen Verschieben führt das Mitziehen eine Leitung mit Stich + Z neu (leitung_route.rs). Die
//! Z-Führung kennt nur die beiden Enden; liegt ein anderes Symbol im Weg, lief die Leitung mitten hindurch.
//!
//! Verfahren (bewusst einfach und deterministisch, kein allgemeiner Router):
//! 1. Die Standardführung bleibt, wenn sie kein Hindernis kreuzt.
//! 2. Sonst ein Teilstück quer zu seiner Richtung verschieben (Lage des Mittelstücks einer Z-Führung, Länge eines
//!    Stichs, Umweg an einem Ende ohne Anschlussrichtung) oder an einem Eckpunkt andersherum knicken (alternative
//!    Knickrichtung, zählt als Verschiebung um das kürzere Nachbarstück). Verschoben wird in Rasterschritten, die
//!    kleinste freie Verschiebung gewinnt, bei gleicher Verschiebung der kürzere Weg, dann der mit weniger Knicken.
//! 3. Findet sich nichts Freies, bleibt die Standardführung.
//!
//! Warum kein A*-Router (wie z. B. der Gitter-Router von QElectroTech/KiCad): Die Leitung soll bei jedem Bild der
//! Verschiebung (bzw. der Zeichenvorschau) gleich aussehen und nur wenig springen; ein Gitter-Router findet zwar
//! immer einen Weg, wählt aber bei kleinen Änderungen oft ganz andere Wege mit vielen Knicken, ist teurer und
//! bräuchte Kostenregeln für Stiche, Knicke und Nähe zu Leitungen. Die Verschiebung eines Teilstücks erhält die Form
//! der Standardführung (gleiche Knickfolge), bleibt auf dem Raster und deckt die Fälle aus W2 ab (Symbol zwischen
//! Anfang und Ziel). Ein Router als eigener Befehl („Leitung neu führen“) bleibt möglich (todo, Vergleich Empf. 9).
//!
//! Hindernisse sind die Umrisse der Symbole (`canvas::obj_bounds`). Ein Teilstück kreuzt einen Umriss erst, wenn es
//! mehr als `RAND` in sein Inneres eindringt – am Rand entlang und aus einem Anschluss am Rand heraus ist erlaubt.
//! Umrisse, in deren Innerem ein Ende der Leitung liegt, zählen nicht (nicht vermeidbar).

use crate::canvas;
use crate::leitung_fuehrung::bereinigen;
use crate::leitung_route::laeuft_hinein;
use crate::model::{Kind, Sheet, P};
use crate::symbols::SymDef;

/// Umriss (links oben, rechts unten) in mm.
pub type Rechteck = (P, P);

/// Umriss eines Symbols mit seinen Anschlüssen.
#[derive(Clone, Debug, PartialEq)]
pub struct Umriss {
    pub id: u64,
    pub r: Rechteck,
    pub pins: Vec<P>,
}

/// Eindringtiefe in mm, ab der ein Teilstück als „durch das Symbol“ gilt.
pub const RAND: f32 = 0.3;
/// Weiteste Verschiebung eines Teilstücks in Rasterschritten.
const MAX_SCHRITTE: i32 = 24;
const EPS: f32 = 1e-3;

/// Umrisse aller Symbole des Blattes.
pub fn umrisse(sheet: &Sheet, lib: &[SymDef]) -> Vec<Umriss> {
    sheet
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }))
        .map(|o| Umriss { id: o.id, r: canvas::obj_bounds(o, lib), pins: canvas::obj_pins(o, lib) })
        .collect()
}

/// Hindernisse aus `umrisse` ohne die Symbole `ohne` und ohne die Symbole, die an einem der Punkte `enden` einen
/// Anschluss haben (Symbole, an denen die Leitung endet).
pub fn hindernisse(umrisse: &[Umriss], ohne: &[u64], enden: &[P]) -> Vec<Rechteck> {
    umrisse
        .iter()
        .filter(|u| !ohne.contains(&u.id) && !u.pins.iter().any(|q| enden.iter().any(|e| e.dist(*q) <= crate::mitziehen::PIN_TOL)))
        .map(|u| u.r)
        .collect()
}

/// Liegt `p` mehr als `RAND` im Inneren von `r`?
fn innen(p: P, r: &Rechteck) -> bool {
    p.x > r.0.x + RAND && p.x < r.1.x - RAND && p.y > r.0.y + RAND && p.y < r.1.y - RAND
}

/// Dringt das Teilstück `a`–`b` mehr als `RAND` in das Innere von `r` ein? (Clipping nach Liang–Barsky)
pub fn kreuzt(a: P, b: P, r: &Rechteck) -> bool {
    let (mn, mx) = (P::new(r.0.x + RAND, r.0.y + RAND), P::new(r.1.x - RAND, r.1.y - RAND));
    if mn.x >= mx.x || mn.y >= mx.y {
        return false;
    }
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, d, lo, hi) in [(a.x, b.x - a.x, mn.x, mx.x), (a.y, b.y - a.y, mn.y, mx.y)] {
        if d.abs() < 1e-6 {
            // parallel: muss echt zwischen den Grenzen liegen
            if p <= lo || p >= hi {
                return false;
            }
        } else {
            let (u, v) = ((lo - p) / d, (hi - p) / d);
            let (u, v) = if u < v { (u, v) } else { (v, u) };
            t0 = t0.max(u);
            t1 = t1.min(v);
        }
    }
    t1 - t0 > 1e-6
}

/// Anzahl der Hindernisse, die der Weg `pts` kreuzt.
pub fn kreuzungen(pts: &[P], hind: &[Rechteck]) -> usize {
    hind.iter().filter(|r| pts.windows(2).any(|w| kreuzt(w[0], w[1], r))).count()
}

/// Hindernisse aus `hind`, die für den Weg von `a` nach `b` zählen: ohne die Umrisse, in deren Innerem ein Ende
/// liegt (dort ist das Hindurchlaufen nicht vermeidbar).
pub fn wirksam(hind: &[Rechteck], a: P, b: P) -> Vec<Rechteck> {
    hind.iter().filter(|r| !innen(a, r) && !innen(b, r)).copied().collect()
}

fn laenge(pts: &[P]) -> f32 {
    pts.windows(2).map(|w| w[0].dist(w[1])).sum()
}

/// Lage eines Teilstücks: Some(true) waagerecht, Some(false) senkrecht, None schräg oder Länge 0.
fn lage(a: P, b: P) -> Option<bool> {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    if dy < EPS && dx >= EPS {
        Some(true)
    } else if dx < EPS && dy >= EPS {
        Some(false)
    } else {
        None
    }
}

fn skalar(a: P, b: P) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Einheitsrichtung und Länge von `a` nach `b`.
fn richtung(a: P, b: P) -> (P, f32) {
    let d = b.sub(a);
    let l = skalar(d, d).sqrt();
    if l < 1e-6 {
        (P::new(0.0, 0.0), 0.0)
    } else {
        (P::new(d.x / l, d.y / l), l)
    }
}

/// Darf `c` die Standardführung `weg` ersetzen? Gleiche Enden, rechtwinklig, läuft nicht in ein Symbol hinein,
/// Stiche in Anschlussrichtung bleiben (mindestens ein halbes Raster bzw. so lang wie vorher), beim Zeichnen
/// (`zug`) dieselbe Lage der ersten beiden Teilstücke (Zugrichtung, KiCad-Posture, Leertaste).
fn zulaessig(c: &[P], weg: &[P], da: Option<P>, db: Option<P>, raster: f32, zug: bool) -> bool {
    if c.len() < 2 || c.windows(2).any(|w| lage(w[0], w[1]).is_none()) || laeuft_hinein(c, da, db) {
        return false;
    }
    let stich = raster / 2.0;
    // Stich am Anfang (Richtung nach außen) und am Ende (Leitung kommt von außen, also entgegen der Richtung)
    let stich_bleibt = |d: Option<P>, w0: (P, P), c0: (P, P), vz: f32| {
        let Some(d) = d else { return true };
        let (rw, lw) = richtung(w0.0, w0.1);
        if skalar(rw, d) * vz <= 0.5 {
            return true;
        }
        let (rc, lc) = richtung(c0.0, c0.1);
        skalar(rc, d) * vz > 0.5 && lc >= lw.min(stich) - EPS
    };
    let (n, m) = (weg.len(), c.len());
    if !stich_bleibt(da, (weg[0], weg[1]), (c[0], c[1]), 1.0) || !stich_bleibt(db, (weg[n - 1], weg[n - 2]), (c[m - 1], c[m - 2]), 1.0) {
        return false;
    }
    if zug {
        for i in 0..2.min(n - 1) {
            if i + 1 >= m || lage(weg[i], weg[i + 1]) != lage(c[i], c[i + 1]) {
                return false;
            }
        }
    }
    true
}

/// Kürzester zulässiger, freier Kandidat.
fn bester(kandidaten: Vec<Vec<P>>, weg: &[P], da: Option<P>, db: Option<P>, hind: &[Rechteck], raster: f32, zug: bool) -> Option<Vec<P>> {
    let mut best: Option<(f32, usize, Vec<P>)> = None;
    for c in kandidaten {
        if !zulaessig(&c, weg, da, db, raster, zug) || kreuzungen(&c, hind) > 0 {
            continue;
        }
        let (l, n) = (laenge(&c), c.len());
        if best.as_ref().is_none_or(|(bl, bn, _)| l < bl - EPS || ((l - bl).abs() <= EPS && n < *bn)) {
            best = Some((l, n, c));
        }
    }
    best.map(|(_, _, c)| c)
}

/// Standardführung `weg` (rechtwinklig, Richtung nach außen `da`/`db` an den Enden) um die Hindernisse `hind`
/// herumführen (Verfahren im Modulkommentar); `raster` = Schrittweite, `zug` = beim Zeichnen die Zugrichtung
/// erhalten. Ohne freien Weg bleibt `weg`.
pub fn umgehen(weg: &[P], da: Option<P>, db: Option<P>, hind: &[Rechteck], raster: f32, zug: bool) -> Vec<P> {
    let weg = bereinigen(weg.to_vec());
    if weg.len() < 2 || raster <= 0.0 || hind.is_empty() {
        return weg;
    }
    let (a, b) = (weg[0], weg[weg.len() - 1]);
    let hind = wirksam(hind, a, b);
    if kreuzungen(&weg, &hind) == 0 {
        return weg;
    }
    let n = weg.len();
    // alternative Knickrichtung an einem Eckpunkt (gegenüberliegende Ecke des Rechtecks der Nachbarn); als
    // Verschiebung zählt das kürzere der beiden Nachbarstücke
    let flips: Vec<(f32, Vec<P>)> = (1..n - 1)
        .filter(|&i| lage(weg[i - 1], weg[i]).is_some() && lage(weg[i], weg[i + 1]).is_some() && lage(weg[i - 1], weg[i]) != lage(weg[i], weg[i + 1]))
        .map(|i| {
            let mut c = weg.clone();
            c[i] = weg[i - 1].add(weg[i + 1]).sub(weg[i]);
            (weg[i - 1].dist(weg[i]).min(weg[i].dist(weg[i + 1])), bereinigen(c))
        })
        .collect();
    // ein Teilstück quer verschieben; Enden doppelt, damit auch das erste/letzte Stück wandern kann (die festen
    // Enden bleiben, es entsteht ein Querstück)
    let mut pad = vec![a];
    pad.extend(weg.iter().copied());
    pad.push(b);
    for k in 1..=MAX_SCHRITTE {
        let weite = k as f32 * raster;
        let mut kandidaten: Vec<Vec<P>> = flips.iter().filter(|(v, _)| *v <= weite + EPS && *v > weite - raster + EPS).map(|(_, c)| c.clone()).collect();
        for i in 1..pad.len() - 2 {
            let Some(waag) = lage(pad[i], pad[i + 1]) else { continue };
            for vz in [1.0f32, -1.0] {
                let d = if waag { P::new(0.0, vz * weite) } else { P::new(vz * weite, 0.0) };
                let mut c = pad.clone();
                c[i] = c[i].add(d);
                c[i + 1] = c[i + 1].add(d);
                kandidaten.push(bereinigen(c));
            }
        }
        if let Some(c) = bester(kandidaten, &weg, da, db, &hind, raster, zug) {
            return c;
        }
    }
    weg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }
    const UNTEN: Option<P> = Some(P { x: 0.0, y: 1.0 });
    const OBEN: Option<P> = Some(P { x: 0.0, y: -1.0 });

    #[test]
    fn kreuzen_nur_im_inneren() {
        let r = (p(10.0, 10.0), p(20.0, 30.0));
        assert!(kreuzt(p(0.0, 20.0), p(30.0, 20.0), &r));
        assert!(kreuzt(p(15.0, 0.0), p(15.0, 15.0), &r));
        // am Rand entlang bzw. knapp hinein: nicht
        assert!(!kreuzt(p(0.0, 10.0), p(30.0, 10.0), &r));
        assert!(!kreuzt(p(10.2, 0.0), p(10.2, 40.0), &r));
        // daneben, davor
        assert!(!kreuzt(p(0.0, 35.0), p(30.0, 35.0), &r));
        assert!(!kreuzt(p(15.0, 0.0), p(15.0, 9.0), &r));
        // aus einem Anschluss am Rand nach außen
        assert!(!kreuzt(p(15.0, 30.0), p(15.0, 40.0), &r));
    }

    #[test]
    fn freier_weg_bleibt() {
        let w = vec![p(0.0, 0.0), p(0.0, 2.5), p(40.0, 2.5), p(40.0, 50.0)];
        let hind = [(p(100.0, 100.0), p(110.0, 110.0))];
        assert_eq!(umgehen(&w, UNTEN, OBEN, &hind, 5.0, false), w);
    }

    #[test]
    fn mittelstueck_weicht_aus() {
        // Z-Führung von (0,0) nach unten zu (60,80) von oben; Mittelstück senkrecht bei x = 30 durch ein Symbol
        let w = crate::leitung_route::mit_stichen(p(0.0, 0.0), UNTEN, p(60.0, 80.0), OBEN, 2.5);
        assert_eq!(w, vec![p(0.0, 0.0), p(0.0, 2.5), p(30.0, 2.5), p(30.0, 77.5), p(60.0, 77.5), p(60.0, 80.0)]);
        let hind = [(p(25.0, 30.0), p(37.0, 50.0))];
        assert_eq!(kreuzungen(&w, &hind), 1);
        let r = umgehen(&w, UNTEN, OBEN, &hind, 5.0, false);
        // kleinste Verschiebung auf dem Raster: ein Raster nach links auf den Rand (x = 25, der Rand zählt nicht)
        assert_eq!(r, vec![p(0.0, 0.0), p(0.0, 2.5), p(25.0, 2.5), p(25.0, 77.5), p(60.0, 77.5), p(60.0, 80.0)]);
        assert_eq!(kreuzungen(&r, &hind), 0);
    }

    #[test]
    fn l_form_verschieben_oder_andersherum() {
        // L ohne Anschlussrichtungen: erst waagerecht, Ecke (50,0) liegt in einem Symbol → waagerechtes Stück auf
        // den Rand des Symbols (zwei Raster), andersherum knicken wäre eine Verschiebung um 40
        let w = vec![p(0.0, 0.0), p(50.0, 0.0), p(50.0, 40.0)];
        let hind = [(p(40.0, -10.0), p(60.0, 10.0))];
        let r = umgehen(&w, None, None, &hind, 5.0, false);
        assert_eq!(r, vec![p(0.0, 0.0), p(0.0, 10.0), p(50.0, 10.0), p(50.0, 40.0)]);
        // beim Zeichnen bleibt die Zugrichtung (erst waagerecht): das senkrechte Stück wandert
        let r = umgehen(&w, None, None, &hind, 5.0, true);
        assert_eq!(r, vec![p(0.0, 0.0), p(40.0, 0.0), p(40.0, 40.0), p(50.0, 40.0)]);
        // kurzes zweites Stück: andersherum knicken ist die kleinste Verschiebung
        let w = vec![p(0.0, 0.0), p(50.0, 0.0), p(50.0, 4.0)];
        let hind = [(p(40.0, -10.0), p(60.0, 2.0))];
        assert_eq!(umgehen(&w, None, None, &hind, 10.0, false), vec![p(0.0, 0.0), p(0.0, 4.0), p(50.0, 4.0)]);
    }

    #[test]
    fn stich_bleibt_erhalten() {
        // Stich nach unten, dann waagerecht durch ein Symbol: das waagerechte Stück darf tiefer, nicht höher als der Stich
        let w = vec![p(0.0, 0.0), p(0.0, 2.5), p(60.0, 2.5), p(60.0, 0.0)];
        let hind = [(p(20.0, -5.0), p(30.0, 6.0))];
        let r = umgehen(&w, UNTEN, UNTEN, &hind, 5.0, false);
        assert_eq!(kreuzungen(&r, &hind), 0, "{r:?}");
        assert!(r[1].y >= 2.5 && r[1].x == 0.0, "{r:?}");
        assert!(!laeuft_hinein(&r, UNTEN, UNTEN));
    }

    #[test]
    fn unvermeidbar_bleibt_standard() {
        // Ende liegt im Inneren eines Umrisses: zählt nicht; Hindernis über das ganze Blatt: Standard bleibt
        let w = vec![p(0.0, 0.0), p(0.0, 2.5), p(30.0, 2.5), p(30.0, 77.5), p(60.0, 77.5), p(60.0, 80.0)];
        assert_eq!(umgehen(&w, UNTEN, OBEN, &[(p(55.0, 70.0), p(65.0, 90.0))], 5.0, false), w);
        let riesig = [(p(-500.0, 20.0), p(500.0, 40.0))];
        assert_eq!(umgehen(&w, UNTEN, OBEN, &riesig, 5.0, false), w);
    }

    #[test]
    fn hindernisse_ohne_endsymbole() {
        let u = vec![
            Umriss { id: 1, r: (p(0.0, 0.0), p(10.0, 10.0)), pins: vec![p(5.0, 10.0)] },
            Umriss { id: 2, r: (p(20.0, 0.0), p(30.0, 10.0)), pins: vec![p(25.0, 10.0)] },
            Umriss { id: 3, r: (p(40.0, 0.0), p(50.0, 10.0)), pins: vec![p(45.0, 10.0)] },
        ];
        assert_eq!(hindernisse(&u, &[3], &[p(5.0, 10.0)]), vec![u[1].r]);
    }
}
