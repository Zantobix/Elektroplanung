//! Umwege einer Leitung auflösen (Nutzer 25.11.2026, Bilder 1/2 und Video): nach einer Bearbeitung bleiben bei der
//! formerhaltenden Nachführung manchmal
//! - **Schleifen**: die Leitung kreuzt sich selbst (z. B. nach unten, rechts, hoch und wieder zurück über sich) – der
//!   Teil zwischen den beiden Kreuzungsstrecken entfällt, die Leitung läuft durch den Kreuzungspunkt;
//! - **Höcker** (U-Form): ein Mittelstück mit zwei entgegengesetzten Querstücken – es rückt um die Länge des kürzeren
//!   Querstücks zurück, das kürzere entfällt (am Anschluss bleibt mindestens der Stich).
//!
//! Ein vereinfachter Weg wird nur genommen, wenn er kürzer ist und `ok` ihn zulässt (Aufrufer: nichts Fremdes
//! berühren, kein Symbol durchqueren, auf keiner anderen Leitung liegen); Anfang und Ende bleiben, wo sie sind.

use crate::app::App;
use crate::model::*;

const T: f32 = 0.01;

fn laenge(w: &[P]) -> f32 {
    w.windows(2).map(|v| v[0].dist(v[1])).sum()
}

/// Doppelte Punkte und Zwischenpunkte auf geraden Strecken entfernen.
pub fn bereinigen(w: Vec<P>) -> Vec<P> {
    let mut v: Vec<P> = vec![];
    for p in w {
        if v.last().is_some_and(|q| q.dist(p) < T) {
            continue;
        }
        if v.len() >= 2 {
            let (a, b) = (v[v.len() - 2], v[v.len() - 1]);
            let gerade = ((a.x - b.x).abs() < T && (b.x - p.x).abs() < T) || ((a.y - b.y).abs() < T && (b.y - p.y).abs() < T);
            if gerade {
                v.pop();
            }
        }
        v.push(p);
    }
    v
}

/// Schnittpunkt zweier achsparalleler Strecken (eine waagerecht, eine senkrecht; Ränder eingeschlossen).
fn schnitt(a: P, b: P, c: P, d: P) -> Option<P> {
    let waag = |p: P, q: P| (p.y - q.y).abs() < T;
    let senk = |p: P, q: P| (p.x - q.x).abs() < T;
    let zw = |v: f32, x: f32, y: f32| v >= x.min(y) - T && v <= x.max(y) + T;
    if waag(a, b) && senk(c, d) && zw(c.x, a.x, b.x) && zw(a.y, c.y, d.y) {
        return Some(P::new(c.x, a.y));
    }
    if senk(a, b) && waag(c, d) && zw(a.x, c.x, d.x) && zw(c.y, a.y, b.y) {
        return Some(P::new(a.x, c.y));
    }
    None
}

/// Kandidaten: Schleifen herausgeschnitten und Höcker flachgedrückt (je ein Schritt).
fn kandidaten(w: &[P], stich: f32) -> Vec<Vec<P>> {
    let n = w.len();
    let mut k = vec![];
    // Schleifen: Strecke i kreuzt Strecke j (j ≥ i + 2)
    for i in 0..n.saturating_sub(1) {
        for j in i + 2..n.saturating_sub(1) {
            if let Some(x) = schnitt(w[i], w[i + 1], w[j], w[j + 1]) {
                let mut v: Vec<P> = w[..=i].to_vec();
                v.push(x);
                v.extend_from_slice(&w[j + 1..]);
                k.push(bereinigen(v));
            }
        }
    }
    // Höcker: p0→p1 und p2→p3 entgegengesetzt, beide quer zu p1→p2 – nur im Innern der Leitung: ein Querstück am
    // Anschluss ist dessen Stich und oft bewusst so lang gezeichnet (maustest_licht_lampe_hin_und_zurueck)
    for i in 1..n.saturating_sub(4).max(1) {
        if i + 3 > n - 2 {
            break;
        }
        let (p0, p1, p2, p3) = (w[i], w[i + 1], w[i + 2], w[i + 3]);
        let d1 = p1.sub(p0);
        let d3 = p3.sub(p2);
        let (l1, l3) = (d1.x.abs() + d1.y.abs(), d3.x.abs() + d3.y.abs());
        if l1 < T || l3 < T {
            continue;
        }
        let e1 = P::new(d1.x / l1, d1.y / l1);
        let e3 = P::new(d3.x / l3, d3.y / l3);
        let mitte = p2.sub(p1);
        let quer = (e1.x * mitte.x + e1.y * mitte.y).abs() < T;
        if !(quer && (e1.x + e3.x).abs() < T && (e1.y + e3.y).abs() < T) {
            continue;
        }
        let _ = stich;
        let m = l1.min(l3);
        if m < T {
            continue;
        }
        let mut v = w.to_vec();
        v[i + 1] = p1.sub(P::new(e1.x * m, e1.y * m));
        v[i + 2] = p2.sub(P::new(e1.x * m, e1.y * m));
        k.push(bereinigen(v));
    }
    k
}

