//! Leitungen folgen den Anschlüssen beim Vergrößern/Verkleinern (Rahmengriff) und Drehen/Spiegeln verdrahteter
//! Symbole (Prüfliste Runde 12, C1/C2).
//!
//! Beim Verschieben bewegen sich alle Anschlüsse um dasselbe `d` (mitziehen.rs). Beim Skalieren, Drehen und
//! Spiegeln wandert dagegen jeder Anschluss für sich: Die Leitungsenden werden je Anschluss auf seine neue Lage
//! gesetzt (alter Anschluss → neuer Anschluss desselben Objekts, gleiche Reihenfolge von `canvas::obj_pins`).
//! Je Leitung gilt der Versatz ihres Endes; hängen beide Enden an bewegten Anschlüssen mit verschiedenem Versatz,
//! folgt erst der Anfang, dann das Ende (jeweils `mitziehen::folgen_um`, Formerhalt bzw. Neuführung wie beim
//! Mitziehen). Hat sich die Richtung eines Anschlusses gedreht (Drehen/Spiegeln), passt die alte Form nicht mehr:
//! die Leitung wird mit Stich + Z neu geführt und umgeht dabei Symbole (leitung_hindernis.rs). Enden auf
//! mitskalierten Leitungen wandern an dieselbe Stelle des Zuges (`mitziehen::abzweig_lage`).

use crate::app::App;
use crate::canvas;
use crate::mitziehen::{abzweig_lage, folgen, folgen_um, Mitzieh, PIN_TOL};
use crate::model::*;
use crate::symbols::SymDef;

const EPS: f32 = 1e-3;

/// Abbildung alter auf neue Anschlusslagen der veränderten Objekte.
#[derive(Default, Debug)]
pub struct Abbild {
    /// alter Anschluss, neuer Anschluss, Fangabstand
    pins: Vec<(P, P, f32)>,
    /// mitveränderte Leitungen: alte und neue Punkte
    wires: Vec<(Vec<P>, Vec<P>)>,
}

impl Abbild {
    /// Aus den Objekten vor der Änderung (`alt`) und dem Blatt danach.
    pub fn neu(alt: &[Obj], sheet: &Sheet, lib: &[SymDef]) -> Self {
        let mut a = Abbild::default();
        for o0 in alt {
            let Some(o) = sheet.objects.iter().find(|o| o.id == o0.id) else { continue };
            if let (Kind::Wire { pts: p0, .. }, Kind::Wire { pts: p1, .. }) = (&o0.kind, &o.kind) {
                a.wires.push((p0.clone(), p1.clone()));
                continue;
            }
            let (v0, v1) = (canvas::obj_pins(o0, lib), canvas::obj_pins(o, lib));
            if v0.len() != v1.len() {
                continue;
            }
            let tol = if matches!(o0.kind, Kind::Terminal { .. }) { crate::klemme_leitung::KLEMME_R + PIN_TOL } else { PIN_TOL };
            a.pins.extend(v0.into_iter().zip(v1).map(|(p, q)| (p, q, tol)));
        }
        a
    }

    /// Neue Lage des Leitungsendes `e` (vorher an einem Anschluss bzw. auf einer veränderten Leitung).
    pub fn ziel(&self, e: P) -> Option<P> {
        let pin = self.pins.iter().filter(|(p, _, t)| p.dist(e) <= *t).min_by(|x, y| x.0.dist(e).total_cmp(&y.0.dist(e)));
        if let Some((p, q, _)) = pin {
            return Some(e.add(q.sub(*p)));
        }
        self.wires.iter().find(|(a, _)| a.windows(2).any(|s| seg_dist(e, s[0], s[1]) <= PIN_TOL)).map(|(a, n)| abzweig_lage(a, n, e))
    }
}

/// Neue Punkte der angeschlossenen Leitungen `f` (aus `mitziehen::find_followers` vor der Änderung) nach der
/// Änderung der Objekte `alt` auf dem Blatt `sheet`: (Kennung, Punkte, Verschiebung der ganzen Leitung).
/// Strecken aller Leitungen und Schienen außer der Leitung `id`.
fn strecken_ohne(sheet: &Sheet, id: u64) -> Vec<(P, P)> {
    sheet
        .objects
        .iter()
        .filter(|o| o.id != id)
        .flat_map(|o| match &o.kind {
            crate::model::Kind::Wire { pts, .. } => pts.windows(2).map(|v| (v[0], v[1])).collect::<Vec<_>>(),
            crate::model::Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
            _ => vec![],
        })
        .collect()
}

