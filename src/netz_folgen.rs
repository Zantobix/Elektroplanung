//! Leitungsnetz nachführen – ein Verfahren für jede Änderung (Runde 13, Nutzerwunsch „das ganze Leitungsthema
//! generalisieren“).
//!
//! Vorher gab es je Bearbeitung eine eigene Behandlung (Verschieben: `mitziehen`, Teilstück/Knick ziehen: keine,
//! Skalieren/Drehen: `anschluss_folgen`, markierte Leitungen an Schienen: `schiene_halten`, Pfeiltasten: wieder
//! eigene Aufrufe). Dabei fehlten Fälle, z. B. folgte eine Leitung, die als Abzweig auf einer anderen endet, nicht,
//! wenn man ein Teilstück der anderen zog (Nutzerbild 23.11.2026).
//!
//! Jetzt gilt für **jede** Änderung dasselbe:
//! 1. Vor der Änderung wird der Stand des Blattes gemerkt (`vorher`), dazu die Objekte, die der Nutzer direkt
//!    ändert (`primaer`: verschobene/gedrehte/skalierte Objekte, die gezogene Leitung).
//! 2. Nach der Änderung bildet ein **Abbild** alte auf neue Lagen ab: Anschlüsse der geänderten Objekte (je
//!    Anschluss, also auch beim Drehen und Skalieren), Teilstücke geänderter Leitungen, Sammelschienen.
//! 3. Jedes Leitungsende, das vorher an einer dieser Lagen hing, bekommt sein Ziel aus dem Abbild; die Leitung wird
//!    mit dem gemeinsamen Verfahren geführt (`fuehren`: ganz verschieben, Form halten, kurze Neuführung, Ausweichen).
//!    Geführte Leitungen kommen selbst ins Abbild – Abzweige auf ihnen folgen ebenso (wiederholt bis zur Ruhe).
//! 4. Primär verschobene Leitungen, deren Enden an etwas hingen, das **nicht** mitbewegt wurde (Anschluss, Klemme,
//!    andere Leitung, Schiene), bleiben dort angeschlossen (`halten`).
//!
//! Gerechnet wird immer vom Stand `vorher` aus (jedes Bild beim Ziehen neu), so bleiben keine Reste früherer Bilder.

use crate::canvas;
use crate::mitziehen::{abzweig_lage, folgen_um, PIN_TOL};
use crate::model::*;
use crate::symbols::SymDef;

const EPS: f32 = 1e-3;

/// Einstellungen des Nachführens.
pub struct NetzOpt<'a> {
    /// Stich der Neuführung (halbes Raster)
    pub stich: f32,
    /// primär verschobene Leitungen an ihren festen Anschlüssen halten (Verschieben, Pfeiltasten)
    pub halten: bool,
    /// fixierte Leitungen folgen nicht
    pub fix_active: bool,
    /// neu entstehende innere Knicke auf das Raster
    pub rund: &'a dyn Fn(P) -> P,
}

/// Abbildung alter auf neue Lagen der geänderten Objekte.
#[derive(Default, Debug, Clone)]
pub struct Abbild {
    /// Objekt, alter Anschluss, neuer Anschluss, Fangabstand
    pins: Vec<(u64, P, P, f32)>,
    /// Leitung: alte und neue Punkte
    wires: Vec<(u64, Vec<P>, Vec<P>)>,
    /// Sammelschiene: alte und neue Strecke (links, rechts)
    schienen: Vec<(u64, (P, P), (P, P))>,
}

fn schiene_strecke(k: &Kind) -> Option<(P, P)> {
    match k {
        Kind::Busbar { a, x2, .. } => Some((P::new(a.x.min(*x2), a.y), P::new(a.x.max(*x2), a.y))),
        _ => None,
    }
}

fn pin_tol(k: &Kind) -> f32 {
    if matches!(k, Kind::Terminal { .. }) {
        crate::klemme_leitung::KLEMME_R + PIN_TOL
    } else {
        PIN_TOL
    }
}

