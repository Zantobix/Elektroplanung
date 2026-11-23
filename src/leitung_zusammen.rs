//! Leitungen, die ohne Abzweig aneinanderstoßen, zu einer zusammenfassen (Nutzer 23.11.2026: „wenn zwei Leitungen
//! mit gleichem Potential in einer Flucht liegen, sollen diese zu einer reduziert werden“).
//!
//! Treffen sich genau zwei Leitungsenden an einem Punkt, an dem sonst nichts hängt (kein Anschluss, keine
//! Klemmstelle, keine Schiene, keine dritte Leitung, kein Abzweig im Innern), sind beide elektrisch eine Leitung.
//! Nach einer Bearbeitung werden sie zu einer Leitung (ein Zug) vereint: Die erste behält Objekt und Originalblock,
//! Bezeichnungen der zweiten wandern mit, die zweite entfällt. Liegen die Endstücke in einer Flucht, verschwindet
//! der Stoßpunkt ganz (`bereinigen`), sonst bleibt er als Knick. Voraussetzung: gleiche Leitung (Nummer, Stift,
//! Darstellung) und keine von beiden fixiert. Vorbild: Zusammenfassen von Leitungen in KiCad (`SchematicCleanup`,
//! nur die Idee).

use crate::canvas;
use crate::mitziehen::PIN_TOL;
use crate::model::*;
use crate::symbols::SymDef;

/// Wie viele andere Dinge (außer den Leitungen `ohne`) hängen am Punkt `p`?
fn sonst_am_punkt(sheet: &Sheet, lib: &[SymDef], ohne: &[u64], p: P) -> bool {
    sheet.objects.iter().filter(|o| !ohne.contains(&o.id)).any(|o| match &o.kind {
        Kind::Wire { pts, .. } => pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) <= PIN_TOL),
        Kind::Busbar { a, x2, .. } => (p.y - a.y).abs() <= PIN_TOL && p.x >= a.x.min(*x2) - PIN_TOL && p.x <= a.x.max(*x2) + PIN_TOL,
        Kind::Terminal { pos, .. } => pos.dist(p) <= crate::klemme_leitung::KLEMME_R + PIN_TOL,
        _ => canvas::obj_pins(o, lib).iter().any(|q| q.dist(p) <= PIN_TOL),
    })
}

fn zug(sheet: &Sheet, id: u64) -> Option<(Vec<P>, (usize, usize, u8), bool)> {
    let o = sheet.objects.iter().find(|o| o.id == id)?;
    match &o.kind {
        Kind::Wire { pts, leitung, opts, .. } if pts.len() >= 2 => Some((pts.clone(), (o.pen, *leitung, *opts), o.fixed)),
        _ => None,
    }
}

