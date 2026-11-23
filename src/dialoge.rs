//! Weitere Dialoge nach den Formularen des Originals: Projekteigenschaften (TPrjEigen), Projektinfo (TInfo),
//! Drucken (TForm3) und Potential (TPotential).

use crate::app::App;
use crate::model::*;
use crate::props::{bold, tool_window};
use eframe::egui::{self, Color32, RichText, Vec2};

pub enum Dlg2 {
    Projekt { name: String, status: usize, owner: String, kunde: String, schrift: String, alter_klemmenplan: bool },
    Info,
    Drucken { anlage: usize, drucker: usize, drucker_liste: Vec<String>, art: usize, von: String, bis: String, datum: String, skal: bool },
    Potential { idx: usize, name: String, spannung: String, strom: String },
}

const SCHRIFTEN: &[&str] = &["Arial", "Courier New", "Tahoma", "Times New Roman", "Verdana"];

fn lbl(ui: &mut egui::Ui, w: f32, t: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
    ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, t, bold(13.0), Color32::BLACK);
}

/// Drucker des Systems (Windows: Get-Printer); erster Eintrag = Standarddrucker.
fn drucker() -> Vec<String> {
    let mut v = vec!["Standarddrucker".to_string()];
    v.extend(crate::pdf::printers());
    v
}

impl App {
    pub fn open_projekt_dialog(&mut self) {
        let p = &self.project;
        self.dlg2 = Some(Dlg2::Projekt {
            name: p.name.clone(),
            status: p.abgeschlossen as usize,
            owner: p.owner.clone(),
            kunde: p.kunde.clone(),
            schrift: if p.schrift.is_empty() { "Times New Roman".into() } else { p.schrift.clone() },
            alter_klemmenplan: p.alter_klemmenplan,
        });
    }

    pub fn open_druck_dialog(&mut self) {
        // Anlagenliste wie 0x49c75c: „Alle“ vorn (Index 0), vorgewählt bei mehreren Anlagen, sonst die einzige;
        // „An Papiergröße anpassen“ ist im Formular nicht gesetzt
        let a = crate::dialog_regeln::druck_anlage_vorwahl(self.project.anlagen.len());
        self.dlg2 = Some(Dlg2::Drucken { anlage: a, drucker: 0, drucker_liste: drucker(), art: 0, von: String::new(), bis: String::new(), datum: today(), skal: false });
    }

    pub fn open_potential_dialog(&mut self, idx: usize) {
        let name = self.project.potentiale.get(idx).cloned().unwrap_or_default();
        let (spannung, strom) = self.project.pot_werte.get(&name).cloned().unwrap_or_default();
        // Anzeige wie TIn mit Kommastellen=3 (P_Spannung/P_Strom, FSetZahl 0x51edec): „230,000“, leer „0,000“
        let (spannung, strom) = (crate::dialog_regeln::zahl_text_leer0(&spannung, 3), crate::dialog_regeln::zahl_text_leer0(&strom, 3));
        self.dlg2 = Some(Dlg2::Potential { idx, name, spannung, strom });
    }

