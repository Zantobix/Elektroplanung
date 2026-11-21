//! Projekt prüfen (Tools › Projekt prüfen): Symbolanschlüsse ohne Verbindung und doppelt vergebene
//! Betriebsmittelkennzeichnungen. Reine Funktionen auf dem Datenmodell; Anzeige und Sprung in `suche.rs`.

use crate::canvas;
use crate::model::*;
use crate::symbols::{self, SymDef};
use std::collections::BTreeMap;

/// Fangabstand für Anschlüsse in mm (wie die Verbindungspunkte in `canvas::junctions`)
pub const TOL: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HinweisArt {
    OffenerAnschluss,
    DoppelteBmk,
}

impl HinweisArt {
    pub fn name(self) -> &'static str {
        match self {
            HinweisArt::OffenerAnschluss => "Offener Anschluss",
            HinweisArt::DoppelteBmk => "Doppelte BMK",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hinweis {
    pub sheet: SheetRef,
    /// Objekt-Id des betroffenen Symbols
    pub id: u64,
    pub art: HinweisArt,
    pub text: String,
    /// Lage des Anschlusses bzw. des Symbols in mm
    pub pos: P,
}

/// Ganzes Projekt prüfen: je Blatt die offenen Anschlüsse, je Anlage die doppelten Kennzeichnungen.
pub fn pruefen(project: &Project, lib: &[SymDef]) -> Vec<Hinweis> {
    let mut out = vec![];
    for (ai, a) in project.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            offene_anschluesse(SheetRef { anlage: ai, sheet: si }, a, s, lib, &mut out);
        }
        doppelte_bmk(ai, a, lib, &mut out);
    }
    out
}

/// Bezeichnung eines Symbols für Hinweistexte: angezeigte BMK, sonst Name der Symboldefinition.
fn symbol_name(o: &Obj, a: &Anlage, s: &Sheet, lib: &[SymDef]) -> String {
    let Kind::Symbol { key, bmk, ext, .. } = &o.kind else { return String::new() };
    let shown = symbols::display_bmk(bmk, ext.counter, &canvas::bmk_ctx(a, s));
    if !shown.is_empty() {
        return shown;
    }
    symbols::find(lib, key).map(|d| d.name.clone()).unwrap_or_else(|| key.clone())
}

/// Anschlüsse von Symbolen, an denen weder ein Leitungsende noch ein anderer Anschluss oder eine Klemmstelle
/// liegt. Als verbunden gilt auch ein Anschluss auf dem Verlauf einer Leitung oder Sammelschiene (Abzweig).
/// Symbole, deren Bibliotheksdefinition keine Anschlüsse hat (oder fehlt), werden übergangen.
pub fn offene_anschluesse(r: SheetRef, a: &Anlage, s: &Sheet, lib: &[SymDef], out: &mut Vec<Hinweis>) {
    // Gegenstellen des Blattes: Punkte (Anschlüsse anderer Symbole, Klemmstellen) und Strecken (Leitungen, Schienen)
    let mut points: Vec<(u64, P)> = vec![];
    let mut segs: Vec<(P, P)> = vec![];
    for o in &s.objects {
        match &o.kind {
            Kind::Wire { pts, .. } => {
                if pts.len() == 1 {
                    points.push((o.id, pts[0]));
                }
                segs.extend(pts.windows(2).map(|w| (w[0], w[1])));
            }
            Kind::Busbar { a, x2, .. } => segs.push((*a, P::new(*x2, a.y))),
            Kind::Terminal { pos, .. } => points.push((o.id, *pos)),
            Kind::Symbol { .. } => points.extend(canvas::obj_pins(o, lib).into_iter().map(|p| (o.id, p))),
            _ => {}
        }
    }
    for o in &s.objects {
        let Kind::Symbol { key, .. } = &o.kind else { continue };
        if symbols::find(lib, key).is_none_or(|d| d.pins.is_empty()) {
            continue;
        }
        for (k, p) in canvas::obj_pins(o, lib).into_iter().enumerate() {
            let verbunden = points.iter().any(|(id, q)| *id != o.id && q.dist(p) <= TOL) || segs.iter().any(|(sa, sb)| seg_dist(p, *sa, *sb) <= TOL);
            if !verbunden {
                out.push(Hinweis {
                    sheet: r,
                    id: o.id,
                    art: HinweisArt::OffenerAnschluss,
                    text: format!("{}: Anschluss {} ist nicht verbunden", symbol_name(o, a, s, lib), k + 1),
                    pos: p,
                });
            }
        }
    }
}