/// Leitungen `ids` (gerade bearbeitet) mit Leitungen zusammenfassen, die ohne Abzweig an sie stoßen. Liefert die
/// entfallenen Leitungen.
pub fn zusammenfassen(sheet: &mut Sheet, lib: &[SymDef], ids: &[u64]) -> Vec<u64> {
    let mut entfernt = vec![];
    let mut offen: Vec<u64> = ids.to_vec();
    let mut runden = 0;
    while let Some(a) = offen.pop() {
        runden += 1;
        if runden > 10_000 {
            break;
        }
        let Some((ap, art_a, fix_a)) = zug(sheet, a) else { continue };
        if fix_a {
            continue;
        }
        let enden = [ap[0], ap[ap.len() - 1]];
        let mut fund = None;
        'suche: for (k, e) in enden.iter().enumerate() {
            // genau eine andere Leitung mit einem Ende hier
            let am: Vec<(u64, usize)> = sheet
                .objects
                .iter()
                .filter(|o| o.id != a)
                .filter_map(|o| match &o.kind {
                    Kind::Wire { pts, .. } if pts.len() >= 2 => {
                        if pts[0].dist(*e) <= PIN_TOL {
                            Some((o.id, 0))
                        } else if pts[pts.len() - 1].dist(*e) <= PIN_TOL {
                            Some((o.id, 1))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect();
            if am.len() != 1 {
                continue;
            }
            let (b, kb) = am[0];
            let Some((_, art_b, fix_b)) = zug(sheet, b) else { continue };
            if fix_b || art_b != art_a || b == a {
                continue;
            }
            if sonst_am_punkt(sheet, lib, &[a, b], *e) {
                continue 'suche;
            }
            // eine Leitung, die mit beiden Enden an sich selbst bzw. an derselben anderen stößt, nicht schließen
            if enden[1 - k].dist(*e) <= PIN_TOL {
                continue;
            }
            fund = Some((k, b, kb));
            break;
        }
        let Some((k, b, kb)) = fund else { continue };
        // Es bleibt die Leitung mit Originalblock, sonst die nicht bearbeitete, sonst die ältere; die andere geht in
        // ihr auf
        let roh = |id: u64| sheet.objects.iter().find(|o| o.id == id).is_some_and(|o| o.raw.is_some());
        let rang = |id: u64| (!roh(id), ids.contains(&id), id);
        let tauschen = rang(b) < rang(a);
        let (a, k, b, kb) = if tauschen {
            let e = enden[k];
            let Some((bp0, _, _)) = zug(sheet, b) else { continue };
            let kb2 = if bp0[0].dist(e) <= PIN_TOL { 0 } else { 1 };
            (b, kb2, a, k)
        } else {
            (a, k, b, kb)
        };
        let Some((ap, _, _)) = zug(sheet, a) else { continue };
        let Some((bp, _, _)) = zug(sheet, b) else { continue };
        // A so ausrichten, dass sie am Stoßpunkt endet, B so, dass sie dort beginnt
        let mut v = ap.clone();
        if k == 0 {
            v.reverse();
        }
        let mut w = bp.clone();
        if kb == 1 {
            w.reverse();
        }
        v.extend_from_slice(&w[1..]);
        if k == 0 {
            v.reverse();
        }
        let v = crate::leitung_fuehrung::bereinigen(v);
        let bez_b = match sheet.objects.iter().find(|o| o.id == b).map(|o| &o.kind) {
            Some(Kind::Wire { bez, .. }) => bez.clone(),
            _ => vec![],
        };
        if let Some(Kind::Wire { pts, bez, .. }) = sheet.objects.iter_mut().find(|o| o.id == a).map(|o| &mut o.kind) {
            *pts = v;
            bez.extend(bez_b);
        }
        sheet.objects.retain(|o| o.id != b);
        entfernt.push(b);
        offen.retain(|x| *x != b);
        offen.push(a);
    }
    entfernt
}

const T: f32 = 0.01;

fn waag(a: P, b: P) -> bool {
    (a.y - b.y).abs() < T && (a.x - b.x).abs() >= T
}

fn senk(a: P, b: P) -> bool {
    (a.x - b.x).abs() < T && (a.y - b.y).abs() >= T
}

/// Treppe am Abgriff nachziehen (Nutzervideo 23.11.2026: „oben ist noch das kleine Stückchen, das soll auch
/// automatisch rübergezogen werden“): Beginnt eine Leitung auf einer Schiene bzw. im Innern einer anderen Leitung mit
/// quer – längs – quer (in dieselbe Richtung vom Wirt weg), rückt der Anfang auf dem Wirt in die Flucht des dritten
/// Stücks; zwei Knicke entfallen. Nur, wenn die Stelle auf dem Wirt frei ist und die neue Strecke kein Symbol
/// durchquert und auf keiner Leitung liegt. Für beide Enden, wiederholt.
pub fn ende_nachziehen(sheet: &Sheet, lib: &[SymDef], id: u64, pts: &[P]) -> Option<Vec<P>> {
    let mut v = crate::leitung_fuehrung::bereinigen(pts.to_vec());
    let mut geaendert = false;
    let belegt: Vec<P> = sheet.objects.iter().filter(|o| o.id != id).flat_map(|o| canvas::obj_pins(o, lib)).collect();
    let umr = crate::leitung_hindernis::umrisse(sheet, lib);
    let andere: Vec<(P, P)> = sheet
        .objects
        .iter()
        .filter(|o| o.id != id)
        .flat_map(|o| match &o.kind {
            Kind::Wire { pts, .. } => pts.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>(),
            Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
            _ => vec![],
        })
        .collect();
    for _ in 0..2 {
        loop {
            if v.len() < 4 {
                break;
            }
            let Some((_, ha, hb)) = crate::netz_folgen::wirt_bei(sheet, id, v[0]) else { break };
            let host_waag = waag(ha, hb);
            let quer = |a: P, b: P| if host_waag { senk(a, b) } else { waag(a, b) };
            let laengs = |a: P, b: P| if host_waag { waag(a, b) } else { senk(a, b) };
            if !(quer(v[0], v[1]) && laengs(v[1], v[2]) && quer(v[2], v[3])) {
                break;
            }
            // beide Querstücke vom Wirt weg in dieselbe Richtung
            let weg = |a: P, b: P| if host_waag { (b.y - a.y).signum() } else { (b.x - a.x).signum() };
            if weg(v[0], v[1]) != weg(v[2], v[3]) {
                break;
            }
            let q = if host_waag { P::new(v[2].x, ha.y) } else { P::new(ha.x, v[2].y) };
            let (lo, hi) = if host_waag { (ha.x.min(hb.x), ha.x.max(hb.x)) } else { (ha.y.min(hb.y), ha.y.max(hb.y)) };
            let l = if host_waag { q.x } else { q.y };
            if l <= lo + T || l >= hi - T || belegt.iter().any(|x| x.dist(q) < 0.1) {
                break;
            }
            // auf den entfallenden Stücken hängt nichts (Abzweig, Anschluss) – sonst bliebe es in der Luft
            let traegt = belegt.iter().any(|x| x.dist(v[0]) > PIN_TOL && (seg_dist(*x, v[0], v[1]) <= PIN_TOL || seg_dist(*x, v[1], v[2]) <= PIN_TOL));
            if traegt {
                break;
            }
            let mut w = vec![q];
            w.extend_from_slice(&v[2..]);
            let w = crate::leitung_fuehrung::bereinigen(w);
            let hind = crate::leitung_hindernis::wirksam(&crate::leitung_hindernis::hindernisse(&umr, &[], &[]), w[0], w[w.len() - 1]);
            if crate::leitung_hindernis::kreuzungen(&w[..2.min(w.len())], &hind) > 0 || ueberlappt(&w[..2], &andere) {
                break;
            }
            v = w;
            geaendert = true;
        }
        v.reverse();
    }
    geaendert.then_some(v)
}

fn ueberlappt(w: &[P], andere: &[(P, P)]) -> bool {
    w.windows(2).any(|s| {
        let (a, b) = (s[0], s[1]);
        andere.iter().any(|(c, d)| {
            if waag(a, b) && (c.y - d.y).abs() < T && (a.y - c.y).abs() < T {
                a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x)) > 0.5
            } else if senk(a, b) && (c.x - d.x).abs() < T && (a.x - c.x).abs() < T {
                a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y)) > 0.5
            } else {
                false
            }
        })
    })
}

