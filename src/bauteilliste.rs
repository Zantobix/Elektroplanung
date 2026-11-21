//! Kontextmenü „Bauteilliste › Bearbeiten“ an Symbol oder Klemmstelle: Fenster mit der Liste der zugeordneten
//! Bauteile, in die man Bauteile aus dem Baum zieht.
//!
//! Belegt aus dem Original: der Dialog ist TSSpiegel in der Betriebsart 1 (Feld +0x324; Betriebsart 0 ist der
//! Kontaktspiegel). Kopfzeile (0x4b6b6c) „Nr | Bezeichnung | Typ | Intern“. ListeDragOver (0x4b6cb8) nimmt nur in
//! Betriebsart 1 an: gezogen aus dem Reiter Bauteile (TeilListe) oder aus dem Projektbaum, Gruppe Bauteile (6), und
//! nur, wenn das Bauteil im Projekt gefunden wird (0x417b24). ListeDragDrop (0x4b6e80) füllt die letzte (leere)
//! Zeile: Nummer, Name (BTL +0x2c), Typ (+0x54), Intern Nr (+0xa4) und merkt die Kennung des Bauteils für die Zeile.
//! Im .epr steht die Liste als IDL-Unterblöcke des Symbols bzw. der Klemmstelle (epr_zusatz.rs).
//!
//! Der Nachbau führt je Objekt **ein** Bauteil (`Project::part_of`): Ablegen ersetzt die Zuordnung.

use crate::app::App;
use crate::model::*;
use eframe::egui;

/// Nutzlast beim Ziehen eines Bauteils aus dem Baum: Index in `Project::bauteile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BauteilZug(pub usize);

/// Zustand des Fensters: Objekt (Symbol oder Klemmstelle) und zugeordnetes Bauteil.
#[derive(Clone, Debug, PartialEq)]
pub struct BauteilListeDlg {
    pub obj: u64,
    pub bauteil: Option<usize>,
}

/// Zeilen der Liste wie im Original: Nr, Bezeichnung (Name), Typ, Intern (Nummer).
pub fn zeilen(p: &Project, bauteil: Option<usize>) -> Vec<[String; 4]> {
    bauteil.and_then(|i| p.bauteile.get(i)).map(|b| vec![["1".to_string(), b.name.clone(), b.typ.clone(), b.nummer.clone()]]).unwrap_or_default()
}

/// Ablegen eines gezogenen Bauteils: nur Bauteile des Projekts (wie ListeDragOver).
pub fn ablegen(d: &mut BauteilListeDlg, p: &Project, zug: BauteilZug) -> bool {
    if zug.0 >= p.bauteile.len() {
        return false;
    }
    d.bauteil = Some(zug.0);
    true
}

impl App {
    /// Bauteilliste › Bearbeiten (Bauteilliste1Click 0x49d57c): nur für Symbole und Klemmstellen.
    pub fn bauteilliste_oeffnen(&mut self, obj: u64) {
        let ok = self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.id == obj && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })));
        if ok {
            self.zusatz.bauteilliste = Some(BauteilListeDlg { obj, bauteil: self.project.part_of.get(&obj).copied() });
        }
    }

    /// Fenster zeichnen (nicht modal wie im Original, damit man aus dem Baum ziehen kann) und übernehmen.
    pub fn bauteilliste_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.zusatz.bauteilliste.take() else { return };
        let mut done: Option<bool> = None;
        let mut neu = false;
        let namen: Vec<String> = self.project.bauteile.iter().map(|b| format!("{} {}", b.name, b.typ).trim().to_string()).collect();
        egui::Window::new("Bauteilliste").collapsible(false).resizable(false).default_width(300.0).show(ctx, |ui| {
            let rows = zeilen(&self.project, d.bauteil);
            let (_, gezogen) = ui.dnd_drop_zone::<BauteilZug, _>(egui::Frame::default().inner_margin(4.0), |ui| {
                ui.set_min_size(egui::Vec2::new(290.0, 120.0));
                egui::Grid::new("bt_liste").striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Nr", "Bezeichnung", "Typ", "Intern"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for r in &rows {
                        for c in r {
                            ui.label(c.as_str());
                        }
                        ui.end_row();
                    }
                });
                if rows.is_empty() {
                    ui.weak("Bauteil aus dem Reiter „Bauteile“ hierher ziehen");
                }
            });
            if let Some(z) = gezogen {
                ablegen(&mut d, &self.project, *z);
            }
            // Nachbau: Auswahl ohne Ziehen und neues Bauteil
            ui.horizontal(|ui| {
                let mut i = d.bauteil.unwrap_or(usize::MAX);
                egui::ComboBox::from_id_salt("bt_liste_wahl").width(170.0).selected_text(namen.get(i).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                    for (k, n) in namen.iter().enumerate() {
                        ui.selectable_value(&mut i, k, n);
                    }
                });
                if i < namen.len() {
                    d.bauteil = Some(i);
                }
                if ui.button("Neues Bauteil …").clicked() {
                    neu = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Zeile löschen").clicked() {
                    d.bauteil = None;
                }
                ui.add_space(30.0);
                if ui.button("Abbruch").clicked() {
                    done = Some(false);
                }
                if ui.button("OK").clicked() {
                    done = Some(true);
                }
            });
        });
        if neu {
            self.part_assign_after = Some(d.obj);
            self.new_bauteil_dialog();
            return;
        }
        match done {
            None => self.zusatz.bauteilliste = Some(d),
            Some(false) => {}
            Some(true) => {
                if self.project.part_of.get(&d.obj).copied() != d.bauteil {
                    self.checkpoint();
                    match d.bauteil {
                        Some(i) => self.project.part_of.insert(d.obj, i),
                        None => self.project.part_of.remove(&d.obj),
                    };
                    self.dirty = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ablegen_und_zeilen() {
        let mut p = Project::new_default();
        p.bauteile.push(Bauteil { name: "Schütz".into(), typ: "DILM12".into(), nummer: "K1".into(), ..Default::default() });
        let mut d = BauteilListeDlg { obj: 5, bauteil: None };
        assert!(zeilen(&p, d.bauteil).is_empty());
        // unbekanntes Bauteil wird nicht angenommen
        assert!(!ablegen(&mut d, &p, BauteilZug(3)));
        assert!(ablegen(&mut d, &p, BauteilZug(0)));
        assert_eq!(zeilen(&p, d.bauteil), vec![["1".to_string(), "Schütz".into(), "DILM12".into(), "K1".into()]]);
    }
}
