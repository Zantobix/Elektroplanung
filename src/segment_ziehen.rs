//! Teilstück einer Leitung mit der Maus verschieben, ohne sie zu trennen (Maus-Befund M2, 22.11.2026).
//!
//! Bisher verschob das Ziehen an einer Leitung die ganze Leitung; ihre Enden lösten sich von den Anschlüssen.
//! Das Original (Gegenprobe unter Wine) verschiebt nur das gegriffene Teilstück quer zu seiner Richtung; die
//! Nachbarteilstücke werden länger bzw. kürzer, die Enden bleiben an ihren Anschlüssen. Ist das gegriffene
//! Teilstück das erste oder letzte, entsteht am festen Ende ein neues Querstück.
//!
//! Regeln neu geschrieben nach KiCad Eeschema `SCH_MOVE_TOOL::orthoLineDrag` (GPL; kein Code übernommen): Ein
//! Nachbarstück, das quer zum gegriffenen liegt, wird gestreckt bzw. gestaucht; liegt es nicht so (schräg, in
//! Flucht) oder ist dort das feste Leitungsende, wird ein Knick (Querstück über den alten Eckpunkt) eingefügt.
//! Danach verschmelzen Stücke der Länge 0 und Stücke, die auf dem vorigen zurücklaufen
//! (`leitung_fuehrung::bereinigen`), sodass weder doppelte Punkte noch Rückläufer bleiben. Gerechnet wird immer
//! von der Ausgangslage aus; zurück in die Ausgangslage ergibt also wieder die (bereinigte) Ausgangsleitung.

use crate::app::{App, Drag};
use crate::model::{Kind, P};

const EPS: f32 = 1e-3;

/// Abstand von `p` zur Strecke `a`–`b`.
fn abstand(p: P, a: P, b: P) -> f32 {
    let d = b.sub(a);
    let l2 = d.x * d.x + d.y * d.y;
    if l2 < 1e-9 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * d.x + (p.y - a.y) * d.y) / l2).clamp(0.0, 1.0);
    p.dist(P::new(a.x + d.x * t, a.y + d.y * t))
}

/// Index des Teilstücks von `pts`, das `m` (Toleranz `tol`) trifft; `None` auf den Leitungsenden (dort beginnt
/// das Ziehen eine neue Leitung bzw. verschiebt wie bisher).
pub fn teilstueck_bei(pts: &[P], m: P, tol: f32) -> Option<usize> {
    if pts.len() < 2 || m.dist(pts[0]) <= tol || m.dist(pts[pts.len() - 1]) <= tol {
        return None;
    }
    (0..pts.len() - 1).filter(|&i| abstand(m, pts[i], pts[i + 1]) <= tol).min_by(|&i, &j| abstand(m, pts[i], pts[i + 1]).total_cmp(&abstand(m, pts[j], pts[j + 1])))
}

/// Verschiebung `d` quer zum Teilstück `a`–`b` (waagerecht: nur senkrecht, senkrecht: nur waagerecht, schräg: ganz).
pub fn quer(a: P, b: P, d: P) -> P {
    if (a.y - b.y).abs() < EPS {
        P::new(0.0, d.y)
    } else if (a.x - b.x).abs() < EPS {
        P::new(d.x, 0.0)
    } else {
        d
    }
}

fn achsengerade(a: P, b: P) -> bool {
    (a.x - b.x).abs() < EPS || (a.y - b.y).abs() < EPS
}