/// Durchgehender Strang (Nutzervideo 23.11.2026: „wenn ein Knoten auf einen anderen gezogen wird, sollen die
/// Leitungen auch zusammengelegt werden“): Endet Leitung A an einem Knick von Leitung B und setzt dort ein Stück von
/// B gerade fort, übernimmt A den Teil von B ab dem Knick; B endet am Knick (als Abzweig auf A). Gleiche Leitung und
/// Darstellung, keine fixiert. Liefert, ob sich etwas geändert hat.
pub fn durchgehend(sheet: &mut Sheet, lib: &[SymDef], ids: &[u64]) -> bool {
    // Anschlüsse und Klemmstellen: an ihnen wird nicht durchgehend zusammengelegt (sonst läge der Anschluss mitten
    // auf der Leitung)
    let anschluesse: Vec<P> = sheet.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Verweis { .. })).flat_map(|o| canvas::obj_pins(o, lib)).collect();
    let klemmen: Vec<P> = sheet.objects.iter().filter_map(|o| if let Kind::Terminal { pos, .. } = &o.kind { Some(*pos) } else { None }).collect();
    let mut geaendert = false;
    for &a in ids {
        for _ in 0..2 {
            let Some((ap, art_a, fix_a)) = zug(sheet, a) else { break };
            if fix_a {
                break;
            }
            let mut fund = None;
            'such: for k in [0usize, 1] {
                let n = ap.len();
                let (e, nb) = if k == 1 { (ap[n - 1], ap[n - 2]) } else { (ap[0], ap[1]) };
                if anschluesse.iter().any(|q| q.dist(e) <= PIN_TOL) || klemmen.iter().any(|q| q.dist(e) <= crate::klemme_leitung::KLEMME_R + PIN_TOL) {
                    continue;
                }
                let r = e.sub(nb);
                for o in sheet.objects.iter().filter(|o| o.id != a) {
                    let Kind::Wire { pts: bp, .. } = &o.kind else { continue };
                    let Some((_, art_b, fix_b)) = zug(sheet, o.id) else { continue };
                    if fix_b || art_b != art_a {
                        continue;
                    }
                    // innerer Knick von B bei e
                    for j in 1..bp.len().saturating_sub(1) {
                        if bp[j].dist(e) > PIN_TOL {
                            continue;
                        }
                        for (seite, q) in [(1i32, bp[j + 1]), (-1, bp[j - 1])] {
                            let d = q.sub(e);
                            // gerade Fortsetzung: gleiche Richtung wie das Endstück von A
                            let fort = (r.x * d.y - r.y * d.x).abs() < 1e-3 && r.x * d.x + r.y * d.y > 0.0;
                            if fort {
                                fund = Some((k, o.id, j, seite));
                                break 'such;
                            }
                        }
                    }
                }
            }
            let Some((k, b, j, seite)) = fund else { break };
            let Some((bp, _, _)) = zug(sheet, b) else { break };
            // Teil von B ab dem Knick in Fortsetzungsrichtung, Rest bleibt B
            let (weiter, rest): (Vec<P>, Vec<P>) = if seite == 1 { (bp[j..].to_vec(), bp[..=j].to_vec()) } else { (bp[..=j].iter().rev().copied().collect(), bp[j..].to_vec()) };
            let mut v = ap.clone();
            if k == 0 {
                v.reverse();
            }
            v.extend_from_slice(&weiter[1..]);
            if k == 0 {
                v.reverse();
            }
            let v = crate::leitung_fuehrung::bereinigen(v);
            for o in sheet.objects.iter_mut() {
                if let Kind::Wire { pts, .. } = &mut o.kind {
                    if o.id == a {
                        *pts = v.clone();
                    } else if o.id == b {
                        *pts = crate::leitung_fuehrung::bereinigen(rest.clone());
                    }
                }
            }
            geaendert = true;
        }
    }
    geaendert
}

