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

/// Leitung einer verschobenen Markierung, deren Enden an nicht verschobenen Schienen bzw. Anschlüssen hängen.
#[derive(Clone, Debug, Default)]
pub struct Halt {
    pub id: u64,
    pub pts: Vec<P>,
    pub schienen: [Option<Schiene>; 2],
    /// Ende an einem festen Punkt (Anschluss eines nicht verschobenen Symbols, Kreis einer nicht verschobenen
    /// Klemmstelle, nicht verschobene Leitung): bleibt dort (Nutzer-Befund Runde 11, Leitung löste sich von der
    /// Klemmstelle)
    pub punkte: [Option<P>; 2],
}

/// Ende `v[0]` (schon verschoben) wieder mit dem festen Punkt `q` verbinden. `alt` = Richtung des ersten Stücks
/// vor dem Verschieben (vom Ende weg). Läuft die Verschiebung vom Anschluss weg, wird das Endstück in seiner
/// Richtung länger und ein Querstück schließt an; sonst entsteht das Querstück am Anschluss und das Endstück wird
/// kürzer (Rückläufer verschmelzen in `bereinigen`).
fn anbinden(v: &mut Vec<P>, q: P, alt: Option<P>) {
    let p0 = v[0];
    let d = p0.sub(q);
    if d.x.abs() < EPS && d.y.abs() < EPS {
        return;
    }
    match alt {
        Some(r) => {
            let entlang = d.x * r.x + d.y * r.y;
            let ecke = if entlang >= 0.0 { q.add(P::new(r.x * entlang, r.y * entlang)) } else { q.add(P::new(d.x - r.x * entlang, d.y - r.y * entlang)) };
            v.insert(0, ecke);
            v.insert(0, q);
        }
        None => v.insert(0, q),
    }
}

/// Achsenrichtung des ersten Stücks (Einheitsvektor) oder `None` (schräg bzw. Länge 0).
fn achse(a: P, b: P) -> Option<P> {
    let d = b.sub(a);
    if d.x.abs() < EPS && d.y.abs() >= EPS {
        Some(P::new(0.0, d.y.signum()))
    } else if d.y.abs() < EPS && d.x.abs() >= EPS {
        Some(P::new(d.x.signum(), 0.0))
    } else {
        None
    }
}

/// Leitungen unter `moved`, deren Anfang oder Ende auf einer Schiene bzw. an einem festen Anschluss liegt, der
/// nicht mit verschoben wird.
pub fn finden(sheet: &Sheet, lib: &[crate::symbols::SymDef], moved: &[u64]) -> Vec<Halt> {
    use crate::mitziehen::PIN_TOL;
    let sch = schienen(sheet, moved);
    let fest: Vec<&crate::model::Obj> = sheet.objects.iter().filter(|o| !moved.contains(&o.id)).collect();
    let pins: Vec<P> = fest.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| crate::canvas::obj_pins(o, lib)).collect();
    let klemmen: Vec<P> = fest.iter().filter_map(|o| if let Kind::Terminal { pos, .. } = &o.kind { Some(*pos) } else { None }).collect();
    let r = crate::klemme_leitung::KLEMME_R + PIN_TOL;
    let fester_punkt = |e: P| -> bool {
        pins.iter().any(|q| q.dist(e) <= PIN_TOL)
            || klemmen.iter().any(|k| k.dist(e) <= r)
            || fest.iter().any(|o| match &o.kind {
                // Abzweig auf einer festen Leitung; Ende an Ende zählt nicht (die andere Leitung folgt, mitziehen.rs)
                Kind::Wire { pts, .. } if pts.len() >= 2 => {
                    e.dist(pts[0]) > PIN_TOL && e.dist(pts[pts.len() - 1]) > PIN_TOL && pts.windows(2).any(|w| crate::model::seg_dist(e, w[0], w[1]) <= PIN_TOL)
                }
                _ => false,
            })
    };
    sheet
        .objects
        .iter()
        .filter(|o| moved.contains(&o.id))
        .filter_map(|o| {
            let Kind::Wire { pts, .. } = &o.kind else { return None };
            if pts.len() < 2 {
                return None;
            }
            let enden = [pts[0], pts[pts.len() - 1]];
            let schienen = enden.map(|e| schiene_bei(e, &sch));
            let punkte = [0, 1].map(|i| (schienen[i].is_none() && fester_punkt(enden[i])).then_some(enden[i]));
            let halt = schienen.iter().any(|s| s.is_some()) || punkte.iter().any(|p| p.is_some());
            halt.then(|| Halt { id: o.id, pts: pts.clone(), schienen, punkte })
        })
        .collect()
}

