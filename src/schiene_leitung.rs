//! Leitung von einer Sammelschiene zu einem Bauteil ziehen (todo.md Punkt 1 „Sammelschiene“, 22.11.2026).
//!
//! Über einer Sammelschiene zeigt der Zeiger wie über einem Anschluss das Leitungszeichen (hover.rs), ein Druck
//! begann aber keine Leitung – es entstand nichts. Jetzt beginnt das Ziehen auf einer (nicht markierten) Schiene
//! eine Leitung wie an einem Anschluss. Der Anfang wandert dabei auf der Schiene in die Flucht des Ziels: der
//! Leiter verlässt die Schiene senkrecht, der Knoten sitzt am Abgriff (Gegenrichtung zu 3c42c89, wo der Leiter
//! senkrecht auf der Schiene endet). Loslassen ohne Ziehen bzw. auf derselben Schiene legt keine Leitung an,
//! sondern markiert die Schiene (Klicken und Doppelklick auf die Schiene bleiben so wie bisher).

use crate::app::{App, Drag};
use crate::leitung_fuehrung::Fuehrung;
use crate::model::{Kind, Sheet, P};

/// Sammelschiene unter `m` (Toleranz `tol`): Kennung, Höhe und waagerechte Ausdehnung.
pub fn schiene_unter(sheet: &Sheet, m: P, tol: f32) -> Option<(u64, (f32, f32, f32))> {
    sheet.objects.iter().rev().find_map(|o| match &o.kind {
        Kind::Busbar { a, x2, .. } => {
            let (x1, x2) = (a.x.min(*x2), a.x.max(*x2));
            ((m.y - a.y).abs() <= tol && m.x >= x1 - tol && m.x <= x2 + tol).then_some((o.id, (a.y, x1, x2)))
        }
        _ => None,
    })
}

/// Liegt `ende` auf der Schiene, von der `f` ausgeht (dann entsteht keine Leitung)?
pub fn auf_eigener_schiene(f: &Fuehrung, ende: P) -> bool {
    f.schiene.is_some_and(|(y, x1, x2)| (ende.y - y).abs() < 0.01 && ende.x >= x1 - 0.01 && ende.x <= x2 + 0.01)
}

impl App {
    /// Beim Drücken (Auswahlwerkzeug, ohne Strg) auf eine nicht markierte Sammelschiene: Leitung beginnen.
    pub fn leitung_an_schiene_beginnen(&mut self, m: P, tol: f32) -> bool {
        if self.sym_edit.is_some() {
            return false;
        }
        let Some((id, s)) = self.sheet().and_then(|sh| schiene_unter(sh, m, tol)) else { return false };
        if self.selection.contains(&id) {
            return false;
        }
        self.drag = Drag::Wire { f: Fuehrung::von_schiene(m, s) };
        true
    }

    /// Beim Loslassen einer Leitung, die auf einer Schiene begann, ohne Ziel abseits der Schiene: Schiene unter dem
    /// Anfang markieren statt eine Leitung anzulegen. Liefert, ob das Loslassen damit erledigt ist.
    pub fn schiene_loslassen(&mut self, f: &Fuehrung, ende: P) -> bool {
        if f.schiene.is_none() || !(auf_eigener_schiene(f, ende) || ende.dist(f.start()) < 0.01) {
            return false;
        }
        let id = self.sheet().and_then(|sh| schiene_unter(sh, f.start(), 0.05)).map(|(id, _)| id);
        if let Some(id) = id {
            self.selection = vec![id];
        }
        self.drag = Drag::None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Obj;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    fn schiene(id: u64, y: f32) -> Obj {
        let kind = Kind::Busbar { a: p(20.0, y), x2: 260.0, name: "L1".into(), label: p(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 };
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn schiene_unter_maus() {
        let mut s = Sheet::new(1);
        s.objects.push(schiene(7, 30.0));
        assert_eq!(schiene_unter(&s, p(100.0, 30.3), 0.5), Some((7, (30.0, 20.0, 260.0))));
        assert_eq!(schiene_unter(&s, p(100.0, 31.0), 0.5), None);
    }

    #[test]
    fn leiter_verlaesst_schiene_senkrecht_zum_anschluss() {
        // Druck bei x = 100 auf der Schiene, Ziel F1:1 bei (41,3 | 80): Abgriff wandert in die Flucht des Anschlusses
        let f = Fuehrung::von_schiene(p(100.0, 30.2), (30.0, 20.0, 260.0));
        assert_eq!(f.start(), p(100.0, 30.0));
        assert_eq!(f.weg(p(41.3, 80.0)), vec![p(41.3, 30.0), p(41.3, 80.0)]);
        // Ziel links außerhalb der Schiene: Abgriff am Schienenanfang, erst senkrecht, dann waagerecht
        assert_eq!(f.weg(p(10.0, 80.0)), vec![p(20.0, 30.0), p(20.0, 80.0), p(10.0, 80.0)]);
        assert!(!auf_eigener_schiene(&f, p(41.3, 80.0)));
        assert!(auf_eigener_schiene(&f, p(150.0, 30.0)));
    }

    #[test]
    fn klickbetrieb_von_der_schiene() {
        let mut f = Fuehrung::von_schiene(p(100.0, 30.0), (30.0, 20.0, 260.0));
        f.klickbetrieb = true;
        // erster Eckpunkt: senkrecht unter dem Klick von der Schiene ab
        assert!(f.klick(p(60.0, 50.0), 1.0));
        assert_eq!(f.pts, vec![p(60.0, 30.0), p(60.0, 50.0)]);
        assert_eq!(f.weg(p(80.0, 70.0)).first(), Some(&p(60.0, 30.0)));
    }
}
