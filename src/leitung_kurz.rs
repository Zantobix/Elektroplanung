//! Kurze Neuführung beim Mitziehen (Nutzerwunsch Runde 12, Bild 3: „wenn Objekte verschoben werden, sollen die
//! neu verlegten Leitungen möglichst kurz und ohne Kreuzungen verlegt werden, dass die Knicke minimal sind“).
//!
//! Das formerhaltende Mitziehen (`leitung_profil.rs`) behält die Knicke einer Leitung; nach einer kleinen
//! Verschiebung entstand so z. B. zwischen Klemme und Lampe ein Weg mit vier Knicken, wo zwei genügen. Hier werden
//! einfache Wege zwischen den neuen Enden geprüft – gerade, L in beiden Lagen, Z über die Mitte, jeweils mit dem
//! Stich aus den Anschlüssen – und der mit den wenigsten Knicken (bei Gleichstand der kürzeste) genommen, wenn er
//! - weniger Knicke hat als der formerhaltende Weg,
//! - in keinen Anschluss von der falschen Seite läuft (`laeuft_hinein`),
//! - kein Symbol durchquert,
//! - auf keiner anderen Leitung bzw. Schiene liegt und
//! - nicht mehr andere Leitungen kreuzt als der formerhaltende Weg.

use crate::leitung_fuehrung::bereinigen;
use crate::leitung_hindernis::{kreuzungen, Rechteck};
use crate::leitung_route::{laeuft_hinein, mit_stichen};
use crate::model::P;

const T: f32 = 0.05;

/// Zahl der Knicke eines Weges.
pub fn knicke(w: &[P]) -> usize {
    bereinigen(w.to_vec()).len().saturating_sub(2)
}

fn laenge(w: &[P]) -> f32 {
    w.windows(2).map(|v| v[0].dist(v[1])).sum()
}

/// Echte Kreuzungen (quer, im Innern beider Strecken) des Weges mit den Strecken `andere`.
pub fn kreuzungen_mit(w: &[P], andere: &[(P, P)]) -> usize {
    let mut n = 0;
    for v in w.windows(2) {
        let (a, b) = (v[0], v[1]);
        for (c, d) in andere {
            let h1 = (a.y - b.y).abs() < T;
            let h2 = (c.y - d.y).abs() < T;
            if h1 == h2 {
                continue;
            }
            let ((h0, h1p), (v0, v1)) = if h1 { ((a, b), (*c, *d)) } else { ((*c, *d), (a, b)) };
            let x = v0.x;
            let y = h0.y;
            let innen_h = x > h0.x.min(h1p.x) + T && x < h0.x.max(h1p.x) - T;
            let innen_v = y > v0.y.min(v1.y) + T && y < v0.y.max(v1.y) - T;
            if innen_h && innen_v {
                n += 1;
            }
        }
    }
    n
}

/// Liegt ein Teilstück von `w` in der Flucht einer Strecke aus `andere` mit gemeinsamer Länge über 0,5 mm?
pub fn ueberlappt(w: &[P], andere: &[(P, P)]) -> bool {
    w.windows(2).any(|v| {
        let (a, b) = (v[0], v[1]);
        andere.iter().any(|(c, d)| {
            if (a.y - b.y).abs() < T && (c.y - d.y).abs() < T && (a.y - c.y).abs() < T {
                a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x)) > 0.5
            } else if (a.x - b.x).abs() < T && (c.x - d.x).abs() < T && (a.x - c.x).abs() < T {
                a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y)) > 0.5
            } else {
                false
            }
        })
    })
}

/// Verlässt der Weg den Anfang in Richtung `da` und erreicht er das Ende entgegen `db` (nicht seitlich)?
fn in_richtung(w: &[P], da: Option<P>, db: Option<P>) -> bool {
    let n = w.len();
    let r = |a: P, b: P| {
        let d = b.sub(a);
        let l = (d.x * d.x + d.y * d.y).sqrt().max(1e-6);
        P::new(d.x / l, d.y / l)
    };
    let s = |a: P, b: P| a.x * b.x + a.y * b.y;
    da.is_none_or(|d| s(r(w[0], w[1]), d) > 0.5) && db.is_none_or(|d| s(r(w[n - 2], w[n - 1]), d) < -0.5)
}

