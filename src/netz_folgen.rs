//! Leitungsnetz nachführen – ein Verfahren für jede Änderung (Runde 13, Nutzerwunsch „das ganze Leitungsthema
//! generalisieren“).
//!
//! Vorher gab es je Bearbeitung eine eigene Behandlung (Verschieben: `mitziehen`, Teilstück/Knick ziehen: keine,
//! Skalieren/Drehen: eigenes Modul `anschluss_folgen`, markierte Leitungen an Schienen: `schiene_halten`,
//! Pfeiltasten: wieder eigene Aufrufe); diese Ablaufteile sind entfallen, `mitziehen`/`leitung_profil`/
//! `leitung_kurz`/`leitung_hindernis`/`schiene_halten` liefern nur noch die Bausteine der Führung. Dabei fehlten Fälle, z. B. folgte eine Leitung, die als Abzweig auf einer anderen endet, nicht,
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
    /// gemerkte Leitungsformen je Quadrant (leitung_gedaechtnis.rs)
    pub formen: Option<&'a crate::leitung_gedaechtnis::Gedaechtnis>,
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

    /// Neue Lage des Leitungsendes `e`, wenn es an einem Anschluss eines geänderten Objekts hing.
    pub fn ziel_anschluss(&self, e: P) -> Option<P> {
        let pin = self.pins.iter().filter(|(_, p, _, t)| p.dist(e) <= *t).min_by(|x, y| x.1.dist(e).total_cmp(&y.1.dist(e)));
        pin.map(|(_, p, q, _)| e.add(q.sub(*p)))
    }

    /// Neue Lage des Leitungsendes `e` der Leitung `eigen`, wenn es vorher an einer abgebildeten Lage hing.
    pub fn ziel(&self, eigen: u64, e: P) -> Option<P> {
        if let Some(q) = self.ziel_anschluss(e) {
            return Some(q);
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

/// Alle Teilstücke waagerecht oder senkrecht?
pub fn rechtwinklig(w: &[P]) -> bool {
    w.windows(2).all(|v| (v[0].x - v[1].x).abs() < 0.01 || (v[0].y - v[1].y).abs() < 0.01)
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
    // an einem Punkt des Zuges, den es nachher noch gibt (Ende oder Knick, der stehen bleibt): bleibt
    if alt.iter().any(|q| q.dist(e) <= PIN_TOL) && neu.iter().any(|q| q.dist(e) <= PIN_TOL) {
        return e;
    }
    // liegt e nachher noch auf einem Stück gleicher Richtung wie eines der alten Stücke durch e (z. B. Abzweig an
    // einer Ecke, deren waagerechtes Stück bleibt, während das senkrechte wandert): bleibt – sonst zog eine Ecke
    // ganze Ketten von Abzweigen mit (W2 Blatt 27, Klemmenreihe um 2,5 mm verschoben)
    let richtung = |a: P, b: P| if waag(a, b) && !senk(a, b) { 1 } else if senk(a, b) && !waag(a, b) { 2 } else { 0 };
    let alt_r: Vec<i32> = alt.windows(2).filter(|s| seg_dist(e, s[0], s[1]) <= PIN_TOL).map(|s| richtung(s[0], s[1])).filter(|r| *r != 0).collect();
    if neu.windows(2).any(|s| seg_dist(e, s[0], s[1]) <= PIN_TOL && alt_r.contains(&richtung(s[0], s[1]))) {
        return e;
    }
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
pub fn fuehren(vorher: &Sheet, jetzt: &Sheet, lib: &[SymDef], id: u64, alt: &[P], ziel: [Option<P>; 2], opt: &NetzOpt, bewegt_ids: &[u64]) -> Option<Gefuehrt> {
    fuehren_mit(vorher, jetzt, jetzt, lib, id, alt, ziel, opt, bewegt_ids)
}

/// Wie `fuehren`; `lagen` liefert die anderen Leitungen für die Prüfung auf Überdeckung (beim Nachführen ohne die
/// Folgeleitungen, deren Ausgangslage nicht mehr gilt).
#[allow(clippy::too_many_arguments)]
pub fn fuehren_mit(vorher: &Sheet, jetzt: &Sheet, lagen: &Sheet, lib: &[SymDef], id: u64, alt: &[P], ziel: [Option<P>; 2], opt: &NetzOpt, bewegt_ids: &[u64]) -> Option<Gefuehrt> {
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
    // Richtung am neuen Ende: am Anschluss dessen Richtung; auf einer Leitung bzw. Schiene quer dazu zur Seite des
    // anderen Endes (sonst lief ein Abzweig, dessen Wirt über ihn hinweg verschoben wurde, über den Wirt hinaus und
    // kam von der anderen Seite zurück – Maustest Runde 13)
    // (auch am festen Ende: liegt es auf einer Leitung und wandert das andere Ende auf deren andere Seite)
    let dirs = [0, 1].map(|i| {
        let q = ziel[i].unwrap_or(enden[i]);
        let anderes = ziel[1 - i].unwrap_or(enden[1 - i]);
        crate::leitung_route::anschluss_richtung(jetzt, lib, q).or_else(|| crate::leitung_route::richtung_auf_leitung_ohne(jetzt, q, anderes, id)).or(alt_dirs[i])
    });
    // Profil nur ohne fremde Anschlüsse im Innern (T-Abzweig, Knoten), sonst nur das Endstück
    let innen = |q: P| q.dist(enden[0]) > PIN_TOL && q.dist(enden[1]) > PIN_TOL && alt.windows(2).any(|w| seg_dist(q, w[0], w[1]) <= PIN_TOL);
    let profil = !vorher.objects.iter().filter(|o| o.id != id && matches!(o.kind, Kind::Wire { .. } | Kind::Symbol { .. })).flat_map(|o| canvas::obj_pins(o, lib)).any(innen);
    let gedreht = (0..2).any(|i| ziel[i].is_some() && matches!((alt_dirs[i], dirs[i]), (Some(a), Some(b)) if a.x * b.x + a.y * b.y < 0.5) && crate::leitung_route::anschluss_richtung(vorher, lib, enden[i]).is_some());
    let (a, b) = (ziel[0].unwrap_or(enden[0]), ziel[1].unwrap_or(enden[1]));
    let umrisse = lh::umrisse(jetzt, lib);
    let hind = lh::wirksam(&lh::hindernisse(&umrisse, &[], &[]), a, b);
    let andere = strecken_ohne(lagen, id);
    // Leitungen, die mit dieser verbunden sind (Ende auf ihr bzw. ihr Ende auf dieser): Überdeckung mit ihnen ist
    // kein Hindernis (gleiches Netz, das Aufräumen vereinigt sie), wohl aber mit allen anderen
    let auf = |q: P, w: &[P]| w.windows(2).any(|s| seg_dist(q, s[0], s[1]) <= PIN_TOL);
    let fremd: Vec<(P, P)> = lagen
        .objects
        .iter()
        .filter(|o| o.id != id)
        .flat_map(|o| match &o.kind {
            Kind::Wire { pts, .. } if pts.len() >= 2 => {
                let verbunden = auf(pts[0], alt) || auf(pts[pts.len() - 1], alt) || auf(enden[0], pts) || auf(enden[1], pts);
                if verbunden {
                    vec![]
                } else {
                    pts.windows(2).map(|v| (v[0], v[1])).collect::<Vec<_>>()
                }
            }
            Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
            _ => vec![],
        })
        .collect();
    // Enden an Anschlüssen bzw. Klemmstellen (nachher): dort wandert kein Abgriff auf einer Schiene
    let am_anschluss = |q: P| {
        jetzt.objects.iter().any(|o| match &o.kind {
            Kind::Terminal { pos, .. } => pos.dist(q) <= crate::klemme_leitung::KLEMME_R + PIN_TOL,
            Kind::Symbol { .. } | Kind::Verweis { .. } => canvas::obj_pins(o, lib).iter().any(|x| x.dist(q) <= PIN_TOL),
            _ => false,
        })
    };
    let fest_enden = [am_anschluss(ziel[0].unwrap_or(enden[0])), am_anschluss(ziel[1].unwrap_or(enden[1]))];
    let sch = crate::schiene_halten::schienen(jetzt, &[]);
    if gedreht {
        // Richtung eines Anschlusses gedreht (Drehen/Spiegeln): einfacher Weg, sonst Stich + Z um Symbole herum
        let ok = |w: &[P]| zulaessig(w, &hind, &fremd, &umrisse);
        let pts = crate::leitung_kurz::bester_mit(a, dirs[0], b, dirs[1], opt.stich, &hind, &andere, &ok).unwrap_or_else(|| {
            let weg = crate::leitung_route::mit_stichen(a, dirs[0], b, dirs[1], opt.stich);
            let um = lh::umgehen(&weg, dirs[0], dirs[1], &hind, 2.0 * opt.stich, false);
            if ok(&um) {
                um
            } else {
                // kein zulässiger einfacher Weg: Form halten (Enden nachgeführt)
                let mut p = alt.to_vec();
                for (i, ends) in [(0, [true, false]), (1, [false, true])] {
                    if ziel[i].is_some() {
                        p = folgen_um(&p, ends, d[i], dirs, profil, opt.stich, &hind, opt.rund);
                    }
                }
                if ok(&p) {
                    p
                } else {
                    um
                }
            }
        });
        return Some(Gefuehrt { pts: crate::schiene_halten::abgreifen_ausser(&pts, &sch, fest_enden), ganz: None });
    }
    // ein Ende bewegt, das andere hängt als Abzweig quer auf einem Gleis: gerade aufs Gleis (Abgriff wandert mit)
    let bewegt = [ziel[0].is_some() && d[0].dist(P::default()) > 1e-5, ziel[1].is_some() && d[1].dist(P::default()) > 1e-5];
    if bewegt[0] != bewegt[1] {
        let k = if bewegt[0] { 0 } else { 1 };
        let gl = crate::schiene_halten::gleise(jetzt);
        if let Some(g) = crate::schiene_halten::gerade_zum_gleis(id, alt, bewegt, d[k], alt_dirs, &gl).filter(|g| zulaessig(g, &hind, &fremd, &umrisse)) {
            return Some(Gefuehrt { pts: g, ganz: None });
        }
    }
    // beide Enden wieder genau dort, wo eine Form gemerkt wurde (Rückkehr, Kriterium L10): diese Form, wenn zulässig
    if bewegt[0] || bewegt[1] {
        let mut f = opt.formen.and_then(|g| g.genau(id, a, b));
        // nur ein Ende bewegt, das andere hängt auf einem Wirt: dort darf es zurückgleiten
        if f.is_none() && bewegt[0] != bewegt[1] {
            let (k, q, anders) = if bewegt[0] { (0, a, b) } else { (1, b, a) };
            if let Some((_, w0, w1)) = wirt_bei(jetzt, id, anders) {
                f = opt.formen.and_then(|g| g.mit_ende(id, k, q, &|p| seg_dist(p, w0, w1) <= PIN_TOL));
            }
        }
        if let Some(f) = f {
            if f.as_slice() != alt && zulaessig(&f, &hind, &fremd, &umrisse) {
                return Some(Gefuehrt { pts: f, ganz: None });
            }
        }
    }
    // Quadrant gewechselt und für den neuen ist eine Form gemerkt (nach QElectroTech): diese Form, wenn sie zulässig
    // ist – nur ein Ende bewegt und dieses an einem Anschluss (das andere bleibt liegen), ohne fremde Anschlüsse im
    // Innern
    if bewegt[0] != bewegt[1] && fest_enden[if bewegt[0] { 0 } else { 1 }] && profil && n >= 2 {
        let neu_q = crate::leitung_gedaechtnis::quadrant(a, b);
        if neu_q != crate::leitung_gedaechtnis::quadrant(alt[0], alt[n - 1]) {
            if let Some(f) = opt.formen.and_then(|g| g.form(id, a, b, opt.rund)) {
                let raus = |i: usize, q: P, nb: P| dirs[i].is_none_or(|d| (nb.x - q.x) * d.x + (nb.y - q.y) * d.y > 0.0);
                let m = f.len();
                if m >= 2 && rechtwinklig(&f) && raus(0, f[0], f[1]) && raus(1, f[m - 1], f[m - 2]) && zulaessig(&f, &hind, &fremd, &umrisse) {
                    return Some(Gefuehrt { pts: f, ganz: None });
                }
            }
        }
    }
    let mut p = alt.to_vec();
    for (i, ends) in [(0, [true, false]), (1, [false, true])] {
        if bewegt[i] {
            p = folgen_um(&p, ends, d[i], dirs, profil, opt.stich, &hind, opt.rund);
        }
    }
    let p = crate::schiene_halten::abgreifen_ausser(&p, &sch, fest_enden);
    // einfacher Weg mit weniger Knicken, wenn er nichts verschlechtert (leitung_kurz.rs)
    let ok = |w: &[P]| zulaessig(w, &hind, &fremd, &umrisse);
    let p = crate::leitung_kurz::kuerzer_mit(&p, dirs[0], dirs[1], opt.stich, &hind, &andere, &ok).unwrap_or(p);
    // ein Ende auf einer anderen Leitung bzw. Schiene darf auf ihr gleiten, wenn das Knicke spart (Nutzerbild
    // 23.11.2026: Abzweig mit Ecke kurz vor dem Wirt)
    let p = gleiten(jetzt, lib, id, p, dirs, opt.stich, &umrisse, &andere, bewegt_ids, fest_enden);
    // Zulässigkeit (Nutzervideo 16:22: Leitung lief durch eine andere Lampe und auf deren Leitung – nach dem
    // Aufräumen hing die Lampe am falschen Anschluss): kein fremdes Symbol durchqueren, auf keiner Leitung liegen
    let p = if !zulaessig(&p, &hind, &fremd, &umrisse) {
        let (a, b) = (p[0], p[p.len() - 1]);
        crate::leitung_kurz::bester_mit(a, dirs[0], b, dirs[1], opt.stich, &hind, &andere, &ok).unwrap_or_else(|| {
            let weg = crate::leitung_route::mit_stichen(a, dirs[0], b, dirs[1], opt.stich);
            let um = lh::umgehen(&weg, dirs[0], dirs[1], &hind, 2.0 * opt.stich, false);
            // Notweg nur, wenn er zulässig ist; sonst bleibt die formerhaltende Führung
            if zulaessig(&um, &hind, &fremd, &umrisse) {
                um
            } else {
                p.clone()
            }
        })
    } else {
        p
    };
    Some(Gefuehrt { pts: p, ganz: None })
}

/// Darf eine geführte Leitung so liegen? Kein fremdes Symbol durchqueren (auch kein Knick im Innern eines Umrisses),
/// auf keiner anderen Leitung bzw. Schiene liegen und über keinen fremden Anschluss laufen (sonst wäre sie dort
/// elektrisch angeschlossen). Anschlüsse an den eigenen Enden zählen nicht.
pub fn zulaessig(w: &[P], hind: &[crate::leitung_hindernis::Rechteck], andere: &[(P, P)], umrisse: &[crate::leitung_hindernis::Umriss]) -> bool {
    if w.len() < 2 {
        return true;
    }
    if crate::leitung_hindernis::kreuzungen(w, hind) > 0 || crate::leitung_kurz::ueberlappt(w, andere) {
        return false;
    }
    let (a, b) = (w[0], w[w.len() - 1]);
    let innen = |q: P, r: &crate::leitung_hindernis::Rechteck| {
        let (mn, mx) = (P::new(r.0.x.min(r.1.x), r.0.y.min(r.1.y)), P::new(r.0.x.max(r.1.x), r.0.y.max(r.1.y)));
        q.x > mn.x + 0.1 && q.x < mx.x - 0.1 && q.y > mn.y + 0.1 && q.y < mx.y - 0.1
    };
    // Knick im Innern eines Symbols, an dessen Anschluss die Leitung nicht endet
    for u in umrisse {
        let eigen = u.pins.iter().any(|q| q.dist(a) < 0.05 || q.dist(b) < 0.05);
        if !eigen && w[1..w.len() - 1].iter().any(|q| innen(*q, &u.r)) {
            return false;
        }
        // fremder Anschluss auf dem Weg
        for q in &u.pins {
            if q.dist(a) < 0.05 || q.dist(b) < 0.05 {
                continue;
            }
            if w.windows(2).any(|s| seg_dist(*q, s[0], s[1]) < 0.05) {
                return false;
            }
        }
    }
    true
}

/// Waagerechtes bzw. senkrechtes Stück einer anderen Leitung (nicht `ohne`) bzw. Schiene, in dessen Innerem `e`
/// liegt (Wirt eines Abzweigs).
pub fn wirt_bei(sheet: &Sheet, ohne: u64, e: P) -> Option<(u64, P, P)> {
    const R: f32 = 0.05;
    for o in sheet.objects.iter().filter(|o| o.id != ohne) {
        let stuecke: Vec<(P, P)> = match &o.kind {
            Kind::Wire { pts, .. } => pts.windows(2).map(|w| (w[0], w[1])).collect(),
            Kind::Busbar { a, x2, .. } => vec![(P::new(a.x.min(*x2), a.y), P::new(a.x.max(*x2), a.y))],
            _ => continue,
        };
        for (a, b) in stuecke {
            let innen = if waag(a, b) && !senk(a, b) {
                (e.y - a.y).abs() < R && e.x > a.x.min(b.x) + R && e.x < a.x.max(b.x) - R
            } else if senk(a, b) && !waag(a, b) {
                (e.x - a.x).abs() < R && e.y > a.y.min(b.y) + R && e.y < a.y.max(b.y) - R
            } else {
                false
            };
            if innen {
                return Some((o.id, a, b));
            }
        }
    }
    None
}

/// Bewertung eines Weges: Knicke, dann Kreuzungen mit anderen Leitungen, dann Länge (Nutzer: „möglichst immer die
/// Anzahl an Ecken und Kreuzungen und Längen minimieren“).
pub fn bewertung(w: &[P], andere: &[(P, P)]) -> (usize, usize, i64) {
    let l: f32 = w.windows(2).map(|v| v[0].dist(v[1])).sum();
    (crate::leitung_kurz::knicke(w), crate::leitung_kurz::kreuzungen_mit(w, andere), (l * 10.0).round() as i64)
}

/// Endet bzw. beginnt die Leitung `p` (Kennung `id`) im Innern einer anderen Leitung bzw. auf einer Schiene, darf
/// dieses Ende auf dem Wirt gleiten: Stellen in der Flucht des anderen Endes, seines Stichs und der Knicke werden
/// geprüft (`leitung_kurz::bester`: kein Symbol durchquert, nicht auf einer Leitung, nicht in einen Anschluss
/// hinein); genommen wird der Weg mit der besten `bewertung`, wenn er besser ist. An Stellen, an denen schon etwas
/// am Wirt hängt, gleitet das Ende nicht (sonst entstünde eine neue Verbindung).
#[allow(clippy::too_many_arguments)]
/// Auf Leitungen in `bewegt_ids` (in diesem Schritt selbst geführt, im Blatt noch in alter Lage) gleitet nichts.
/// Enden mit `fest` (an Anschluss bzw. Klemmstelle) gleiten nie.
pub fn gleiten(jetzt: &Sheet, lib: &[SymDef], id: u64, p: Vec<P>, dirs: [Option<P>; 2], stich: f32, umrisse: &[crate::leitung_hindernis::Umriss], andere: &[(P, P)], bewegt_ids: &[u64], fest: [bool; 2]) -> Vec<P> {
    use crate::leitung_hindernis as lh;
    let n = p.len();
    if n < 3 {
        return p;
    }
    let mut best = p.clone();
    let mut wert = bewertung(&p, andere);
    // Anschlüsse und Leitungsenden anderer Objekte (dort darf das Ende nicht hingleiten)
    let belegt: Vec<P> = jetzt.objects.iter().filter(|o| o.id != id).flat_map(|o| canvas::obj_pins(o, lib)).collect();
    for k in [1usize, 0] {
        if fest[k] {
            continue;
        }
        let e = if k == 1 { p[n - 1] } else { p[0] };
        let Some((wid, a, b)) = wirt_bei(jetzt, id, e) else { continue };
        if bewegt_ids.contains(&wid) {
            continue;
        }
        let h = waag(a, b);
        let (anf, da) = if k == 1 { (p[0], dirs[0]) } else { (p[n - 1], dirs[1]) };
        // Lagen entlang des Wirts: Flucht des anderen Endes, seines Stichs und aller Punkte des Weges
        let mut lagen: Vec<f32> = p.iter().map(|q| if h { q.x } else { q.y }).collect();
        if let Some(d) = da {
            let s1 = anf.add(P::new(d.x * stich, d.y * stich));
            lagen.push(if h { s1.x } else { s1.y });
        }
        let (lo, hi) = if h { (a.x.min(b.x), a.x.max(b.x)) } else { (a.y.min(b.y), a.y.max(b.y)) };
        for l in lagen {
            if l <= lo + 0.05 || l >= hi - 0.05 {
                continue;
            }
            let q = if h { P::new(l, a.y) } else { P::new(a.x, l) };
            if q.dist(e) > 0.01 && belegt.iter().any(|x| x.dist(q) < 0.1) {
                continue;
            }
            let dq = crate::leitung_route::richtung_auf_leitung_ohne(jetzt, q, anf, id);
            let hind = lh::wirksam(&lh::hindernisse(umrisse, &[], &[]), anf, q);
            let ok = |w: &[P]| {
                let mut v = w.to_vec();
                if k == 0 {
                    v.reverse();
                }
                zulaessig(&v, &hind, andere, umrisse)
            };
            let Some(mut w) = crate::leitung_kurz::bester_mit(anf, da, q, dq, stich, &hind, andere, &ok) else { continue };
            if k == 0 {
                w.reverse();
            }
            if !zulaessig(&w, &hind, andere, umrisse) {
                continue;
            }
            let v = bewertung(&w, andere);
            if v < wert {
                wert = v;
                best = w;
            }
        }
    }
    best
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
        // Leitungen, die in diesem Schritt (letzter Durchlauf) selbst geführt werden
        let bewegt_ids: Vec<u64> = folger.iter().map(|(id, _)| *id).collect();
        // geändert: primär oder in diesem Schritt geführt
        let mut geaendert: Vec<u64> = primaer.to_vec();
        geaendert.extend(&bewegt_ids);
        // Lagen für die Überdeckungsprüfung: ohne die Folgeleitungen dieses Schritts – ihre Ausgangslage gilt nicht
        // mehr (Maustest 23.11.2026: die untere Leitung einer nach unten verschobenen Lampe lag in der Ausgangslage
        // dort, wo die obere gerade weiterlaufen sollte – die obere bekam einen U-Umweg). Ihre neue Lage wird nicht
        // eingesetzt: dann hingen die Folgeleitungen gegenseitig voneinander ab und kämen nicht zur Ruhe.
        let mut lagen = s.clone();
        lagen.objects.retain(|o| !bewegt_ids.contains(&o.id));
        for o in vorher.iter().filter(|o| folger_von(o)) {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            let n = pts.len();
            // ein Ende, das (auch) an etwas Unverändertem hängt (Anschluss, Klemme, Leitung im Innern, Schiene),
            // folgt nur einem verschobenen Anschluss, nicht einer geänderten Leitung, die zufällig denselben Punkt
            // berührt (Nutzervideo: E2-Abzweig wanderte mit einer anderen Leitung, die am selben Punkt auf N endete)
            let ziel_von = |e: P| {
                if let Some(q) = abb.ziel_anschluss(e) {
                    return Some(q);
                }
                if fest_an(&vs, lib, &geaendert, o.id, e).is_some() {
                    return None;
                }
                abb.ziel(o.id, e)
            };
            let ziel = [ziel_von(pts[0]), ziel_von(pts[n - 1])];
            if ziel == [None, None] {
                continue;
            }
            if let Some(mut g) = fuehren_mit(&vs, s, &lagen, lib, o.id, pts, ziel, opt, &bewegt_ids) {
                // Absicherung (Nutzerbild Runde 13: schräge Leitung nach dem Verschieben einer Lampe): eine vorher
                // rechtwinklige Leitung bleibt rechtwinklig – sonst neu mit Stichen um Symbole herum
                if rechtwinklig(pts) && !rechtwinklig(&g.pts) && g.pts.len() >= 2 {
                    let (a, b) = (g.pts[0], g.pts[g.pts.len() - 1]);
                    let da = crate::leitung_route::anschluss_richtung(s, lib, a);
                    let db = crate::leitung_route::anschluss_richtung(s, lib, b);
                    let umr = crate::leitung_hindernis::umrisse(s, lib);
                    let hind = crate::leitung_hindernis::wirksam(&crate::leitung_hindernis::hindernisse(&umr, &[], &[]), a, b);
                    let weg = crate::leitung_route::mit_stichen(a, da, b, db, opt.stich);
                    g.pts = crate::leitung_hindernis::umgehen(&weg, da, db, &hind, 2.0 * opt.stich, false);
                    g.ganz = None;
                }
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
        // Achse des ersten echten Teilstücks vom Ende aus (doppelte Punkte überspringen, sonst entstand ein schräges
        // Stück)
        let achse_ab = |z: Vec<P>| {
            let a = *z.first()?;
            z.iter().copied().find(|b| b.dist(a) >= EPS).and_then(|b| crate::schiene_halten::achse(a, b))
        };
        let mut v = pts.clone();
        match &fest[0] {
            Some(Fest::Schiene(sch)) => binden(&mut v, *sch),
            Some(Fest::Punkt(q)) => anbinden(&mut v, *q, achse_ab(p0.clone())),
            None => {}
        }
        v.reverse();
        match &fest[1] {
            Some(Fest::Schiene(sch)) => binden(&mut v, *sch),
            Some(Fest::Punkt(q)) => anbinden(&mut v, *q, achse_ab(p0.iter().rev().copied().collect())),
            None => {}
        }
        v.reverse();
        *pts = crate::leitung_fuehrung::bereinigen(abgreifen(&v, &alle));
    }
}

/// Zahl der gemerkten Blattstände für die Rückkehr.
const RUECKKEHR: usize = 8;

impl crate::app::App {
    /// Vor dem Verschieben markierter Symbole, Klemmstellen und Verweise (nach dem Rückgängig-Punkt): Leitungen, die an
    /// ihren Anschlüssen enden, mit stoßenden Leitungen in Flucht zu einer zusammenfassen. Nutzervideo 23.11.2026
    /// 19:04: X1.1 auf ihrer Leitung nach oben gezogen – die Leitung darüber bestand aus drei Stücken; das kurze an
    /// der Klemme hätte über die Stoßstelle zurücklaufen müssen und wurde als U-Schleife neben der Leitung geführt.
    /// Als eine Leitung wird sie einfach kürzer.
    pub fn stoesse_an_markierung_zusammenfassen(&mut self) {
        let Some(s) = self.sheet() else { return };
        // Anschlüsse mit Fangweite (Klemmstelle: Leitungen enden am Kreis, nicht in der Mitte)
        let pins: Vec<(P, f32)> = s
            .objects
            .iter()
            .filter(|o| self.selection.contains(&o.id) && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Verweis { .. }))
            .flat_map(|o| canvas::obj_pins(o, &self.lib).into_iter().map(|q| (q, pin_tol(&o.kind))).collect::<Vec<_>>())
            .collect();
        if pins.is_empty() {
            return;
        }
        let ids: Vec<u64> = s
            .objects
            .iter()
            .filter(|o| !self.selection.contains(&o.id))
            .filter(|o| match &o.kind {
                Kind::Wire { pts, .. } if pts.len() >= 2 => [pts[0], pts[pts.len() - 1]].iter().any(|e| pins.iter().any(|(q, t)| q.dist(*e) <= *t)),
                _ => false,
            })
            .map(|o| o.id)
            .collect();
        if !ids.is_empty() {
            self.leitungen_zusammenfassen(&ids);
        }
    }

    /// Stehen alle Objekte außer Leitungen wieder genau wie in einem zu Gestenbeginn gemerkten Stand (Symbol hin und
    /// zurück, viermal gedreht …), gelten dessen Leitungen – der neueste passende Stand, damit Handänderungen
    /// erhalten bleiben (Kriterium L10, Maustest 23.11.2026).
    pub fn rueckkehr_anwenden(&mut self) {
        let (Some(r), Some(s)) = (self.cur(), self.sheet()) else { return };
        let ohne = |v: &[Obj]| -> Vec<Obj> { v.iter().filter(|o| !matches!(o.kind, Kind::Wire { .. })).cloned().collect() };
        let jetzt = ohne(&s.objects);
        let Some(objs) = self.rueckkehr.iter().rev().find(|(r0, o0)| *r0 == r && ohne(o0) == jetzt).map(|(_, o)| o.clone()) else {
            self.rueckkehr_gilt = false;
            return;
        };
        let anders = s.objects != objs;
        self.rueckkehr_gilt = true;
        if anders {
            self.with_sheet(|s, _| s.objects = objs);
        }
    }

    /// Stand des aktuellen Blattes vor einer Änderung merken (Beginn des Ziehens, vor Drehen/Pfeiltaste).
    pub fn netz_merken(&mut self) {
        self.rueckkehr_gilt = false;
        self.netz_gueltig = None;
        self.netz_vorher = self.sheet().map(|s| s.objects.clone()).unwrap_or_default();
        if let Some(r) = self.cur() {
            if self.rueckkehr.last().is_none_or(|(r0, o0)| *r0 != r || *o0 != self.netz_vorher) {
                self.rueckkehr.push((r, self.netz_vorher.clone()));
                if self.rueckkehr.len() > RUECKKEHR {
                    self.rueckkehr.remove(0);
                }
            }
        }
        // aktuelle Form jeder Leitung für ihren Quadranten merken (leitung_gedaechtnis.rs)
        for o in &self.netz_vorher {
            if let Kind::Wire { pts, .. } = &o.kind {
                self.formen.merken(o.id, pts);
            }
        }
    }

    /// Leitungsnetz nach der Änderung der Objekte `primaer` nachführen (vom gemerkten Stand aus). `halten`: primär
    /// verschobene Leitungen bleiben an festen Anschlüssen. Ohne eigenen Rückgängig-Schritt.
    pub fn netz_nachfuehren(&mut self, primaer: &[u64], halten: bool) {
        if self.netz_vorher.is_empty() {
            return;
        }
        let vorher = std::mem::take(&mut self.netz_vorher);
        let lib = self.lib.clone();
        let stich = self.stich();
        let fix_active = self.fix_active;
        let (fang, no_snap, ursprung) = (self.fang(), self.no_snap, self.raster_ursprung());
        let rund = move |p: P| if fang > 0.0 && !no_snap { crate::raster::fang(p, fang, ursprung) } else { p };
        let formen = std::mem::take(&mut self.formen);
        let opt = NetzOpt { stich, halten, fix_active, rund: &rund, formen: Some(&formen) };
        self.with_sheet(|s, _| {
            nachfuehren(&vorher, s, &lib, primaer, &opt);
        });
        self.formen = formen;
        // nur wenn ausschließlich Symbole, Klemmstellen usw. bewegt werden – beim Ziehen von Leitungen ändern sich nur
        // Leitungen, das wäre sonst immer eine „Rückkehr“
        let nur_objekte = self.sheet().is_some_and(|s| s.objects.iter().filter(|o| primaer.contains(&o.id)).all(|o| !matches!(o.kind, Kind::Wire { .. })));
        if nur_objekte && !primaer.is_empty() {
            self.rueckkehr_anwenden();
        } else {
            self.rueckkehr_gilt = false;
        }
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
        self.netz_waechter(primaer);
    }
}
