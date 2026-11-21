//! Dialog „Eigentümer“ (Teilformular TSOwner im Dialog TObjekt_Eigenschaften): drei Auswahllisten Anlage, Blatt,
//! Symbol, mit denen der Eigentümer eines Symbols (VSY › OWR, siehe eigentuemer.rs) gewählt wird. Die Listen
//! hängen wie im Original voneinander ab (O_Anlage › SVerweis1VW_AnlageChange 0x4a468c, O_Blatt ›
//! SVerweis1VW_BlattChange 0x4a48f4); FormShow 0x4a27f8 setzt „Keine Zuordnung“ an den Anfang der Liste.
//! „Keine Zuordnung“ entfernt den Eigentümer.

use crate::model::*;
use eframe::egui::{self, Color32, Vec2};

pub const KEINE: &str = "Keine Zuordnung";

/// Zustand des Dialogs; Listen beim Öffnen aus dem Projekt gelesen.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnerProps {
    /// Symbole, deren Eigentümer gesetzt wird
    pub ids: Vec<u64>,
    pub anlagen: Vec<String>,
    /// je Anlage die Blätter
    pub blaetter: Vec<Vec<String>>,
    /// je Anlage und Blatt die Symbole mit Kennzeichnung (Id, Anzeige)
    pub symbole: Vec<Vec<Vec<(u64, String)>>>,
    pub anlage: usize,
    pub blatt: usize,
    /// 0 = „Keine Zuordnung“, sonst Index + 1 in `symbole[anlage][blatt]`
    pub symbol: usize,
}

impl OwnerProps {
    /// Listen aufbauen; vorgewählt wird der vorhandene Eigentümer, sonst das Blatt `hier`.
    pub fn neu(p: &Project, ids: Vec<u64>, hier: SheetRef) -> Self {
        let anlagen: Vec<String> = p.anlagen.iter().map(|a| a.name.clone()).collect();
        let blaetter: Vec<Vec<String>> = p.anlagen.iter().map(|a| a.sheets.iter().map(|s| format!("[{}] {}", s.number, s.name)).collect()).collect();
        let symbole: Vec<Vec<Vec<(u64, String)>>> = p
            .anlagen
            .iter()
            .map(|a| {
                a.sheets
                    .iter()
                    .map(|s| {
                        let ctx = crate::canvas::bmk_ctx(a, s);
                        s.objects
                            .iter()
                            .filter(|o| matches!(&o.kind, Kind::Symbol { bmk, .. } if !bmk.is_empty()) && !ids.contains(&o.id))
                            .map(|o| (o.id, crate::eigentuemer::anzeige_bmk(p, o, &ctx)))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let mut d = OwnerProps { ids, anlagen, blaetter, symbole, anlage: hier.anlage, blatt: hier.sheet, symbol: 0 };
        // vorhandener (direkter) Eigentümer des ersten Symbols
        let owner = d.ids.first().and_then(|id| p.find_obj(*id)).and_then(|(_, o)| match &o.kind {
            Kind::Symbol { ext, .. } if ext.eigentuemer != 0 => Some(ext.eigentuemer),
            _ => None,
        });
        if let Some((r, _)) = owner.and_then(|w| p.find_obj(w)) {
            d.anlage = r.anlage;
            d.blatt = r.sheet;
            d.symbol = d.liste().iter().position(|(i, _)| Some(*i) == owner).map(|i| i + 1).unwrap_or(0);
        }
        d
    }

    /// Symbole des gewählten Blattes.
    pub fn liste(&self) -> &[(u64, String)] {
        self.symbole.get(self.anlage).and_then(|a| a.get(self.blatt)).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Gewählter Eigentümer (None = „Keine Zuordnung“).
    pub fn gewaehlt(&self) -> Option<u64> {
        self.symbol.checked_sub(1).and_then(|i| self.liste().get(i)).map(|(id, _)| *id)
    }
}

/// Eigentümer setzen oder (bei „Keine Zuordnung“) entfernen; Rückgabe: ob sich etwas geändert hat.
pub fn anwenden(p: &mut Project, d: &OwnerProps) -> bool {
    match d.gewaehlt() {
        Some(w) => crate::eigentuemer::setzen(p, w, &d.ids),
        None => {
            let mut changed = false;
            for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects) {
                if let (true, Kind::Symbol { ext, .. }) = (d.ids.contains(&o.id), &mut o.kind) {
                    if ext.eigentuemer != 0 {
                        ext.eigentuemer = 0;
                        changed = true;
                    }
                }
            }
            changed
        }
    }
}

fn beschriftung(ui: &mut egui::Ui, t: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(92.0, 20.0), egui::Sense::hover());
    ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, t, crate::props::bold(13.0), Color32::BLACK);
}

fn auswahl(ui: &mut egui::Ui, id: &str, items: &[String], sel: &mut usize) -> bool {
    let mut neu = *sel;
    egui::ComboBox::from_id_salt(id).width(181.0).selected_text(items.get(*sel).cloned().unwrap_or_default()).show_ui(ui, |ui| {
        for (i, t) in items.iter().enumerate() {
            ui.selectable_value(&mut neu, i, t);
        }
    });
    let changed = neu != *sel;
    *sel = neu;
    changed
}

/// Inhalt: Anlage, Blatt, Symbol (wie TSOwner, Beschriftungen rechtsbündig).
pub fn ui(ui: &mut egui::Ui, d: &mut OwnerProps) {
    ui.horizontal(|ui| {
        beschriftung(ui, "Anlage");
        if auswahl(ui, "own_anl", &d.anlagen.clone(), &mut d.anlage) {
            d.blatt = 0;
            d.symbol = 0;
        }
    });
    ui.horizontal(|ui| {
        beschriftung(ui, "Blatt");
        let b = d.blaetter.get(d.anlage).cloned().unwrap_or_default();
        if auswahl(ui, "own_blt", &b, &mut d.blatt) {
            d.symbol = 0;
        }
    });
    ui.horizontal(|ui| {
        beschriftung(ui, "Symbol");
        let s: Vec<String> = std::iter::once(KEINE.to_string()).chain(d.liste().iter().map(|(_, t)| t.clone())).collect();
        auswahl(ui, "own_sym", &s, &mut d.symbol);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, bmk: &str) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "k".into(), pos: P::new(10.0 * id as f32, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn setzen_und_entfernen() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1"));
        p.anlagen[0].sheets[0].objects.push(sym(2, "-Q1"));
        let hier = SheetRef { anlage: 0, sheet: 0 };
        let mut d = OwnerProps::neu(&p, vec![2], hier);
        // das Symbol selbst steht nicht in der Liste
        assert_eq!(d.liste().iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![1]);
        assert_eq!(d.symbol, 0);
        d.symbol = 1;
        assert!(anwenden(&mut p, &d));
        let eig = |p: &Project| match &p.find_obj(2).unwrap().1.kind {
            Kind::Symbol { ext, .. } => ext.eigentuemer,
            _ => 0,
        };
        assert_eq!(eig(&p), 1);
        // erneut öffnen: Eigentümer vorgewählt
        let d2 = OwnerProps::neu(&p, vec![2], hier);
        assert_eq!(d2.gewaehlt(), Some(1));
        // „Keine Zuordnung“ entfernt ihn
        let mut d3 = d2.clone();
        d3.symbol = 0;
        assert!(anwenden(&mut p, &d3));
        assert_eq!(eig(&p), 0);
    }
}