/// Bester einfacher Weg von `a` nach `b` (wenigste Knicke, dann kürzester) nach den Regeln im Modulkopf, ohne
/// Vergleich mit einem vorhandenen Weg (Neuführung nach Drehen/Spiegeln).
pub fn bester(a: P, da: Option<P>, b: P, db: Option<P>, s: f32, hind: &[Rechteck], andere: &[(P, P)]) -> Option<Vec<P>> {
    bester_mit(a, da, b, db, s, hind, andere, &|_| true)
}

/// Wie `bester`, nur Wege, die zusätzlich `ok` erfüllen (z. B. über keinen fremden Anschluss, netz_folgen::zulaessig).
#[allow(clippy::too_many_arguments)]
pub fn bester_mit(a: P, da: Option<P>, b: P, db: Option<P>, s: f32, hind: &[Rechteck], andere: &[(P, P)], ok: &dyn Fn(&[P]) -> bool) -> Option<Vec<P>> {
    kandidaten(a, da, b, db, s)
        .into_iter()
        .filter(|w| ok(w))
        .filter(|w| !laeuft_hinein(w, da, db) && in_richtung(w, da, db))
        .filter(|w| kreuzungen(w, hind) == 0)
        .filter(|w| !ueberlappt(w, andere))
        .min_by(|x, y| knicke(x).cmp(&knicke(y)).then(kreuzungen_mit(x, andere).cmp(&kreuzungen_mit(y, andere))).then(laenge(x).total_cmp(&laenge(y))))
}

fn kandidaten(a: P, da: Option<P>, b: P, db: Option<P>, s: f32) -> Vec<Vec<P>> {
    let a1 = da.map(|d| P::new(a.x + d.x * s, a.y + d.y * s)).unwrap_or(a);
    let b1 = db.map(|d| P::new(b.x + d.x * s, b.y + d.y * s)).unwrap_or(b);
    let mut kand: Vec<Vec<P>> = vec![vec![a, b], mit_stichen(a, da, b, db, s)];
    for (p, q) in [(a, b), (a1, b1), (a, b1), (a1, b)] {
        kand.push(vec![a, p, P::new(p.x, q.y), q, b]);
        kand.push(vec![a, p, P::new(q.x, p.y), q, b]);
    }
    // Z mit dem Mittelstück auf weiteren Lagen (Mitte und Stichschritte um die Stiche herum), damit ein Weg um ein
    // Symbol bzw. neben einer Schiene vorbei gefunden wird
    let mut lagen_y = vec![(a1.y + b1.y) / 2.0];
    let mut lagen_x = vec![(a1.x + b1.x) / 2.0];
    for k in 1..=8 {
        let d = k as f32 * s;
        lagen_y.extend([a1.y + d, a1.y - d, b1.y + d, b1.y - d]);
        lagen_x.extend([a1.x + d, a1.x - d, b1.x + d, b1.x - d]);
    }
    for m in lagen_y {
        kand.push(vec![a, a1, P::new(a1.x, m), P::new(b1.x, m), b1, b]);
    }
    for m in lagen_x {
        kand.push(vec![a, a1, P::new(m, a1.y), P::new(m, b1.y), b1, b]);
    }
    kand.into_iter()
        .map(bereinigen)
        .filter(|w| w.len() >= 2 && w.windows(2).all(|v| (v[0].x - v[1].x).abs() < T || (v[0].y - v[1].y).abs() < T))
        .collect()
}

