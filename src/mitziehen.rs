//! Leitungen beim Verschieben von Symbolen und Klemmstellen mitziehen.
//!
//! Endet eine (nicht markierte) Leitung auf einem Anschluss eines verschobenen Objekts, folgt ihr Endpunkt dem
//! Anschluss. Die Form bleibt nach dem Profil-Verfahren (leitung_profil.rs, Vorbild QElectroTech): Längenänderung
//! je Achse auf die Teilstücke dieser Achse verteilt, Knicke und Rechtwinkligkeit bleiben. Liegt das Ziel danach in
//! einem anderen Quadranten oder liefe die Leitung in ein Symbol hinein, wird sie mit Stich + Z neu geführt
//! (leitung_route.rs). Gerade Leitungen, schräge Endstücke und Leitungen mit fremden Anschlüssen im Innern
//! (T-Abzweig, Knoten) folgen wie bisher nur mit dem Endstück: war es waagerecht bzw. senkrecht, bleibt es das –
//! der Nachbarpunkt wird quer mitgeführt oder es entsteht ein Eckpunkt.

use crate::app::App;
use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

/// Abstand, bis zu dem ein Leitungsende als mit einem Anschluss verbunden gilt (mm).
pub const PIN_TOL: f32 = 0.05;
const EPS: f32 = 1e-3;