impl Abbild {
    /// Ein geändertes Objekt aufnehmen (Zustand vorher `o0`, nachher `o1`).
    pub fn aufnehmen(&mut self, o0: &Obj, o1: &Obj, lib: &[SymDef]) {
        match (&o0.kind, &o1.kind) {
            (Kind::Wire { pts: p0, .. }, Kind::Wire { pts: p1, .. }) => {
                if p0 != p1 {
                    self.wires.push((o0.id, p0.clone(), p1.clone()));
                }
            }
            (k0, k1) if schiene_strecke(k0).is_some() => {
                if let (Some(a), Some(b)) = (schiene_strecke(k0), schiene_strecke(k1)) {
                    self.schienen.push((o0.id, a, b));
                }
            }
            _ => {
                let (v0, v1) = (canvas::obj_pins(o0, lib), canvas::obj_pins(o1, lib));
                if v0.len() == v1.len() {
                    let t = pin_tol(&o0.kind);
                    self.pins.extend(v0.into_iter().zip(v1).filter(|(p, q)| p.dist(*q) > 1e-6).map(|(p, q)| (o0.id, p, q, t)));
                }
            }
        }
    }

    /// Neue Lage des Leitungsendes `e` der Leitung `eigen`, wenn es vorher an einer abgebildeten Lage hing.
    pub fn ziel(&self, eigen: u64, e: P) -> Option<P> {
        let pin = self.pins.iter().filter(|(_, p, _, t)| p.dist(e) <= *t).min_by(|x, y| x.1.dist(e).total_cmp(&y.1.dist(e)));
        if let Some((_, p, q, _)) = pin {
            return Some(e.add(q.sub(*p)));
        }
        for (id, alt, neu) in &self.wires {
            if *id != eigen && alt.windows(2).any(|s| seg_dist(e, s[0], s[1]) <= PIN_TOL) {
                return Some(lage_auf(alt, neu, e));
            }
        }
        for (_, (a0, b0), (a1, b1)) in &self.schienen {
            if (e.y - a0.y).abs() <= PIN_TOL && e.x >= a0.x - PIN_TOL && e.x <= b0.x + PIN_TOL {
                // gleiche Länge: mitverschieben; sonst x behalten (auf die neue Strecke begrenzt)
                return Some(if ((b0.x - a0.x) - (b1.x - a1.x)).abs() < EPS { e.add(a1.sub(*a0)) } else { P::new(e.x.clamp(a1.x, b1.x), a1.y) });
            }
        }
        None
    }
}

fn waag(a: P, b: P) -> bool {
    (a.y - b.y).abs() < EPS
}

fn senk(a: P, b: P) -> bool {
    (a.x - b.x).abs() < EPS
}

/// Lage des Punkts `e` (auf dem Zug `alt`) auf dem geänderten Zug `neu`:
/// - liegt das alte Teilstück noch in derselben Flucht und enthält `e` (Nachbarstück nur gestreckt): bleibt `e`;
/// - gleiche Punktzahl: gleiches Teilstück, gleicher Anteil (ein Knick bleibt ein Knick);
/// - sonst das nächste gleichgerichtete Teilstück (z. B. das verschobene Teilstück), `e` darauf gelotet.
pub fn lage_auf(alt: &[P], neu: &[P], e: P) -> P {
    let Some(i) = (0..alt.len().saturating_sub(1)).find(|&i| seg_dist(e, alt[i], alt[i + 1]) <= PIN_TOL) else {
        return abzweig_lage(alt, neu, e);
    };
    let (a, b) = (alt[i], alt[i + 1]);
    let (h, v) = (waag(a, b) && !senk(a, b), senk(a, b) && !waag(a, b));
    // Ende genau an einem Eckpunkt: dem Eckpunkt folgen (gleiche Punktzahl) bzw. Knick-Regel unten
    if neu.windows(2).any(|s| seg_dist(e, s[0], s[1]) <= PIN_TOL && ((h && waag(s[0], s[1])) || (v && senk(s[0], s[1])))) && alt.len() != neu.len() {
        return e;
    }
    if alt.len() == neu.len() {
        let (c, d) = (neu[i], neu[i + 1]);
        // Flucht unverändert und e noch darauf: bleibt
        if seg_dist(e, c, d) <= PIN_TOL && ((h && waag(c, d) && (c.y - a.y).abs() < EPS) || (v && senk(c, d) && (c.x - a.x).abs() < EPS)) {
            return e;
        }
        // beide Enden des Stücks gleich verschoben: mitverschieben
        let (da, db) = (c.sub(a), d.sub(b));
        if da.dist(db) < EPS {
            return e.add(da);
        }
        // gestrecktes bzw. quer verschobenes Stück gleicher Richtung: Lot (Lage entlang des Stücks bleibt, auf das
        // Stück begrenzt) – nicht anteilig, sonst landet der Abzweig neben dem Raster (W2 Blatt 27: Kette von
        // Abzweigen wanderte auf y = 114,807)
        if h && waag(c, d) && !senk(c, d) {
            return P::new(e.x.clamp(c.x.min(d.x), c.x.max(d.x)), c.y);
        }
        if v && senk(c, d) && !waag(c, d) {
            return P::new(c.x, e.y.clamp(c.y.min(d.y), c.y.max(d.y)));
        }
        return abzweig_lage(alt, neu, e);
    }
    if !(h || v) {
        return abzweig_lage(alt, neu, e);
    }
    let mut best: Option<(f32, P)> = None;
    for s in neu.windows(2) {
        let (c, d) = (s[0], s[1]);
        let passt = (h && waag(c, d) && !senk(c, d)) || (v && senk(c, d) && !waag(c, d));
        if !passt {
            continue;
        }
        let q = if h { P::new(e.x.clamp(c.x.min(d.x), c.x.max(d.x)), c.y) } else { P::new(c.x, e.y.clamp(c.y.min(d.y), c.y.max(d.y))) };
        let k = q.dist(e);
        if best.is_none_or(|(b, _)| k < b) {
            best = Some((k, q));
        }
    }
    best.map(|(_, q)| q).unwrap_or_else(|| abzweig_lage(alt, neu, e))
}

