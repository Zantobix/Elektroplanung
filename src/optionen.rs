//! Dialog "Option" wie im Original (Formular TForm4: Reiter Programm, Projektvorgaben1, Projektvorgaben2,
//! Datenbank). Die Einstellungen werden dauerhaft gespeichert; Starteinstellungen gelten beim Programmstart,
//! Projektvorgaben für neue Projekte und Anlagen.

use crate::app::App;
use crate::model::*;
use crate::props::{bold, tool_window};
use eframe::egui::{self, Color32, RichText, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Optionen {
    // Programm › Starteinstellung
    pub raster_idx: usize,
    pub rand_ursprung: bool,
    pub gitter: bool,
    pub pfadfang: bool,
    pub autoverbindung: bool,
    pub rahmen_groesse: bool,
    pub fixierung: bool,
    // Programm › Programmverhalten
    pub ungueltige_loeschen: bool,
    /// Doppelklick auf Verweise: 0 = Ziel anzeigen, 1 = Eigenschaften
    pub doppelklick_verweis: usize,
    pub backup: bool,
    /// vor Drucken/PDF-Export das Projekt prüfen und bei Hinweisen nachfragen
    pub pruefen_vor_druck: bool,
    // Programm › Symboleditor
    pub sym_breite: f32,
    pub sym_hoehe: f32,
    // Programm › Farben, Maus
    pub blatt_farbe: [u8; 3],
    pub mausrad_umkehren: bool,
    // Projektvorgaben1
    pub format: (i32, i32),
    pub hochformat: bool,
    pub raender: [i32; 4],
    pub pfad_hoehe: i32,
    pub pfade: i32,
    pub ab_null: bool,
    /// Schriftfeld: 0 = Standard, 1 = aus Datei
    pub schriftfeld: usize,
    pub schriftfeld_datei: String,
    pub schrift: String,
    pub size_titel: i32,
    pub size_wert: i32,
    pub size_verbg: i32,
    pub size_pfad: i32,
    pub sms_abstand: f32,
    // Projektvorgaben2: Vorgabe Titelposition (x, y in mm)
    pub titel_sms: (f32, f32),
    pub titel_klemme: (f32, f32),
    pub titel_verweis: (f32, f32),
    // Datenbank (MiniSQL-Server des Originals)
    pub server: String,
    pub port: String,
    pub user: String,
    pub passwort: String,
    /// eigene Tastenkürzel (Befehl → Kombination), sonst Vorgaben aus kuerzel.rs
    pub tasten: std::collections::BTreeMap<String, String>,
}

impl Default for Optionen {
    fn default() -> Self {
        let a = AnlSettings::default();
        Optionen {
            raster_idx: 6,
            rand_ursprung: true,
            gitter: true,
            pfadfang: true,
            autoverbindung: true,
            rahmen_groesse: true,
            fixierung: true,
            ungueltige_loeschen: true,
            doppelklick_verweis: 0,
            backup: true,
            pruefen_vor_druck: true,
            sym_breite: 100.0,
            sym_hoehe: 100.0,
            blatt_farbe: [238, 221, 221],
            mausrad_umkehren: false,
            format: a.format,
            hochformat: false,
            raender: a.raender,
            pfad_hoehe: a.pfad_hoehe,
            pfade: a.pfade,
            ab_null: a.ab_null,
            schriftfeld: 0,
            schriftfeld_datei: String::new(),
            schrift: "Times New Roman".into(),
            size_titel: a.size_titel,
            size_wert: a.size_wert,
            size_verbg: a.size_verbg,
            size_pfad: a.size_pfad,
            sms_abstand: 5.0,
            titel_sms: (0.0, -2.0),
            titel_klemme: (1.5, -1.2),
            titel_verweis: (7.0, -1.0),
            server: "localhost".into(),
            port: String::new(),
            user: "root".into(),
            passwort: String::new(),
            tasten: Default::default(),
        }
    }
}

impl Optionen {
    /// Anlagen-Vorgaben für neue Anlagen.
    pub fn anlage(&self) -> AnlSettings {
        AnlSettings {
            format: self.format,
            raender: self.raender,
            pfade: self.pfade,
            pfad_hoehe: self.pfad_hoehe,
            ab_null: self.ab_null,
            size_titel: self.size_titel,
            size_wert: self.size_wert,
            size_verbg: self.size_verbg,
            size_pfad: self.size_pfad,
            ..AnlSettings::default()
        }
    }
}

/// Zustand des offenen Dialogs (Kopie der Optionen + Reiter + Texteingaben).
pub struct OptDlg {
    pub o: Optionen,
    pub tab: usize,
    pub t: Vec<String>,
    /// Befehl, dessen neue Tastenkombination gerade aufgenommen wird
    pub capture: Option<&'static str>,
}

const FORMATE: &[(&str, (i32, i32))] = &[("A4", (2100, 2970)), ("A3", (2970, 4200)), ("A2", (4200, 5940)), ("A1", (5940, 8410)), ("Benutzerdefiniert", (0, 0))];
const ZIEL: &[&str] = &["Ziel anzeigen", "Eigenschaften"];

fn f1(v: f32) -> String {
    format!("{v:.1}").replace('.', ",")
}
fn n1(s: &str, old: f32) -> f32 {
    s.trim().replace(',', ".").parse().unwrap_or(old)
}
fn i01(s: &str, old: i32) -> i32 {
    (n1(s, old as f32 / 10.0) * 10.0).round() as i32
}

fn mark(ui: &mut egui::Ui, v: &mut bool, t: &str) {
    ui.checkbox(v, RichText::new(t).font(bold(13.0)));
}
fn field(ui: &mut egui::Ui, l: &str, w: f32, v: &mut String, ew: f32) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
        ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, l, bold(13.0), Color32::BLACK);
        ui.add_sized([ew, 20.0], egui::TextEdit::singleline(v));
    });
}