/// Punkte nach dem Verschieben des Teilstücks `idx` von `orig` um `d` (schon quer). Anfang und Ende bleiben fest;
/// wird ein Nachbarstück schräg (oder ist das Teilstück das erste/letzte), entsteht ein Querstück über den alten
/// Eckpunkt.
pub fn verschieben(orig: &[P], idx: usize, d: P) -> Vec<P> {
    let n = orig.len();
    if n < 2 || idx + 1 >= n {
        return orig.to_vec();
    }
    let a = P::new(orig[idx].x + d.x, orig[idx].y + d.y);
    let b = P::new(orig[idx + 1].x + d.x, orig[idx + 1].y + d.y);
    let mut out: Vec<P> = orig[..idx].to_vec();
    // vorne: am festen Anfang bzw. bei schrägem Nachbarstück den alten Eckpunkt behalten
    if idx == 0 || !achsengerade(orig[idx - 1], a) {
        out.push(orig[idx]);
    }
    out.push(a);
    out.push(b);
    if idx + 1 == n - 1 || !achsengerade(b, orig[idx + 2]) {
        out.push(orig[idx + 1]);
    }
    out.extend_from_slice(&orig[(idx + 2).min(n)..]);
    crate::leitung_fuehrung::bereinigen(out)
}

impl App {
    /// Beim Drücken auf eine einzeln markierte Leitung: Teilstück-Ziehen beginnen. Liefert, ob es begonnen hat.
    pub fn teilstueck_greifen(&mut self, id: u64, m: P, tol: f32) -> bool {
        if self.selection != [id] {
            return false;
        }
        let Some(o) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)) else { return false };
        if o.fixed && self.fix_active {
            return false;
        }
        let Kind::Wire { pts, .. } = &o.kind else { return false };
        let Some(idx) = teilstueck_bei(pts, m, tol) else { return false };
        let orig = pts.clone();
        self.drag = Drag::Teilstueck { id, idx, orig, start: self.snap(m), moved: false };
        true
    }

    /// Maus bewegt während des Teilstück-Ziehens.
    pub fn teilstueck_ziehen(&mut self, m: P) {
        let Drag::Teilstueck { id, idx, orig, start, moved } = self.drag.clone() else { return };
        let d = quer(orig[idx], orig[idx + 1], self.snap(m).sub(start));
        if !moved && d.x.abs() < 1e-4 && d.y.abs() < 1e-4 {
            return;
        }
        if !moved {
            self.checkpoint();
        }
        // Abgriff an der Sammelschiene wandert mit (keine Leitung auf der Schiene, schiene_halten.rs)
        let sch = self.sheet().map(|s| crate::schiene_halten::schienen(s, &[])).unwrap_or_default();
        let neu = crate::schiene_halten::abgreifen(&verschieben(&orig, idx, d), &sch);
        self.with_sheet(|s, _| {
            if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                *pts = neu;
            }
        });
        self.drag = Drag::Teilstueck { id, idx, orig, start, moved: true };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn einzelnes_gerades_stueck_bekommt_querstuecke() {
        // Q1:14 (40,100) → Klemme (80,100), nach unten gezogen: Enden bleiben, zwei senkrechte Querstücke
        let neu = verschieben(&[p(40.0, 100.0), p(80.0, 100.0)], 0, p(0.0, 20.0));
        assert_eq!(neu, vec![p(40.0, 100.0), p(40.0, 120.0), p(80.0, 120.0), p(80.0, 100.0)]);
    }

    #[test]
    fn mittelstueck_verlaengert_nachbarn() {
        // Z-Leitung wie im Original-Bild: das untere waagerechte Stück nach unten ziehen
        let alt = vec![p(40.0, 100.0), p(40.0, 105.0), p(70.0, 105.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)];
        let neu = verschieben(&alt, 1, p(0.0, 15.0));
        assert_eq!(neu, vec![p(40.0, 100.0), p(40.0, 120.0), p(70.0, 120.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)]);
    }

    #[test]
    fn quer_nur_senkrecht_zum_stueck() {
        assert_eq!(quer(p(0.0, 0.0), p(10.0, 0.0), p(3.0, 4.0)), p(0.0, 4.0));
        assert_eq!(quer(p(0.0, 0.0), p(0.0, 10.0), p(3.0, 4.0)), p(3.0, 0.0));
    }

    #[test]
    fn enden_treffen_kein_teilstueck() {
        let pts = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)];
        assert_eq!(teilstueck_bei(&pts, p(0.2, 0.0), 0.5), None);
        assert_eq!(teilstueck_bei(&pts, p(5.0, 0.3), 0.5), Some(0));
        assert_eq!(teilstueck_bei(&pts, p(10.2, 6.0), 0.5), Some(1));
    }

    /// Keine doppelten Punkte, keine Nullstücke, keine zurücklaufenden Stücke, alles achsgerade.
    fn sauber(v: &[P]) {
        for w in v.windows(2) {
            assert!(w[0].dist(w[1]) > 0.01, "doppelter Punkt {v:?}");
            assert!(achsengerade(w[0], w[1]), "schräg {v:?}");
        }
        for w in v.windows(3) {
            let (d1, d2) = (w[1].sub(w[0]), w[2].sub(w[1]));
            let parallel = (d1.x * d2.y - d1.y * d2.x).abs() < 1e-3;
            assert!(!parallel, "Stücke in Flucht bzw. zurücklaufend {v:?}");
        }
    }

    #[test]
    fn nullstuecke_verschmelzen() {
        // Z-Leitung: Mittelstück auf die Höhe des Anfangs gezogen → erstes Stück hat Länge 0, gerade Leitung
        let alt = vec![p(0.0, 0.0), p(0.0, 10.0), p(20.0, 10.0), p(20.0, 20.0)];
        let neu = verschieben(&alt, 1, p(0.0, -10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)]);
        sauber(&neu);
        // U-Leitung ganz flach gezogen: eine gerade Leitung von Ende zu Ende
        let alt = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(20.0, 10.0), p(20.0, 0.0), p(30.0, 0.0)];
        let neu = verschieben(&alt, 2, p(0.0, -10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(30.0, 0.0)]);
        // erstes Stück auf die Höhe des Folgestücks: Querstück und Nachbar verschmelzen
        let alt = vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0), p(30.0, 10.0)];
        let neu = verschieben(&alt, 0, p(0.0, 10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(0.0, 10.0), p(30.0, 10.0)]);
        sauber(&neu);
    }

    #[test]
    fn hin_und_zurueck_ohne_rueckstaende() {
        let alt = vec![p(0.0, 0.0), p(0.0, 10.0), p(20.0, 10.0), p(20.0, 30.0), p(40.0, 30.0)];
        for idx in 0..alt.len() - 1 {
            for d in [-30.0f32, -20.0, -10.0, -5.0, 5.0, 10.0, 20.0, 30.0] {
                let dd = quer(alt[idx], alt[idx + 1], p(d, d));
                let neu = verschieben(&alt, idx, dd);
                sauber(&neu);
                // Enden bleiben angeschlossen
                assert_eq!((neu[0], *neu.last().unwrap()), (alt[0], alt[4]), "idx {idx} d {d}");
            }
            // zurück in die Ausgangslage: wieder die Ausgangsleitung
            assert_eq!(verschieben(&alt, idx, p(0.0, 0.0)), alt);
        }
    }

    #[test]
    fn bereinigen_laeuft_zurueck_bis_zum_vorgaenger() {
        use crate::leitung_fuehrung::bereinigen;
        // Rückläufer bis genau auf den vorigen Eckpunkt: kein doppelter Punkt
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(10.0, 0.0), p(20.0, 0.0)]), vec![p(0.0, 0.0), p(20.0, 0.0)]);
        // über den Anfang hinaus zurück: Anfang bleibt, Leitung läuft in die Gegenrichtung
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(10.0, 0.0), p(-5.0, 0.0)]), vec![p(0.0, 0.0), p(-5.0, 0.0)]);
        // doppelte Endpunkte
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(0.0, 0.0), p(10.0, 0.0), p(10.0, 0.0)]), vec![p(0.0, 0.0), p(10.0, 0.0)]);
    }
}