/// Richtung nach außen am Leitungsende `p` (Nachbarpunkt `nb`): am Symbolanschluss dessen Richtung, sonst die
/// Richtung des Endstücks; schräges Endstück: keine.
fn ende_richtung(sheet: &Sheet, lib: &[SymDef], p: P, nb: P) -> Option<P> {
    if let Some(r) = crate::leitung_route::anschluss_richtung(sheet, lib, p) {
        return Some(r);
    }
    let d = nb.sub(p);
    if d.y.abs() < EPS && d.x.abs() >= EPS {
        Some(P::new(d.x.signum(), 0.0))
    } else if d.x.abs() < EPS && d.y.abs() >= EPS {
        Some(P::new(0.0, d.y.signum()))
    } else {
        None
    }
}

fn nachbar(pts: &[P], ende: usize) -> P {
    let e = if ende == 0 { pts[0] } else { pts[pts.len() - 1] };
    let it: Box<dyn Iterator<Item = &P>> = if ende == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
    it.copied().find(|q| q.dist(e) >= EPS).unwrap_or(e)
}

/// Strecken aller Leitungen und Schienen außer `ohne`.
fn strecken_ohne(sheet: &Sheet, ohne: u64) -> Vec<(P, P)> {
    sheet
        .objects
        .iter()
        .filter(|o| o.id != ohne)
        .flat_map(|o| match &o.kind {
            Kind::Wire { pts, .. } => pts.windows(2).map(|v| (v[0], v[1])).collect::<Vec<_>>(),
            Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
            _ => vec![],
        })
        .collect()
}

/// Ergebnis der Führung einer Leitung: neue Punkte und – wenn sie ganz verschoben wurde – die Verschiebung.
pub struct Gefuehrt {
    pub pts: Vec<P>,
    pub ganz: Option<P>,
}

