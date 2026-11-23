//! Leitungsführung entlang des Mauswegs (Nutzerwunsch Runde 12: „das Leitungsrouting soll entlang der Maus gehen!
//! Wenn ich nach unten und dann nach rechts ziehe, soll so auch die Leitung gehen“).
//!
//! Bisher bestimmte nur die erste Bewegung die Richtung des ersten Teilstücks, der Rest war ein Knick bzw. eine
//! Z-Führung zum Ende. Zog man von einem Anschluss, der nach oben zeigt, nach unten und dann nach rechts, lief die
//! Leitung erst nach oben, dann nach rechts, dann nach unten – sie „sprang um“.
//!
//! Regel (bewusste Erweiterung, Runde 13 überarbeitet – Nutzer: „nicht bei jeder Richtungsänderung ein Knick,
//! sondern erst, wenn es sinnvoll ist“):
//! - Der laufende Teil ist immer ein einfacher Knick (L) vom letzten Anker (Anfang bzw. letzter fester Knick) zum
//!   Ende; seine erste Richtung bleibt, bis das erste Stück Länge 0 erreicht (`leitung_knick.rs`). Eine einfache
//!   Richtungsänderung der Maus braucht also keinen eigenen Knick.
//! - Ein fester Knick entsteht erst bei einer **Kehre**: Die Maus war mindestens zwei Raster quer zur Achse unterwegs
//!   und läuft dann auf der Achse um mindestens zwei Raster zurück (U-Form, z. B. unter einem Symbol herum). Der
//!   Knick sitzt an der weitesten Lage auf der Achse; danach läuft ein neues L quer dazu.
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

impl crate::app::App {
    /// Beim Ziehen einer Leitung: Maus bei `m` (Zoom `z` px/mm) – Ende fangen, Zugrichtung und Knicke nach dem
    /// Mausweg fortschreiben. Einzige Stelle für Oberfläche und Tests (tests/leitung_zeichnen.rs).
    pub fn leitung_zug_bewegen(&mut self, m: P, z: f32) {
        let crate::app::Drag::Wire { f } = &self.drag else { return };
        let ende = self.leitungsende(f, m, z);
        let da = self.sheet().and_then(|s| crate::leitung_route::anschluss_richtung(s, &self.lib, f.start()));
        let raster = self.raster();
        if let crate::app::Drag::Wire { f } = &mut self.drag {
            let vorher = f.zuletzt;
            f.bewegen_mit(ende, da, 6.0 / z);
            // Knicke entlang des Mauswegs; kleinste Strecke ein Raster
            if vorher.is_none_or(|v| v.dist(ende) >= 0.01) {
                f.bahn_fortschreiben(vorher, ende, raster.max(2.5));
            }
        }
        // Vorschau dieses Bildes merken (Formhalt im nächsten)
        let w = match &self.drag {
            crate::app::Drag::Wire { f } => self.leitungsweg(f, ende),
            _ => return,
        };
        if let crate::app::Drag::Wire { f } = &mut self.drag {
            f.vorschau = w;
        }
    }

