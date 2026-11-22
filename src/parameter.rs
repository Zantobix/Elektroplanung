//! Symbolparameter und benutzerdefinierte Symboltexte wie im Original.
//!
//! Blattmenü › „Parameter“ und › „Alternativtext › Benutzerdefiniert“ rufen im Original denselben Handler
//! (Alternativtext1Click): Er füllt die Tabelle TTextAbfrage mit den Parametern (GRD › PRA) bzw. den Textfeldern
//! des Symbols und schreibt die Werte nach OK in die Instanz (VSY › SPA bzw. VSY › ALT). Hat das Symbol keine
//! Parameter, meldet das Original „Keine Parameter vorhanden!“.
//!
//! Ablage im .epr (geprüft an W2.epr):
//! - GRD › PRA: Feld 0 = Parameternummer, STR 1 = Name ("Typ"), STR 2 = Vorgabe ("B16"), ohne Nullbyte.
//! - VSY › SPA: Feld 0 = Parameternummer, STR = Wert ("B10"); steht nach TIT/ALT und vor FKT/OWR.
//! - VSY › ALT: Feld 0 = Textnummer, Feld 1 = 1, STR = Text ("PE"); VSY Feld 21 bleibt 0.

use crate::app::{App, Dialog};
use crate::eprfile::{encode_cp1252, Block};
use crate::model::*;
use crate::symbols::{self, Prim, SymDef};
use eframe::egui::{self, Color32, RichText, Vec2};

/// Offene Tabelle (TTextAbfrage) für ein Symbol: Modus 1 = benutzerdefinierte Texte, 2 = Parameter.
/// Zeilen: (Nummer, Name bzw. Standardtext, Wert).
#[derive(Clone, Debug)]
pub struct ParamDlg {
    pub r: SheetRef,
    pub id: u64,
    pub mode: u8,
    pub rows: Vec<(u32, String, String)>,
}

/// Zeilen der Parametertabelle: je Parameter der Definition der Wert der Instanz, sonst die Vorgabe.
pub fn param_rows(def: &SymDef, inst: &[(u32, String)]) -> Vec<(u32, String, String)> {
    def.params
        .iter()
        .map(|(nr, name, dflt)| (*nr, name.clone(), inst.iter().find(|(n, _)| n == nr).map(|(_, v)| v.clone()).unwrap_or_else(|| dflt.clone())))
        .collect()
}

/// Textfelder des Symbols (Textnummer > 0) mit Standardtext und benutzerdefiniertem Text der Instanz.
pub fn text_rows(def: &SymDef, custom: &[(u32, String)]) -> Vec<(u32, String, String)> {
    let mut out: Vec<(u32, String, String)> = vec![];
    for (i, p) in def.prims.iter().enumerate() {
        let nr = def.text_idx.get(i).copied().unwrap_or(0);
        let std = match p {
            Prim::Text(_, s, _) | Prim::TextBox(_, _, s, _) => s.clone(),
            _ => continue,
        };
        // Text 1 mit Vorlage (%…) ist die BMK und wird nicht ersetzt
        if nr == 0 || (nr == 1 && std.contains('%')) || out.iter().any(|(n, _, _)| *n == nr) {
            continue;
        }
        let cur = custom.iter().find(|(n, _)| *n == nr).map(|(_, t)| t.clone()).unwrap_or_else(|| std.clone());
        out.push((nr, std, cur));
    }
    out.sort_by_key(|r| r.0);
    out
}

/// Ergebnis der Texttabelle: nur abweichende Texte werden als benutzerdefiniert gespeichert.
pub fn texts_from_rows(rows: &[(u32, String, String)]) -> Vec<(u32, String)> {
    rows.iter().filter(|(_, std, v)| v != std).map(|(n, _, v)| (*n, v.clone())).collect()
}

/// VSY-Unterblöcke SPA (Parameter) und ALT (benutzerdefinierte Texte) aus der Instanz neu aufbauen.
/// Lage wie im Original: nach TIT/WRT/ALT, vor FKT/OWR.
pub fn sync_vsy_kids(b: &mut Block, params: &[(u32, String)], texts: &[(u32, String)]) {
    b.kids.retain(|k| !k.is("SPA") && !k.is("ALT"));
    let at = b.kids.iter().position(|k| !k.is("TIT") && !k.is("WRT")).unwrap_or(b.kids.len());
    let mut add: Vec<Block> = vec![];
    for (nr, t) in texts {
        let mut d = (*nr as i32).to_le_bytes().to_vec();
        d.extend(1i32.to_le_bytes());
        let mut a = Block::new("ALT", d);
        a.kids.push(Block::new("STR", encode_cp1252(t)));
        add.push(a);
    }
    for (nr, v) in params {
        let mut s = Block::new("SPA", (*nr as i32).to_le_bytes().to_vec());
        s.kids.push(Block::new("STR", encode_cp1252(v)));
        add.push(s);
    }
    b.kids.splice(at..at, add);
}