/// Leitung `id` (Punkte vorher `alt`) mit den Enden an `ziel` führen. `vorher`/`jetzt` = Blatt vor bzw. nach der
/// Änderung (für Richtungen, Hindernisse, andere Leitungen).
#[allow(clippy::too_many_arguments)]
pub fn fuehren(vorher: &Sheet, jetzt: &Sheet, lib: &[SymDef], id: u64, alt: &[P], ziel: [Option<P>; 2], opt: &NetzOpt) -> Option<Gefuehrt> {
    use crate::leitung_hindernis as lh;
    let n = alt.len();
    if n < 2 {
        return None;
    }
    let enden = [alt[0], alt[n - 1]];
    let d = [0, 1].map(|i| ziel[i].map(|q| q.sub(enden[i])).unwrap_or_default());
    if (0..2).all(|i| ziel[i].is_none() || (d[i].x.abs() < 1e-5 && d[i].y.abs() < 1e-5)) {
        return None;
    }
    // beide Enden um dieselbe Strecke: ganze Leitung verschieben
    if ziel[0].is_some() && ziel[1].is_some() && d[0].dist(d[1]) < EPS {
        return Some(Gefuehrt { pts: alt.iter().map(|p| p.add(d[0])).collect(), ganz: Some(d[0]) });
    }
    let alt_dirs = [ende_richtung(vorher, lib, enden[0], nachbar(alt, 0)), ende_richtung(vorher, lib, enden[1], nachbar(alt, 1))];
    let dirs = [0, 1].map(|i| match ziel[i] {
        Some(q) => crate::leitung_route::anschluss_richtung(jetzt, lib, q).or(alt_dirs[i]),
        None => alt_dirs[i],
    });
    // Profil nur ohne fremde Anschlüsse im Innern (T-Abzweig, Knoten), sonst nur das Endstück
    let innen = |q: P| q.dist(enden[0]) > PIN_TOL && q.dist(enden[1]) > PIN_TOL && alt.windows(2).any(|w| seg_dist(q, w[0], w[1]) <= PIN_TOL);
    let profil = !vorher.objects.iter().filter(|o| o.id != id && matches!(o.kind, Kind::Wire { .. } | Kind::Symbol { .. })).flat_map(|o| canvas::obj_pins(o, lib)).any(innen);
    let gedreht = (0..2).any(|i| ziel[i].is_some() && matches!((alt_dirs[i], dirs[i]), (Some(a), Some(b)) if a.x * b.x + a.y * b.y < 0.5) && crate::leitung_route::anschluss_richtung(vorher, lib, enden[i]).is_some());
    let (a, b) = (ziel[0].unwrap_or(enden[0]), ziel[1].unwrap_or(enden[1]));
    let umrisse = lh::umrisse(jetzt, lib);
    let hind = lh::wirksam(&lh::hindernisse(&umrisse, &[], &[]), a, b);
    let andere = strecken_ohne(jetzt, id);
    let sch = crate::schiene_halten::schienen(jetzt, &[]);
    if gedreht {
        // Richtung eines Anschlusses gedreht (Drehen/Spiegeln): einfacher Weg, sonst Stich + Z um Symbole herum
        let pts = crate::leitung_kurz::bester(a, dirs[0], b, dirs[1], opt.stich, &hind, &andere).unwrap_or_else(|| {
            let weg = crate::leitung_route::mit_stichen(a, dirs[0], b, dirs[1], opt.stich);
            lh::umgehen(&weg, dirs[0], dirs[1], &hind, 2.0 * opt.stich, false)
        });
        return Some(Gefuehrt { pts: crate::schiene_halten::abgreifen(&pts, &sch), ganz: None });
    }
    // ein Ende bewegt, das andere hängt als Abzweig quer auf einem Gleis: gerade aufs Gleis (Abgriff wandert mit)
    let bewegt = [ziel[0].is_some() && d[0].dist(P::default()) > 1e-5, ziel[1].is_some() && d[1].dist(P::default()) > 1e-5];
    if bewegt[0] != bewegt[1] {
        let k = if bewegt[0] { 0 } else { 1 };
        let gl = crate::schiene_halten::gleise(jetzt);
        if let Some(g) = crate::schiene_halten::gerade_zum_gleis(id, alt, bewegt, d[k], alt_dirs, &gl).filter(|g| lh::kreuzungen(g, &hind) == 0) {
            return Some(Gefuehrt { pts: g, ganz: None });
        }
    }
    let mut p = alt.to_vec();
    for (i, ends) in [(0, [true, false]), (1, [false, true])] {
        if bewegt[i] {
            p = folgen_um(&p, ends, d[i], dirs, profil, opt.stich, &hind, opt.rund);
        }
    }
    let p = crate::schiene_halten::abgreifen(&p, &sch);
    // einfacher Weg mit weniger Knicken, wenn er nichts verschlechtert (leitung_kurz.rs)
    let p = crate::leitung_kurz::kuerzer(&p, dirs[0], dirs[1], opt.stich, &hind, &andere).unwrap_or(p);
    Some(Gefuehrt { pts: p, ganz: None })
}

/// Wo hängt das Ende `e` der primär geänderten Leitung `id` fest (vor der Änderung, an einem Objekt, das sich nicht
/// mitbewegt)? Schiene: das Ende gleitet auf ihr; Punkt: das Ende bleibt dort.
enum Fest {
    Schiene(crate::schiene_halten::Schiene),
    Punkt(P),
}

