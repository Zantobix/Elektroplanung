//! Netz-Index zur Laufzeit: welche Leitungen und Verbindungsobjekte eines Blattes elektrisch zusammenhängen.
//!
//! Neu geschrieben nach QElectroTech `Conductor::relatedPotentialConductors` (GPL-2.0-or-later; kein Code
//! übernommen): Ein Potential sind alle Leiter, die über gemeinsame Anschlüsse verbunden sind. Das Original kennt
//! keinen Netzbegriff; es speichert nur je Leitungsende die Anschlussart (`knoten.rs`, `vbg_enden.rs`). Der Index
//! wird deshalb aus der Geometrie berechnet und nicht gespeichert; die Datei bleibt unverändert.
//!
//! Verbunden sind (Toleranz wie die Knotenpunkte):
//! - zwei Leitungsenden am selben Ort, auch am selben Symbolanschluss (Fang `mitziehen::PIN_TOL`),
//! - ein Leitungsende im Innern einer anderen Leitung (T-Abzweig, Art 3) – reine Kreuzungen nicht,
//! - ein Leitungsende auf einer Sammelschiene (Art 2) mit der Schiene; gleichnamige Schienen eines Blattes sind
//!   ein Potential,
//! - ein Leitungsende am Kreis einer Klemmstelle (Art 5; in eine Leitung eingesetzte Klemmen verbinden beide Teile),
//! - ein Leitungsende an der Spitze eines Verweises.
//!
//! Ein Symbol verbindet seine Anschlüsse nicht untereinander (Kontakt, Spule …); es ist kein Teil eines Netzes,
//! seine berührten Anschlüsse liefert `anschluesse`.

use crate::canvas::obj_pins;
use crate::model::{seg_dist, Kind, Sheet, P};
use crate::symbols::SymDef;

const T: f32 = 0.05;

/// Einfache Vereinigungs-Suche über die Objektindizes des Blattes.
struct Mengen(Vec<usize>);

impl Mengen {
    fn wurzel(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }

