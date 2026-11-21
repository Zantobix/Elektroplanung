//! Eigenschaften mehrerer markierter Objekte gemeinsam setzen: Stift, Fixierung und – wenn nur Texte markiert
//! sind – Textgröße und Schriftstil. Werte, die sich bei den Objekten unterscheiden, bleiben unverändert,
//! solange sie im Dialog nicht gesetzt werden („unverändert“).

use crate::app::App;
use crate::model::*;
use crate::props::{bold, tool_window};
use eframe::egui::{self, Color32, RichText, Vec2};

/// Zustand des Dialogs; `None` = unterschiedlich/unverändert.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiProps {
    pub ids: Vec<u64>,
    pub pen: Option<usize>,
    pub fixed: Option<bool>,
    /// alle markierten Objekte sind Texte bzw. Textboxen (Größe einstellbar)
    pub texts: bool,
    /// alle markierten Objekte sind Texte (Schriftstil einstellbar)
    pub styles: bool,
    /// Textgröße in mm als Eingabe; leer = unverändert
    pub size: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub frame: Option<bool>,
}

/// Gemeinsamer Wert aller Elemente oder `None`.
fn gemeinsam<T: PartialEq + Copy>(mut it: impl Iterator<Item = T>) -> Option<T> {
    let first = it.next()?;
    it.all(|v| v == first).then_some(first)
}

fn text_size(k: &Kind) -> Option<f32> {
    match k {
        Kind::Text { size, .. } => Some(*size),
        _ => None,
    }
}

fn text_style(k: &Kind) -> Option<&TextStyle> {
    match k {
        Kind::Text { style, .. } => Some(style),
        _ => None,
    }
}

fn fmt_mm(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

impl MultiProps {
    /// Dialogzustand aus den markierten Objekten.
    pub fn from_objs(objs: &[Obj]) -> Self {
        let texts = !objs.is_empty() && objs.iter().all(|o| text_size(&o.kind).is_some());
        let styles = !objs.is_empty() && objs.iter().all(|o| text_style(&o.kind).is_some());
        let st = |f: fn(&TextStyle) -> bool| if styles { gemeinsam(objs.iter().filter_map(|o| text_style(&o.kind)).map(f)) } else { None };
        MultiProps {
            ids: objs.iter().map(|o| o.id).collect(),
            pen: gemeinsam(objs.iter().map(|o| o.pen)),
            fixed: gemeinsam(objs.iter().map(|o| o.fixed)),
            texts,
            styles,
            size: if texts { gemeinsam(objs.iter().filter_map(|o| text_size(&o.kind))).map(fmt_mm).unwrap_or_default() } else { String::new() },
            bold: st(|s| s.bold),
            italic: st(|s| s.italic),
            underline: st(|s| s.underline),
            frame: st(|s| s.frame),
        }
    }

    /// Gesetzte Werte auf die Objekte `ids` übertragen. Liefert, ob sich etwas geändert hat.
    pub fn apply_to(&self, objs: &mut [Obj]) -> bool {
        let size = self.size.trim().replace(',', ".").parse::<f32>().ok().filter(|v| *v > 0.0);
        let mut changed = false;
        for o in objs.iter_mut().filter(|o| self.ids.contains(&o.id)) {
            let before = o.clone();
            if let Some(p) = self.pen {
                o.pen = p;
            }
            if let Some(f) = self.fixed {
                o.fixed = f;
            }
            if self.texts {
                if let (Some(v), Kind::Text { size, .. }) = (size, &mut o.kind) {
                    *size = v;
                }
            }
            if self.styles {
                if let Kind::Text { style, .. } = &mut o.kind {
                    for (val, dst) in [(self.bold, &mut style.bold), (self.italic, &mut style.italic), (self.underline, &mut style.underline), (self.frame, &mut style.frame)] {
                        if let Some(v) = val {
                            *dst = v;
                        }
                    }
                }
            }
            changed |= *o != before;
        }
        changed
    }
}

/// Auswahl mit „unverändert“ für einen Ja/Nein-Wert.
fn drei(ui: &mut egui::Ui, id: &str, label: &str, v: &mut Option<bool>) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(90.0, 20.0), egui::Sense::hover());
        ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, label, bold(13.0), Color32::BLACK);
        let name = |v: Option<bool>| match v {
            None => "(unverändert)",
            Some(true) => "Ja",
            Some(false) => "Nein",
        };
        egui::ComboBox::from_id_salt(id).width(150.0).selected_text(name(*v)).show_ui(ui, |ui| {
            for c in [None, Some(true), Some(false)] {
                ui.selectable_value(v, c, name(c));
            }
        });
    });
}

impl App {
    /// Befehl „Eigenschaften“ (Doppelklick, Kontextmenü): bei mehreren markierten Objekten der gemeinsame Dialog,
    /// sonst die Eigenschaften des Objekts `id`.
    pub fn props_command(&mut self, id: u64) {
        if self.selection.len() > 1 && self.selection.contains(&id) {
            self.open_multi_props();
        } else {
            self.edit_props(id);
        }
    }