/// GRD › PRA-Blöcke einer Symboldefinition.
pub fn pra_blocks(params: &[(u32, String, String)]) -> Vec<Block> {
    params
        .iter()
        .map(|(nr, name, dflt)| {
            let mut b = Block::new("PRA", (*nr as i32).to_le_bytes().to_vec());
            b.kids.push(Block::new("STR", encode_cp1252(name)));
            b.kids.push(Block::new("STR", encode_cp1252(dflt)));
            b
        })
        .collect()
}

/// Parameterliste aus dem Symboleditor übernehmen: Zeilen ohne Namen entfallen, neue Zeilen (Nummer 0)
/// bekommen die nächste freie Nummer.
pub fn clean_params(rows: &[(u32, String, String)]) -> Vec<(u32, String, String)> {
    let mut next = rows.iter().map(|r| r.0).max().unwrap_or(0) + 1;
    rows.iter()
        .filter(|(_, n, _)| !n.trim().is_empty())
        .map(|(nr, n, v)| {
            let nr = if *nr == 0 {
                next += 1;
                next - 1
            } else {
                *nr
            };
            (nr, n.trim().to_string(), v.clone())
        })
        .collect()
}

/// Tabelle „Parameter“ im Symboleditor (TSymEigenschaften, Reiter „Stifte / Parameter“): Name und Vorgabe,
/// eine leere Zeile am Ende legt einen neuen Parameter an, Knopf „Parameter Löschen“.
pub fn param_table_ui(ui: &mut egui::Ui, rows: &mut Vec<(u32, String, String)>, sel: &mut usize) {
    if rows.last().map(|r| !r.1.is_empty() || !r.2.is_empty()).unwrap_or(true) {
        rows.push((0, String::new(), String::new()));
    }
    let w = 323.0;
    ui.vertical(|ui| {
        ui.set_width(w);
        egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0, Color32::from_gray(160))).show(ui, |ui| {
            ui.set_min_size(Vec2::new(w - 4.0, 270.0));
            egui::ScrollArea::vertical().id_salt("symparam").max_height(270.0).show(ui, |ui| {
                egui::Grid::new("symparamgrid").num_columns(3).show(ui, |ui| {
                    // Überschriften wie TSymEigenschaften (0x4795dc): blaue Titelzeile, darunter die Spalten
                    ui.label("");
                    ui.label(RichText::new(crate::meldungen::SE_PARAMETER).font(crate::props::bold(13.0)).color(Color32::BLUE));
                    ui.end_row();
                    ui.label("");
                    ui.label(RichText::new(crate::meldungen::SE_PARAMETERNAME).font(crate::props::bold(13.0)));
                    ui.label(RichText::new(crate::meldungen::SE_DEFAULTWERT).font(crate::props::bold(13.0)));
                    ui.end_row();
                    for (i, (_, n, v)) in rows.iter_mut().enumerate() {
                        ui.label(format!("{}", i + 1));
                        let a = ui.add(egui::TextEdit::singleline(n).desired_width(140.0));
                        let b = ui.add(egui::TextEdit::singleline(v).desired_width(120.0));
                        if a.gained_focus() || b.gained_focus() {
                            *sel = i;
                        }
                        ui.end_row();
                    }
                });
            });
        });
        let real = rows.len().saturating_sub(1);
        ui.horizontal(|ui| {
            ui.add_space(100.0);
            if ui.add_enabled(*sel < real, egui::Button::new("Parameter Löschen").min_size(Vec2::new(113.0, 25.0))).clicked() {
                rows.remove(*sel);
                *sel = sel.saturating_sub(1);
            }
        });
    });
}

