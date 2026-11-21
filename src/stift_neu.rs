//! „Neuer Stift“ (Werkzeugleiste, Projektbaum, Symbolleiste des Symboleditors) wie im Original.
//!
//! Belegt aus dem Original (ToolButton7Click 0x4684b4): der Dialog TStift erscheint mit Vorgaben, angelegt wird der
//! Stift erst, wenn er mit OK schließt (ShowModal = 1, dann 0x406200); „Abbrechen“ legt nichts an. Knopf der
//! Hauptleiste und Eintrag im Projektbaum legen einen Projektstift an (auch während der Symbolbearbeitung), der
//! Knopf der Symbolleiste einen Stift des Symbols.

use crate::app::App;
use crate::extra::XDialog;
use crate::model::Stift;

/// Vorgaben eines neuen Stiftes (Name „Stift n“, durchgezogen, 0,25 mm, dunkelblau wie bisher im Nachbau).
pub fn vorgabe(n: usize) -> Stift {
    Stift { name: format!("Stift {n}"), style: 0, color: [0, 0, 160], width: 0.25 }
}

/// Projektstift `idx` übernehmen: vorhandenen ändern, `idx == len` = neuer Stift. Rückgabe: ob neu angelegt.
pub fn projekt_setzen(stifte: &mut Vec<Stift>, idx: usize, s: Stift) -> bool {
    let n = stifte.len();
    match stifte.get_mut(idx) {
        Some(x) => {
            *x = s;
            false
        }
        None if idx == n => {
            stifte.push(s);
            true
        }
        None => false,
    }
}

/// Stift `nr` des Symbols übernehmen: vorhandenen ändern, sonst neu anlegen. Rückgabe: ob neu angelegt.
pub fn symbol_setzen(pens: &mut Vec<(u32, Stift)>, nr: u32, s: Stift) -> bool {
    match pens.iter_mut().find(|(n, _)| *n == nr) {
        Some((_, x)) => {
            *x = s;
            false
        }
        None => {
            pens.push((nr, s));
            true
        }
    }
}

impl App {
    /// „Neuer Stift“: Dialog mit Vorgaben öffnen; `symbol` = Knopf der Symbolleiste im Symboleditor.
    pub fn neuer_stift(&mut self, symbol: bool) {
        let (idx, s) = match self.sym_edit.as_ref().filter(|_| symbol) {
            Some(e) => {
                let nr = crate::symbol_stifte::neue_nummer(&e.pens);
                (usize::MAX - nr as usize, vorgabe(nr as usize))
            }
            None => (self.project.stifte.len(), vorgabe(self.project.stifte.len() + 1)),
        };
        self.xdlg = Some(XDialog::Stift { idx, name: s.name, style: 0, width: format!("{:.2}", s.width).replace('.', ","), color: eframe::egui::Color32::from_rgb(s.color[0], s.color[1], s.color[2]) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erst_bei_ok_anlegen() {
        let mut v = vec![vorgabe(1)];
        assert!(projekt_setzen(&mut v, 1, vorgabe(2)));
        assert_eq!(v.len(), 2);
        // vorhandenen ändern, Lücke nicht anlegen
        assert!(!projekt_setzen(&mut v, 0, vorgabe(9)));
        assert!(!projekt_setzen(&mut v, 5, vorgabe(5)));
        assert_eq!((v.len(), v[0].name.as_str()), (2, "Stift 9"));
        let mut p: Vec<(u32, Stift)> = vec![];
        assert!(symbol_setzen(&mut p, 4, vorgabe(4)));
        assert!(!symbol_setzen(&mut p, 4, vorgabe(7)));
        assert_eq!((p.len(), p[0].1.name.as_str()), (1, "Stift 7"));
    }
}