    pub fn open_multi_props(&mut self) {
        let objs = self.selected_objs();
        if objs.len() > 1 {
            self.multi_props = Some(MultiProps::from_objs(&objs));
        }
    }

    /// Dialog „Eigenschaften (mehrere Objekte)“; Übernehmen ist ein Rückgängig-Schritt.
    pub fn multi_props_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.multi_props.take() else { return };
        let pens: Vec<String> = self.project.stifte.iter().map(|s| s.name.clone()).collect();
        let h = 112.0 + if d.texts { 30.0 } else { 0.0 } + if d.styles { 120.0 } else { 0.0 };
        let mut result: Option<bool> = None;
        let closed = tool_window(ctx, "multi_props", "Eigenschaften", [300.0, h], egui::Order::Middle, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 5.0);
            ui.label(RichText::new(format!("{} Objekte markiert", d.ids.len())).font(bold(13.0)));
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(90.0, 20.0), egui::Sense::hover());
                ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, "Stift", bold(13.0), Color32::BLACK);
                let cur = d.pen.and_then(|p| pens.get(p).cloned()).unwrap_or_else(|| "(unverändert)".into());
                egui::ComboBox::from_id_salt("mp_pen").width(150.0).selected_text(cur).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.pen, None, "(unverändert)");
                    for (i, n) in pens.iter().enumerate() {
                        ui.selectable_value(&mut d.pen, Some(i), n);
                    }
                });
            });
            drei(ui, "mp_fix", "Fixiert", &mut d.fixed);
            if d.texts {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(90.0, 20.0), egui::Sense::hover());
                    ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, "Textgröße", bold(13.0), Color32::BLACK);
                    ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut d.size).hint_text("unverändert"));
                    ui.label(RichText::new("mm").font(bold(13.0)));
                });
            }
            if d.styles {
                drei(ui, "mp_b", "Fett", &mut d.bold);
                drei(ui, "mp_i", "Kursiv", &mut d.italic);
                drei(ui, "mp_u", "Unterstrichen", &mut d.underline);
                drei(ui, "mp_r", "Rahmen", &mut d.frame);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                if ui.add_sized([90.0, 25.0], egui::Button::new("Abbruch")).clicked() {
                    result = Some(false);
                }
                ui.add_space(30.0);
                if ui.add_sized([90.0, 25.0], egui::Button::new("OK")).clicked() {
                    result = Some(true);
                }
            });
        });
        if closed {
            result = Some(false);
        }
        match result {
            None => self.multi_props = Some(d),
            Some(false) => {}
            Some(true) => self.apply_multi_props(&d),
        }
    }

    pub fn apply_multi_props(&mut self, d: &MultiProps) {
        let Some(mut objs) = self.sheet().map(|s| s.objects.clone()) else { return };
        if !d.apply_to(&mut objs) {
            return;
        }
        self.checkpoint();
        self.with_sheet(|s, _| s.objects = objs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(id: u64, size: f32, bold: bool, pen: usize) -> Obj {
        Obj { id, kind: Kind::Text { pos: P::new(0.0, 0.0), text: "x".into(), size, align: Align::Left, vertical: false, style: TextStyle { bold, ..Default::default() } }, pen, fixed: false, group: None, raw: None }
    }

    #[test]
    fn gemeinsame_werte_und_uebernahme() {
        let objs = vec![text(1, 3.5, true, 0), text(2, 3.5, false, 1), text(3, 5.0, false, 1)];
        let d = MultiProps::from_objs(&objs[..2]);
        assert!(d.texts && d.styles);
        assert_eq!((d.pen, d.fixed, d.size.as_str(), d.bold), (None, Some(false), "3,5", None));
        // Stift und Fett setzen, Größe ändern – Objekt 3 ist nicht markiert
        let mut objs2 = objs.clone();
        let d = MultiProps { pen: Some(2), bold: Some(true), size: "2,5".into(), ..d };
        assert!(d.apply_to(&mut objs2));
        for o in &objs2[..2] {
            assert_eq!(o.pen, 2);
            assert!(matches!(&o.kind, Kind::Text { size, style, .. } if *size == 2.5 && style.bold));
        }
        assert_eq!(objs2[2], objs[2]);
        // nichts gesetzt → keine Änderung
        let mut objs3 = objs.clone();
        assert!(!MultiProps::from_objs(&objs[..2]).apply_to(&mut objs3));
    }

    #[test]
    fn gemischte_markierung_ohne_textwerte() {
        let line = Obj { id: 9, kind: Kind::Line { a: P::new(0.0, 0.0), b: P::new(1.0, 1.0) }, pen: 0, fixed: true, group: None, raw: None };
        let objs = vec![text(1, 3.5, false, 0), line];
        let d = MultiProps::from_objs(&objs);
        assert!(!d.texts && !d.styles);
        assert_eq!((d.pen, d.fixed), (Some(0), None));
        let mut o2 = objs.clone();
        let d = MultiProps { fixed: Some(true), size: "9".into(), ..d };
        assert!(d.apply_to(&mut o2));
        assert!(o2.iter().all(|o| o.fixed));
        assert!(matches!(&o2[0].kind, Kind::Text { size, .. } if *size == 3.5));
    }
}
