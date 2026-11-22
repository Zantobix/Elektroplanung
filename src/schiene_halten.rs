//! Leitungen bleiben an der Sammelschiene (Nutzer-Befunde 22.11.2026):
//!
//! - **Abgriff wandert mit:** Wird das Teilstück einer Leitung verschoben, die an einer Sammelschiene beginnt
//!   (oder ein angeschlossenes Symbol), lief die Leitung bisher ein Stück auf der Schiene entlang und der
//!   Verbindungspunkt blieb an der alten Stelle. Jetzt wandert der Abgriff auf der Schiene bis dorthin, wo die
//!   Leitung die Schiene verlässt – keine Leitung liegt auf der Schiene.
//! - **Kein Abreißen:** Wird viel markiert und verschoben (Leitungen ja, Schiene nicht), riss eine Leitung von der
//!   Schiene ab. Jetzt bleibt ihr Ende auf der Schiene; das Endstück wird länger bzw. kürzer.

use crate::model::{Kind, Sheet, P};

const EPS: f32 = 0.01;

/// Waagerechte Sammelschiene: Höhe und Ausdehnung (y, x1, x2) mit x1 ≤ x2.
pub type Schiene = (f32, f32, f32);

/// Alle Sammelschienen des Blattes außer `ohne`.
pub fn schienen(sheet: &Sheet, ohne: &[u64]) -> Vec<Schiene> {
    sheet
        .objects
        .iter()
        .filter(|o| !ohne.contains(&o.id))
        .filter_map(|o| match &o.kind {
            Kind::Busbar { a, x2, .. } => Some((a.y, a.x.min(*x2), a.x.max(*x2))),
            _ => None,
        })
        .collect()
}

fn auf(p: P, s: &Schiene) -> bool {
    (p.y - s.0).abs() < EPS && p.x >= s.1 - EPS && p.x <= s.2 + EPS
}

/// Schiene, auf der `p` liegt.
pub fn schiene_bei(p: P, sch: &[Schiene]) -> Option<Schiene> {
    sch.iter().find(|s| auf(p, s)).copied()
}

/// Läuft der Anfang bzw. das Ende der Leitung auf einer Schiene entlang, beginnt sie dort, wo sie die Schiene
/// verlässt (der Abgriff wandert mit). Eine Leitung, die ganz auf der Schiene liegt, bleibt unverändert.
pub fn abgreifen(pts: &[P], sch: &[Schiene]) -> Vec<P> {
    let mut v = crate::leitung_fuehrung::bereinigen(pts.to_vec());
    for _ in 0..2 {
        while v.len() > 2 {
            let Some(s) = schiene_bei(v[0], sch) else { break };
            if !auf(v[1], &s) {
                break;
            }
            v.remove(0);
        }
        v.reverse();
    }
    v
}

/// Ende `p0` (Nachbarpunkt `p1`, beide schon verschoben) wieder an die Schiene `s` binden: Punkte, die vor `p0`
/// einzusetzen sind bzw. `p0` ersetzen. Senkrechtes Endstück: nur `p0` rückt auf die Schiene.
fn binden(v: &mut Vec<P>, s: Schiene) {
    let p0 = v[0];
    let x = p0.x.clamp(s.1, s.2);
    let neu = P::new(x, s.0);
    if neu.dist(p0) < EPS {
        return;
    }
    let senkrecht = v.len() >= 2 && (v[1].x - p0.x).abs() < EPS;
    if senkrecht && (x - p0.x).abs() < EPS {
        v[0] = neu;
    } else if (x - p0.x).abs() < EPS {
        v.insert(0, neu);
    } else {
        // über das Schienenende hinaus: senkrecht auf das Schienenende, dann waagerecht
        v.insert(0, P::new(x, p0.y));
        v.insert(0, neu);
    }
}

/// Leitung einer verschobenen Markierung, deren Enden an nicht verschobenen Schienen hängen.
#[derive(Clone, Debug)]
pub struct Halt {
    pub id: u64,
    pub pts: Vec<P>,
    pub schienen: [Option<Schiene>; 2],
}

/// Leitungen unter `moved`, deren Anfang oder Ende auf einer Schiene liegt, die nicht mit verschoben wird.
pub fn finden(sheet: &Sheet, moved: &[u64]) -> Vec<Halt> {
    let sch = schienen(sheet, moved);
    if sch.is_empty() {
        return vec![];
    }
    sheet
        .objects
        .iter()
        .filter(|o| moved.contains(&o.id))
        .filter_map(|o| {
            let Kind::Wire { pts, .. } = &o.kind else { return None };
            if pts.len() < 2 {
                return None;
            }
            let schienen = [schiene_bei(pts[0], &sch), schiene_bei(pts[pts.len() - 1], &sch)];
            (schienen[0].is_some() || schienen[1].is_some()).then(|| Halt { id: o.id, pts: pts.clone(), schienen })
        })
        .collect()
}