impl App {
    /// Blattmenü › Parameter (mode 2) bzw. › Alternativtext › Benutzerdefiniert (mode 1).
    pub fn open_param_dialog(&mut self, id: u64, mode: u8) {
        let Some(r) = self.cur() else { return };
        let Some(o) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).cloned() else { return };
        let Kind::Symbol { key, ext, .. } = &o.kind else { return };
        let Some(def) = symbols::find(&self.lib, key) else { return };
        let rows = if mode == 2 { param_rows(def, &ext.params) } else { text_rows(def, &ext.texts) };
        if rows.is_empty() {
            if mode == 2 {
                self.dialog = Some(Dialog::Message("Keine Parameter vorhanden!".into()));
            } else {
                // Original (Alternativtext1Click)
                crate::meldungen::hinweis(crate::meldungen::T_ACHTUNG, crate::meldungen::KEINE_ALTERNATIVTEXTE);
            }
            return;
        }
        self.zusatz.param = Some(ParamDlg { r, id, mode, rows });
    }

    fn apply_param_dialog(&mut self, d: ParamDlg) {
        if self.cur() != Some(d.r) {
            return;
        }
        self.checkpoint();
        self.with_sheet(|s, _| {
            if let Some(Kind::Symbol { ext, .. }) = s.objects.iter_mut().find(|o| o.id == d.id).map(|o| &mut o.kind) {
                if d.mode == 2 {
                    // wie das Original: alle Zeilen der Tabelle werden zu Werten der Instanz
                    ext.params = d.rows.iter().map(|(n, _, v)| (*n, v.clone())).collect();
                } else {
                    ext.texts = texts_from_rows(&d.rows);
                    ext.alt = 0;
                }
            }
        });
        self.dirty = true;
    }

    pub fn param_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.zusatz.param.take() else { return };
        let mut done: Option<bool> = None;
        let title = if d.mode == 2 { "Parameter" } else { "Benutzerdefinierter Text" };
        let closed = crate::props::tool_window(ctx, "paramdlg", title, [376.0, 266.0], egui::Order::Foreground, |ui| {
            egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0, Color32::from_gray(160))).show(ui, |ui| {
                ui.set_min_size(Vec2::new(360.0, 200.0));
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    egui::Grid::new("paramgrid").striped(true).num_columns(2).show(ui, |ui| {
                        // Kopfzeile wie 0x479064: Parameter/Wert bzw. original Text/alternativ Text
                        let head = if d.mode == 2 { (crate::meldungen::TA_PARAMETER, crate::meldungen::TA_WERT) } else { (crate::meldungen::TA_ORIGINAL, crate::meldungen::TA_ALTERNATIV) };
                        ui.label(RichText::new(head.0).font(crate::props::bold(13.0)));
                        ui.label(RichText::new(head.1).font(crate::props::bold(13.0)));
                        ui.end_row();
                        for (_, name, v) in d.rows.iter_mut() {
                            ui.label(name.as_str());
                            ui.add(egui::TextEdit::singleline(v).desired_width(200.0));
                            ui.end_row();
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(58.0);
                if ui.add(egui::Button::new("OK").min_size(Vec2::new(75.0, 25.0))).clicked() {
                    done = Some(true);
                }
                ui.add_space(96.0);
                if ui.add(egui::Button::new("Abbruch").min_size(Vec2::new(75.0, 25.0))).clicked() {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => self.apply_param_dialog(d),
            Some(false) => {}
            None if closed => {}
            None => self.zusatz.param = Some(d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> SymDef {
        let mut d = SymDef::empty("t", "RCD", "-F%C");
        d.params = vec![(1, "Typ".into(), "A".into()), (2, "IdN".into(), "30mA".into())];
        d.prims = vec![Prim::Text(P::new(0.0, 0.0), "-F%C".into(), 2.5), Prim::Line(P::new(0.0, 0.0), P::new(1.0, 1.0), 0), Prim::Text(P::new(0.0, 0.0), "N".into(), 2.5)];
        d.text_idx = vec![1, 0, 2];
        d
    }

    #[test]
    fn parameter_zeilen_mit_vorgabe_und_wert() {
        let rows = param_rows(&def(), &[(2, "300mA".into())]);
        assert_eq!(rows, vec![(1, "Typ".into(), "A".into()), (2, "IdN".into(), "300mA".into())]);
    }

    #[test]
    fn texte_ohne_bmk_vorlage() {
        let rows = text_rows(&def(), &[(2, "PE".into())]);
        assert_eq!(rows, vec![(2, "N".into(), "PE".into())]);
        assert_eq!(texts_from_rows(&rows), vec![(2, "PE".to_string())]);
        assert!(texts_from_rows(&[(2, "N".into(), "N".into())]).is_empty());
    }

    #[test]
    fn neue_parameter_bekommen_naechste_nummer() {
        let rows = vec![(2, "Typ".into(), "B16".into()), (0, " In ".into(), "16A".into()), (0, "".into(), "".into()), (0, "x".into(), "".into())];
        assert_eq!(clean_params(&rows), vec![(2, "Typ".into(), "B16".into()), (3, "In".into(), "16A".into()), (4, "x".into(), "".into())]);
    }

    #[test]
    fn spa_und_alt_in_original_reihenfolge() {
        let mut b = Block::new("VSY", vec![0; 160]);
        b.kids = vec![Block::new("TIT", b"-F%C\0".to_vec()), Block::new("SPA", 1i32.to_le_bytes().to_vec()), Block::new("FKT", b"x\0".to_vec())];
        sync_vsy_kids(&mut b, &[(1, "B10".into())], &[(1, "PE".into())]);
        let tags: Vec<&str> = b.kids.iter().map(|k| k.tag_str()).collect();
        assert_eq!(tags, ["TIT", "ALT", "SPA", "FKT"]);
        assert_eq!(b.kids[1].data, [1, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(b.kids[2].kids[0].data, b"B10");
        let pra = pra_blocks(&[(1, "Typ".into(), "B16".into())]);
        assert_eq!(pra[0].data, [1, 0, 0, 0]);
        assert_eq!(pra[0].kids[0].data, b"Typ");
        assert_eq!(pra[0].kids[1].data, b"B16");
    }
}