impl App {
    pub fn open_optionen(&mut self) {
        let mut o = self.opt.clone();
        // aktuelle Programmzustände übernehmen
        o.blatt_farbe = [self.sheet_color.r(), self.sheet_color.g(), self.sheet_color.b()];
        o.mausrad_umkehren = self.invert_wheel;
        let t = vec![
            f1(o.sym_breite),
            f1(o.sym_hoehe),
            f1(o.format.0 as f32 / 10.0),
            f1(o.format.1 as f32 / 10.0),
            f1(o.raender[0] as f32 / 10.0),
            f1(o.raender[1] as f32 / 10.0),
            f1(o.raender[2] as f32 / 10.0),
            f1(o.raender[3] as f32 / 10.0),
            f1(o.pfad_hoehe as f32 / 10.0),
            o.pfade.to_string(),
            o.size_titel.to_string(),
            o.size_wert.to_string(),
            o.size_verbg.to_string(),
            o.size_pfad.to_string(),
            f1(o.sms_abstand),
            f1(o.titel_sms.0),
            f1(o.titel_sms.1),
            f1(o.titel_klemme.0),
            f1(o.titel_klemme.1),
            f1(o.titel_verweis.0),
            f1(o.titel_verweis.1),
        ];
        self.opt_dlg = Some(OptDlg { o, tab: 0, t, capture: None });
    }