    fn vereinen(&mut self, a: usize, b: usize) {
        let (a, b) = (self.wurzel(a), self.wurzel(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// Nimmt das Objekt am Netz teil (Leitung oder Verbindungsobjekt)?
fn netzobjekt(k: &Kind) -> bool {
    matches!(k, Kind::Wire { .. } | Kind::Busbar { .. } | Kind::Terminal { .. } | Kind::Verweis { .. })
}

fn im_innern(pts: &[P], p: P) -> bool {
    pts.len() >= 2 && pts[0].dist(p) > T && pts[pts.len() - 1].dist(p) > T && pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) < T)
}

/// Berührt das Leitungsende `e` das Verbindungsobjekt `k` (Schiene, Klemmstelle, Verweis)?
fn beruehrt(k: &Kind, e: P) -> bool {
    match k {
        Kind::Busbar { a, x2, .. } => (e.y - a.y).abs() < T && e.x >= a.x.min(*x2) - T && e.x <= a.x.max(*x2) + T,
        Kind::Terminal { pos, .. } => pos.dist(e) <= crate::klemme_leitung::KLEMME_R + T,
        Kind::Verweis { pos, .. } => pos.dist(e) < T,
        _ => false,
    }
}

/// Alle Netze des Blattes: je Netz die Kennungen seiner Leitungen und Verbindungsobjekte (aufsteigend sortiert).
/// Jede Leitung gehört zu genau einem Netz; ein Verbindungsobjekt ohne Leitung bildet ein eigenes Netz.
pub fn netze(sheet: &Sheet, lib: &[SymDef]) -> Vec<Vec<u64>> {
    netze_ohne(sheet, lib, &[])
}

/// Wie `netze`; die Leitungsenden `ohne` (Leitung, 0 = Anfang bzw. 1 = Ende) verbinden nichts (Netz-Wächter: ein
/// freies Ende, das der Nutzer gezielt auf etwas zieht).
pub fn netze_ohne(sheet: &Sheet, lib: &[SymDef], ohne: &[(u64, usize)]) -> Vec<Vec<u64>> {
    let objs: Vec<usize> = (0..sheet.objects.len()).filter(|&i| netzobjekt(&sheet.objects[i].kind)).collect();
    let mut m = Mengen((0..sheet.objects.len()).collect());
    let enden: Vec<(usize, P)> = objs
        .iter()
        .filter_map(|&i| match &sheet.objects[i].kind {
            Kind::Wire { pts, .. } if !pts.is_empty() => Some([(i, 0usize, pts[0]), (i, 1, pts[pts.len() - 1])]),
            _ => None,
        })
        .flatten()
        .filter(|(i, k, _)| !ohne.contains(&(sheet.objects[*i].id, *k)))
        .map(|(i, _, p)| (i, p))
        .collect();
    // Symbolanschlüsse: Enden am selben Anschluss hängen zusammen, auch wenn sie nicht genau aufeinander liegen
    let pins: Vec<P> = sheet.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| obj_pins(o, lib)).collect();
    let pin_von = |e: P| pins.iter().position(|q| q.dist(e) <= crate::mitziehen::PIN_TOL);
    let ende_pins: Vec<Option<usize>> = enden.iter().map(|(_, e)| pin_von(*e)).collect();
    for (k, &(i, e)) in enden.iter().enumerate() {
        for (l, &(j, f)) in enden.iter().enumerate().skip(k + 1) {
            if i != j && (e.dist(f) < T || (ende_pins[k].is_some() && ende_pins[k] == ende_pins[l])) {
                m.vereinen(i, j);
            }
        }
        for &j in &objs {
            let ok = match &sheet.objects[j].kind {
                Kind::Wire { pts, .. } => j != i && im_innern(pts, e),
                k => beruehrt(k, e),
            };
            if ok {
                m.vereinen(i, j);
            }
        }
    }
    // gleichnamige Sammelschienen eines Blattes
    let schienen: Vec<(usize, &str)> = objs.iter().filter_map(|&i| if let Kind::Busbar { name, .. } = &sheet.objects[i].kind { Some((i, name.as_str())) } else { None }).collect();
    for (k, (i, a)) in schienen.iter().enumerate() {
        for (j, b) in &schienen[k + 1..] {
            if !a.trim().is_empty() && a.trim() == b.trim() {
                m.vereinen(*i, *j);
            }
        }
    }
    let mut gruppen: Vec<(usize, Vec<u64>)> = vec![];
    for &i in &objs {
        let w = m.wurzel(i);
        match gruppen.iter_mut().find(|(r, _)| *r == w) {
            Some((_, v)) => v.push(sheet.objects[i].id),
            None => gruppen.push((w, vec![sheet.objects[i].id])),
        }
    }
    gruppen
        .into_iter()
        .map(|(_, mut v)| {
            v.sort_unstable();
            v
        })
        .collect()
}

/// Netz, zu dem das Objekt `id` (Leitung oder Verbindungsobjekt) gehört; leer, wenn es keins ist.
pub fn netz_von(sheet: &Sheet, lib: &[SymDef], id: u64) -> Vec<u64> {
    netze(sheet, lib).into_iter().find(|n| n.contains(&id)).unwrap_or_default()
}

/// Nur die Leitungen eines Netzes.
pub fn leitungen(sheet: &Sheet, netz: &[u64]) -> Vec<u64> {
    sheet.objects.iter().filter(|o| netz.contains(&o.id) && matches!(o.kind, Kind::Wire { .. })).map(|o| o.id).collect()
}

/// Symbolanschlüsse, an denen eine Leitung des Netzes endet: (Kennung des Symbols, Lage des Anschlusses).
pub fn anschluesse(sheet: &Sheet, lib: &[SymDef], netz: &[u64]) -> Vec<(u64, P)> {
    let enden: Vec<P> = sheet
        .objects
        .iter()
        .filter(|o| netz.contains(&o.id))
        .filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None })
        .filter(|pts| !pts.is_empty())
        .flat_map(|pts| [pts[0], pts[pts.len() - 1]])
        .collect();
    sheet
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }))
        .flat_map(|o| obj_pins(o, lib).into_iter().map(move |q| (o.id, q)))
        .filter(|(_, q)| enden.iter().any(|e| e.dist(*q) <= crate::mitziehen::PIN_TOL))
        .collect()
}