/// Punkte der Leitung nach dem Verschieben um `d` (gesamt): Enden an den Schienen bleiben dort.
pub fn verschoben(h: &Halt, d: P, alle: &[Schiene]) -> Vec<P> {
    let n = h.pts.len();
    let mut v: Vec<P> = h.pts.iter().map(|p| p.add(d)).collect();
    if let Some(s) = h.schienen[0] {
        binden(&mut v, s);
    } else if let Some(q) = h.punkte[0] {
        anbinden(&mut v, q, achse(h.pts[0], h.pts[1]));
    }
    v.reverse();
    if let Some(s) = h.schienen[1] {
        binden(&mut v, s);
    } else if let Some(q) = h.punkte[1] {
        anbinden(&mut v, q, achse(h.pts[n - 1], h.pts[n - 2]));
    }
    v.reverse();
    abgreifen(&v, alle)
}

/// Achsengerades Gleis, auf dem ein Abgriff wandern darf: Sammelschiene oder gerades Stück einer Leitung
/// (Leitung als „Schiene“, z. B. die Ausgänge „-F2.L“/„-F2.N“ in W2, an denen Abzweige mit Knoten hängen).
#[derive(Clone, Copy, Debug)]
pub struct Gleis {
    pub id: u64,
    pub a: P,
    pub b: P,
    pub schiene: bool,
}

/// Alle Gleise des Blattes: Sammelschienen und achsengerade Stücke aller Leitungen.
pub fn gleise(sheet: &Sheet) -> Vec<Gleis> {
    let mut v = vec![];
    for o in &sheet.objects {
        match &o.kind {
            Kind::Busbar { a, x2, .. } => v.push(Gleis { id: o.id, a: P::new(a.x.min(*x2), a.y), b: P::new(a.x.max(*x2), a.y), schiene: true }),
            Kind::Wire { pts, .. } => {
                for w in pts.windows(2) {
                    if (w[0].x - w[1].x).abs() < EPS || (w[0].y - w[1].y).abs() < EPS {
                        v.push(Gleis { id: o.id, a: w[0], b: w[1], schiene: false });
                    }
                }
            }
            _ => {}
        }
    }
    v
}

fn waag(g: &Gleis) -> bool {
    (g.a.y - g.b.y).abs() < EPS
}

/// Liegt `p` auf dem Gleis? Bei Leitungen nur im Innern des Stücks (an einem Ende/Knick hängt dort sonst eine
/// Verbindung, die beim Wandern verloren ginge).
fn auf_gleis(p: P, g: &Gleis, rand: f32) -> bool {
    let r = if g.schiene { -EPS } else { rand };
    if waag(g) {
        (p.y - g.a.y).abs() < EPS && p.x > g.a.x.min(g.b.x) + r && p.x < g.a.x.max(g.b.x) - r
    } else {
        (p.x - g.a.x).abs() < EPS && p.y > g.a.y.min(g.b.y) + r && p.y < g.a.y.max(g.b.y) - r
    }
}

/// Überdeckt die Strecke `a`–`b` (achsengerade) ein Stück eines Gleises der Länge > 0,5 mm?
fn ueberdeckt(a: P, b: P, g: &Gleis) -> bool {
    let ov = |p0: f32, p1: f32, q0: f32, q1: f32| p0.max(p1).min(q0.max(q1)) - p0.min(p1).max(q0.min(q1));
    if (a.x - b.x).abs() < EPS && !waag(g) && (a.x - g.a.x).abs() < EPS {
        ov(a.y, b.y, g.a.y, g.b.y) > 0.5
    } else if (a.y - b.y).abs() < EPS && waag(g) && (a.y - g.a.y).abs() < EPS {
        ov(a.x, b.x, g.a.x, g.b.x) > 0.5
    } else {
        false
    }
}

