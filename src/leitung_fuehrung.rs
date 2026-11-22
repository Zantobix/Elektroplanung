//! Rechtwinklige Leitungsführung beim Zeichnen einer Leitung von einem Anschluss aus.
//!
//! Original (TForm1, Zustand +0x78d = 10, Leitung +0xb2c mit Punktliste +0x40): ein Klick (Loslassen) auf einen
//! Anschluss beginnt die Leitung; jeder weitere Klick auf freie Fläche setzt einen Eckpunkt. Der Klickpunkt wird
//! dabei rechtwinklig zum letzten Punkt ausgerichtet: ist die waagerechte Abweichung kleiner als die senkrechte,
//! übernimmt er die x-Lage des letzten Punktes (senkrechtes Teilstück), sonst dessen y-Lage (waagerechtes
//! Teilstück). Klicks näher als 6 px am letzten Punkt zählen nicht; setzt ein Klick das letzte Teilstück gerade
//! fort, wird dessen Endpunkt verschoben statt ein neuer Punkt angehängt. Ein Klick auf einen Anschluss beendet
//! die Leitung, Rechtsklick bricht ab.
//!
//! Zusätzlich (Nachbau): Ziehen von einem Anschluss zum Ziel legt die Leitung in einem Zug; die erste deutliche
//! Mausbewegung (waagerecht/senkrecht) bestimmt das erste Teilstück. Kehrt die Maus zum Anfang zurück, wird die
//! Richtung neu bestimmt.

use crate::model::P;

/// Richtung eines Teilstücks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Richtung {
    Waagerecht,
    Senkrecht,
}

/// Richtung nach der Regel des Originals: senkrecht nur, wenn die senkrechte Abweichung größer ist.
pub fn vorherrschend(d: P) -> Richtung {
    if d.x.abs() < d.y.abs() {
        Richtung::Senkrecht
    } else {
        Richtung::Waagerecht
    }
}

/// Punkt `m` rechtwinklig zum letzten Punkt `l` ausrichten (Eckpunkt-Klick des Originals).
pub fn ausrichten(l: P, m: P) -> P {
    match vorherrschend(m.sub(l)) {
        Richtung::Senkrecht => P::new(l.x, m.y),
        Richtung::Waagerecht => P::new(m.x, l.y),
    }
}

/// Weg von `a` nach `b` mit höchstens einem Knick; `erst` gibt die Richtung des ersten Teilstücks vor
/// (ohne Vorgabe die vorherrschende Richtung).
pub fn knick(a: P, b: P, erst: Option<Richtung>) -> Vec<P> {
    if (a.x - b.x).abs() < 0.01 || (a.y - b.y).abs() < 0.01 {
        return vec![a, b];
    }
    let corner = match erst.unwrap_or_else(|| vorherrschend(b.sub(a))) {
        Richtung::Waagerecht => P::new(b.x, a.y),
        Richtung::Senkrecht => P::new(a.x, b.y),
    };
    vec![a, corner, b]
}

