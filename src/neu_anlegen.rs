//! Neu anlegen und Löschen im Projektexplorer wie im Original.
//!
//! Anlegen (ToolButton3Click 0x461018 Potential, NeueLeitung1Click 0x40b274 Leitung, ToolButton15Click 0x4610f4
//! Klemmleiste, Textersetzungsliste1Click 0x490558 Platzhaltergruppe, NeueSammelschine1Click 0x40b7ac
//! Sammelschiene): Das Original legt den Eintrag mit Vorgaben an, öffnet seinen Eigenschaftendialog und entfernt
//! ihn wieder, wenn der Dialog abgebrochen wird (die Sammelschiene entsteht erst bei OK). Der Nachbau legt an,
//! merkt sich den Rückgängig-Stand und stellt ihn bei Abbrechen wieder her.
//!
//! Löschen (Lschen1Click 0x4320d4, Zweige „Sammelschienen“, „Stifte“, „Potentiale“, „Klemmleisten“,
//! „Leitungen“): das Original nimmt den Eintrag nur aus seiner Liste (0x433404, 0x42ae9c, 0x42b880, 0x42b928,
//! 0x42ad84); Objekte, die ihn benutzen, behalten ihre Kennung. Im Nachbau verweisen Leitungen und Objekte über
//! den Listenplatz, deshalb rücken sie nach; wer den gelöschten Eintrag benutzte, bekommt den ersten.

use crate::app::{App, TAction};
use crate::model::*;

/// Name der n-ten Platzhaltergruppe wie im Original (0x406a5c): „Gruppe“ und zweistellige Nummer.
pub fn platzhalter_name(n: usize) -> String {
    format!("Gruppe{:02}", n % 100)
}

/// Vorgaben einer neuen Leitung wie im Original (0x4045f0): Querschnitt 1,00 mm², 1 Ader, schwarz, ohne
/// Kennzeichnung, durchgezogen; Name „Leitung <Kennung>“.
pub fn leitung_vorgabe(kennung: i32) -> Leitung {
    Leitung { name: format!("Leitung {kennung}"), color: [0, 0, 0], width_mm: 0.25, quer: 1.0, adern: 1, kennz: 0, style: 0, id: kennung }
}

/// Nächste freie Kennung für Leitungen und Potentiale (das Original zählt beide mit demselben Zähler des Projekts,
/// PRJ +0x94).
pub fn naechste_kennung(p: &Project) -> i32 {
    let l = crate::epr_projekt::leitung_ids(p).into_iter().max().unwrap_or(0);
    let pot = crate::epr_projekt::pot_ids(p).into_iter().max().unwrap_or(0);
    l.max(pot).max(0) + 1
}

/// Leitung `i` löschen; Verbindungen rücken nach (benutzten sie die gelöschte, gilt die erste).
pub fn leitung_loeschen(p: &mut Project, i: usize) -> bool {
    if i >= p.leitungen.len() {
        return false;
    }
    p.leitungen.remove(i);
    for a in &mut p.anlagen {
        for s in &mut a.sheets {
            for o in &mut s.objects {
                if let Kind::Wire { leitung, .. } = &mut o.kind {
                    if *leitung == i {
                        *leitung = 0;
                    } else if *leitung > i {
                        *leitung -= 1;
                    }
                }
            }
        }
    }
    true
}

/// Eigenen Stift `i` löschen (die festen Stifte 0–3 bleiben); Objekte und Sammelschienen rücken nach.
pub fn stift_loeschen(p: &mut Project, i: usize) -> bool {
    if i < crate::epr_projekt::FESTE_STIFTE || i >= p.stifte.len() {
        return false;
    }
    p.stifte.remove(i);
    let nach = |pen: &mut usize| {
        if *pen == i {
            *pen = 0;
        } else if *pen > i {
            *pen -= 1;
        }
    };
    for a in &mut p.anlagen {
        for s in &mut a.sheets {
            for o in &mut s.objects {
                nach(&mut o.pen);
            }
        }
        for set in a.schienen.values_mut() {
            nach(&mut set.stift);
        }
    }
    true
}

