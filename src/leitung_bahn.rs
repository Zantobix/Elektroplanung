//! Leitungsführung entlang des Mauswegs (Nutzerwunsch Runde 12: „das Leitungsrouting soll entlang der Maus gehen!
//! Wenn ich nach unten und dann nach rechts ziehe, soll so auch die Leitung gehen“).
//!
//! Bisher bestimmte nur die erste Bewegung die Richtung des ersten Teilstücks, der Rest war ein Knick bzw. eine
//! Z-Führung zum Ende. Zog man von einem Anschluss, der nach oben zeigt, nach unten und dann nach rechts, lief die
//! Leitung erst nach oben, dann nach rechts, dann nach unten – sie „sprang um“.
//!
//! Regel (bewusste Erweiterung):
//! - Das gefangene Leitungsende wird verfolgt. Das laufende Teilstück hat eine Achse (zuerst die Richtung des ersten
//!   Teilstücks, `leitung_knick.rs`). Verlässt die Maus diese Achse quer um mindestens `min_quer` und war das
//!   Teilstück mindestens `min_lang` lang, entsteht an der letzten Mauslage auf der Achse ein Knick; das neue
//!   Teilstück läuft quer dazu.
//! - Fährt die Maus auf das vorige Teilstück zurück, verschwindet der letzte Knick wieder.
//! - Der Weg ist Anfang → Knicke → Knick zum Ende entlang der laufenden Achse.
//! - Läuft der so entstandene Weg am Anfang in das eigene Symbol bzw. fehlen Knicke, gilt die bisherige Führung
//!   (`leitung_zug.rs`, Stiche, Z über die Mitte).

use crate::leitung_fuehrung::{bereinigen, Fuehrung, Richtung};
use crate::model::P;

fn auf_achse(a: P, q: P, r: Richtung) -> P {
    match r {
        Richtung::Waagerecht => P::new(q.x, a.y),
        Richtung::Senkrecht => P::new(a.x, q.y),
    }
}