    pub fn dialoge2_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dlg2.take() else { return };
        let mut result: Option<bool> = None;
        let anlagen: Vec<String> = self.project.anlagen.iter().map(|a| a.name.clone()).collect();
        let (title, size) = match &d {
            Dlg2::Projekt { .. } => ("Projekteigenschaften", [351.0, 289.0]),
            Dlg2::Info => ("Projektinfo", [328.0, 187.0]),
            Dlg2::Drucken { .. } => ("Drucken", [406.0, 349.0]),
            Dlg2::Potential { .. } => ("Potential", [289.0, 120.0]),
        };
        let info = (self.project.erstellt.clone(), self.project.geaendert.clone(), self.project.gedruckt.clone(), self.project.anlagen.len(), self.project.total_sheets());
        let closed = tool_window(ctx, "dlg2", title, size, egui::Order::Middle, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 5.0);
            match &mut d {
                Dlg2::Projekt { name, status, owner, kunde, schrift, alter_klemmenplan } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Projektname");
                        ui.add_sized([240.0, 20.0], egui::TextEdit::singleline(name));
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Projektstatus");
                        let items = ["Projekt wird bearbeitet", "Projekt ist abgeschlossen"];
                        egui::ComboBox::from_id_salt("pst").width(234.0).selected_text(items[(*status).min(1)]).show_ui(ui, |ui| {
                            for (i, t) in items.iter().enumerate() {
                                ui.selectable_value(status, i, *t);
                            }
                        });
                    });
                    ui.horizontal_top(|ui| {
                        lbl(ui, 90.0, "Eigentümer");
                        ui.add_sized([240.0, 52.0], egui::TextEdit::multiline(owner));
                    });
                    ui.horizontal_top(|ui| {
                        lbl(ui, 90.0, "Kunde");
                        ui.add_sized([240.0, 52.0], egui::TextEdit::multiline(kunde));
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Standardschrift");
                        egui::ComboBox::from_id_salt("pschrift").width(160.0).selected_text(schrift.clone()).show_ui(ui, |ui| {
                            for f in SCHRIFTEN {
                                ui.selectable_value(schrift, f.to_string(), *f);
                            }
                        });
                    });
                    ui.checkbox(alter_klemmenplan, RichText::new("Alter Klemmenplan").font(bold(13.0)));
                }
                Dlg2::Info => {
                    let (e, g, dr, a, b) = &info;
                    egui::Grid::new("pinfo").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
                        for (l, v) in [("Erstellt:", e.clone()), ("Letzte Änderung", g.clone()), ("Gedruckt:", if dr.is_empty() { "??.??.????".into() } else { dr.clone() }), ("Anzahl Anlagen", a.to_string()), ("Anzahl Blätter", b.to_string())] {
                            ui.label(RichText::new(l).font(bold(13.0)));
                            ui.label(v);
                            ui.end_row();
                        }
                    });
                }
                Dlg2::Drucken { anlage, drucker, drucker_liste, art, von, bis, datum, skal } => {
                    // Aufbau wie TForm3: oben Drucker mit Bildknopf „Druckereinstellung“, darunter Anlage
                    ui.horizontal(|ui| {
                        lbl(ui, 92.0, "Drucker");
                        egui::ComboBox::from_id_salt("ddr").width(250.0).selected_text(drucker_liste.get(*drucker).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                            for (i, p) in drucker_liste.iter().enumerate() {
                                ui.selectable_value(drucker, i, p);
                            }
                        });
                        // SpeedButton1Click 0x4adf1c: Druckereinrichtung (TPrinterSetupDialog), danach Druckerwahl
                        if crate::tabelle::icon_knopf(ui, crate::icons::Icon::EditDrucker, "Druckereinstellung") {
                            let name = if *drucker > 0 { drucker_liste.get(*drucker).cloned() } else { None };
                            crate::pdf::printer_settings(name.as_deref());
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        lbl(ui, 92.0, "Anlage");
                        // AlleClick 0x4adc78: Anlage gesperrt bei „Aktuelles Blatt“ oder nur einer Anlage
                        let frei = crate::dialog_regeln::druck_anlage_frei(*art, anlagen.len());
                        ui.add_enabled_ui(frei, |ui| {
                            let liste: Vec<String> = std::iter::once("Alle".to_string()).chain(anlagen.iter().cloned()).collect();
                            egui::ComboBox::from_id_salt("danl").width(250.0).selected_text(liste.get(*anlage).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                                for (i, a) in liste.iter().enumerate() {
                                    ui.selectable_value(anlage, i, a);
                                }
                            });
                        });
                    });
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(390.0);
                        ui.label(RichText::new("Blatt").font(bold(13.0)));
                        ui.radio_value(art, 0, RichText::new("Alle Blätter").font(bold(13.0)));
                        ui.radio_value(art, 1, RichText::new("Aktuelles Blatt").font(bold(13.0)));
                        ui.radio_value(art, 3, RichText::new("Alle offenen Blätter").font(bold(13.0)));
                        ui.horizontal(|ui| {
                            ui.radio_value(art, 2, RichText::new("Blätter").font(bold(13.0)));
                            // VonChange 0x4adef8: Eingabe in Von/Bis wählt „Blätter“
                            ui.label("Von");
                            if ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(von)).changed() {
                                *art = 2;
                            }
                            ui.label("Bis");
                            if ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(bis)).changed() {
                                *art = 2;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.radio_value(art, 4, RichText::new("Änderung seit dem:").font(bold(13.0)));
                            // DatumChange 0x4adf68: Datum ändern wählt „Änderung seit dem“
                            if ui.add_sized([90.0, 20.0], egui::TextEdit::singleline(datum)).changed() {
                                *art = 4;
                            }
                        });
                        ui.radio_value(art, 5, RichText::new("Änderung seit dem letzten Ausdruck (Anleitung!)").font(bold(13.0)));
                    });
                    ui.checkbox(skal, RichText::new("An Papiergröße anpassen").font(bold(13.0)));
                }
                Dlg2::Potential { name, spannung, strom, .. } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Name");
                        ui.add_sized([170.0, 20.0], egui::TextEdit::singleline(name));
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Nennspannung");
                        ui.add_sized([90.0, 20.0], egui::TextEdit::singleline(spannung));
                        ui.label(RichText::new("V").font(bold(13.0)));
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Nennstrom");
                        ui.add_sized([90.0, 20.0], egui::TextEdit::singleline(strom));
                        ui.label(RichText::new("A").font(bold(13.0)));
                    });
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                let ok_label = if matches!(d, Dlg2::Drucken { .. }) { "Drucken" } else { "OK" };
                if !matches!(d, Dlg2::Info) && ui.add_sized([90.0, 25.0], egui::Button::new("Abbruch")).clicked() {
                    result = Some(false);
                }
                ui.add_space(30.0);
                if ui.add_sized([90.0, 25.0], egui::Button::new(ok_label)).clicked() {
                    result = Some(true);
                }
            });
        });
        if closed {
            result = Some(false);
        }
        match result {
            None => self.dlg2 = Some(d),
            Some(false) => self.neu_verwerfen(),
            Some(true) => self.apply_dlg2(d),
        }
    }

    fn apply_dlg2(&mut self, d: Dlg2) {
        match d {
            Dlg2::Projekt { name, status, owner, kunde, schrift, alter_klemmenplan } => {
                self.checkpoint();
                let p = &mut self.project;
                p.name = name;
                p.abgeschlossen = status == 1;
                p.owner = owner;
                p.kunde = kunde;
                p.schrift = if schrift == "Times New Roman" { String::new() } else { schrift };
                p.alter_klemmenplan = alter_klemmenplan;
                self.dirty = true;
            }
            Dlg2::Info => {}
            Dlg2::Potential { idx, name, spannung, strom } => {
                self.checkpoint();
                if let Some(p) = self.project.potentiale.get_mut(idx) {
                    let old = std::mem::replace(p, name.clone());
                    self.project.pot_werte.remove(&old);
                }
                // gespeichert wie bisher („230“, 0 = leer)
                let norm = |t: &str| crate::epr_projekt::tausendstel(t).map(crate::epr_projekt::als_text).unwrap_or_else(|| t.to_string());
                let (spannung, strom) = (norm(&spannung), norm(&strom));
                if !spannung.is_empty() || !strom.is_empty() {
                    self.project.pot_werte.insert(name, (spannung, strom));
                }
                self.dirty = true;
            }
            Dlg2::Drucken { anlage, drucker, drucker_liste, art, von, bis, datum, skal } => {
                // Blattauswahl wie im Original; "Änderung seit …" nach den Blattdaten (druck.rs)
                let gedruckt = self.project.gedruckt.clone();
                // Meldungen des Originals (Drucken1Click)
                if art == 5 && gedruckt.trim().is_empty() {
                    crate::meldungen::hinweis(crate::meldungen::T_HINWEIS, crate::meldungen::DRUCK_NIE_GEDRUCKT);
                    return;
                }
                // „Alle offenen Blätter“ ohne Blattreiter (Drucken1Click, Radioknopf AlleO)
                if art == 3 && self.tabs.is_empty() {
                    crate::meldungen::hinweis(crate::meldungen::T_HINWEIS, crate::meldungen::DRUCK_KEIN_BLATT);
                    return;
                }
                let mut sel: Vec<SheetRef> = vec![];
                // Anlage 0 = „Alle“; „Aktuelles Blatt“: Anlagenwahl gesperrt, es gilt die Anlage des aktuellen Blattes
                let gewaehlt: Vec<usize> = match (art, anlage) {
                    (1, _) => self.cur().map(|c| vec![c.anlage]).unwrap_or_default(),
                    (_, 0) => (0..self.project.anlagen.len()).collect(),
                    (_, a) => vec![a - 1],
                };
                for anlage in gewaehlt {
                    let Some(a) = self.project.anlagen.get(anlage) else { continue };
                    let (v, b): (u32, u32) = (von.trim().parse().unwrap_or(1), bis.trim().parse().unwrap_or(u32::MAX));
                    for (si, s) in a.sheets.iter().enumerate() {
                        let r = SheetRef { anlage, sheet: si };
                        let take = match art {
                            1 => self.cur() == Some(r),
                            2 => s.number >= v && s.number <= b,
                            3 => self.tabs.contains(&r),
                            4 => crate::druck::geaendert_seit(s, &datum),
                            5 => crate::druck::geaendert_seit(s, &gedruckt),
                            _ => true,
                        };
                        if take {
                            sel.push(r);
                        }
                    }
                }
                if sel.is_empty() {
                    crate::meldungen::hinweis(crate::meldungen::T_HINWEIS, crate::meldungen::DRUCK_NICHTS);
                    return;
                }
                self.print_sel = Some(sel);
                self.print_printer = (drucker > 0).then(|| drucker_liste[drucker].clone());
                self.print_skal = skal;
                // das Druckdatum setzt das Original erst nach dem Ausdruck (view.rs make_pdf)
                self.pdf_req = Some((std::env::temp_dir().join("Elektroplanung_Druck.pdf"), true));
            }
        }
    }
}