    /// Formhalt gegen Umspringen (Runde 13, Zufallsbahnen in tests/leitung_zeichnen.rs): Weicht der neue Weg `w`
    /// stark von der Vorschau des letzten Bildes ab (z. B. Umweg um das eigene Symbol erst unten, dann oben herum),
    /// wird stattdessen die letzte Vorschau mit ihrem Ende an `ende` nachgeführt (Profil wie beim Mitziehen,
    /// mitziehen.rs ohne Profil: das Endstück behält seine Richtung) – sofern das zulässig ist (kein Symbol durchquert, nicht in einen Anschluss hinein, auf
    /// keiner Leitung) und höchstens 2,5 Raster länger als `w`. So wechselt die Führung erst, wenn die neue Form
    /// deutlich besser ist.
    pub fn formhalt(&self, f: &Fuehrung, ende: P, w: Vec<P>) -> Vec<P> {
        let alt = &f.vorschau;
        if alt.len() < 2 || w.len() < 2 || f.klickbetrieb || alt[0].dist(w[0]) > 0.01 {
            return w;
        }
        let raster = self.raster();
        if abweichung(alt, &w) <= raster * 1.01 + raster / 2.0 + 0.5 {
            return w;
        }
        let Some(sheet) = self.sheet() else { return w };
        // Endstück nachführen: das letzte Teilstück behält seine Richtung, der Punkt davor gleitet mit bzw. ein Knick
        // entsteht (wie das Mitziehen ohne Profil, mitziehen.rs)
        let neu = *w.last().unwrap();
        let d = neu.sub(*alt.last().unwrap());
        let c = crate::mitziehen::folgen(alt, [false, true], d, [None, None], false, raster / 2.0, &|p| p);
        if c.last().is_none_or(|p| p.dist(neu) > 0.01) || c[0].dist(w[0]) > 0.01 {
            return w;
        }
        let c = bereinigen(c);
        let da = crate::leitung_route::anschluss_richtung(sheet, &self.lib, c[0]);
        let db = crate::leitung_route::anschluss_richtung(sheet, &self.lib, ende);
        let hind: Vec<_> = crate::leitung_hindernis::umrisse(sheet, &self.lib).into_iter().map(|u| u.r).collect();
        let hind = crate::leitung_hindernis::wirksam(&hind, c[0], *c.last().unwrap());
        let gut = c.windows(2).all(|v| achse_von(v[0], v[1]).is_some())
            && crate::leitung_hindernis::kreuzungen(&c, &hind) == 0
            && !crate::leitung_route::laeuft_hinein(&c, da, db)
            && !crate::leitung_route::ueberlappt(sheet, &c)
            && laenge(&c) <= laenge(&w) + 2.5 * raster + 0.01;
        if gut {
            c
        } else {
            w
        }
    }
}

fn laenge(w: &[P]) -> f32 {
    w.windows(2).map(|v| v[0].dist(v[1])).sum()
}

fn skalar(a: P, b: P) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Größter Abstand zweier Linienzüge (in beide Richtungen, abgetastet): wie weit sich die Vorschau sichtbar bewegt.
pub fn abweichung(u: &[P], v: &[P]) -> f32 {
    fn zum_zug(q: P, v: &[P]) -> f32 {
        if v.len() == 1 {
            return q.dist(v[0]);
        }
        v.windows(2)
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let d = b.sub(a);
                let l2 = skalar(d, d);
                let t = if l2 < 1e-9 { 0.0 } else { (skalar(q.sub(a), d) / l2).clamp(0.0, 1.0) };
                q.dist(P::new(a.x + d.x * t, a.y + d.y * t))
            })
            .fold(f32::MAX, f32::min)
    }
    fn proben(u: &[P]) -> Vec<P> {
        let mut out = vec![u[0]];
        for w in u.windows(2) {
            for i in 1..=10 {
                let t = i as f32 / 10.0;
                out.push(P::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
            }
        }
        out
    }
    if u.is_empty() || v.is_empty() {
        return 0.0;
    }
    let a = proben(u).into_iter().map(|q| zum_zug(q, v)).fold(0.0, f32::max);
    let b = proben(v).into_iter().map(|q| zum_zug(q, u)).fold(0.0, f32::max);
    a.max(b)
}

impl crate::app::App {
    /// Vorschau der gerade gezogenen Leitung bei der Maus `m` (wie sie beim Loslassen entstünde).
    pub fn leitung_vorschau(&self, m: P, z: f32) -> Option<Vec<P>> {
        let crate::app::Drag::Wire { f } = &self.drag else { return None };
        let ende = self.leitungsende(f, m, z);
        Some(self.leitungsweg(f, ende))
    }
}

