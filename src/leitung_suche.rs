//! Rastersuche für den Weg einer neu gezeichneten Leitung, wenn die festen Formen (Winkel, Z, U, längere Stiche in
//! `leitung_route.rs`) alle etwas Fremdes berühren würden (NR1, 24.11.2026).
//!
//! Gesucht wird ein achsparalleler Weg auf einem Gitter aus Rasterlinien (dazu die Fluchten von Anfang, Ende und
//! ihren Stichen) mit Dijkstra. Hart verboten sind Teilstücke, die einen fremden Anschluss, eine fremde Klemmstelle
//! oder ein fremdes Leitungsende berühren oder auf einer fremden Leitung/Schiene liegen, und Knicke auf einer fremden
//! Leitung – also alles, was beim Anlegen eine Verbindung herstellen würde. Kreuzen ist erlaubt. Weich (Kosten) sind
//! Länge, Knicke, das Durchqueren eines Symbols und das Verlassen bzw. Erreichen eines Anschlusses gegen seine
//! Richtung. „Fremd“ heißt: nicht in den Netzen von Anfang und Ende (`fremd_blatt`).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::canvas;
use crate::leitung_hindernis::Rechteck;
use crate::model::*;
use crate::symbols::SymDef;

const T: f32 = 0.05;

struct Fremdes {
    pins: Vec<P>,
    enden: Vec<P>,
    klemmen: Vec<P>,
    strecken: Vec<(P, P)>,
    /// Leitungen und Schienen der eigenen Netze: darauf zu laufen ist erlaubt, aber teuer (Verschmelzen mit der
    /// vorhandenen Leitung kann sonst über eine Kreuzung hinweg verbinden)
    eigen: Vec<(P, P)>,
}

impl Fremdes {
    fn neu(s: &Sheet, fb: &Sheet, lib: &[SymDef], a: P, e: P) -> Self {
        let weg = |q: &P| q.dist(a) > T && q.dist(e) > T;
        let mut f = Fremdes { pins: vec![], enden: vec![], klemmen: vec![], strecken: vec![], eigen: vec![] };
        for o in s.objects.iter().filter(|o| !fb.objects.iter().any(|x| x.id == o.id)) {
            match &o.kind {
                Kind::Wire { pts, .. } => f.eigen.extend(pts.windows(2).map(|v| (v[0], v[1]))),
                Kind::Busbar { a: b, x2, .. } => f.eigen.push((*b, P::new(*x2, b.y))),
                _ => {}
            }
        }
        for o in &fb.objects {
            match &o.kind {
                Kind::Wire { pts, .. } => {
                    f.enden.extend(pts.first().into_iter().chain(pts.last()).copied().filter(weg));
                    f.strecken.extend(pts.windows(2).map(|v| (v[0], v[1])));
                }
                Kind::Busbar { a: b, x2, .. } => f.strecken.push((*b, P::new(*x2, b.y))),
                Kind::Terminal { pos, .. } => {
                    let r = crate::klemme_leitung::KLEMME_R + T;
                    if pos.dist(a) > r && pos.dist(e) > r {
                        f.klemmen.push(*pos);
                    }
                }
                Kind::Symbol { .. } => f.pins.extend(canvas::obj_pins(o, lib).into_iter().filter(weg)),
                _ => {}
            }
        }
        f
    }

    /// Darf das achsparallele Teilstück `u`–`v` gelegt werden?
    fn strecke_frei(&self, u: P, v: P) -> bool {
        let auf = |q: P| seg_dist(q, u, v) < T;
        if self.pins.iter().chain(&self.enden).any(|q| auf(*q)) {
            return false;
        }
        if self.klemmen.iter().any(|q| seg_dist(*q, u, v) < crate::klemme_leitung::KLEMME_R + T) {
            return false;
        }
        !auf_strecken(&self.strecken, u, v)
    }
}

/// Liegt das achsparallele Teilstück `u`–`v` (auch nur berührend in derselben Flucht) auf einer der Strecken?
fn auf_strecken(strecken: &[(P, P)], u: P, v: P) -> bool {
    let waag = (u.y - v.y).abs() < T;
    strecken.iter().any(|(c, d)| {
        if waag && (c.y - d.y).abs() < T && (u.y - c.y).abs() < T {
            u.x.max(v.x).min(c.x.max(d.x)) - u.x.min(v.x).max(c.x.min(d.x)) > -T
        } else if !waag && (c.x - d.x).abs() < T && (u.x - c.x).abs() < T {
            u.y.max(v.y).min(c.y.max(d.y)) - u.y.min(v.y).max(c.y.min(d.y)) > -T
        } else {
            false
        }
    })
}

impl Fremdes {
    /// Darf am Punkt `q` geknickt werden (nicht auf einer fremden Leitung)?
    fn knick_frei(&self, q: P) -> bool {
        !self.strecken.iter().any(|(c, d)| seg_dist(q, *c, *d) < T)
    }
}

