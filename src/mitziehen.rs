//! Leitungen beim Verschieben von Symbolen und Klemmstellen mitziehen.
//!
//! Endet eine (nicht markierte) Leitung auf einem Anschluss eines verschobenen Objekts, folgt ihr Endpunkt dem
//! Anschluss. War das Endstück waagerecht bzw. senkrecht, bleibt es das: der Nachbarpunkt wird quer mitgeführt
//! oder es entsteht ein Eckpunkt.

use crate::app::App;
use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

/// Abstand, bis zu dem ein Leitungsende als mit einem Anschluss verbunden gilt (mm).
pub const PIN_TOL: f32 = 0.05;
const EPS: f32 = 1e-3;

/// Mitgezogene Leitung: Kennung, Punkte vor dem Verschieben, welche Enden (Anfang, Ende) folgen.
#[derive(Clone, Debug, PartialEq)]
pub struct Mitzieh {
    pub id: u64,
    pub pts: Vec<P>,
    pub ends: [bool; 2],
}

fn waagerecht(a: P, b: P) -> bool {
    (a.y - b.y).abs() < EPS && (a.x - b.x).abs() >= EPS
}

fn senkrecht(a: P, b: P) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() >= EPS
}

/// Doppelte aufeinanderfolgende Punkte entfernen (mindestens zwei Punkte bleiben).
fn bereinigen(pts: Vec<P>) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(pts.len());
    for p in &pts {
        if out.last().is_none_or(|q| q.dist(*p) >= EPS) {
            out.push(*p);
        }
    }
    if out.len() < 2 {
        return vec![pts[0], pts[pts.len() - 1]];
    }
    out
}

/// Neue Punkte einer Leitung, deren Enden `ends` (Anfang, Ende) um `d` verschoben werden. `rund` legt neu
/// entstehende Zwischenpunkte auf das Raster (z. B. `App::snap`).
pub fn follow_wire(pts: &[P], ends: [bool; 2], d: P, rund: &dyn Fn(P) -> P) -> Vec<P> {
    if pts.len() < 2 || !(ends[0] || ends[1]) {
        return pts.to_vec();
    }
    if ends[0] && ends[1] {
        // beide Enden hängen an verschobenen Objekten → ganze Leitung verschieben
        return pts.iter().map(|p| p.add(d)).collect();
    }
    if ends[1] {
        let rev: Vec<P> = pts.iter().rev().copied().collect();
        let mut out = follow_start(&rev, d, rund);
        out.reverse();
        return out;
    }
    follow_start(pts, d, rund)
}

/// Anfangspunkt um `d` verschieben, Endstück möglichst rechtwinklig halten.
fn follow_start(pts: &[P], d: P, rund: &dyn Fn(P) -> P) -> Vec<P> {
    let (p0, p1) = (pts[0], pts[1]);
    let n0 = p0.add(d);
    let (h, v) = (waagerecht(p0, p1), senkrecht(p0, p1));
    let mut out = pts.to_vec();
    out[0] = n0;
    if !h && !v {
        return bereinigen(out);
    }
    // Endstück bleibt ohne Zutun waagerecht/senkrecht (Verschiebung entlang des Stücks)
    if (h && (n0.y - p1.y).abs() < EPS) || (v && (n0.x - p1.x).abs() < EPS) {
        return bereinigen(out);
    }
    if pts.len() == 2 {
        // gerade Leitung: Z-Form, beide Enden behalten ihre Richtung; Knick in der Mitte (auf dem Raster)
        let m = rund(P::new((n0.x + p1.x) / 2.0, (n0.y + p1.y) / 2.0));
        let mid = if h { vec![P::new(m.x, n0.y), P::new(m.x, p1.y)] } else { vec![P::new(n0.x, m.y), P::new(p1.x, m.y)] };
        let mut z = vec![n0];
        z.extend(mid);
        z.push(p1);
        return bereinigen(z);
    }
    // Nachbarpunkt quer mitführen, wenn das folgende Stück dabei seine Richtung behält; sonst Eckpunkt einfügen
    let p2 = pts[2];
    let np1 = if h { P::new(p1.x, n0.y) } else { P::new(n0.x, p1.y) };
    let folgt = if h { senkrecht(p1, p2) } else { waagerecht(p1, p2) };
    if folgt {
        out[1] = np1;
    } else {
        out.insert(1, np1);
    }
    bereinigen(out)
}

