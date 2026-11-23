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

impl crate::app::App {
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