/// Mitgezogene Leitung: Kennung, Punkte vor dem Verschieben, welche Enden (Anfang, Ende) folgen, Richtung nach
/// außen an den Enden (Symbolanschluss bzw. Klemmstelle, für die Neuführung) und ob das Profil gilt (keine fremden
/// Anschlüsse im Innern der Leitung).
#[derive(Clone, Debug, PartialEq)]
pub struct Mitzieh {
    pub id: u64,
    pub pts: Vec<P>,
    pub ends: [bool; 2],
    pub dirs: [Option<P>; 2],
    pub profil: bool,
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

/// Neue Punkte einer Leitung, deren Enden `ends` (Anfang, Ende) um `d` verschoben werden (Profil, ohne
/// Anschlussrichtungen). `rund` legt neu entstehende Zwischenpunkte auf das Raster (z. B. `App::snap`).
pub fn follow_wire(pts: &[P], ends: [bool; 2], d: P, rund: &dyn Fn(P) -> P) -> Vec<P> {
    folgen(pts, ends, d, [None, None], true, 0.0, rund)
}

/// Wie `follow_wire`, mit Richtung nach außen an den Enden (`dirs`, für Neuführung und die Prüfung „läuft in das
/// Symbol“), Profil an/aus und Stichlänge `stich` der Neuführung.
pub fn folgen(pts: &[P], ends: [bool; 2], d: P, dirs: [Option<P>; 2], profil: bool, stich: f32, rund: &dyn Fn(P) -> P) -> Vec<P> {
    if pts.len() < 2 || !(ends[0] || ends[1]) {
        return pts.to_vec();
    }
    if ends[0] && ends[1] {
        // beide Enden hängen an verschobenen Objekten → ganze Leitung verschieben
        return pts.iter().map(|p| p.add(d)).collect();
    }
    if ends[1] {
        let rev: Vec<P> = pts.iter().rev().copied().collect();
        let mut out = folgen_anfang(&rev, d, [dirs[1], dirs[0]], profil, stich, rund);
        out.reverse();
        return out;
    }
    folgen_anfang(pts, d, dirs, profil, stich, rund)
}

/// Anfang um `d` verschieben: Profil, sonst Neuführung bzw. Endstück (siehe Modulkommentar).
fn folgen_anfang(pts: &[P], d: P, dirs: [Option<P>; 2], profil: bool, stich: f32, rund: &dyn Fn(P) -> P) -> Vec<P> {
    use crate::leitung_profil::{nachfuehren, Ergebnis};
    use crate::leitung_route::{laeuft_hinein, mit_stichen};
    // doppelte Punkte (in W2 z. B. am Anfang) würden das Endstück verdecken
    let pts = &bereinigen(pts.to_vec());
    let neu = pts[0].add(d);
    let ende = pts[pts.len() - 1];
    if !profil {
        return follow_start(pts, d, rund);
    }
    let neu_fuehren = || mit_stichen(neu, dirs[0], ende, dirs[1], stich);
    let out = match nachfuehren(pts, neu, rund) {
        Ergebnis::Form(p) => p,
        Ergebnis::Neufuehren => return neu_fuehren(),
        Ergebnis::Unpassend => follow_start(pts, d, rund),
    };
    // lief die Leitung vorher sauber aus den Anschlüssen heraus, jetzt aber hinein (Stück auf 0 geschrumpft): neu
    if laeuft_hinein(&out, dirs[0], dirs[1]) && !laeuft_hinein(pts, dirs[0], dirs[1]) {
        return neu_fuehren();
    }
    out
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
    let am_pin = |p: P| pins.iter().position(|(q, t)| q.dist(p) <= *t);
    // fremde Anschlusspunkte: Enden anderer Leitungen und Symbolanschlüsse (Abzweig/Knoten im Innern einer Leitung)
    let anker: Vec<(u64, P)> = sheet
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Wire { .. } | Kind::Symbol { .. }))
        .flat_map(|o| canvas::obj_pins(o, lib).into_iter().map(move |q| (o.id, q)))
        .collect();
    sheet
        .objects
        .iter()
        .filter(|o| !exclude.contains(&o.id) && !moved.contains(&o.id) && !(o.fixed && fix_active))
        .filter_map(|o| {
            let Kind::Wire { pts, .. } = &o.kind else { return None };
            if pts.len() < 2 {
                return None;
            }
            let (a, e) = (pts[0], pts[pts.len() - 1]);
            let (ia, ie) = (am_pin(a), am_pin(e));
            let mut ends = [ia.is_some(), ie.is_some()];
            // sehr kurze Leitung mit beiden Enden im Kreis derselben Klemme (W2: 0,3 mm von der Klemme zur Schiene):
            // nur das Ende am Kreis (0,8 mm vor dem Mittelpunkt, wie es das Original speichert) hängt an der Klemme
            if let (Some(i), Some(j)) = (ia, ie) {
                if i == j {
                    let r = crate::klemme_leitung::KLEMME_R;
                    let naeher_anfang = (pins[i].0.dist(a) - r).abs() <= (pins[i].0.dist(e) - r).abs();
                    ends = [naeher_anfang, !naeher_anfang];
                }
            }
            if !(ends[0] || ends[1]) {
                return None;
            }
            let innen = |q: P| q.dist(a) > PIN_TOL && q.dist(e) > PIN_TOL && pts.windows(2).any(|w| seg_dist(q, w[0], w[1]) <= PIN_TOL);
            let profil = !anker.iter().any(|(id, q)| *id != o.id && innen(*q));
            // Nachbarpunkt = erster vom Ende verschiedene Punkt (doppelte Punkte überspringen)
            let nb = |mut it: Box<dyn Iterator<Item = &P> + '_>, p: P| it.find(|q| q.dist(p) >= EPS).copied().unwrap_or(p);
            let dirs = [ende_richtung(sheet, lib, a, nb(Box::new(pts.iter()), a)), ende_richtung(sheet, lib, e, nb(Box::new(pts.iter().rev()), e))];
            Some(Mitzieh { id: o.id, pts: pts.clone(), ends, dirs, profil })
        })
        .collect()
}

/// Richtung nach außen am Leitungsende `p` (Nachbarpunkt `nb`): am Symbolanschluss dessen Richtung, sonst die
/// Richtung des Endstücks (an einer Klemmstelle tritt die Leitung so wieder in derselben Flucht an die Klemme, an
/// einer anderen Leitung bleibt der Abzweig erhalten); schräges Endstück: keine.
fn ende_richtung(sheet: &Sheet, lib: &[SymDef], p: P, nb: P) -> Option<P> {
    if let Some(r) = crate::leitung_route::anschluss_richtung(sheet, lib, p) {
        return Some(r);
    }
    let (dx, dy) = (nb.x - p.x, nb.y - p.y);
    if dy.abs() < EPS && dx.abs() >= EPS {
        Some(P::new(dx.signum(), 0.0))
    } else if dx.abs() < EPS && dy.abs() >= EPS {
        Some(P::new(0.0, dy.signum()))
    } else {
        None
    }
}