impl crate::app::App {
    /// Nach einer Bearbeitung: Treppen am Abgriff nachziehen und Leitungen, die an einem Knick einer anderen gerade
    /// weiterlaufen, zu einem Strang zusammenlegen (ohne eigenen Rückgängig-Schritt).
    pub fn leitungen_glaetten(&mut self, ids: &[u64]) {
        if self.sym_edit.is_some() {
            return;
        }
        let lib = self.lib.clone();
        let neu: Vec<(u64, Vec<P>)> = match self.sheet() {
            Some(s) => ids
                .iter()
                .filter_map(|id| match s.objects.iter().find(|o| o.id == *id).map(|o| &o.kind) {
                    Some(Kind::Wire { pts, .. }) => ende_nachziehen(s, &lib, *id, pts).map(|p| (*id, p)),
                    _ => None,
                })
                .collect(),
            None => vec![],
        };
        let ids = ids.to_vec();
        self.with_sheet(|s, _| {
            for (id, p) in neu {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *pts = p;
                }
            }
            durchgehend(s, &lib, &ids);
        });
    }

    /// Nach einer Bearbeitung: Leitungen `ids` mit stoßenden Leitungen zusammenfassen (ohne eigenen
    /// Rückgängig-Schritt). Entfallene Leitungen verlassen die Markierung.
    pub fn leitungen_zusammenfassen(&mut self, ids: &[u64]) {
        if self.sym_edit.is_some() {
            return;
        }
        let lib = self.lib.clone();
        let weg = self.with_sheet(|s, _| zusammenfassen(s, &lib, ids)).unwrap_or_default();
        if !weg.is_empty() {
            self.selection.retain(|id| !weg.contains(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
        Obj { id, kind: Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn pts(s: &Sheet, id: u64) -> Vec<P> {
        match &s.objects.iter().find(|o| o.id == id).unwrap().kind {
            Kind::Wire { pts, .. } => pts.clone(),
            _ => panic!(),
        }
    }

    #[test]
    fn in_einer_flucht_wird_eine_leitung() {
        let mut s = Sheet::new(1);
        s.objects.push(wire(1, &[(0.0, 0.0), (50.0, 0.0)]));
        s.objects.push(wire(2, &[(80.0, 0.0), (50.0, 0.0)]));
        // bearbeitet wurde 1: sie geht in der unveränderten 2 auf
        assert_eq!(zusammenfassen(&mut s, &[], &[1]), vec![1]);
        assert_eq!(pts(&s, 2), vec![P::new(80.0, 0.0), P::new(0.0, 0.0)]);
    }

    #[test]
    fn ecke_bleibt_als_knick() {
        let mut s = Sheet::new(1);
        s.objects.push(wire(1, &[(0.0, 0.0), (50.0, 0.0)]));
        s.objects.push(wire(2, &[(50.0, 0.0), (50.0, 40.0)]));
        zusammenfassen(&mut s, &[], &[2]);
        assert_eq!(s.objects.len(), 1);
        // die ältere Leitung (kleinere Id) bleibt
        assert_eq!(pts(&s, 1), vec![P::new(0.0, 0.0), P::new(50.0, 0.0), P::new(50.0, 40.0)]);
    }

    #[test]
    fn abzweig_oder_dritte_leitung_verhindert() {
        let mut s = Sheet::new(1);
        s.objects.push(wire(1, &[(0.0, 0.0), (50.0, 0.0)]));
        s.objects.push(wire(2, &[(50.0, 0.0), (90.0, 0.0)]));
        s.objects.push(wire(3, &[(50.0, 0.0), (50.0, 30.0)]));
        assert!(zusammenfassen(&mut s, &[], &[1, 2, 3]).is_empty());
        // andere Leitungsnummer
        let mut s = Sheet::new(1);
        s.objects.push(wire(1, &[(0.0, 0.0), (50.0, 0.0)]));
        let mut b = wire(2, &[(50.0, 0.0), (90.0, 0.0)]);
        if let Kind::Wire { leitung, .. } = &mut b.kind {
            *leitung = 3;
        }
        s.objects.push(b);
        assert!(zusammenfassen(&mut s, &[], &[1]).is_empty());
    }
}