/// Doppelte Punkte und Zwischenpunkte auf einer Geraden entfernen.
pub fn bereinigen(pts: Vec<P>) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().map(|l| l.dist(p) < 0.01).unwrap_or(false) {
            continue;
        }
        if out.len() >= 2 {
            let (u, v) = (out[out.len() - 2], out[out.len() - 1]);
            let gerade = ((u.x - v.x).abs() < 0.01 && (v.x - p.x).abs() < 0.01) || ((u.y - v.y).abs() < 0.01 && (v.y - p.y).abs() < 0.01);
            if gerade {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// Zustand einer Leitung, die gerade gezeichnet wird.
#[derive(Clone, Debug, PartialEq)]
pub struct Fuehrung {
    /// Feste Punkte: Anfang (Anschluss) und im Klickbetrieb die gesetzten Eckpunkte
    pub pts: Vec<P>,
    /// Richtung des ersten Teilstücks beim Ziehen (aus der ersten deutlichen Mausbewegung)
    pub richtung: Option<Richtung>,
    /// Klickbetrieb wie im Original (nach einem Klick auf den Anschluss ohne Ziehen)
    pub klickbetrieb: bool,
    /// Beginn auf einer Sammelschiene (Höhe, waagerechte Ausdehnung): der Anfang wandert auf der Schiene in die
    /// Flucht des Ziels bzw. des ersten Eckpunkts, der Leiter verlässt die Schiene senkrecht
    pub schiene: Option<(f32, f32, f32)>,
}

impl Fuehrung {
    pub fn neu(start: P) -> Self {
        Fuehrung { pts: vec![start], richtung: None, klickbetrieb: false, schiene: None }
    }

    /// Leitung, die auf einer Sammelschiene `schiene` (Höhe, x von, x bis) bei `m` beginnt.
    pub fn von_schiene(m: P, schiene: (f32, f32, f32)) -> Self {
        let (y, x1, x2) = schiene;
        Fuehrung { pts: vec![P::new(m.x.clamp(x1, x2), y)], richtung: Some(Richtung::Senkrecht), klickbetrieb: false, schiene: Some(schiene) }
    }

    /// Beginnt die Leitung auf einer Schiene und ist noch kein Eckpunkt gesetzt: Anfang in die Flucht von `ziel`
    /// legen (senkrecht von der Schiene ab, Knoten am Abgriff).
    pub fn anfang_zu(&mut self, ziel: P) {
        if let (Some((y, x1, x2)), 1) = (self.schiene, self.pts.len()) {
            self.pts[0] = P::new(ziel.x.clamp(x1, x2), y);
            self.richtung = Some(Richtung::Senkrecht);
        }
    }

    pub fn start(&self) -> P {
        self.pts[0]
    }

    pub fn letzter(&self) -> P {
        *self.pts.last().unwrap()
    }

    /// Beim Ziehen die Maus `m` verfolgen: die erste Bewegung über `schwelle` (mm) hinaus legt die Richtung des
    /// ersten Teilstücks fest; liegt die Maus wieder innerhalb der Schwelle um den Anfang, wird neu entschieden.
    pub fn bewegen(&mut self, m: P, schwelle: f32) {
        if self.klickbetrieb || self.schiene.is_some() {
            return;
        }
        let d = m.sub(self.start());
        if d.x.abs() <= schwelle && d.y.abs() <= schwelle {
            self.richtung = None;
        } else if self.richtung.is_none() {
            self.richtung = Some(vorherrschend(d));
        }
    }

    /// Leitungsverlauf bis `ende` (Vorschau und Ergebnis): beim Ziehen ein Knick nach der Zugrichtung, im
    /// Klickbetrieb die gesetzten Punkte und ein Knick nach der Regel des Originals zum Ende.
    pub fn weg(&self, ende: P) -> Vec<P> {
        let mut f = self.clone();
        f.anfang_zu(ende);
        let erst = if f.schiene.is_some() && f.pts.len() == 1 { Some(Richtung::Senkrecht) } else { None };
        let mut pts = f.pts.clone();
        let l = pts.pop().unwrap();
        let erst = erst.or(if self.klickbetrieb { None } else { self.richtung });
        pts.extend(knick(l, ende, erst));
        bereinigen(pts)
    }

    /// Eckpunkt-Klick bei `m` im Klickbetrieb (Regel des Originals, `min` = 6 px in mm). Liefert, ob sich die
    /// Punktliste geändert hat.
    pub fn klick(&mut self, m: P, min: f32) -> bool {
        // erster Eckpunkt nach dem Beginn auf einer Schiene: senkrecht unter/über dem Klick von der Schiene ab
        if self.schiene.is_some() && self.pts.len() == 1 {
            if (m.y - self.start().y).abs() < min {
                return false;
            }
            self.anfang_zu(m);
            self.pts.push(P::new(self.start().x, m.y));
            return true;
        }
        let l = self.letzter();
        if (m.x - l.x).abs() < min && (m.y - l.y).abs() < min {
            return false;
        }
        let q = ausrichten(l, m);
        let n = self.pts.len();
        if n >= 2 {
            let v = self.pts[n - 2];
            // setzt das letzte Teilstück gerade fort: dessen Endpunkt verschieben
            if ((v.x - l.x).abs() < 0.01 && (l.x - q.x).abs() < 0.01) || ((v.y - l.y).abs() < 0.01 && (l.y - q.y).abs() < 0.01) {
                self.pts[n - 1] = q;
                if self.pts[n - 1].dist(v) < 0.01 {
                    self.pts.pop();
                }
                return true;
            }
        }
        self.pts.push(q);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn erste_zugrichtung_bestimmt_knick() {
        // Ziel liegt weiter rechts als unten, die Maus geht aber zuerst nach unten: erst senkrecht
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.bewegen(p(0.2, 3.0), 1.0);
        f.bewegen(p(20.0, 5.0), 1.0);
        assert_eq!(f.richtung, Some(Richtung::Senkrecht));
        assert_eq!(f.weg(p(20.0, 5.0)), vec![p(0.0, 0.0), p(0.0, 5.0), p(20.0, 5.0)]);
        // umgekehrt: erst waagerecht, obwohl das Ziel weiter unten liegt
        let mut g = Fuehrung::neu(p(0.0, 0.0));
        g.bewegen(p(3.0, 0.5), 1.0);
        g.bewegen(p(5.0, 20.0), 1.0);
        assert_eq!(g.weg(p(5.0, 20.0)), vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 20.0)]);
    }

    #[test]
    fn kleine_bewegung_entscheidet_nicht() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.bewegen(p(0.5, 0.8), 1.0);
        assert_eq!(f.richtung, None);
        // ohne Richtung: vorherrschende Richtung
        assert_eq!(f.weg(p(10.0, 4.0)), vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 4.0)]);
        assert_eq!(f.weg(p(4.0, 10.0)), vec![p(0.0, 0.0), p(0.0, 10.0), p(4.0, 10.0)]);
    }

    #[test]
    fn umkehren_zum_anfang_entscheidet_neu() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.bewegen(p(5.0, 0.0), 1.0);
        assert_eq!(f.richtung, Some(Richtung::Waagerecht));
        // zurück zum Anschluss und dann nach oben
        f.bewegen(p(0.3, 0.2), 1.0);
        assert_eq!(f.richtung, None);
        f.bewegen(p(0.0, -4.0), 1.0);
        f.bewegen(p(8.0, -10.0), 1.0);
        assert_eq!(f.richtung, Some(Richtung::Senkrecht));
        assert_eq!(f.weg(p(8.0, -10.0)), vec![p(0.0, 0.0), p(0.0, -10.0), p(8.0, -10.0)]);
    }

    #[test]
    fn gerade_ohne_knick() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.bewegen(p(0.0, 5.0), 1.0);
        assert_eq!(f.weg(p(0.0, 12.0)), vec![p(0.0, 0.0), p(0.0, 12.0)]);
        // Richtung senkrecht, Ziel aber auf gleicher Höhe: gerade waagerecht
        assert_eq!(f.weg(p(9.0, 0.0)), vec![p(0.0, 0.0), p(9.0, 0.0)]);
    }

    #[test]
    fn klickbetrieb_wie_original() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.klickbetrieb = true;
        // zu nah: nichts
        assert!(!f.klick(p(0.5, 0.5), 1.5));
        // waagerecht überwiegt → y vom letzten Punkt
        assert!(f.klick(p(10.0, 3.0), 1.5));
        assert_eq!(f.pts, vec![p(0.0, 0.0), p(10.0, 0.0)]);
        // gleiche Richtung weiter: Endpunkt verschoben statt neuer Punkt
        assert!(f.klick(p(15.0, -2.0), 1.5));
        assert_eq!(f.pts, vec![p(0.0, 0.0), p(15.0, 0.0)]);
        // senkrecht überwiegt → x vom letzten Punkt
        assert!(f.klick(p(13.0, 20.0), 1.5));
        assert_eq!(f.pts, vec![p(0.0, 0.0), p(15.0, 0.0), p(15.0, 20.0)]);
        // Gleichstand: waagerecht (Regel des Originals)
        assert_eq!(ausrichten(p(0.0, 0.0), p(4.0, 4.0)), p(4.0, 0.0));
        // Ende an einem Anschluss: Knick nach der vorherrschenden Richtung
        assert_eq!(f.weg(p(30.0, 25.0)), vec![p(0.0, 0.0), p(15.0, 0.0), p(15.0, 20.0), p(30.0, 20.0), p(30.0, 25.0)]);
        // Ende in Verlängerung des letzten Teilstücks: kein überflüssiger Zwischenpunkt
        assert_eq!(f.weg(p(15.0, 30.0)), vec![p(0.0, 0.0), p(15.0, 0.0), p(15.0, 30.0)]);
    }

    #[test]
    fn klick_zurueck_auf_vorigen_punkt_entfernt_teilstueck() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.klickbetrieb = true;
        f.klick(p(10.0, 0.0), 1.0);
        f.klick(p(10.0, 10.0), 1.0);
        f.klick(p(10.0, 0.0), 1.0);
        assert_eq!(f.pts, vec![p(0.0, 0.0), p(10.0, 0.0)]);
    }

    #[test]
    fn bereinigen_entfernt_zwischenpunkte() {
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(0.0, 0.0), p(5.0, 0.0), p(9.0, 0.0), p(9.0, 3.0)]), vec![p(0.0, 0.0), p(9.0, 0.0), p(9.0, 3.0)]);
    }
}