/// Leitungskette um die Leitung `id` (Mehrfachklick, Nutzer 24.11.2026 nach PADS): an jedem Ende weiter, solange dort
/// genau eine andere Leitung mit ihrem Ende anschließt und sonst nichts (kein Anschluss, keine Klemmstelle, keine
/// Schiene, kein Abzweig) – also bis zum nächsten Knoten.
pub fn kette(sheet: &Sheet, lib: &[SymDef], id: u64) -> Vec<u64> {
    let enden = |o: &crate::model::Obj| match &o.kind {
        Kind::Wire { pts, .. } if pts.len() >= 2 => Some([pts[0], pts[pts.len() - 1]]),
        _ => None,
    };
    let pins: Vec<P> = sheet.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| obj_pins(o, lib)).collect();
    let mut out = vec![id];
    let mut offen = vec![id];
    while let Some(w) = offen.pop() {
        let Some(e) = sheet.objects.iter().find(|o| o.id == w).and_then(|o| enden(o)) else { continue };
        for p in e {
            if pins.iter().any(|q| q.dist(p) <= crate::mitziehen::PIN_TOL) {
                continue;
            }
            let mut naechste = vec![];
            let mut sonst = false;
            for o in sheet.objects.iter().filter(|o| o.id != w) {
                match &o.kind {
                    Kind::Wire { pts, .. } if pts.len() >= 2 => {
                        if pts[0].dist(p) < T || pts[pts.len() - 1].dist(p) < T {
                            naechste.push(o.id);
                        } else if im_innern(pts, p) {
                            sonst = true;
                        }
                    }
                    k => {
                        if beruehrt(k, p) {
                            sonst = true;
                        }
                    }
                }
            }
            // auch ein Abzweig im Innern einer Kettenleitung an diesem Punkt zählt nicht weiter (Knoten)
            if !sonst && naechste.len() == 1 && !out.contains(&naechste[0]) {
                out.push(naechste[0]);
                offen.push(naechste[0]);
            }
        }
    }
    out.sort_unstable();
    out
}

impl crate::app::App {
    /// Doppelklick auf eine Leitung: die Kette bis zum nächsten Knoten markieren.
    pub fn kette_markieren(&mut self, id: u64) {
        let Some(s) = self.sheet() else { return };
        let ids = kette(s, &self.lib, id);
        self.info = format!("Leitung bis zum nächsten Knoten: {} Leitung(en) (Dreifachklick: ganzes Netz)", ids.len());
        self.selection = ids;
    }

    /// Kontextmenü „Netz markieren“ (bewusste Erweiterung): alle Leitungen markieren, die mit der Leitung `id`
    /// elektrisch zusammenhängen.
    pub fn netz_markieren(&mut self, id: u64) {
        let Some(s) = self.sheet() else { return };
        let netz = netz_von(s, &self.lib, id);
        let ids = leitungen(s, &netz);
        if ids.is_empty() {
            return;
        }
        self.info = format!("Netz: {} Leitungen, {} Anschlüsse", ids.len(), anschluesse(s, &self.lib, &netz).len());
        self.selection = ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Obj;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    fn leitung(id: u64, pts: Vec<P>) -> Obj {
        obj(id, Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 })
    }

