//! Knickpunkt einer Leitung greifen und ziehen (Metrik-Lücke 3, todo „Zeiger: Knickpunkte von Leitungen“).
//!
//! Original: Die Trefferprüfung der Leitungen (0x474028, beim Überfahren aus MouseMove 0x41138b ohne Ausgabepunkt
//! gerufen) prüft zuerst jeden **inneren** Eckpunkt (nicht Anfang und Ende) in einem Kästchen ± Fangweite um den
//! Punkt; trifft sie, setzt sie Treffer 2 (Leitung) und F+0x954 = Nummer des Eckpunkts (ab 1). Die Zeigerwahl
//! (0x415f10) gibt dann Zeiger 5 (Ressource 2, Pfeil mit Raute und vier Pfeilspitzen) statt des Verschiebekreuzes.
//! Beim Drücken wird die Nummer nach F+0x960 übernommen; MouseMove (0x40c473) bearbeitet die Leitung nur, wenn sie
//! gesetzt ist.
//!
//! Nachbau: Ziehen am Knick verschiebt den Eckpunkt frei; die beiden anliegenden Teilstücke werden wie beim
//! Teilstück-Ziehen (segment_ziehen.rs) quer zu ihrer Richtung mitgeführt, die Leitungsenden bleiben an ihren
//! Anschlüssen. Wie genau das Original den Knick führt, ist noch im Original zu prüfen.

use crate::model::P;
use crate::segment_ziehen::quer;

const EPS: f32 = 1e-3;

/// Innerer Eckpunkt von `pts` im Kästchen ± `tol` um `m` (der nächstgelegene).
pub fn knick_bei(pts: &[P], m: P, tol: f32) -> Option<usize> {
    if pts.len() < 3 {
        return None;
    }
    (1..pts.len() - 1)
        .filter(|&i| (pts[i].x - m.x).abs() <= tol && (pts[i].y - m.y).abs() <= tol)
        .min_by(|&i, &j| pts[i].dist(m).total_cmp(&pts[j].dist(m)))
}

fn gleich(a: P, b: P) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

/// Punkte nach dem Verschieben des inneren Eckpunkts `i` von `orig` um `d`: erst das Teilstück davor, dann das
/// danach, jeweils quer zu seiner Richtung (`segment_ziehen::verschieben`).
pub fn knick_verschieben(orig: &[P], i: usize, d: P) -> Vec<P> {
    knick_verschieben_frei(orig, i, d, [false, false])
}

/// Wie `knick_verschieben`; freie Leitungsenden (`frei`) wandern mit (segment_ziehen::verschieben_frei).
pub fn knick_verschieben_frei(orig: &[P], i: usize, d: P, frei: [bool; 2]) -> Vec<P> {
    if i == 0 || i + 1 >= orig.len() {
        return orig.to_vec();
    }
    let d1 = quer(orig[i - 1], orig[i], d);
    let pts1 = if d1.x.abs() < EPS && d1.y.abs() < EPS { orig.to_vec() } else { crate::segment_ziehen::verschieben_frei(orig, i - 1, d1, frei) };
    // der Knick nach dem ersten Schritt: dort beginnt das zweite Teilstück (zum alten Nachbarpunkt)
    let v = P::new(orig[i].x + d1.x, orig[i].y + d1.y);
    let Some(j) = (0..pts1.len().saturating_sub(1)).find(|&j| gleich(pts1[j], v) && gleich(pts1[j + 1], orig[i + 1])) else { return pts1 };
    let d2 = quer(pts1[j], pts1[j + 1], d);
    if d2.x.abs() < EPS && d2.y.abs() < EPS {
        return pts1;
    }
    crate::segment_ziehen::verschieben_frei(&pts1, j, d2, frei)
}

impl crate::app::App {
    /// Innerer Knick der Leitung `id` unter der Maus, wenn sie sich ziehen lässt (nicht fixiert bei aktiver Fixierung).
    pub fn knick_unter(&self, id: u64, m: P, tol: f32) -> Option<usize> {
        let o = self.sheet()?.objects.iter().find(|o| o.id == id)?;
        if o.fixed && self.fix_active {
            return None;
        }
        match &o.kind {
            crate::model::Kind::Wire { pts, .. } => knick_bei(pts, m, tol),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn nur_innere_eckpunkte() {
        let pts = [p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)];
        assert_eq!(knick_bei(&pts, p(20.5, 0.4), 1.0), Some(1));
        assert_eq!(knick_bei(&pts, p(0.0, 0.0), 1.0), None);
        assert_eq!(knick_bei(&pts, p(20.0, 20.0), 1.0), None);
        assert_eq!(knick_bei(&pts, p(10.0, 0.0), 1.0), None);
        assert_eq!(knick_bei(&pts[..2], p(20.0, 0.0), 1.0), None);
    }

    #[test]
    fn l_knick_frei_verschieben() {
        // L-Leitung, Knick nach rechts unten: Enden bleiben, Querstücke an den festen Enden
        let neu = knick_verschieben(&[p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)], 1, p(5.0, 5.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(0.0, 5.0), p(25.0, 5.0), p(25.0, 20.0), p(20.0, 20.0)]);
    }

    #[test]
    fn z_knick_verlaengert_nachbarn() {
        // Knick in der Mitte einer längeren Leitung: Nachbarstücke werden gestreckt, kein neues Querstück
        let alt = [p(0.0, 0.0), p(0.0, 10.0), p(30.0, 10.0), p(30.0, 30.0), p(50.0, 30.0)];
        let neu = knick_verschieben(&alt, 2, p(5.0, -4.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(0.0, 6.0), p(35.0, 6.0), p(35.0, 30.0), p(50.0, 30.0)]);
    }

    #[test]
    fn nur_eine_richtung() {
        let alt = [p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)];
        // waagerecht gezogen: nur das senkrechte Stück wandert
        assert_eq!(knick_verschieben(&alt, 1, p(5.0, 0.0)), vec![p(0.0, 0.0), p(25.0, 0.0), p(25.0, 20.0), p(20.0, 20.0)]);
        assert_eq!(knick_verschieben(&alt, 1, p(0.0, 0.0)), alt.to_vec());
    }
}