fn achse_von(a: P, b: P) -> Option<Richtung> {
    if (a.y - b.y).abs() < 0.01 && (a.x - b.x).abs() >= 0.01 {
        Some(Richtung::Waagerecht)
    } else if (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() >= 0.01 {
        Some(Richtung::Senkrecht)
    } else {
        None
    }
}

/// Liegt `q` auf der achsengeraden Strecke `a`–`b` (ohne `b`)?
fn auf_strecke(q: P, a: P, b: P) -> bool {
    let t = 0.01;
    let (x1, x2) = (a.x.min(b.x) - t, a.x.max(b.x) + t);
    let (y1, y2) = (a.y.min(b.y) - t, a.y.max(b.y) + t);
    q.x >= x1 && q.x <= x2 && q.y >= y1 && q.y <= y2 && q.dist(b) >= 0.01
}

impl Fuehrung {
    /// Knicke nach dem Mausweg fortschreiben (nach `bewegen_mit`, mit dem gefangenen Ende `ende` und dem Ende des
    /// vorigen Bildes `vorher`). `min` = kleinste Länge eines Teilstücks bzw. Querbewegung (mm, z. B. ein Raster).
    pub fn bahn_fortschreiben(&mut self, vorher: Option<P>, ende: P, min: f32) {
        if self.klickbetrieb || self.schiene.is_some() {
            self.bahn.clear();
            return;
        }
        let a = self.start();
        // zurück auf das vorige Teilstück: letzten Knick wegnehmen
        while let Some(&c) = self.bahn.last() {
            let n = self.bahn.len();
            let vor = if n >= 2 { self.bahn[n - 2] } else { a };
            if auf_strecke(ende, vor, c) && ende.dist(c) >= min * 0.5 {
                self.bahn.pop();
            } else {
                break;
            }
        }
        if self.bahn.is_empty() {
            // erstes Teilstück: Achse aus leitung_knick (Form halten bis Länge 0)
            self.bahn_achse = self.richtung;
        }
        let Some(ax) = self.bahn_achse else { return };
        let Some(z) = vorher else { return };
        let letzt = *self.bahn.last().unwrap_or(&a);
        let d = ende.sub(letzt);
        let quer = match ax {
            Richtung::Waagerecht => d.y.abs(),
            Richtung::Senkrecht => d.x.abs(),
        };
        // Lage des laufenden Teilstücks bis zur letzten Mauslage
        let ecke = auf_achse(letzt, z, ax);
        let lang = ecke.dist(letzt);
        if quer >= min && lang >= min {
            self.bahn.push(ecke);
            self.bahn_achse = Some(crate::leitung_knick::andere(ax));
        }
    }

    /// Weg nach dem Mausweg bis `ende`, falls Knicke aufgezeichnet sind (sonst `None`).
    pub fn bahn_weg(&self, ende: P) -> Option<Vec<P>> {
        if self.bahn.is_empty() || self.klickbetrieb || self.schiene.is_some() {
            return None;
        }
        let ax = self.bahn_achse?;
        let letzt = *self.bahn.last()?;
        let mut w = vec![self.start()];
        w.extend(self.bahn.iter().copied());
        w.push(auf_achse(letzt, ende, ax));
        w.push(ende);
        // achsengerade halten: jeder Knick liegt auf der Achse seines Vorgängers
        let w = bereinigen(w);
        w.windows(2).all(|v| achse_von(v[0], v[1]).is_some()).then_some(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    /// Mausbahn in Rasterschritten durchspielen wie die Oberfläche (bewegen_mit + bahn_fortschreiben).
    fn ziehen(start: P, ecken: &[P], da: Option<P>) -> Fuehrung {
        let mut f = Fuehrung::neu(start);
        let mut alt: Option<P> = None;
        let mut vorher = start;
        for e in ecken {
            let n = (vorher.dist(*e) / 5.0).ceil().max(1.0) as usize;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let m = P::new(vorher.x + (e.x - vorher.x) * t, vorher.y + (e.y - vorher.y) * t);
                f.bewegen_mit(m, da, 3.0);
                f.bahn_fortschreiben(alt, m, 5.0);
                alt = Some(m);
            }
            vorher = *e;
        }
        f
    }

    #[test]
    fn erst_runter_dann_rechts() {
        let f = ziehen(p(0.0, 0.0), &[p(0.0, 40.0), p(60.0, 40.0)], Some(p(0.0, -1.0)));
        assert_eq!(f.bahn_weg(p(60.0, 40.0)), Some(vec![p(0.0, 0.0), p(0.0, 40.0), p(60.0, 40.0)]));
    }

    #[test]
    fn erst_rechts_dann_runter_dann_rechts() {
        let f = ziehen(p(0.0, 0.0), &[p(30.0, 0.0), p(30.0, 50.0), p(80.0, 50.0)], None);
        assert_eq!(f.bahn_weg(p(80.0, 50.0)), Some(vec![p(0.0, 0.0), p(30.0, 0.0), p(30.0, 50.0), p(80.0, 50.0)]));
    }

    #[test]
    fn zurueck_nimmt_knick_weg() {
        let f = ziehen(p(0.0, 0.0), &[p(0.0, 40.0), p(30.0, 40.0), p(0.0, 40.0), p(0.0, 20.0)], None);
        // zurück auf das erste Teilstück: kein Knick mehr
        assert!(f.bahn.is_empty(), "{:?}", f.bahn);
    }

    #[test]
    fn wie_oberflaeche_raster_zehn() {
        // Lampe unten (Anschluss nach unten), Maus in kleinen Schritten, Ende auf 10 mm gefangen
        let fang = |q: P| P::new((q.x / 10.0).round() * 10.0, (q.y / 10.0).round() * 10.0);
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let da = Some(p(0.0, 1.0));
        let mut m = p(0.0, 0.0);
        let mut schritte = vec![];
        for i in 1..=50 { schritte.push(p(0.0, i as f32)); }
        for i in 1..=40 { schritte.push(p(i as f32, 50.0)); }
        for i in 1..=30 { schritte.push(p(40.0, 50.0 - i as f32)); }
        for q in schritte {
            let e = fang(q);
            let vorher = f.zuletzt;
            f.bewegen_mit(e, da, 3.0);
            if vorher.is_none_or(|v| v.dist(e) >= 0.01) {
                f.bahn_fortschreiben(vorher, e, 10.0);
            }
            m = e;
        }
        let w = f.zugweg(m, da, None, 5.0);
        assert_eq!(w, vec![p(0.0, 0.0), p(0.0, 50.0), p(40.0, 50.0), p(40.0, 20.0)], "{:?} {:?}", f.bahn, f.bahn_achse);
    }

    #[test]
    fn gerade_ohne_knick() {
        let f = ziehen(p(0.0, 0.0), &[p(0.0, 40.0)], None);
        assert_eq!(f.bahn_weg(p(0.0, 40.0)), None);
    }
}