/// Koordinaten einer Achse: Rasterlinien im Bereich `lo..hi` (Schrittweite höchstens `max_n` Linien) und die
/// festen Werte `fest`.
fn achse(lo: f32, hi: f32, raster: f32, fest: &[f32], max_n: usize) -> Vec<f32> {
    let mut schritt = raster.max(1.0);
    while (hi - lo) / schritt > max_n as f32 {
        schritt *= 2.0;
    }
    let mut v: Vec<f32> = fest.to_vec();
    let mut x = (lo / schritt).floor() * schritt;
    while x <= hi + T {
        v.push(x);
        x += schritt;
    }
    v.sort_by(f32::total_cmp);
    v.dedup_by(|x, y| (*x - *y).abs() < T);
    v
}

/// Weg von `a` (Richtung `da`) nach `e` (Richtung `db`, Anschluss zeigt nach außen) auf dem Blatt `fb` ohne die
/// eigenen Netze. `None`, wenn es keinen gibt.
#[allow(clippy::too_many_arguments)]
pub fn suchen(s: &Sheet, fb: &Sheet, lib: &[SymDef], a: P, da: Option<P>, e: P, db: Option<P>, raster: f32, stich: f32, hind: &[Rechteck]) -> Option<Vec<P>> {
    let fr = Fremdes::neu(s, fb, lib, a, e);
    let st = |p: P, d: Option<P>| d.map(|d| P::new(p.x + d.x * stich, p.y + d.y * stich)).unwrap_or(p);
    let (sa, se) = (st(a, da), st(e, db));
    let rand = raster.max(2.5) * 12.0;
    let (x0, x1) = (a.x.min(e.x) - rand, a.x.max(e.x) + rand);
    let (y0, y1) = (a.y.min(e.y) - rand, a.y.max(e.y) + rand);
    let xs = achse(x0, x1, raster, &[a.x, e.x, sa.x, se.x], 90);
    let ys = achse(y0, y1, raster, &[a.y, e.y, sa.y, se.y], 90);
    let (nx, ny) = (xs.len(), ys.len());
    let idx = |p: P| Some((xs.iter().position(|x| (x - p.x).abs() < T)?, ys.iter().position(|y| (y - p.y).abs() < T)?));
    let (ia, ie) = (idx(a)?, idx(e)?);
    // Zustand: Knoten × Ankunftsrichtung (0 rechts, 1 links, 2 unten, 3 oben, 4 Anfang)
    const R: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let n = nx * ny * 5;
    let id = |i: usize, j: usize, r: usize| (j * nx + i) * 5 + r;
    let mut dist = vec![f32::INFINITY; n];
    let mut vor = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    let s0 = id(ia.0, ia.1, 4);
    dist[s0] = 0.0;
    heap.push(Reverse((0u64, s0)));
    let knick = raster.max(2.5) * 2.0;
    let gegen = 40.0 * raster.max(2.5);
    let symbol = 60.0 * raster.max(2.5);
    let schluessel = |c: f32| (c * 1000.0) as u64;
    let rich = |r: usize| P::new(R[r].0 as f32, R[r].1 as f32);
    let gleich = |u: P, v: Option<P>| v.is_some_and(|v| (u.x * v.x + u.y * v.y) > 0.99);
    let mut ziel = None;
    while let Some(Reverse((_, z))) = heap.pop() {
        let (k, r_an) = (z / 5, z % 5);
        let (i, j) = (k % nx, k / nx);
        let c = dist[z];
        if (i, j) == ie {
            ziel = Some(z);
            break;
        }
        let p = P::new(xs[i], ys[j]);
        for (r, (di, dj)) in R.iter().enumerate() {
            let (ni, nj) = (i as i32 + di, j as i32 + dj);
            if ni < 0 || nj < 0 || ni >= nx as i32 || nj >= ny as i32 {
                continue;
            }
            // nicht umkehren
            if r_an < 4 && (R[r_an].0 + di == 0 && R[r_an].1 + dj == 0) {
                continue;
            }
            let q = P::new(xs[ni as usize], ys[nj as usize]);
            let mut kosten = p.dist(q);
            if r_an < 4 && r_an != r {
                if !fr.knick_frei(p) {
                    continue;
                }
                kosten += knick;
            }
            if r_an == 4 && da.is_some() && !gleich(rich(r), da) {
                kosten += gegen;
            }
            if !fr.strecke_frei(p, q) {
                continue;
            }
            if (ni as usize, nj as usize) == ie {
                // am Ende: gegen die Anschlussrichtung ankommen
                if let Some(d) = db {
                    if !gleich(rich(r), Some(P::new(-d.x, -d.y))) {
                        kosten += gegen;
                    }
                }
            }
            if auf_strecken(&fr.eigen, p, q) {
                kosten += symbol;
            }
            if crate::leitung_hindernis::kreuzungen(&[p, q], hind) > 0 {
                kosten += symbol;
            }
            let nz = id(ni as usize, nj as usize, r);
            if c + kosten < dist[nz] - 1e-4 {
                dist[nz] = c + kosten;
                vor[nz] = z;
                heap.push(Reverse((schluessel(c + kosten), nz)));
            }
        }
    }
    let mut z = ziel?;
    let mut weg = vec![];
    loop {
        let k = z / 5;
        weg.push(P::new(xs[k % nx], ys[k / nx]));
        if vor[z] == usize::MAX {
            break;
        }
        z = vor[z];
    }
    weg.reverse();
    Some(crate::leitung_fuehrung::bereinigen(weg))
}