    fn schiene(id: u64, y: f32, name: &str) -> Obj {
        obj(id, Kind::Busbar { a: p(0.0, y), x2: 200.0, name: name.into(), label: p(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 })
    }

    #[test]
    fn abzweig_schiene_klemme_aber_keine_kreuzung() {
        let mut s = Sheet::new(1);
        s.objects.push(schiene(1, 10.0, "L1"));
        s.objects.push(leitung(2, vec![p(50.0, 10.0), p(50.0, 80.0)]));
        // T-Abzweig auf Leitung 2
        s.objects.push(leitung(3, vec![p(90.0, 40.0), p(50.0, 40.0)]));
        // kreuzt Leitung 2 nur
        s.objects.push(leitung(4, vec![p(30.0, 60.0), p(70.0, 60.0)]));
        // Klemmstelle in einer Leitung: zwei Teile enden 0,8 mm vor der Mitte
        let r = crate::klemme_leitung::KLEMME_R;
        s.objects.push(obj(5, Kind::Terminal { pos: p(120.0, 50.0), leiste: "X1".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }));
        s.objects.push(leitung(6, vec![p(120.0, 10.0), p(120.0, 50.0 - r)]));
        s.objects.push(leitung(7, vec![p(120.0, 50.0 + r), p(120.0, 90.0)]));
        // zweite Schiene L1 weiter unten, eigene Leitung, und eine fremde Schiene L2
        s.objects.push(schiene(8, 150.0, "L1"));
        s.objects.push(leitung(9, vec![p(10.0, 150.0), p(10.0, 190.0)]));
        s.objects.push(schiene(10, 170.0, "L2"));
        // Ende an Ende in einer Flucht
        s.objects.push(leitung(11, vec![p(30.0, 60.0), p(30.0, 120.0)]));
        let mut n = netze(&s, &[]);
        n.sort();
        assert_eq!(n, vec![vec![1, 2, 3, 5, 6, 7, 8, 9], vec![4, 11], vec![10]]);
        assert_eq!(netz_von(&s, &[], 4), vec![4, 11]);
        assert_eq!(leitungen(&s, &netz_von(&s, &[], 3)), vec![2, 3, 6, 7, 9]);
        assert!(netz_von(&s, &[], 99).is_empty());
    }

    #[test]
    fn symbol_trennt_seine_anschluesse() {
        // zwei Leitungen an je einem Ende an derselben Stelle (Anschluss) bilden ein Netz, eine dritte daneben nicht
        let mut s = Sheet::new(1);
        s.objects.push(leitung(1, vec![p(0.0, 0.0), p(20.0, 0.0)]));
        s.objects.push(leitung(2, vec![p(20.0, 0.0), p(20.0, 30.0)]));
        s.objects.push(leitung(3, vec![p(20.0, 40.0), p(20.0, 60.0)]));
        let mut n = netze(&s, &[]);
        n.sort();
        assert_eq!(n, vec![vec![1, 2], vec![3]]);
        assert!(anschluesse(&s, &[], &[1, 2]).is_empty());
    }

    #[test]
    fn kette_bis_zum_knoten() {
        let w = |id: u64, v: &[(f32, f32)]| Obj { id, kind: Kind::Wire { pts: v.iter().map(|&(x, y)| p(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None };
        let mut s = Sheet::new(1);
        // 1 → 2 → 3 Ende an Ende, an (20|10) ein Knoten mit 4 und 5; 6 hängt frei an 5
        s.objects.push(w(1, &[(0.0, 0.0), (10.0, 0.0)]));
        s.objects.push(w(2, &[(10.0, 0.0), (10.0, 10.0)]));
        s.objects.push(w(3, &[(10.0, 10.0), (20.0, 10.0)]));
        s.objects.push(w(4, &[(20.0, 10.0), (30.0, 10.0)]));
        s.objects.push(w(5, &[(20.0, 10.0), (20.0, 30.0)]));
        s.objects.push(w(6, &[(20.0, 30.0), (40.0, 30.0)]));
        assert_eq!(kette(&s, &[], 2), vec![1, 2, 3]);
        assert_eq!(kette(&s, &[], 5), vec![5, 6]);
        assert_eq!(kette(&s, &[], 4), vec![4]);
        // ganzes Netz: alle sechs
        assert_eq!(netze(&s, &[]).len(), 1);
    }
}