/// Doppelt vergebene angezeigte BMK innerhalb einer Anlage. Gleiche Kennzeichnungen sind normal, wenn sie
/// verschiedene Symbolarten verbinden (Kontakte einer Spule, Eigentümer › Setzen); ein Fehler ist es nur, wenn
/// alle Symbole mit dieser BMK dieselbe Symbolart (Bibliotheksschlüssel) haben.
pub fn doppelte_bmk(ai: usize, a: &Anlage, lib: &[SymDef], out: &mut Vec<Hinweis>) {
    // BMK → (Blatt, Objekt, Schlüssel, Lage)
    let mut groups: BTreeMap<String, Vec<(usize, u64, &str, P)>> = BTreeMap::new();
    for (si, s) in a.sheets.iter().enumerate() {
        let ctx = canvas::bmk_ctx(a, s);
        for o in &s.objects {
            let Kind::Symbol { key, pos, bmk, ext, .. } = &o.kind else { continue };
            let shown = symbols::display_bmk(bmk, ext.counter, &ctx);
            // leere oder reine Vorzeichen ("-", "=", "+") zählen nicht
            if shown.trim_matches(|c: char| c == '-' || c == '=' || c == '+' || c.is_whitespace()).is_empty() {
                continue;
            }
            groups.entry(shown).or_default().push((si, o.id, key.as_str(), *pos));
        }
    }
    for (bmk, list) in groups {
        if list.len() < 2 || list.iter().any(|(_, _, k, _)| *k != list[0].2) {
            continue;
        }
        // gleiche Symbole mit gleicher BMK auf einem Blatt sind eine gewollte Gruppe (z. B. mehrere Leuchten
        // eines Stromkreises im Installationsplan) – gemeldet wird nur, wenn die BMK auf mehreren Blättern steht
        if list.iter().all(|(si, ..)| *si == list[0].0) {
            continue;
        }
        let name = symbols::find(lib, list[0].2).map(|d| d.name.clone()).unwrap_or_else(|| list[0].2.to_string());
        for (si, id, _, pos) in &list {
            out.push(Hinweis {
                sheet: SheetRef { anlage: ai, sheet: *si },
                id: *id,
                art: HinweisArt::DoppelteBmk,
                text: format!("{bmk} ist {}-mal vergeben ({name})", list.len()),
                pos: *pos,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Symbol mit zwei Anschlüssen links/rechts (±5 mm) ohne Bibliothek.
    fn def(key: &str) -> SymDef {
        let mut d = SymDef::empty(key, key, "-K");
        d.pins = vec![P::new(-5.0, 0.0), P::new(5.0, 0.0)];
        d.prims = vec![symbols::Prim::Line(P::new(-5.0, 0.0), P::new(5.0, 0.0), 0)];
        d
    }

    fn sym(id: u64, key: &str, pos: P, bmk: &str) -> Obj {
        Obj {
            id,
            kind: Kind::Symbol { key: key.into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
            pen: 0,
            fixed: false,
            group: None,
            raw: None,
        }
    }

    fn wire(id: u64, pts: Vec<P>) -> Obj {
        Obj { id, kind: Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn findet_offenen_anschluss_und_doppelte_bmk() {
        let lib = vec![def("spule"), def("kontakt"), {
            let mut d = SymDef::empty("ohne", "ohne", "");
            d.prims = vec![symbols::Prim::Line(P::new(0.0, 0.0), P::new(1.0, 0.0), 0)];
            d
        }];
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        // K1 links an einer Leitung, rechts offen; K1-Kontakt auf demselben Blatt ist gewollt (andere Symbolart)
        s.objects.push(sym(1, "spule", P::new(50.0, 50.0), "-K1"));
        s.objects.push(wire(2, vec![P::new(30.0, 50.0), P::new(45.0, 50.0)]));
        // zweite Spule K2 und dritte Spule K2 (gleiche Symbolart → doppelt), beide voll angeschlossen
        s.objects.push(sym(3, "spule", P::new(50.0, 80.0), "-K2"));
        s.objects.push(wire(4, vec![P::new(30.0, 80.0), P::new(45.0, 80.0)]));
        s.objects.push(wire(5, vec![P::new(55.0, 80.0), P::new(70.0, 80.0)]));
        // gleiche Spule zweimal auf demselben Blatt = gewollte Gruppe, kein Hinweis
        s.objects.push(sym(13, "spule", P::new(50.0, 140.0), "-K3"));
        s.objects.push(sym(14, "spule", P::new(50.0, 160.0), "-K3"));
        s.objects.push(wire(15, vec![P::new(40.0, 140.0), P::new(80.0, 140.0)]));
        s.objects.push(wire(16, vec![P::new(40.0, 160.0), P::new(80.0, 160.0)]));
        s.objects.push(sym(8, "kontakt", P::new(100.0, 50.0), "-K1"));
        s.objects.push(sym(9, "kontakt", P::new(110.0, 50.0), "-K1"));
        s.objects.push(wire(10, vec![P::new(80.0, 50.0), P::new(95.0, 50.0)]));
        s.objects.push(wire(11, vec![P::new(115.0, 50.0), P::new(115.0, 60.0)]));
        // Symbol ohne Anschlüsse in der Definition und ohne BMK: nie ein Hinweis
        s.objects.push(sym(12, "ohne", P::new(150.0, 150.0), ""));
        // zweite -K2-Spule auf einem anderen Blatt → doppelt
        let mut s2 = Sheet::new(2);
        s2.objects.push(sym(6, "spule", P::new(50.0, 110.0), "-K2"));
        // Anschluss mitten auf einer Leitung (Abzweig) und Anschluss an Anschluss gelten als verbunden
        s2.objects.push(wire(7, vec![P::new(40.0, 110.0), P::new(80.0, 110.0)]));
        p.anlagen[0].sheets.push(s2);
        let h = pruefen(&p, &lib);
        let offen: Vec<_> = h.iter().filter(|x| x.art == HinweisArt::OffenerAnschluss).collect();
        assert_eq!(offen.len(), 1, "{h:#?}");
        assert_eq!(offen[0].id, 1);
        assert!(offen[0].pos.dist(P::new(55.0, 50.0)) < 1e-4);
        assert!(offen[0].text.contains("-K1") && offen[0].text.contains("Anschluss 2"), "{}", offen[0].text);
        let mut doppelt: Vec<u64> = h.iter().filter(|x| x.art == HinweisArt::DoppelteBmk).map(|x| x.id).collect();
        doppelt.sort();
        // K1 (Spule + zwei Kontakte) ist kein Fehler, K2 zweimal als Spule schon
        assert_eq!(doppelt, vec![3, 6], "{h:#?}");
    }

    #[test]
    fn doppelte_bmk_nur_innerhalb_der_anlage() {
        let lib = vec![def("spule")];
        let mut p = Project::new_default();
        let mut a2 = p.anlagen[0].clone();
        a2.name = "Anlage 2".into();
        p.anlagen.push(a2);
        p.anlagen[0].sheets[0].objects.push(sym(1, "spule", P::new(50.0, 50.0), "-K1"));
        p.anlagen[1].sheets[0].objects.push(sym(2, "spule", P::new(50.0, 50.0), "-K1"));
        let h = pruefen(&p, &lib);
        assert!(h.iter().all(|x| x.art != HinweisArt::DoppelteBmk), "{h:#?}");
    }
}