    pub fn optionen_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.opt_dlg.take() else { return };
        let mut result: Option<bool> = None;
        let mut pick_lib = false;
        let lib_info = self.lib_info.clone();
        let rasters: Vec<String> = crate::app::RASTER.iter().map(|r| r.0.to_string()).collect();
        let closed = tool_window(ctx, "optionen", "Option", [611.0, 395.0], egui::Order::Middle, |ui| {
            ui.horizontal(|ui| {
                for (i, t) in ["Programm", "Projektvorgaben1", "Projektvorgaben2", "Datenbank", "Tastenkürzel"].iter().enumerate() {
                    if ui.selectable_label(d.tab == i, *t).clicked() {
                        d.tab = i;
                    }
                }
            });
            ui.separator();
            let capture = &mut d.capture;
            let o = &mut d.o;
            let t = &mut d.t;
            egui::Frame::new().show(ui, |ui| {
                ui.set_min_height(318.0);
                match d.tab {
                    0 => {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(255.0);
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(245.0);
                                    ui.label("Starteinstellung");
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Rasterfang").font(bold(13.0)));
                                        egui::ComboBox::from_id_salt("optraster").width(90.0).selected_text(rasters.get(o.raster_idx).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                                            for (i, r) in rasters.iter().enumerate() {
                                                ui.selectable_value(&mut o.raster_idx, i, r);
                                            }
                                        });
                                    });
                                    mark(ui, &mut o.rand_ursprung, "Rand ist Ursprung");
                                    mark(ui, &mut o.gitter, "Gitter anzeigen");
                                    mark(ui, &mut o.pfadfang, "Pfadfang");
                                    mark(ui, &mut o.autoverbindung, "Autoverbindung");
                                    mark(ui, &mut o.rahmen_groesse, "Rahmen für Größe");
                                    mark(ui, &mut o.fixierung, "Fixierung aktivieren");
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(245.0);
                                    ui.label("Maus");
                                    mark(ui, &mut o.mausrad_umkehren, "Mausrad umkehren");
                                });
                            });
                            ui.vertical(|ui| {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(330.0);
                                    ui.label("Programmverhalten");
                                    mark(ui, &mut o.ungueltige_loeschen, "ungültige Verbindungen mit löschen");
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Doppelklick auf Verweise =").font(bold(13.0)));
                                        egui::ComboBox::from_id_salt("optziel").width(110.0).selected_text(ZIEL[o.doppelklick_verweis.min(1)]).show_ui(ui, |ui| {
                                            for (i, z) in ZIEL.iter().enumerate() {
                                                ui.selectable_value(&mut o.doppelklick_verweis, i, *z);
                                            }
                                        });
                                    });
                                    mark(ui, &mut o.backup, "Backup-Datei erzeugen!");
                                    mark(ui, &mut o.pruefen_vor_druck, "Vor dem Drucken Projekt prüfen");
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(330.0);
                                    ui.label("Symboleditor");
                                    field(ui, "max Breite im Editor in mm", 190.0, &mut t[0], 70.0);
                                    field(ui, "max Höhe im Editor in mm", 190.0, &mut t[1], 70.0);
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(330.0);
                                    ui.label("Farben");
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Blatthintergrund im Editor").font(bold(13.0)));
                                        let mut c = Color32::from_rgb(o.blatt_farbe[0], o.blatt_farbe[1], o.blatt_farbe[2]);
                                        if ui.color_edit_button_srgba(&mut c).changed() {
                                            o.blatt_farbe = [c.r(), c.g(), c.b()];
                                        }
                                    });
                                });
                            });
                        });
                    }
                    1 => {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(280.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Blattformat").font(bold(13.0)));
                                    let cur = FORMATE.iter().find(|f| f.1 == o.format).map(|f| f.0).unwrap_or("Benutzerdefiniert");
                                    egui::ComboBox::from_id_salt("optfmt").width(140.0).selected_text(cur).show_ui(ui, |ui| {
                                        for (n, f) in FORMATE {
                                            if ui.selectable_label(cur == *n, *n).clicked() && f.0 > 0 {
                                                o.format = *f;
                                                t[2] = f1(f.0 as f32 / 10.0);
                                                t[3] = f1(f.1 as f32 / 10.0);
                                            }
                                        }
                                    });
                                });
                                let custom = !FORMATE.iter().any(|f| f.1 == o.format && f.1 .0 > 0);
                                ui.add_enabled_ui(custom, |ui| {
                                    field(ui, "Breite in mm", 90.0, &mut t[2], 80.0);
                                    field(ui, "Höhe in mm", 90.0, &mut t[3], 80.0);
                                });
                                mark(ui, &mut o.hochformat, "Hochformat");
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(270.0);
                                    ui.label("Ränder in mm");
                                    egui::Grid::new("optr").show(ui, |ui| {
                                        for (i, l) in ["Oben", "Links", "Unten", "Rechts"].iter().enumerate() {
                                            let k = [4, 6, 5, 7][i];
                                            ui.label(RichText::new(*l).font(bold(13.0)));
                                            ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut t[k]));
                                            if i % 2 == 1 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(270.0);
                                    ui.label("Strompfade");
                                    field(ui, "Höhe in mm", 90.0, &mut t[8], 70.0);
                                    field(ui, "Anzahl", 90.0, &mut t[9], 70.0);
                                    mark(ui, &mut o.ab_null, "Beginn mit '0'");
                                });
                            });
                            ui.vertical(|ui| {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(300.0);
                                    ui.label("Standardschrift");
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Schrift").font(bold(13.0)));
                                        egui::ComboBox::from_id_salt("optschrift").width(160.0).selected_text(o.schrift.clone()).show_ui(ui, |ui| {
                                            for f in ["Arial", "Courier New", "Tahoma", "Times New Roman", "Verdana"] {
                                                ui.selectable_value(&mut o.schrift, f.to_string(), f);
                                            }
                                        });
                                    });
                                    field(ui, "Höhe für Titel", 140.0, &mut t[10], 60.0);
                                    field(ui, "Höhe für Wert", 140.0, &mut t[11], 60.0);
                                    field(ui, "Höhe für Verbindung", 140.0, &mut t[12], 60.0);
                                    field(ui, "Höhe für Strompfade", 140.0, &mut t[13], 60.0);
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(300.0);
                                    ui.label("Schriftfeld");
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_id_salt("optfeld").width(150.0).selected_text(["Vorgabeschriftfeld", "Aus Datei"][o.schriftfeld.min(1)]).show_ui(ui, |ui| {
                                            ui.selectable_value(&mut o.schriftfeld, 0, "Vorgabeschriftfeld");
                                            ui.selectable_value(&mut o.schriftfeld, 1, "Aus Datei");
                                        });
                                        if ui.button("Suchen").clicked() {
                                            if let Some(p) = rfd::FileDialog::new().add_filter("Schriftfelddatei", &["eff", "json"]).pick_file() {
                                                o.schriftfeld_datei = p.display().to_string();
                                                o.schriftfeld = 1;
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Datei").font(bold(13.0)));
                                        ui.add_sized([230.0, 20.0], egui::TextEdit::singleline(&mut o.schriftfeld_datei));
                                    });
                                });
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(300.0);
                                    ui.label("Sammelschienen");
                                    field(ui, "Abstand in mm", 120.0, &mut t[14], 60.0);
                                });
                            });
                        });
                    }
                    2 => {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(290.0);
                            ui.label("Vorgabe Titelposition in mm");
                            egui::Grid::new("opttit").num_columns(3).show(ui, |ui| {
                                ui.label("");
                                ui.label(RichText::new("x").font(bold(13.0)));
                                ui.label(RichText::new("y").font(bold(13.0)));
                                ui.end_row();
                                for (i, l) in ["Sammelschiene", "Klemmstelle", "Verweis"].iter().enumerate() {
                                    ui.label(RichText::new(*l).font(bold(13.0)));
                                    ui.add_sized([55.0, 20.0], egui::TextEdit::singleline(&mut t[15 + i * 2]));
                                    ui.add_sized([55.0, 20.0], egui::TextEdit::singleline(&mut t[16 + i * 2]));
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    4 => {
                        // Tastenkürzel: Befehl, Belegung, Ändern (nächste Taste aufnehmen), Standard, Entfernen
                        if let Some(id) = *capture {
                            if let Some(k) = ui.input(|i| crate::kuerzel::aufnehmen(i)) {
                                if k != "Escape" {
                                    o.tasten.insert(id.to_string(), k);
                                }
                                *capture = None;
                            }
                        }
                        ui.small("Kopieren, Ausschneiden, Einfügen: Strg+C, Strg+X, Strg+V (fest). Pfeiltasten verschieben die Markierung (mit Umschalt um 1 mm).");
                        egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
                            egui::Grid::new("tasten").striped(true).num_columns(4).show(ui, |ui| {
                                for b in crate::kuerzel::BEFEHLE {
                                    ui.label(b.name);
                                    let cur = crate::kuerzel::belegung(&o.tasten, b.id);
                                    if *capture == Some(b.id) {
                                        ui.label(RichText::new("Taste drücken … (Esc = Abbruch)").italics());
                                    } else {
                                        ui.label(RichText::new(if cur.is_empty() { "–".to_string() } else { cur }).font(bold(12.0)));
                                    }
                                    if ui.small_button("Ändern").clicked() {
                                        *capture = Some(b.id);
                                    }
                                    ui.horizontal(|ui| {
                                        if ui.small_button("Standard").clicked() {
                                            o.tasten.remove(b.id);
                                        }
                                        if ui.small_button("Keine").clicked() {
                                            o.tasten.insert(b.id.to_string(), String::new());
                                        }
                                    });
                                    ui.end_row();
                                }
                            });
                        });
                        for (k, v) in crate::kuerzel::doppelt(&o.tasten) {
                            ui.colored_label(Color32::from_rgb(180, 0, 0), format!("{k} ist mehrfach belegt: {}", v.join(", ")));
                        }
                    }
                    _ => {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(290.0);
                            ui.label("Datenbankserver");
                            ui.small("Der Nachbau liest die Bibliothek direkt aus Datenbank\\Symbole.tfi; die Serverangaben werden nur gespeichert.");
                            for (l, v) in [("Server", &mut o.server), ("Port", &mut o.port), ("User", &mut o.user)] {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(l).font(bold(13.0)));
                                    ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(v));
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Passwort").font(bold(13.0)));
                                ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut o.passwort).password(true));
                            });
                        });
                        ui.add_space(8.0);
                        ui.label(lib_info.as_str());
                        if ui.button("Symbol-Datenbank (Ordner „Datenbank“) wählen …").clicked() {
                            pick_lib = true;
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(115.0);
                if ui.add_sized([75.0, 25.0], egui::Button::new("Abbruch")).clicked() {
                    result = Some(false);
                }
                ui.add_space(150.0);
                if ui.add_sized([75.0, 25.0], egui::Button::new("OK")).clicked() {
                    result = Some(true);
                }
            });
        });
        if closed {
            result = Some(false);
        }
        if pick_lib {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                self.load_library(Some(&dir));
            }
        }
        match result {
            None => self.opt_dlg = Some(d),
            Some(false) => {}
            Some(true) => {
                let t = &d.t;
                let o = &mut d.o;
                o.sym_breite = n1(&t[0], o.sym_breite);
                o.sym_hoehe = n1(&t[1], o.sym_hoehe);
                o.format = (i01(&t[2], o.format.0), i01(&t[3], o.format.1));
                for i in 0..4 {
                    o.raender[i] = i01(&t[4 + i], o.raender[i]);
                }
                o.pfad_hoehe = i01(&t[8], o.pfad_hoehe);
                o.pfade = t[9].trim().parse().unwrap_or(o.pfade).clamp(1, 99);
                o.size_titel = t[10].trim().parse().unwrap_or(o.size_titel);
                o.size_wert = t[11].trim().parse().unwrap_or(o.size_wert);
                o.size_verbg = t[12].trim().parse().unwrap_or(o.size_verbg);
                o.size_pfad = t[13].trim().parse().unwrap_or(o.size_pfad);
                o.sms_abstand = n1(&t[14], o.sms_abstand);
                o.titel_sms = (n1(&t[15], o.titel_sms.0), n1(&t[16], o.titel_sms.1));
                o.titel_klemme = (n1(&t[17], o.titel_klemme.0), n1(&t[18], o.titel_klemme.1));
                o.titel_verweis = (n1(&t[19], o.titel_verweis.0), n1(&t[20], o.titel_verweis.1));
                // sofort wirksam: Farbe, Mausrad
                self.sheet_color = Color32::from_rgb(o.blatt_farbe[0], o.blatt_farbe[1], o.blatt_farbe[2]);
                self.invert_wheel = o.mausrad_umkehren;
                self.opt = d.o;
            }
        }
    }

    /// Starteinstellungen der Optionen auf das Programm anwenden (beim Start).
    pub fn apply_start_options(&mut self) {
        let o = self.opt.clone();
        self.raster_idx = o.raster_idx.min(crate::app::RASTER.len() - 1);
        self.grid = o.gitter;
        self.path_align = o.pfadfang;
        self.auto_conn = o.autoverbindung;
        self.size_frame = o.rahmen_groesse;
        self.fix_active = o.fixierung;
        self.invert_wheel = o.mausrad_umkehren;
        self.sheet_color = Color32::from_rgb(o.blatt_farbe[0], o.blatt_farbe[1], o.blatt_farbe[2]);
    }
}