/// Abgriff wandert mit (Nutzer 22.11.2026): Wird das Symbol am einen Ende einer Leitung verschoben und hängt das
/// andere Ende als Abzweig auf einer Schiene oder einer anderen Leitung, die quer zur Anschlussrichtung liegt, wird
/// die Leitung zur geraden Verbindung vom Anschluss auf das Gleis – der Abgriff (Knoten) wandert mit, statt dass
/// die Leitung zum alten Abgriff zurück- und dabei über andere Leitungen läuft. `None`, wenn das nicht geht
/// (Anschluss zeigt nicht auf das Gleis, Lotpunkt außerhalb, gerade Strecke läge auf einer anderen Leitung).
pub fn gerade_zum_gleis(id: u64, pts: &[P], ends: [bool; 2], d: P, dirs: [Option<P>; 2], gl: &[Gleis]) -> Option<Vec<P>> {
    if pts.len() < 2 || ends[0] == ends[1] {
        return None;
    }
    let k = if ends[0] { 0 } else { 1 };
    let (p, q) = if k == 0 { (pts[0].add(d), pts[pts.len() - 1]) } else { (pts[pts.len() - 1].add(d), pts[0]) };
    let dir = dirs[k]?;
    let g = gl.iter().find(|g| g.id != id && auf_gleis(q, g, 0.5))?;
    let ziel = if waag(g) {
        if dir.x.abs() > EPS || (g.a.y - p.y) * dir.y <= EPS {
            return None;
        }
        P::new(p.x, g.a.y)
    } else {
        if dir.y.abs() > EPS || (g.a.x - p.x) * dir.x <= EPS {
            return None;
        }
        P::new(g.a.x, p.y)
    };
    if !auf_gleis(ziel, g, 0.5) || gl.iter().any(|h| h.id != id && ueberdeckt(p, ziel, h)) {
        return None;
    }
    Some(if k == 0 { vec![p, ziel] } else { vec![ziel, p] })
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
        let h = Halt { id: 1, pts: vec![p(80.0, 20.0), p(80.0, 150.0), p(60.0, 150.0)], schienen: [Some(L2), None], ..Default::default() };
        // schräg nach rechts unten: senkrechtes Stück bleibt senkrecht, Anfang auf der Schiene
        let v = verschoben(&h, p(10.0, 30.0), &[L2]);
        assert_eq!(v, vec![p(90.0, 20.0), p(90.0, 180.0), p(70.0, 180.0)]);
        // nach oben über die Schiene hinaus: bleibt trotzdem angeschlossen
        let v = verschoben(&h, p(0.0, -10.0), &[L2]);
        assert_eq!(v[0], p(80.0, 20.0));
    }

    #[test]
    fn ueber_das_schienenende_hinaus() {
        let h = Halt { id: 1, pts: vec![p(290.0, 20.0), p(290.0, 100.0)], schienen: [Some(L2), None], ..Default::default() };
        let v = verschoben(&h, p(30.0, 10.0), &[L2]);
        assert_eq!(v[0], p(300.0, 20.0));
        assert!(v.windows(2).all(|w| (w[0].x - w[1].x).abs() < EPS || (w[0].y - w[1].y).abs() < EPS), "{v:?}");
        assert_eq!(*v.last().unwrap(), p(320.0, 110.0));
    }

    #[test]
    fn abzweig_auf_leitung_wandert_mit() {
        // W2 „Sicherungen -F10, -F1 und -F2“: -F2:L (97.5,97.5) → Abzweig auf -F2.L (y=102); -F2 um 29.5 mm nach rechts
        let schiene = Gleis { id: 9, a: p(60.0, 102.0), b: p(200.0, 102.0), schiene: false };
        let n = Gleis { id: 8, a: p(60.0, 107.0), b: p(200.0, 107.0), schiene: false };
        let pts = [p(97.5, 97.5), p(97.5, 102.0), p(102.5, 102.0)];
        let dirs = [Some(p(0.0, 1.0)), Some(p(1.0, 0.0))];
        let r = gerade_zum_gleis(1, &pts, [true, false], p(29.5, 0.0), dirs, &[schiene, n]);
        assert_eq!(r, Some(vec![p(127.0, 97.5), p(127.0, 102.0)]));
        // Anschluss zeigt vom Gleis weg → nichts
        assert_eq!(gerade_zum_gleis(1, &pts, [true, false], p(29.5, 0.0), [Some(p(0.0, -1.0)), None], &[schiene]), None);
        // Abgriff am Ende der anderen Leitung (Eckverbindung) → nichts
        let eck = Gleis { id: 9, a: p(102.5, 102.0), b: p(200.0, 102.0), schiene: false };
        assert_eq!(gerade_zum_gleis(1, &pts, [true, false], p(29.5, 0.0), dirs, &[eck]), None);
        // gerade Strecke läge auf einer anderen Leitung → nichts
        let senk = Gleis { id: 5, a: p(127.0, 90.0), b: p(127.0, 110.0), schiene: false };
        assert_eq!(gerade_zum_gleis(1, &pts, [true, false], p(29.5, 0.0), dirs, &[schiene, senk]), None);
    }
}