impl Fuehrung {
    /// Knicke nach dem Mausweg fortschreiben (nach `bewegen_mit`, mit dem gefangenen Ende `ende` und dem Ende des
    /// vorigen Bildes `vorher`). `min` = kleinste Länge eines Teilstücks bzw. Querbewegung (mm, z. B. ein Raster).
    pub fn bahn_fortschreiben(&mut self, vorher: Option<P>, ende: P, min: f32) {
        let _ = vorher;
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
                self.bahn_spitze = None;
                self.bahn_achse = match self.bahn.len() {
                    0 => self.richtung,
                    k => {
                        let v2 = if k >= 2 { self.bahn[k - 2] } else { a };
                        achse_von(v2, self.bahn[k - 1]).map(crate::leitung_knick::andere)
                    }
                };
            } else {
                break;
            }
        }
        if self.bahn.is_empty() {
            // erstes Teilstück: Achse aus leitung_knick (Form halten bis Länge 0); kippt sie, beginnt alles neu
            if self.bahn_achse != self.richtung {
                self.bahn_spitze = None;
            }
            self.bahn_achse = self.richtung;
        }
        let Some(ax) = self.bahn_achse else { return };
        let anker = *self.bahn.last().unwrap_or(&a);
        let k = auf_achse(anker, ende, ax);
        let entlang = |q: P| match ax {
            Richtung::Waagerecht => q.x - anker.x,
            Richtung::Senkrecht => q.y - anker.y,
        };
        let erster = entlang(k);
        let quer = k.dist(ende);
        // weiteste Lage des Knicks auf der Achse und die Länge des Querstücks dort
        match self.bahn_spitze {
            Some((e, q)) if erster * e >= 0.0 && erster.abs() < e.abs() - 1e-3 => self.bahn_spitze = Some((e, q)),
            Some((e, q)) if (erster - e).abs() <= 1e-3 => self.bahn_spitze = Some((e, q.max(quer))),
            _ => self.bahn_spitze = Some((erster, quer)),
        }
        // Kehre: die Maus läuft nach einem deutlichen Querstück auf der Achse zurück → Knick an der weitesten Lage
        if let Some((e, q)) = self.bahn_spitze {
            if q >= 2.0 * min && e.abs() >= min * 0.5 && (e - erster) * e.signum() >= 2.0 * min {
                let ecke = match ax {
                    Richtung::Waagerecht => P::new(anker.x + e, anker.y),
                    Richtung::Senkrecht => P::new(anker.x, anker.y + e),
                };
                self.bahn.push(ecke);
                self.bahn_achse = Some(crate::leitung_knick::andere(ax));
                self.bahn_spitze = None;
            }
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
        // ein einfacher Richtungswechsel braucht keinen festen Knick: das L hält die erste Richtung
        let f = ziehen(p(0.0, 0.0), &[p(0.0, 40.0), p(60.0, 40.0)], None);
        assert!(f.bahn.is_empty(), "{:?}", f.bahn);
        assert_eq!(f.zugweg(p(60.0, 40.0), None, None, 2.5), vec![p(0.0, 0.0), p(0.0, 40.0), p(60.0, 40.0)]);
    }

    #[test]
    fn erst_rechts_dann_runter_dann_rechts() {
        // Z-Bewegung ohne Kehre: ein L mit einem Knick statt drei Stücken (Runde 13: „nicht bei jeder
        // Richtungsänderung ein Knick“)
        let f = ziehen(p(0.0, 0.0), &[p(30.0, 0.0), p(30.0, 50.0), p(80.0, 50.0)], None);
        assert!(f.bahn.is_empty(), "{:?}", f.bahn);
        assert_eq!(f.zugweg(p(80.0, 50.0), None, None, 2.5), vec![p(0.0, 0.0), p(80.0, 0.0), p(80.0, 50.0)]);
    }

    #[test]
    fn kehre_haelt_die_u_form() {
        // runter, rechts, wieder hoch (z. B. unter einem Symbol herum): die Maus kehrt um → fester Knick unten
        let f = ziehen(p(0.0, 0.0), &[p(0.0, 40.0), p(60.0, 40.0), p(60.0, 10.0)], None);
        assert_eq!(f.bahn, vec![p(0.0, 40.0)]);
        assert_eq!(f.zugweg(p(60.0, 10.0), None, None, 2.5), vec![p(0.0, 0.0), p(0.0, 40.0), p(60.0, 40.0), p(60.0, 10.0)]);
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
