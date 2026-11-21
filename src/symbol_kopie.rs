//! Symbol bearbeiten aus einem gesetzten Symbol heraus: Beim Übernehmen fragt das Programm, ob die Änderung für
//! alle Symbole dieses Typs gelten soll ("Alle": Definition im Projekt ändern) oder nur für das bearbeitete
//! ("Nur dieses": neue Projekt-Symboldefinition als Kopie mit eigenem Schlüssel und Namen, nur dieses Symbol wird
//! darauf umgestellt; beim Speichern als .epr entsteht dafür ein eigener SYM-Block, siehe epr_write::build).

use crate::app::App;
use crate::model::*;
use crate::symbols::SymDef;
use eframe::egui;

/// Zahl der gesetzten Symbole mit dem Schlüssel `key` im ganzen Projekt.
pub fn anzahl(project: &Project, key: &str) -> usize {
    project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(&o.kind, Kind::Symbol { key: k, .. } if k == key)).count()
}

/// Freier Name für die Kopie: "Name (2)", "Name (3)" … (weder im Projekt noch in der Bibliothek vergeben).
pub fn kopie_name(name: &str, vorhanden: &[&str]) -> String {
    (2..).map(|i| format!("{name} ({i})")).find(|n| !vorhanden.contains(&n.as_str())).unwrap()
}

/// Gesetztes Symbol `id` auf den Schlüssel `key` umstellen.
pub fn umstellen(project: &mut Project, id: u64, key: &str) -> bool {
    for o in project.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).filter(|o| o.id == id) {
        if let Kind::Symbol { key: k, .. } = &mut o.kind {
            *k = key.to_string();
            return true;
        }
    }
    false
}

impl App {
    /// Symboleditor für das gesetzte Symbol `id` öffnen (merkt sich das Symbol für die Frage beim Übernehmen).
    pub fn open_symbol_editor_fuer(&mut self, id: u64) {
        let key = match self.project.find_obj(id).map(|(_, o)| &o.kind) {
            Some(Kind::Symbol { key, .. }) => key.clone(),
            _ => return,
        };
        self.open_symbol_editor(Some(key));
        if let Some(e) = &mut self.sym_edit {
            e.instanz = Some(id);
        }
    }

    /// Knopf "Übernehmen": bei mehreren gesetzten Symbolen dieses Typs erst fragen, sonst direkt übernehmen.
    pub fn symbol_editor_uebernehmen(&mut self) {
        if let Some(e) = &mut self.sym_edit {
            if let Some(id) = e.instanz {
                let n = anzahl(&self.project, &e.key);
                if n >= 2 && self.project.find_obj(id).is_some() {
                    e.frage = Some(n);
                    return;
                }
            }
        }
        self.symbol_editor_commit();
    }

    /// "Nur dieses": Bearbeitung als neue Projekt-Symboldefinition übernehmen und nur das bearbeitete Symbol darauf
    /// umstellen (ein Rückgängig-Schritt).
    pub fn symbol_editor_nur_dieses(&mut self) {
        let neu = format!("proj:{}", self.project.alloc_id());
        let Some(e) = &mut self.sym_edit else { return };
        let Some(id) = e.instanz else { return };
        let alt = e.key.clone();
        // die Bibliothek braucht die Kopie schon vorher (Anschlussbezeichnungen werden über den Schlüssel gelesen)
        if let Some(mut d) = crate::symbols::find(&self.lib, &alt).cloned() {
            d.key = neu.clone();
            self.lib.push(d);
        }
        let vorhanden: Vec<&str> = self.lib.iter().chain(self.project.symdefs.iter()).map(|d: &SymDef| d.name.as_str()).collect();
        e.name = kopie_name(&e.name, &vorhanden);
        e.key = neu.clone();
        // Projektsymbol (nicht in die Benutzerbibliothek schreiben)
        e.haupt = 0;
        e.fav = false;
        e.group = vec!["Projekt".into()];
        self.symbol_editor_commit();
        umstellen(&mut self.project, id, &neu);
        self.info = format!("{} (nur dieses Symbol)", self.info);
    }

    /// Frage des Originals (0x47d564, Überschrift „Hallo Frage!“): „Diese Änderung wirkt sich auf alle Symbole des
    /// Typs:'…' im gesamten Projekt aus. Soll statt dessen ein neues Symbol angelegt werden?“ – Ja = nur dieses
    /// (neues Symbol), Nein = alle ändern, Abbrechen.
    pub fn symbol_frage_ui(&mut self, ctx: &egui::Context) {
        let Some(name) = self.sym_edit.as_ref().filter(|e| e.frage.is_some()).map(|e| e.name.clone()) else { return };
        let mut wahl = 0;
        egui::Modal::new(egui::Id::unique("symbol_frage")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.strong(crate::meldungen::T_FRAGE_SYMBOL);
            ui.label(format!("{}{name}{}", crate::meldungen::SE_AENDERUNG_1, crate::meldungen::SE_AENDERUNG_2));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Ja").clicked() {
                    wahl = 2;
                }
                if ui.button("Nein").clicked() {
                    wahl = 1;
                }
                if ui.button("Abbrechen").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    wahl = 3;
                }
            });
        });
        if wahl > 0 {
            if let Some(e) = &mut self.sym_edit {
                e.frage = None;
            }
        }
        match wahl {
            1 => self.symbol_editor_commit(),
            2 => self.symbol_editor_nur_dieses(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, key: &str) -> Obj {
        Obj { id, kind: Kind::Symbol { key: key.into(), pos: P::default(), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn zaehlen_umstellen_namen() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets.push(Sheet::new(2));
        p.anlagen[0].sheets[0].objects.push(sym(1, "a"));
        p.anlagen[0].sheets[1].objects.push(sym(2, "a"));
        p.anlagen[0].sheets[1].objects.push(sym(3, "b"));
        assert_eq!(anzahl(&p, "a"), 2);
        assert!(umstellen(&mut p, 2, "c"));
        assert_eq!((anzahl(&p, "a"), anzahl(&p, "c")), (1, 1));
        assert!(!umstellen(&mut p, 99, "c"));
        assert_eq!(kopie_name("Schließer", &["Schließer", "Schließer (2)"]), "Schließer (3)");
        assert_eq!(kopie_name("Öffner", &["Schließer (2)"]), "Öffner (2)");
    }
}