fn fest_an(vorher: &Sheet, lib: &[SymDef], bewegt: &[u64], id: u64, e: P) -> Option<Fest> {
    for o in vorher.objects.iter().filter(|o| o.id != id && !bewegt.contains(&o.id)) {
        match &o.kind {
            Kind::Busbar { a, x2, .. } if (e.y - a.y).abs() < 0.01 && e.x >= a.x.min(*x2) - 0.01 && e.x <= a.x.max(*x2) + 0.01 => {
                return Some(Fest::Schiene((a.y, a.x.min(*x2), a.x.max(*x2))));
            }
            Kind::Symbol { .. } | Kind::Verweis { .. } if canvas::obj_pins(o, lib).iter().any(|q| q.dist(e) <= PIN_TOL) => return Some(Fest::Punkt(e)),
            Kind::Terminal { pos, .. } if pos.dist(e) <= crate::klemme_leitung::KLEMME_R + PIN_TOL => return Some(Fest::Punkt(e)),
            // Abzweig auf einer festen Leitung; Ende an Ende zählt nicht (die andere Leitung folgt)
            Kind::Wire { pts, .. } if pts.len() >= 2 && e.dist(pts[0]) > PIN_TOL && e.dist(pts[pts.len() - 1]) > PIN_TOL && pts.windows(2).any(|w| seg_dist(e, w[0], w[1]) <= PIN_TOL) => {
                return Some(Fest::Punkt(e));
            }
            _ => {}
        }
    }
    None
}

/// Leitungsnetz nach einer Änderung nachführen. `vorher` = alle Objekte des Blattes vor der Änderung, `s` = Blatt
/// danach (primäre Objekte schon geändert), `primaer` = vom Nutzer direkt geänderte Objekte. Alle anderen
/// Leitungen werden aus ihrem Stand `vorher` neu berechnet. Rückgabe: geführte Leitungen.
pub fn nachfuehren(vorher: &[Obj], s: &mut Sheet, lib: &[SymDef], primaer: &[u64], opt: &NetzOpt) -> Vec<u64> {
    let vs = Sheet { objects: vorher.to_vec(), ..Sheet::new(s.number) };
    // Folger früherer Bilder zurücksetzen
    for o in s.objects.iter_mut().filter(|o| !primaer.contains(&o.id)) {
        if let (Kind::Wire { pts, bez, .. }, Some(Obj { kind: Kind::Wire { pts: p0, bez: b0, .. }, .. })) = (&mut o.kind, vorher.iter().find(|x| x.id == o.id)) {
            if pts != p0 {
                *pts = p0.clone();
                *bez = b0.clone();
            }
        }
    }
    // primär gehaltene Leitungen an ihren festen Anschlüssen (vor dem Abbild, damit Abzweige auf ihnen folgen)
    if opt.halten {
        halten(&vs, s, lib, primaer);
    }
    let mut basis = Abbild::default();
    for id in primaer {
        if let (Some(o0), Some(o1)) = (vorher.iter().find(|o| o.id == *id), s.objects.iter().find(|o| o.id == *id)) {
            basis.aufnehmen(o0, o1, lib);
        }
    }
    let folger_von = |o: &Obj| !primaer.contains(&o.id) && !(o.fixed && opt.fix_active) && matches!(&o.kind, Kind::Wire { pts, .. } if pts.len() >= 2);
    let mut folger: Vec<(u64, Gefuehrt)> = vec![];
    for _ in 0..40 {
        let mut abb = basis.clone();
        for (id, g) in &folger {
            if let Some(Obj { kind: Kind::Wire { pts, .. }, .. }) = vorher.iter().find(|o| o.id == *id) {
                abb.wires.push((*id, pts.clone(), g.pts.clone()));
            }
        }
        let mut neu: Vec<(u64, Gefuehrt)> = vec![];
        for o in vorher.iter().filter(|o| folger_von(o)) {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            let n = pts.len();
            let ziel = [abb.ziel(o.id, pts[0]), abb.ziel(o.id, pts[n - 1])];
            if ziel == [None, None] {
                continue;
            }
            if let Some(g) = fuehren(&vs, s, lib, o.id, pts, ziel, opt) {
                neu.push((o.id, g));
            }
        }
        let gleich = neu.len() == folger.len() && neu.iter().zip(&folger).all(|(a, b)| a.0 == b.0 && a.1.pts == b.1.pts);
        folger = neu;
        if gleich {
            break;
        }
    }
    let ids: Vec<u64> = folger.iter().map(|(id, _)| *id).collect();
    for (id, g) in folger {
        if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
            if let Kind::Wire { pts, bez, .. } = &mut o.kind {
                if let Some(d) = g.ganz {
                    bez.iter_mut().for_each(|(q, _)| *q = q.add(d));
                }
                *pts = g.pts;
            }
        }
    }
    ids
}