/// Einfacher Weg statt `form` (formerhaltend nachgeführt, Enden schon an der neuen Lage), falls einer die Regeln
/// im Modulkopf erfüllt. `da`/`db` = Richtungen nach außen an Anfang/Ende, `s` = Stich, `hind` = Symbol-Umrisse,
/// `andere` = Strecken anderer Leitungen und Schienen.
pub fn kuerzer(form: &[P], da: Option<P>, db: Option<P>, s: f32, hind: &[Rechteck], andere: &[(P, P)]) -> Option<Vec<P>> {
    kuerzer_mit(form, da, db, s, hind, andere, &|_| true)
}

/// Wie `kuerzer`, nur Wege, die zusätzlich `ok` erfüllen.
#[allow(clippy::too_many_arguments)]
pub fn kuerzer_mit(form: &[P], da: Option<P>, db: Option<P>, s: f32, hind: &[Rechteck], andere: &[(P, P)], ok: &dyn Fn(&[P]) -> bool) -> Option<Vec<P>> {
    let form = bereinigen(form.to_vec());
    if form.len() < 3 {
        return None;
    }
    let (a, b) = (form[0], form[form.len() - 1]);
    // schräge Teilstücke: nichts ändern
    if form.windows(2).any(|v| (v[0].x - v[1].x).abs() >= T && (v[0].y - v[1].y).abs() >= T) {
        return None;
    }
    let k_form = knicke(&form);
    let x_form = kreuzungen_mit(&form, andere);
    kandidaten(a, da, b, db, s)
        .into_iter()
        .filter(|w| knicke(w) < k_form)
        .filter(|w| ok(w))
        .filter(|w| !laeuft_hinein(w, da, db) && in_richtung(w, da, db))
        .filter(|w| kreuzungen(w, hind) == 0)
        .filter(|w| !ueberlappt(w, andere))
        .filter(|w| kreuzungen_mit(w, andere) <= x_form)
        .min_by(|x, y| knicke(x).cmp(&knicke(y)).then(laenge(x).total_cmp(&laenge(y))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn vier_knicke_werden_zwei() {
        // Bild 3: Klemme oben (ohne Richtung), Lampe darunter etwas links versetzt (Anschluss nach oben)
        let form = vec![p(40.0, 60.0), p(40.0, 70.0), p(55.0, 70.0), p(55.0, 80.0), p(30.0, 80.0), p(30.0, 90.0)];
        let w = kuerzer(&form, None, Some(p(0.0, -1.0)), 2.5, &[], &[]).unwrap();
        assert!(knicke(&w) <= 2, "{w:?}");
        assert_eq!((w[0], w[w.len() - 1]), (p(40.0, 60.0), p(30.0, 90.0)));
        // letztes Stück senkrecht von oben in die Lampe
        let n = w.len();
        assert!((w[n - 2].x - w[n - 1].x).abs() < T && w[n - 2].y < w[n - 1].y, "{w:?}");
    }

    #[test]
    fn keine_neue_kreuzung() {
        let form = vec![p(0.0, 0.0), p(0.0, 10.0), p(20.0, 10.0), p(20.0, 0.0), p(40.0, 0.0)];
        // gerade Verbindung wäre (0,0)-(40,0); eine senkrechte Leitung bei x=10 kreuzt sie
        let andere = vec![(p(10.0, -5.0), p(10.0, 5.0))];
        let w = kuerzer(&form, None, None, 2.5, &[], &andere);
        assert!(w.as_ref().is_none_or(|w| kreuzungen_mit(w, &andere) <= kreuzungen_mit(&form, &andere)), "{w:?}");
    }

    #[test]
    fn nicht_auf_anderer_leitung() {
        let form = vec![p(0.0, 0.0), p(0.0, 10.0), p(40.0, 10.0), p(40.0, 0.0)];
        let andere = vec![(p(-5.0, 0.0), p(45.0, 0.0))];
        let w = kuerzer(&form, None, None, 2.5, &[], &andere);
        assert!(w.is_none(), "{w:?}");
    }
}