/// Weg `w` vereinfachen, solange ein kürzerer zulässiger Kandidat existiert.
pub fn aufloesen(w: &[P], stich: f32, ok: &dyn Fn(&[P]) -> bool) -> Vec<P> {
    let mut w = bereinigen(w.to_vec());
    for _ in 0..20 {
        let l = laenge(&w);
        let best = kandidaten(&w, stich)
            .into_iter()
            .filter(|v| v.len() >= 2 && v[0].dist(w[0]) < T && v[v.len() - 1].dist(w[w.len() - 1]) < T)
            .filter(|v| laenge(v) < l - T)
            .filter(|v| ok(v))
            .min_by(|a, b| laenge(a).total_cmp(&laenge(b)));
        match best {
            Some(v) => w = v,
            None => break,
        }
    }
    w
}

impl App {
    /// Umwege der Leitungen `ids` auflösen (Aufräumen nach einer Bearbeitung, leitung_vereinigen.rs).
    pub fn umwege_aufloesen(&mut self, ids: &[u64]) {
        if self.sym_edit.is_some() {
            return;
        }
        let Some(s) = self.sheet().cloned() else { return };
        let lib = self.lib.clone();
        let stich = self.stich();
        let hind: Vec<_> = crate::leitung_hindernis::umrisse(&s, &lib).into_iter().map(|u| u.r).collect();
        let mut neu: Vec<(u64, Vec<P>)> = vec![];
        for id in ids {
            let Some(Kind::Wire { pts, .. }) = s.objects.iter().find(|o| o.id == *id).map(|o| &o.kind) else { continue };
            if pts.len() < 4 {
                continue;
            }
            let mut ohne = s.clone();
            ohne.objects.retain(|o| o.id != *id);
            let (a, e) = (pts[0], pts[pts.len() - 1]);
            let fremd = crate::leitung_route::fremd_blatt(&ohne, &lib, a, e);
            let hind = crate::leitung_hindernis::wirksam(&hind, a, e);
            // Überlappen mit Leitungen des eigenen Netzes ist erlaubt (das Vereinigen fasst sie danach zusammen), mit
            // fremden nicht
            let netz = crate::netz::netz_von(&s, &lib, *id);
            let andere: Vec<(P, P)> = ohne
                .objects
                .iter()
                .filter(|o| !netz.contains(&o.id))
                .flat_map(|o| match &o.kind {
                    Kind::Wire { pts, .. } => pts.windows(2).map(|v| (v[0], v[1])).collect::<Vec<_>>(),
                    Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
                    _ => vec![],
                })
                .collect();
            let ok = |w: &[P]| !crate::leitung_route::beruehrt_fremd(&fremd, &lib, w) && crate::leitung_hindernis::kreuzungen(w, &hind) == 0 && !crate::leitung_kurz::ueberlappt(w, &andere);
            let w = aufloesen(pts, stich, &ok);
            // danach: einfacher Ersatzweg zwischen den Enden mit weniger Knicken (gerade, L, Z, U mit Stichen aus den
            // Anschlüssen; leitung_kurz.rs) – Bild 2: Zickzack unter den Lampen statt eines Stichs auf die Leitung
            let (da, db) = (crate::leitung_route::anschluss_richtung(&ohne, &lib, a), crate::leitung_route::anschluss_richtung(&ohne, &lib, e));
            let w = crate::leitung_kurz::kuerzer_mit(&w, da, db, stich, &hind, &andere, &ok).unwrap_or(w);
            if w.len() >= 2 && &w != pts {
                neu.push((*id, w));
            }
        }
        if neu.is_empty() {
            return;
        }
        self.with_sheet(|s, _| {
            for (id, p) in neu {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *pts = p;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn hoecker_flach() {
        // Bild 2: vom Anschluss (0,0) waagerecht, hoch, rüber, wieder runter, weiter zum Ziel
        let w = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, -5.0), p(20.0, -5.0), p(20.0, 0.0), p(40.0, 0.0)];
        assert_eq!(aufloesen(&w, 2.5, &|_| true), vec![p(0.0, 0.0), p(40.0, 0.0)]);
        // Höcker in einer Leitung mit Ecken davor und danach
        let w = vec![p(0.0, 10.0), p(0.0, 0.0), p(10.0, 0.0), p(10.0, -5.0), p(20.0, -5.0), p(20.0, 0.0), p(40.0, 0.0), p(40.0, 10.0)];
        assert_eq!(aufloesen(&w, 2.5, &|_| true), vec![p(0.0, 10.0), p(0.0, 0.0), p(40.0, 0.0), p(40.0, 10.0)]);
        // nicht zulässig: bleibt
        assert_eq!(aufloesen(&w, 2.5, &|v| v.len() > 4), w);
    }

    #[test]
    fn schleife_weg() {
        // Bild 1: Leitung läuft nach rechts, runter, zurück nach links unter sich durch und wieder hoch über sich
        let w = vec![p(0.0, 0.0), p(30.0, 0.0), p(30.0, 20.0), p(10.0, 20.0), p(10.0, -10.0), p(50.0, -10.0)];
        let r = aufloesen(&w, 2.5, &|_| true);
        assert!(laenge(&r) < laenge(&w) && r[0] == w[0] && r[r.len() - 1] == w[w.len() - 1], "{r:?}");
        assert!(!r.contains(&p(30.0, 20.0)), "Schleife entfernt: {r:?}");
    }

    #[test]
    fn stich_am_anschluss_bleibt() {
        // U direkt an den Anschlüssen: bleibt, wie gezeichnet
        let w = vec![p(0.0, 0.0), p(0.0, -10.0), p(20.0, -10.0), p(20.0, 0.0)];
        assert_eq!(aufloesen(&w, 2.5, &|_| true), w);
    }
}