/// Leitungen, die an Anschlüssen der verschobenen Symbole/Klemmstellen `moved` enden. Leitungen in `exclude`
/// (Markierung) und fixierte Leitungen (bei aktiver Fixierung) bleiben unverändert.
pub fn find_followers(sheet: &Sheet, lib: &[SymDef], moved: &[u64], exclude: &[u64], fix_active: bool) -> Vec<Mitzieh> {
    // Klemmstellen in einer Leitung: die Teile enden KLEMME_R vor dem Mittelpunkt (klemme_leitung.rs) und zählen
    // trotzdem als angeschlossen – sonst blieb beim Verschieben eine Lücke (todo 1, Maus-Befund M4)
    let pins: Vec<(P, f32)> = sheet
        .objects
        .iter()
        .filter(|o| moved.contains(&o.id) && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Verweis { .. }))
        .flat_map(|o| {
            let tol = if matches!(o.kind, Kind::Terminal { .. }) { crate::klemme_leitung::KLEMME_R + PIN_TOL } else { PIN_TOL };
            canvas::obj_pins(o, lib).into_iter().map(move |q| (q, tol))
        })
        .collect();
    if pins.is_empty() {
        return vec![];
    }
    let am_pin = |p: P| pins.iter().any(|(q, t)| q.dist(p) <= *t);
    sheet
        .objects
        .iter()
        .filter(|o| !exclude.contains(&o.id) && !moved.contains(&o.id) && !(o.fixed && fix_active))
        .filter_map(|o| {
            let Kind::Wire { pts, .. } = &o.kind else { return None };
            if pts.len() < 2 {
                return None;
            }
            let ends = [am_pin(pts[0]), am_pin(pts[pts.len() - 1])];
            (ends[0] || ends[1]).then(|| Mitzieh { id: o.id, pts: pts.clone(), ends })
        })
        .collect()
}

/// Liegt bei `p` eine in eine Leitung eingesetzte Klemmstelle (mindestens zwei Leitungsenden am Klemmenkreis)?
pub fn eingesetzte_klemme(sheet: &Sheet, p: P) -> bool {
    let tol = crate::klemme_leitung::KLEMME_R + PIN_TOL;
    let klemme = sheet.objects.iter().any(|o| matches!(&o.kind, Kind::Terminal { pos, .. } if pos.dist(p) <= PIN_TOL));
    if !klemme {
        return false;
    }
    let enden = sheet
        .objects
        .iter()
        .filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None })
        .flat_map(|pts| [pts.first().copied(), pts.last().copied()])
        .flatten()
        .filter(|e| e.dist(p) <= tol)
        .count();
    enden >= 2
}

impl App {
    /// Mitzuziehende Leitungen für die verschobenen Objekte `moved` (Zustand vor dem Verschieben).
    pub fn wire_followers(&self, moved: &[u64]) -> Vec<Mitzieh> {
        match self.sheet() {
            Some(s) => find_followers(s, &self.lib, moved, &self.selection, self.fix_active),
            None => vec![],
        }
    }