pub fn nachgefuehrt(sheet: &Sheet, lib: &[SymDef], alt: &[Obj], f: &[Mitzieh], stich: f32, rund: &dyn Fn(P) -> P) -> Vec<(u64, Vec<P>, Option<P>)> {
    use crate::leitung_hindernis as lh;
    let abb = Abbild::neu(alt, sheet, lib);
    let sch = crate::schiene_halten::schienen(sheet, &[]);
    let umrisse = lh::umrisse(sheet, lib);
    let alle = lh::hindernisse(&umrisse, &[], &[]);
    let (f2, f1): (Vec<&Mitzieh>, Vec<&Mitzieh>) = f.iter().partition(|w| w.wirt.is_some());
    let mut neu: Vec<(u64, Vec<P>, Option<P>)> = vec![];
    for w in f1 {
        let n = w.pts.len();
        if n < 2 {
            continue;
        }
        let enden = [w.pts[0], w.pts[n - 1]];
        let ziel = [0, 1].map(|i| if w.ends[i] { abb.ziel(enden[i]) } else { None });
        if ziel == [None, None] {
            continue;
        }
        let d = [0, 1].map(|i| ziel[i].map(|q| q.sub(enden[i])).unwrap_or_default());
        if d.iter().all(|d| d.x.abs() < 1e-5 && d.y.abs() < 1e-5) {
            continue;
        }
        let dirs = [0, 1].map(|i| match ziel[i] {
            Some(q) => crate::leitung_route::anschluss_richtung(sheet, lib, q).or(w.dirs[i]),
            None => w.dirs[i],
        });
        let gedreht = (0..2).any(|i| ziel[i].is_some() && matches!((w.dirs[i], dirs[i]), (Some(a), Some(b)) if a.x * b.x + a.y * b.y < 0.5));
        let (a, b) = (ziel[0].unwrap_or(enden[0]), ziel[1].unwrap_or(enden[1]));
        let hind = lh::wirksam(&alle, a, b);
        let (pts, ganz) = if gedreht {
            // Richtung eines Anschlusses gedreht: Stich + Z neu, um Symbole herum
            // zuerst ein einfacher Weg, der auf keiner Leitung/Schiene liegt (Maustest Runde 12: nach dem Drehen lief
            // die Zuleitung ein Stück auf der Schiene N)
            let andere = strecken_ohne(sheet, w.id);
            match crate::leitung_kurz::bester(a, dirs[0], b, dirs[1], stich, &hind, &andere) {
                Some(p) => (p, None),
                None => {
                    let weg = crate::leitung_route::mit_stichen(a, dirs[0], b, dirs[1], stich);
                    (lh::umgehen(&weg, dirs[0], dirs[1], &hind, 2.0 * stich, false), None)
                }
            }
        } else if ziel[0].is_some() && ziel[1].is_some() && d[0].dist(d[1]) < EPS {
            (w.pts.iter().map(|p| p.add(d[0])).collect(), Some(d[0]))
        } else {
            let mut p = w.pts.clone();
            for (i, ends) in [(0, [true, false]), (1, [false, true])] {
                if ziel[i].is_some() {
                    p = folgen_um(&p, ends, d[i], dirs, w.profil, stich, &hind, rund);
                }
            }
            (p, None)
        };
        let pts = crate::schiene_halten::abgreifen(&pts, &sch);
        // nicht ganz verschoben: einfacher Weg mit weniger Knicken, wenn er nichts verschlechtert (leitung_kurz.rs)
        let pts = if ganz.is_none() {
            let andere = strecken_ohne(sheet, w.id);
            crate::leitung_kurz::kuerzer(&pts, dirs[0], dirs[1], stich, &hind, &andere).unwrap_or(pts)
        } else {
            pts
        };
        neu.push((w.id, pts, ganz));
    }
    // zweite Stufe: Abzweige auf umgeformten Leitungen folgen ihrem Wirt (wie mitziehen::apply_followers)
    for w in f2 {
        let Some(wirt) = w.wirt else { continue };
        let alt = f.iter().find(|h| h.id == wirt).map(|h| h.pts.clone());
        let jetzt = neu.iter().find(|(id, _, _)| *id == wirt).map(|(_, p, _)| p.clone());
        let (Some(alt), Some(jetzt)) = (alt, jetzt) else { continue };
        let e = if w.ends[0] { w.pts[0] } else { w.pts[w.pts.len() - 1] };
        let de = abzweig_lage(&alt, &jetzt, e).sub(e);
        neu.push((w.id, folgen(&w.pts, w.ends, de, w.dirs, w.profil, stich, &|p| p), None));
    }
    neu
}

impl App {
    /// Nach dem Skalieren/Drehen/Spiegeln der Objekte `alt` (Zustand vorher) die angeschlossenen Leitungen `f`
    /// (vorher ermittelt mit `wire_followers`) den neuen Anschlusslagen folgen lassen. Ohne eigenen
    /// Rückgängig-Schritt (gehört zur Änderung).
    pub fn anschluesse_nachfuehren(&mut self, alt: &[Obj], f: &[Mitzieh]) {
        if f.is_empty() {
            return;
        }
        let stich = self.raster() / 2.0;
        let Some(neu) = self.sheet().map(|s| nachgefuehrt(s, &self.lib, alt, f, stich, &|p| self.snap(p))) else { return };
        // Punkte aus dem Zustand vorher (beim Rahmenziehen wird jedes Bild neu aus `f` berechnet)
        self.with_sheet(|s, _| {
            for (id, pts, ganz) in neu {
                let Some(w) = f.iter().find(|w| w.id == id) else { continue };
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                    if let Kind::Wire { pts: p, bez, .. } = &mut o.kind {
                        if let Some(dd) = ganz {
                            let dd = w.pts[0].add(dd).sub(p[0]);
                            bez.iter_mut().for_each(|(q, _)| *q = q.add(dd));
                        }
                        *p = pts;
                    }
                }
            }
        });
    }
}