/// Kennung einer in eine Leitung eingesetzten Klemmstelle, deren Kreis `p` berührt (Mittelpunkt oder eines der
/// Leitungsenden 0,8 mm davor): Klemme und beide Teilleitungen verhalten sich beim Greifen wie eine Einheit.
pub fn eingesetzte_klemme_bei(sheet: &Sheet, p: P) -> Option<u64> {
    let tol = crate::klemme_leitung::KLEMME_R + PIN_TOL;
    sheet.objects.iter().find_map(|o| match &o.kind {
        Kind::Terminal { pos, .. } if pos.dist(p) <= tol && eingesetzte_klemme(sheet, *pos) => Some(o.id),
        _ => None,
    })
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
        // Stich der Neuführung wie beim Zeichnen ein halbes Raster (leitung_route.rs)
        let stich = self.raster() / 2.0;
        let neu: Vec<(u64, Vec<P>, bool)> =
            f.iter().map(|w| (w.id, folgen(&w.pts, w.ends, d, w.dirs, w.profil, stich, &|p| self.snap(p)), w.ends[0] && w.ends[1])).collect();
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

#[cfg(test)]
mod tests_profil {
    use super::*;

    fn eq(a: &[P], b: &[P]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.dist(*y) < 1e-4)
    }

    fn rechtwinklig(p: &[P]) -> bool {
        crate::leitung_profil::profil(p).is_some()
    }

    fn wire(id: u64, pts: Vec<P>) -> Obj {
        Obj { id, kind: Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn schliesser(id: u64, pos: P) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn klemme(id: u64, pos: P) -> Obj {
        Obj { id, kind: Kind::Terminal { pos, leiste: "X1".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    /// Objekte `moved` um `d` verschieben und die Leitungen wie beim Ziehen mitführen; Rückgabe: Leitungspunkte je Id.
    fn ziehen(s: &mut Sheet, lib: &[SymDef], moved: &[u64], d: P) -> Vec<(u64, Vec<P>)> {
        let f = find_followers(s, lib, moved, moved, false);
        s.objects.iter_mut().filter(|o| moved.contains(&o.id)).for_each(|o| o.kind.translate(d));
        let mut out = vec![];
        for w in &f {
            let neu = folgen(&w.pts, w.ends, d, w.dirs, w.profil, 2.5, &|p| p);
            if let Some(Obj { kind: Kind::Wire { pts, .. }, .. }) = s.objects.iter_mut().find(|o| o.id == w.id) {
                *pts = neu.clone();
            }
            out.push((w.id, neu));
        }
        out
    }

    // Z-Leitung vom unteren Anschluss (50, 107) des Schließers bei (50, 100)
    fn z_blatt() -> Sheet {
        let mut s = Sheet::new(1);
        s.objects.push(schliesser(1, P::new(50.0, 100.0)));
        s.objects.push(wire(2, vec![P::new(50.0, 107.0), P::new(50.0, 117.0), P::new(70.0, 117.0), P::new(70.0, 130.0), P::new(110.0, 130.0), P::new(110.0, 150.0)]));
        s
    }

    #[test]
    fn symbol_waagerecht_verschoben_knicke_bleiben() {
        let lib = crate::symbols::builtin();
        let mut s = z_blatt();
        let r = ziehen(&mut s, &lib, &[1], P::new(10.0, 0.0));
        // Breite 60 → 50: beide waagerechten Stücke (20, 40) je −5; senkrechte unverändert
        let soll = [P::new(60.0, 107.0), P::new(60.0, 117.0), P::new(75.0, 117.0), P::new(75.0, 130.0), P::new(110.0, 130.0), P::new(110.0, 150.0)];
        assert!(eq(&r[0].1, &soll), "{:?}", r[0].1);
    }

    #[test]
    fn symbol_senkrecht_verschoben_laengen_verteilt() {
        let lib = crate::symbols::builtin();
        let mut s = z_blatt();
        let r = ziehen(&mut s, &lib, &[1], P::new(0.0, -6.0));
        // Höhe 43 → 49: drei senkrechte Stücke (10, 13, 20) je +2
        let soll = [P::new(50.0, 101.0), P::new(50.0, 113.0), P::new(70.0, 113.0), P::new(70.0, 128.0), P::new(110.0, 128.0), P::new(110.0, 150.0)];
        assert!(eq(&r[0].1, &soll), "{:?}", r[0].1);
    }

    #[test]
    fn quadrantenwechsel_fuehrt_neu_mit_stich() {
        let lib = crate::symbols::builtin();
        let mut s = z_blatt();
        // Symbol rechts über das Ziel hinaus: Profil passt nicht mehr → Stich nach unten, dann Z zum Ziel
        let r = &ziehen(&mut s, &lib, &[1], P::new(80.0, 0.0))[0].1;
        assert_eq!(r[0], P::new(130.0, 107.0));
        assert_eq!(r[1], P::new(130.0, 109.5), "{r:?}");
        assert_eq!(*r.last().unwrap(), P::new(110.0, 150.0));
        assert!(rechtwinklig(r), "{r:?}");
    }

    #[test]
    fn abzweig_im_innern_nur_endstueck() {
        let lib = crate::symbols::builtin();
        let mut s = z_blatt();
        // T-Abzweig: fremde Leitung endet im Innern → Profil aus, Verhalten wie bisher (nur Endstück)
        s.objects.push(wire(3, vec![P::new(90.0, 130.0), P::new(90.0, 160.0)]));
        let f = find_followers(&s, &lib, &[1], &[1], false);
        assert_eq!(f.len(), 1);
        assert!(!f[0].profil);
        let r = ziehen(&mut s, &lib, &[1], P::new(10.0, 0.0));
        let soll = [P::new(60.0, 107.0), P::new(60.0, 117.0), P::new(70.0, 117.0), P::new(70.0, 130.0), P::new(110.0, 130.0), P::new(110.0, 150.0)];
        assert!(eq(&r[0].1, &soll), "{:?}", r[0].1);
    }

    fn klemme_in_leitung() -> Sheet {
        // L-Leitung (10, 50) → (60, 50) → (60, 80), Klemmstelle bei (30, 50) eingesetzt
        let mut s = Sheet::new(1);
        let (p1, p2) = crate::klemme_leitung::teilen(&[P::new(10.0, 50.0), P::new(60.0, 50.0), P::new(60.0, 80.0)], 0, P::new(30.0, 50.0));
        s.objects.push(wire(1, p1));
        s.objects.push(wire(2, p2));
        s.objects.push(klemme(3, P::new(30.0, 50.0)));
        s
    }

    #[test]
    fn klemme_in_leitung_beide_teile_folgen_ohne_luecke() {
        for d in [P::new(0.0, 10.0), P::new(5.0, -10.0), P::new(-15.0, 0.0), P::new(40.0, 5.0)] {
            let mut s = klemme_in_leitung();
            let r = ziehen(&mut s, &[], &[3], d);
            assert_eq!(r.len(), 2);
            let k = P::new(30.0, 50.0).add(d);
            let a = r.iter().find(|(id, _)| *id == 1).unwrap().1.clone();
            let b = r.iter().find(|(id, _)| *id == 2).unwrap().1.clone();
            // beide Teile enden weiter 0,8 mm vor der Klemme, waagerecht in derselben Flucht
            assert!(a.last().unwrap().dist(P::new(k.x - 0.8, k.y)) < 1e-4, "{d:?} {a:?}");
            assert!(b[0].dist(P::new(k.x + 0.8, k.y)) < 1e-4, "{d:?} {b:?}");
            assert!((a[a.len() - 2].y - k.y).abs() < 1e-4 && (b[1].y - k.y).abs() < 1e-4, "{d:?} {a:?} {b:?}");
            // ferne Enden bleiben
            assert_eq!(a[0], P::new(10.0, 50.0));
            assert_eq!(*b.last().unwrap(), P::new(60.0, 80.0));
            assert!(rechtwinklig(&a) && rechtwinklig(&b), "{d:?} {a:?} {b:?}");
            assert!(eingesetzte_klemme(&s, k));
        }
    }

    #[test]
    fn klemme_greifen_auch_an_den_teilenden() {
        let s = klemme_in_leitung();
        for p in [P::new(30.0, 50.0), P::new(29.2, 50.0), P::new(30.8, 50.0)] {
            assert_eq!(eingesetzte_klemme_bei(&s, p), Some(3), "{p:?}");
        }
        assert_eq!(eingesetzte_klemme_bei(&s, P::new(10.0, 50.0)), None);
        // freie Klemme (nur eine Leitung) zählt nicht
        let mut s = Sheet::new(1);
        s.objects.push(klemme(3, P::new(30.0, 50.0)));
        s.objects.push(wire(1, vec![P::new(10.0, 50.0), P::new(29.2, 50.0)]));
        assert_eq!(eingesetzte_klemme_bei(&s, P::new(29.2, 50.0)), None);
    }
}