/// Potential `i` löschen (mit Nennwerten).
pub fn potential_loeschen(p: &mut Project, i: usize) -> bool {
    if i >= p.potentiale.len() {
        return false;
    }
    let name = p.potentiale.remove(i);
    if !p.potentiale.contains(&name) {
        p.pot_werte.remove(&name);
    }
    true
}

/// Klemmleiste `i` der Anlage `a` löschen (Klemmstellen auf den Blättern behalten ihren Leistennamen).
pub fn klemmleiste_loeschen(p: &mut Project, a: usize, i: usize) -> bool {
    let Some(anl) = p.anlagen.get_mut(a) else { return false };
    if i >= anl.klemmleisten.len() {
        return false;
    }
    let name = anl.klemmleisten.remove(i);
    anl.klemmen.retain(|k| k.name != name);
    true
}

/// Sammelschiene `i` der Anlage `a` aus dem Projektbaum löschen (Schienen auf den Blättern bleiben stehen).
pub fn schiene_loeschen(p: &mut Project, a: usize, i: usize) -> bool {
    let Some(anl) = p.anlagen.get_mut(a) else { return false };
    if i >= anl.sammelschienen.len() {
        return false;
    }
    let name = anl.sammelschienen.remove(i);
    if !anl.sammelschienen.contains(&name) {
        anl.schienen.remove(&name);
    }
    true
}

impl App {
    /// Nach dem Anlegen (mit `checkpoint`): Rückgängig-Stand merken, damit Abbrechen des Dialogs den Eintrag
    /// wieder entfernt.
    pub fn neu_merken(&mut self) {
        self.zusatz.neu = Some(self.undo.len());
    }

    /// Dialog des neu angelegten Eintrags abgebrochen: Stand vor dem Anlegen wiederherstellen.
    pub fn neu_verwerfen(&mut self) {
        let Some(n) = self.zusatz.neu.take() else { return };
        if n == 0 || self.undo.len() != n {
            return;
        }
        if let Some(p) = self.undo.pop() {
            self.project = p;
            self.fix_tabs();
            self.leitung = self.leitung.min(self.project.leitungen.len().saturating_sub(1));
            self.pen = self.pen.min(self.project.stifte.len().saturating_sub(1));
            self.klemme = self.klemme.min(self.klemmleisten().len().saturating_sub(1));
        }
    }

    /// Jedes Bild: ist kein Dialog mehr offen, wurde der neue Eintrag übernommen.
    pub fn neu_pruefen(&mut self) {
        if self.zusatz.neu.is_some() && self.dlg2.is_none() && self.dlg3.is_none() && self.xdlg.is_none() {
            self.zusatz.neu = None;
        }
    }

    /// Neues Potential (Werkzeugleiste, Hinzufügen › Potential, Explorer): anlegen und Dialog öffnen.
    pub fn potential_neu(&mut self) {
        self.new_potential();
        self.neu_merken();
        let i = self.project.potentiale.len() - 1;
        self.open_potential_dialog(i);
    }

    /// Neue Leitung (Werkzeugleiste, Hinzufügen › Leitung, Explorer): anlegen und Dialog öffnen.
    pub fn leitung_neu(&mut self) {
        self.new_leitung();
        self.neu_merken();
        self.open_leitung_dialog(self.leitung);
    }

    /// Neue Klemmleiste (Werkzeugleiste, Hinzufügen › Klemmleiste): anlegen und Dialog öffnen.
    pub fn klemmleiste_neu(&mut self) {
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        let vorher = self.undo.len();
        self.new_klemmleiste();
        if self.undo.len() == vorher {
            return;
        }
        self.neu_merken();
        let i = self.klemme;
        self.open_kleiste_dialog(a, i);
    }