/// Primär geänderte Leitungen, deren Enden vorher an etwas Festem hingen, wieder dort anschließen.
fn halten(vs: &Sheet, s: &mut Sheet, lib: &[SymDef], primaer: &[u64]) {
    use crate::schiene_halten::{abgreifen, anbinden, binden, schienen};
    let alle = schienen(s, &[]);
    for o in s.objects.iter_mut().filter(|o| primaer.contains(&o.id)) {
        let Kind::Wire { pts, .. } = &mut o.kind else { continue };
        let Some(Obj { kind: Kind::Wire { pts: p0, .. }, .. }) = vs.objects.iter().find(|x| x.id == o.id) else { continue };
        if p0.len() < 2 || pts.len() < 2 {
            continue;
        }
        let n0 = p0.len();
        let fest = [fest_an(vs, lib, primaer, o.id, p0[0]), fest_an(vs, lib, primaer, o.id, p0[n0 - 1])];
        if fest.iter().all(|f| f.is_none()) {
            continue;
        }
        let achse = |a: P, b: P| crate::schiene_halten::achse(a, b);
        let mut v = pts.clone();
        match &fest[0] {
            Some(Fest::Schiene(sch)) => binden(&mut v, *sch),
            Some(Fest::Punkt(q)) => anbinden(&mut v, *q, achse(p0[0], p0[1])),
            None => {}
        }
        v.reverse();
        match &fest[1] {
            Some(Fest::Schiene(sch)) => binden(&mut v, *sch),
            Some(Fest::Punkt(q)) => anbinden(&mut v, *q, achse(p0[n0 - 1], p0[n0 - 2])),
            None => {}
        }
        v.reverse();
        *pts = crate::leitung_fuehrung::bereinigen(abgreifen(&v, &alle));
    }
}

impl crate::app::App {
    /// Stand des aktuellen Blattes vor einer Änderung merken (Beginn des Ziehens, vor Drehen/Pfeiltaste).
    pub fn netz_merken(&mut self) {
        self.netz_vorher = self.sheet().map(|s| s.objects.clone()).unwrap_or_default();
    }

    /// Leitungsnetz nach der Änderung der Objekte `primaer` nachführen (vom gemerkten Stand aus). `halten`: primär
    /// verschobene Leitungen bleiben an festen Anschlüssen. Ohne eigenen Rückgängig-Schritt.
    pub fn netz_nachfuehren(&mut self, primaer: &[u64], halten: bool) {
        if self.netz_vorher.is_empty() {
            return;
        }
        let vorher = std::mem::take(&mut self.netz_vorher);
        let lib = self.lib.clone();
        let stich = self.raster() / 2.0;
        let fix_active = self.fix_active;
        let (fang, no_snap, ursprung) = (self.fang(), self.no_snap, self.raster_ursprung());
        let rund = move |p: P| if fang > 0.0 && !no_snap { crate::raster::fang(p, fang, ursprung) } else { p };
        let opt = NetzOpt { stich, halten, fix_active, rund: &rund };
        self.with_sheet(|s, _| {
            nachfuehren(&vorher, s, &lib, primaer, &opt);
        });
        self.netz_vorher = vorher;
    }

    /// Objekte `primaer` gegenüber dem gemerkten Stand um `d` verschoben setzen und das Netz nachführen.
    pub fn netz_verschieben(&mut self, primaer: &[u64], d: P) {
        let vorher = std::mem::take(&mut self.netz_vorher);
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| primaer.contains(&o.id)) {
                if let Some(o0) = vorher.iter().find(|x| x.id == o.id) {
                    let mut k = o0.kind.clone();
                    k.translate(d);
                    o.kind = k;
                }
            }
        });
        self.netz_vorher = vorher;
        self.netz_nachfuehren(primaer, true);
    }
}
