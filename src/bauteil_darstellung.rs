//! Bauteildialog (TSBauteil): Herstellerliste (BFirma) und Gruppe „Darstellung“ (BSymbol, BIndex).
//! Der Nachbau zeigt als Darstellung das Symbol und das Bild der Instanzen, denen das Bauteil zugeordnet ist
//! (Bauteilliste › Bearbeiten); eine eigene Symbolangabe am Bauteil (BTL) liest er noch nicht.

use crate::model::*;
use crate::symbols::{self, SymDef};
use eframe::egui;

/// Hersteller aller Bauteile des Projekts (sortiert, ohne leere und doppelte).
pub fn firmen(p: &Project) -> Vec<String> {
    let mut v: Vec<String> = p.bauteile.iter().map(|b| b.firma.trim().to_string()).filter(|f| !f.is_empty()).collect();
    v.sort();
    v.dedup();
    v
}

/// (Symbol, Bild) der Symbole, denen Bauteil `idx` zugeordnet ist; „Keins“/„Standard“, wenn keins.
pub fn darstellung(p: &Project, lib: &[SymDef], idx: usize) -> (String, String) {
    let ids: Vec<u64> = p.part_of.iter().filter(|(_, v)| **v == idx).map(|(k, _)| *k).collect();
    let mut syms: Vec<(String, String)> = vec![];
    for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| ids.contains(&o.id)) {
        if let Kind::Symbol { key, option, .. } = &o.kind {
            let def = symbols::find(lib, key);
            let name = def.map(|d| d.name.clone()).unwrap_or_else(|| key.clone());
            let bild = def.and_then(|d| d.options.get(*option as usize).cloned()).unwrap_or_else(|| "Standard".into());
            if !syms.contains(&(name.clone(), bild.clone())) {
                syms.push((name, bild));
            }
        }
    }
    match syms.len() {
        0 => ("Keins".into(), "Standard".into()),
        1 => syms.remove(0),
        n => (format!("{} (+{} weitere)", syms[0].0, n - 1), syms[0].1.clone()),
    }
}

/// Eingabefeld Hersteller mit Auswahl der bekannten Hersteller.
pub fn firma_ui(ui: &mut egui::Ui, v: &mut String, firmen: &[String]) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(v).desired_width(150.0));
        ui.add_enabled_ui(!firmen.is_empty(), |ui| {
            ui.menu_button("▾", |ui| {
                for f in firmen {
                    if ui.button(f.as_str()).clicked() {
                        *v = f.clone();
                        ui.close();
                    }
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmen_und_darstellung() {
        let mut p = Project::new_default();
        p.bauteile.push(Bauteil { firma: "Hager".into(), ..Default::default() });
        p.bauteile.push(Bauteil { firma: " ABB".into(), ..Default::default() });
        p.bauteile.push(Bauteil { firma: "Hager".into(), ..Default::default() });
        assert_eq!(firmen(&p), ["ABB", "Hager"]);
        assert_eq!(darstellung(&p, &[], 0), ("Keins".to_string(), "Standard".to_string()));
        let mut d = SymDef::empty("k", "Lampe", "-E%C");
        d.options = vec!["Standard".into(), "Hängend".into()];
        let ext = SymExt::default();
        p.anlagen[0].sheets[0].objects.push(Obj { id: 5, pen: 0, fixed: false, group: None, raw: None, kind: Kind::Symbol { key: "k".into(), pos: P::default(), rot: 0, mirror_h: false, mirror_v: false, bmk: "-E1".into(), value: String::new(), scale: 1.0, option: 1, title: None, flags: 0, ext } });
        p.part_of.insert(5, 1);
        assert_eq!(darstellung(&p, &[d], 1), ("Lampe".to_string(), "Hängend".to_string()));
    }
}