    /// Mitgezogene Leitungen auf die Gesamtverschiebung `d` (seit `wire_followers`) setzen. Ohne eigenen
    /// Rückgängig-Schritt (gehört zum Verschieben).
    pub fn apply_followers(&mut self, f: &[Mitzieh], d: P) {
        if f.is_empty() {
            return;
        }
        let neu: Vec<(u64, Vec<P>, bool)> = f.iter().map(|w| (w.id, follow_wire(&w.pts, w.ends, d, &|p| self.snap(p)), w.ends[0] && w.ends[1])).collect();
        self.with_sheet(|s, _| {
            for (id, pts, ganz) in neu {
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                    if let Kind::Wire { pts: p, bez, .. } = &mut o.kind {
                        // ganz verschobene Leitung: Bezeichnungen mitnehmen (relativ zum alten Anfang)
                        if ganz {
                            let dd = pts[0].sub(p[0]);
                            bez.iter_mut().for_each(|(q, _)| *q = q.add(dd));
                        }
                        *p = pts;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(p: P) -> P {
        p
    }

    fn eq(a: &[P], b: &[P]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.dist(*y) < 1e-4)
    }

    #[test]
    fn gerade_leitung_wird_z_foermig() {
        let pts = [P::new(0.0, 0.0), P::new(50.0, 0.0)];
        // Symbol am Anfang 10 mm nach unten: waagerechte Enden bleiben, Knick in der Mitte
        let r = follow_wire(&pts, [true, false], P::new(0.0, 10.0), &id);
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(25.0, 10.0), P::new(25.0, 0.0), P::new(50.0, 0.0)]), "{r:?}");
        // Verschiebung entlang der Leitung: nur der Endpunkt wandert
        let r = follow_wire(&pts, [true, false], P::new(5.0, 0.0), &id);
        assert!(eq(&r, &[P::new(5.0, 0.0), P::new(50.0, 0.0)]), "{r:?}");
        // Ende statt Anfang, senkrechte Leitung
        let pts = [P::new(0.0, 0.0), P::new(0.0, 40.0)];
        let r = follow_wire(&pts, [false, true], P::new(10.0, 0.0), &id);
        assert!(eq(&r, &[P::new(0.0, 0.0), P::new(0.0, 20.0), P::new(10.0, 20.0), P::new(10.0, 40.0)]), "{r:?}");
    }

    #[test]
    fn nachbarpunkt_wird_mitgefuehrt_oder_ecke_eingefuegt() {
        // L-Form: waagerecht, dann senkrecht → Eckpunkt wandert senkrecht mit
        let pts = [P::new(0.0, 0.0), P::new(30.0, 0.0), P::new(30.0, 40.0)];
        let r = follow_wire(&pts, [true, false], P::new(0.0, 10.0), &id);
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(30.0, 10.0), P::new(30.0, 40.0)]), "{r:?}");
        // waagerecht, waagerecht weiter → neuer Eckpunkt, der alte Punkt bleibt
        let pts = [P::new(0.0, 0.0), P::new(30.0, 0.0), P::new(60.0, 0.0)];
        let r = follow_wire(&pts, [true, false], P::new(0.0, 10.0), &id);
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(30.0, 10.0), P::new(30.0, 0.0), P::new(60.0, 0.0)]), "{r:?}");
        // schräges Endstück: nur der Endpunkt folgt
        let pts = [P::new(0.0, 0.0), P::new(30.0, 30.0)];
        let r = follow_wire(&pts, [true, false], P::new(0.0, 10.0), &id);
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(30.0, 30.0)]), "{r:?}");
    }

    #[test]
    fn beide_enden_verschieben_die_ganze_leitung() {
        let pts = [P::new(0.0, 0.0), P::new(30.0, 0.0), P::new(30.0, 40.0)];
        let r = follow_wire(&pts, [true, true], P::new(5.0, 5.0), &id);
        assert!(eq(&r, &[P::new(5.0, 5.0), P::new(35.0, 5.0), P::new(35.0, 45.0)]));
        assert!(eq(&follow_wire(&pts, [false, false], P::new(5.0, 5.0), &id), &pts));
    }

    #[test]
    fn knick_auf_raster_und_zusammenfallende_punkte() {
        let raster = |p: P| P::new((p.x / 5.0).round() * 5.0, (p.y / 5.0).round() * 5.0);
        let pts = [P::new(0.0, 0.0), P::new(40.0, 0.0)];
        // Mitte bei 21,5 → Raster 20
        let r = follow_wire(&pts, [true, false], P::new(3.0, 10.0), &raster);
        assert!(eq(&r, &[P::new(3.0, 10.0), P::new(20.0, 10.0), P::new(20.0, 0.0), P::new(40.0, 0.0)]), "{r:?}");
        // Punkt fällt auf den Nachbarn: doppelte Punkte entfallen
        let pts = [P::new(0.0, 0.0), P::new(30.0, 0.0), P::new(30.0, 10.0)];
        let r = follow_wire(&pts, [true, false], P::new(0.0, 10.0), &id);
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(30.0, 10.0)]), "{r:?}");
    }

    #[test]
    fn mitzuziehende_leitungen_finden() {
        let lib = crate::symbols::builtin();
        let mut s = Sheet::new(1);
        let sym = |id: u64, pos: P| Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
        let wire = |id: u64, a: P, b: P| Obj { id, kind: Kind::Wire { pts: vec![a, b], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None };
        // Schließer bei (50, 100): Anschlüsse (50, 93) und (50, 107)
        s.objects.push(sym(1, P::new(50.0, 100.0)));
        s.objects.push(wire(2, P::new(50.0, 60.0), P::new(50.0, 93.0)));
        s.objects.push(wire(3, P::new(50.0, 107.0), P::new(50.0, 150.0)));
        s.objects.push(wire(4, P::new(80.0, 60.0), P::new(80.0, 93.0)));
        let mut fixed = wire(5, P::new(50.0, 93.0), P::new(10.0, 93.0));
        fixed.fixed = true;
        s.objects.push(fixed);
        let f = find_followers(&s, &lib, &[1], &[1], true);
        assert_eq!(f.iter().map(|m| (m.id, m.ends)).collect::<Vec<_>>(), vec![(2, [false, true]), (3, [true, false])]);
        // markierte Leitung bewegt sich selbst und folgt nicht
        let f = find_followers(&s, &lib, &[1], &[1, 2], false);
        assert_eq!(f.iter().map(|m| m.id).collect::<Vec<_>>(), vec![3, 5]);
    }
}

#[cfg(test)]
mod tests_klemme {
    use super::*;

    #[test]
    fn eingesetzte_klemmstelle_zieht_beide_teile_mit() {
        let mut s = Sheet::new(1);
        let wire = |id: u64, pts: Vec<P>| Obj { id, kind: Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None };
        s.objects.push(wire(1, vec![P::new(10.0, 50.0), P::new(29.2, 50.0)]));
        s.objects.push(wire(2, vec![P::new(30.8, 50.0), P::new(60.0, 50.0)]));
        s.objects.push(Obj { id: 3, kind: Kind::Terminal { pos: P::new(30.0, 50.0), leiste: "X1".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        assert!(eingesetzte_klemme(&s, P::new(30.0, 50.0)));
        let f = find_followers(&s, &[], &[3], &[3], false);
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|m| m.ends[0] != m.ends[1]));
    }
}
