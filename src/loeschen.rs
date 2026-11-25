//! Löschen wie im Original (Gegenprobe Cloud 25.11.2026: W2 „Sicherungen -F10, -F1 und -F2“ im Original gelöscht,
//! gespeichert und ausgelesen):
//! - Entf auf ein Symbol löscht jede Leitung mit, deren Anfang oder Ende auf einem seiner Anschlüsse liegt (−F2 mit
//!   4 Anschlüssen: 56 → 52 Leitungen). Leitungen, die nur durch den Anschluss laufen, gab es dort nicht – sie
//!   bleiben.
//! - Wird eine Leitung gelöscht, deren Ende am Anfang einer Sammelschiene liegt (Einspeisung), setzt das Original die
//!   Schiene auf die Standardlage zurück (x wie der Werkzeugknopf „Neue Sammelschiene“, `schiene_neu::werkzeug_lage`;
//!   −F2.L 102,5…181,5 → 13…271), die Höhe bleibt.
//!
//! Beides hängt an der Option „Symbol löschen: angeschlossene Leitungen mitlöschen“ (Standard an, Nutzer 25.11.2026);
//! aus = nur die markierten Objekte (bisheriges Verhalten des Nachbaus, Leitungen bleiben als offene Stummel).

use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

/// Fangabstand für Anschluss/Leitungsende in mm.
const TOL: f32 = 0.05;

/// Leitungen, die beim Löschen von `sel` mitgehen: Anfang oder Ende auf einem Anschluss eines markierten Symbols.
pub fn mitzuloeschen(s: &Sheet, lib: &[SymDef], sel: &[u64]) -> Vec<u64> {
    let pins: Vec<P> = s.objects.iter().filter(|o| sel.contains(&o.id) && matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| canvas::obj_pins(o, lib)).collect();
    if pins.is_empty() {
        return vec![];
    }
    s.objects
        .iter()
        .filter(|o| !sel.contains(&o.id))
        .filter(|o| match &o.kind {
            Kind::Wire { pts, .. } if !pts.is_empty() => [pts[0], pts[pts.len() - 1]].iter().any(|e| pins.iter().any(|p| p.dist(*e) <= TOL)),
            _ => false,
        })
        .map(|o| o.id)
        .collect()
}

/// Sammelschienen, deren Anfang an einem Ende einer der gelöschten Leitungen lag, auf die Standardlage setzen.
pub fn schienen_zuruecksetzen(s: &mut Sheet, geloescht: &[Obj]) {
    let enden: Vec<P> = geloescht.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { (!pts.is_empty()).then(|| [pts[0], pts[pts.len() - 1]]) } else { None }).flatten().collect();
    if enden.is_empty() {
        return;
    }
    let (x1, x2n) = crate::schiene_neu::werkzeug_lage(s);
    for o in &mut s.objects {
        if let Kind::Busbar { a, x2, .. } = &mut o.kind {
            if enden.iter().any(|e| e.dist(*a) <= TOL) {
                a.x = x1;
                *x2 = x2n;
            }
        }
    }
}

impl crate::app::App {
    /// Markierte Objekte löschen (Entf, Bearbeiten › Löschen), mit den Nebenwirkungen des Originals (siehe oben).
    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        // nur ein Teilstück markiert (Mehrfachklick Stufe 1): nur dieses Stück löschen
        if self.teilstueck_loeschen() {
            return;
        }
        self.checkpoint();
        let mut sel = self.selection.clone();
        let wie_original = self.opt.loeschen_mit_leitungen;
        let lib = self.lib.clone();
        self.with_sheet(|s, _| {
            if wie_original {
                sel.extend(mitzuloeschen(s, &lib, &sel));
            }
            let weg: Vec<Obj> = s.objects.iter().filter(|o| sel.contains(&o.id)).cloned().collect();
            s.objects.retain(|o| !sel.contains(&o.id));
            if wie_original {
                schienen_zuruecksetzen(s, &weg);
            }
        });
        self.selection.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> SymDef {
        let mut d = SymDef::empty("ls", "ls", "-F");
        d.pins = vec![P::new(0.0, -5.0), P::new(0.0, 5.0)];
        d.prims = vec![crate::symbols::Prim::Line(P::new(0.0, -5.0), P::new(0.0, 5.0), 0)];
        d
    }

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    fn wire(id: u64, pts: Vec<P>) -> Obj {
        obj(id, Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 })
    }

    #[test]
    fn symbol_nimmt_leitungen_mit_und_schiene_springt_zurueck() {
        let lib = vec![def()];
        let mut s = Sheet::new(1);
        let sym = Kind::Symbol { key: "ls".into(), pos: P::new(100.0, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-F2".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() };
        s.objects.push(obj(1, sym));
        // oben an den Anschluss, unten vom Anschluss zum Anfang der Schiene
        s.objects.push(wire(2, vec![P::new(100.0, 20.0), P::new(100.0, 45.0)]));
        s.objects.push(wire(3, vec![P::new(100.0, 55.0), P::new(100.0, 70.0), P::new(110.0, 70.0)]));
        // fremde Leitung daneben und eine, die nur am Anschluss vorbeiläuft (kein Ende dort)
        s.objects.push(wire(4, vec![P::new(150.0, 20.0), P::new(150.0, 80.0)]));
        s.objects.push(wire(5, vec![P::new(90.0, 45.0), P::new(120.0, 45.0)]));
        s.objects.push(obj(6, Kind::Busbar { a: P::new(110.0, 70.0), x2: 200.0, name: "-F2.L".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 }));
        let mit = mitzuloeschen(&s, &lib, &[1]);
        assert_eq!(mit, vec![2, 3]);
        let weg: Vec<Obj> = s.objects.iter().filter(|o| [1, 2, 3].contains(&o.id)).cloned().collect();
        s.objects.retain(|o| ![1, 2, 3].contains(&o.id));
        schienen_zuruecksetzen(&mut s, &weg);
        let Kind::Busbar { a, x2, .. } = &s.objects.iter().find(|o| o.id == 6).unwrap().kind else { panic!() };
        // Querformat, Rand links 5 → 13 … 271, Höhe bleibt
        assert_eq!((a.x, a.y, *x2), (13.0, 70.0, 271.0));
    }
}