/// Punkte der Leitung nach dem Verschieben um `d` (gesamt): Enden an den Schienen bleiben dort.
pub fn verschoben(h: &Halt, d: P, alle: &[Schiene]) -> Vec<P> {
    let mut v: Vec<P> = h.pts.iter().map(|p| p.add(d)).collect();
    if let Some(s) = h.schienen[0] {
        binden(&mut v, s);
    }
    if let Some(s) = h.schienen[1] {
        v.reverse();
        binden(&mut v, s);
        v.reverse();
    }
    abgreifen(&v, alle)
}

impl crate::app::App {
    /// Markierte Leitungen nach dem Verschieben um `d` (gesamt seit `finden`) wieder an ihre Schienen binden.
    pub fn schienen_halten(&mut self, halt: &[Halt], d: P) {
        if halt.is_empty() {
            return;
        }
        let alle = self.sheet().map(|s| schienen(s, &[])).unwrap_or_default();
        let neu: Vec<(u64, Vec<P>)> = halt.iter().map(|h| (h.id, verschoben(h, d, &alle))).collect();
        self.with_sheet(|s, _| {
            for (id, pts) in neu {
                if let Some(Kind::Wire { pts: p, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *p = pts;
                }
            }
        });
    }

    /// Abgriff der Leitung `id` auf der Schiene nachführen (nach Teilstück-Ziehen bzw. Mitziehen).
    pub fn abgriff_nachfuehren(&mut self, ids: &[u64]) {
        let Some(alle) = self.sheet().map(|s| schienen(s, &[])) else { return };
        if alle.is_empty() {
            return;
        }
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| ids.contains(&o.id)) {
                if let Kind::Wire { pts, .. } = &mut o.kind {
                    let neu = abgreifen(pts, &alle);
                    if neu.len() != pts.len() {
                        *pts = neu;
                    }
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

    const L2: Schiene = (20.0, 10.0, 300.0);

    #[test]
    fn abgriff_wandert_bis_zum_verlassen() {
        // Bild des Nutzers: Abgriff bei x=80, Leitung läuft auf L2 bis x=120 und dann nach unten
        let pts = [p(80.0, 20.0), p(120.0, 20.0), p(120.0, 150.0), p(60.0, 150.0)];
        assert_eq!(abgreifen(&pts, &[L2]), vec![p(120.0, 20.0), p(120.0, 150.0), p(60.0, 150.0)]);
        // auch am Ende
        let rev: Vec<P> = pts.iter().rev().copied().collect();
        assert_eq!(abgreifen(&rev, &[L2]), vec![p(60.0, 150.0), p(120.0, 150.0), p(120.0, 20.0)]);
        // Leitung abseits der Schiene bleibt
        let frei = [p(80.0, 30.0), p(120.0, 30.0), p(120.0, 150.0)];
        assert_eq!(abgreifen(&frei, &[L2]), frei.to_vec());
    }

    #[test]
    fn teilstueck_ziehen_mit_abgriff() {
        // senkrechtes erstes Stück von der Schiene nach rechts gezogen: Querstück auf der Schiene entfällt
        let alt = [p(80.0, 20.0), p(80.0, 150.0), p(60.0, 150.0)];
        let neu = crate::segment_ziehen::verschieben(&alt, 0, p(40.0, 0.0));
        assert_eq!(abgreifen(&neu, &[L2]), vec![p(120.0, 20.0), p(120.0, 150.0), p(60.0, 150.0)]);
    }

    #[test]
    fn markierung_verschoben_bleibt_an_schiene() {
        let h = Halt { id: 1, pts: vec![p(80.0, 20.0), p(80.0, 150.0), p(60.0, 150.0)], schienen: [Some(L2), None] };
        // schräg nach rechts unten: senkrechtes Stück bleibt senkrecht, Anfang auf der Schiene
        let v = verschoben(&h, p(10.0, 30.0), &[L2]);
        assert_eq!(v, vec![p(90.0, 20.0), p(90.0, 180.0), p(70.0, 180.0)]);
        // nach oben über die Schiene hinaus: bleibt trotzdem angeschlossen
        let v = verschoben(&h, p(0.0, -10.0), &[L2]);
        assert_eq!(v[0], p(80.0, 20.0));
    }

    #[test]
    fn ueber_das_schienenende_hinaus() {
        let h = Halt { id: 1, pts: vec![p(290.0, 20.0), p(290.0, 100.0)], schienen: [Some(L2), None] };
        let v = verschoben(&h, p(30.0, 10.0), &[L2]);
        assert_eq!(v[0], p(300.0, 20.0));
        assert!(v.windows(2).all(|w| (w[0].x - w[1].x).abs() < EPS || (w[0].y - w[1].y).abs() < EPS), "{v:?}");
        assert_eq!(*v.last().unwrap(), p(320.0, 110.0));
    }
}