    /// Löschen im Projektexplorer für Potential, Leitung, Stift, Klemmleiste und Sammelschiene (nach der
    /// Rückfrage). Rückgabe: Knoten war von dieser Art.
    pub fn explorer_loeschen(&mut self, action: &TAction) -> bool {
        let mut p = self.project.clone();
        let ok = match *action {
            TAction::Potential(i) => potential_loeschen(&mut p, i),
            TAction::Leitung(i) => leitung_loeschen(&mut p, i),
            TAction::Stift(i) => stift_loeschen(&mut p, i),
            TAction::Klemmleiste(a, i) => klemmleiste_loeschen(&mut p, a, i),
            TAction::Schiene(a, i) => schiene_loeschen(&mut p, a, i),
            _ => return false,
        };
        if ok {
            self.checkpoint();
            self.project = p;
            self.leitung = self.leitung.min(self.project.leitungen.len().saturating_sub(1));
            self.pen = self.pen.min(self.project.stifte.len().saturating_sub(1));
            self.klemme = self.klemme.min(self.klemmleisten().len().saturating_sub(1));
            self.dirty = true;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projekt() -> Project {
        let mut p = Project::new_default();
        p.leitungen = vec![leitung_vorgabe(1), leitung_vorgabe(2), leitung_vorgabe(3)];
        let mut s = Sheet::new(1);
        for (id, l) in [(1u64, 0usize), (2, 1), (3, 2)] {
            s.objects.push(Obj { id, kind: Kind::Wire { pts: vec![P::new(0.0, 0.0), P::new(5.0, 0.0)], leitung: l, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        }
        p.anlagen[0].sheets = vec![s];
        p
    }

    fn leitungen(p: &Project) -> Vec<usize> {
        p.anlagen[0].sheets[0].objects.iter().filter_map(|o| if let Kind::Wire { leitung, .. } = o.kind { Some(leitung) } else { None }).collect()
    }

    #[test]
    fn vorgaben_wie_original() {
        assert_eq!(platzhalter_name(1), "Gruppe01");
        assert_eq!(platzhalter_name(12), "Gruppe12");
        let l = leitung_vorgabe(7);
        assert_eq!((l.name.as_str(), l.quer, l.adern, l.color, l.kennz, l.style), ("Leitung 7", 1.0, 1, [0, 0, 0], 0, 0));
    }

    #[test]
    fn leitung_loeschen_rueckt_nach() {
        let mut p = projekt();
        assert!(leitung_loeschen(&mut p, 1));
        assert_eq!(p.leitungen.len(), 2);
        assert_eq!(leitungen(&p), vec![0, 0, 1]);
        assert!(!leitung_loeschen(&mut p, 5));
    }

    #[test]
    fn stift_loeschen_nur_eigene() {
        let mut p = projekt();
        let n = p.stifte.len();
        assert!(!stift_loeschen(&mut p, 0));
        p.stifte.push(Stift { name: "A".into(), style: 0, color: [1, 2, 3], width: 0.5 });
        p.stifte.push(Stift { name: "B".into(), style: 0, color: [1, 2, 3], width: 0.5 });
        p.anlagen[0].sheets[0].objects[0].pen = n;
        p.anlagen[0].sheets[0].objects[1].pen = n + 1;
        assert!(stift_loeschen(&mut p, n));
        assert_eq!(p.stifte.len(), n + 1);
        assert_eq!(p.stifte[n].name, "B");
        let pens: Vec<usize> = p.anlagen[0].sheets[0].objects.iter().map(|o| o.pen).collect();
        assert_eq!(pens, vec![0, n, 0]);
    }

    #[test]
    fn potential_klemmleiste_schiene() {
        let mut p = projekt();
        p.potentiale = vec!["L1".into(), "N".into()];
        p.pot_werte.insert("L1".into(), ("230".into(), "16".into()));
        assert!(potential_loeschen(&mut p, 0));
        assert_eq!(p.potentiale, vec!["N".to_string()]);
        assert!(p.pot_werte.is_empty());
        let n = p.anlagen[0].klemmleisten.len();
        crate::klemmen::new_leiste(&mut p, 0);
        assert!(klemmleiste_loeschen(&mut p, 0, n));
        assert_eq!(p.anlagen[0].klemmleisten.len(), n);
        p.anlagen[0].sammelschienen.push("L1".into());
        p.anlagen[0].schienen.insert("L1".into(), SchieneSet::default());
        let i = p.anlagen[0].sammelschienen.len() - 1;
        assert!(schiene_loeschen(&mut p, 0, i));
        assert!(!p.anlagen[0].schienen.contains_key("L1"));
    }
}
