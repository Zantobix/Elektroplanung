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
    /// Index in `crate::app::RASTER` (ohne "Kein"; gespeichert als "raster")
    #[serde(rename = "raster")]
    pub raster_idx: usize,
    /// Rasterindex älterer Versionen mit "Kein" an Stelle 0 (nur lesen, siehe `migrieren`)
    #[serde(rename = "raster_idx", skip_serializing)]
    pub alt_raster_idx: Option<usize>,
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
    /// beim Platzieren eines Symbols mit mehreren Bildern das Variantenrad zeigen (Original: immer)
    pub variantenrad: bool,
    /// Brückenbögen an Kreuzungen von Leitungen zeichnen (Nachbau-Erweiterung, `bruecken.rs`)
    pub brueckenboegen: bool,
    /// beim Programmstart bei GitHub nach einer neueren Version suchen (Nachbau-Erweiterung, `update_pruefung.rs`)
    pub update_pruefen: bool,
    // Programm › Symboleditor
    pub sym_breite: f32,
    pub sym_hoehe: f32,
    // Programm › Farben, Maus
    pub blatt_farbe: [u8; 3],
    /// Farbe des Rastergitters (Original: "Rasterdarstellung", dort ausgeblendet)
    pub raster_farbe: [u8; 3],
    pub mausrad_umkehren: bool,
    // Projektvorgaben1
    pub format: (i32, i32),
    pub hochformat: bool,
    pub raender: [i32; 4],
    /// Stand der Randvorgabe: 0 = ältere Version mit Links 10 mm als Vorgabe (siehe `migrieren`), 1 = aktuell
    #[serde(default)]
    pub raender_stand: u8,
    pub pfad_hoehe: i32,
    pub pfade: i32,
    pub ab_null: bool,
    /// Schriftfeld: 0 = Standard, 1 = aus Datei, 2 = ohne
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
    /// zuletzt gewählte Rasterweite je Projektdatei (Pfad, Index in RASTER), neueste zuerst – siehe `raster.rs`
    pub raster_je_projekt: Vec<(String, usize)>,
}

impl Default for Optionen {
    fn default() -> Self {
        // Projektvorgaben des Originals ohne INI (Ränder Links 20 mm)
        let a = AnlSettings::neu();
        Optionen {
            raster_idx: 5,
            alt_raster_idx: None,
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
            variantenrad: true,
            brueckenboegen: true,
            update_pruefen: true,
            sym_breite: 100.0,
            sym_hoehe: 100.0,
            blatt_farbe: [238, 221, 221],
            raster_farbe: rgb(RASTER_MITTEL),
            mausrad_umkehren: false,
            format: a.format,
            hochformat: false,
            raender: a.raender,
            raender_stand: 1,
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
            raster_je_projekt: vec![],
        }
    }
}

impl Optionen {
    /// Anlagen-Vorgaben für neue Anlagen.
    /// Gespeicherte Optionen älterer Versionen umrechnen: dort stand "Kein" an Stelle 0 der Rasterauswahl.
    /// "Kein" wird zu ausgeschaltetem Rasterfang (Gitterknopf aus) mit 1,0 mm.
    pub fn migrieren(&mut self) {
        if let Some(a) = self.alt_raster_idx.take() {
            if a == 0 {
                self.gitter = false;
            }
            self.raster_idx = a.saturating_sub(1).min(crate::app::RASTER.len() - 1);
        }
        // ältere Versionen gaben Links 10 mm vor (Ränder von W2); unverändert gespeichert → Vorgabe des Originals
        if self.raender_stand == 0 {
            if self.raender == crate::rahmen::RAENDER_W2 {
                self.raender = crate::rahmen::RAENDER_NEU;
            }
            self.raender_stand = 1;
        }
    }

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

/// Rasterfarben zur Auswahl: hell (frühere Vorgabe), mittel (Vorgabe), kräftig
pub const RASTER_HELL: Color32 = Color32::from_rgb(224, 208, 208);
pub const RASTER_MITTEL: Color32 = Color32::from_rgb(208, 188, 188);
pub const RASTER_KRAEFTIG: Color32 = Color32::from_rgb(192, 168, 168);

fn rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

fn farbe(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Blattformate der Projektvorgaben: wie im Original (FormatChange 0x4a0918) A3, A4, A5 und Benutzerdefiniert
/// (nur dann sind Breite/Höhe frei); A2/A1 sind Ergänzungen des Nachbaus.
const FORMATE: &[(&str, (i32, i32))] = &[("A3", (2970, 4200)), ("A4", (2100, 2970)), ("A5", (1480, 2100)), ("A2", (4200, 5940)), ("A1", (5940, 8410)), ("Benutzerdefiniert", (0, 0))];
/// Auswahl „Doppelklick auf Verweise =“: Einträge wie im Original (TForm4 0x4a0856/0x4a086d: „Eigenschaften“,
/// „Zielsprung“; Option1Click 0x4223cd speichert Index ≠ 0 = Zielsprung) als (gespeicherter Wert, Text);
/// gespeichert bleibt 0 = Ziel anzeigen, 1 = Eigenschaften.
const ZIEL: &[(usize, &str)] = &[(1, "Eigenschaften"), (0, "Zielsprung")];

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
        o.raster_farbe = rgb(self.grid_color);
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
                                    mark(ui, &mut o.gitter, "Rasterfang ein (Gitter anzeigen)");
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
                                        egui::ComboBox::from_id_salt("optziel").width(110.0).selected_text(if o.doppelklick_verweis == 0 { ZIEL[1].1 } else { ZIEL[0].1 }).show_ui(ui, |ui| {
                                            for (v, z) in ZIEL {
                                                ui.selectable_value(&mut o.doppelklick_verweis, *v, *z);
                                            }
                                        });
                                    });
                                    mark(ui, &mut o.backup, "Backup-Datei erzeugen!");
                                    mark(ui, &mut o.pruefen_vor_druck, "Vor dem Drucken Projekt prüfen");
                                    mark(ui, &mut o.variantenrad, "Variantenrad beim Platzieren");
                                    mark(ui, &mut o.brueckenboegen, "Brückenbögen an Kreuzungen");
                                    mark(ui, &mut o.update_pruefen, "Beim Start nach neuer Version suchen");
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
                                    // Rasterstärke: im Original ausgeblendetes Feld "Rasterdarstellung"; wirkt sofort
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Rasterdarstellung").font(bold(13.0)));
                                        let mut c = farbe(o.raster_farbe);
                                        if ui.color_edit_button_srgba(&mut c).changed() {
                                            o.raster_farbe = rgb(c);
                                        }
                                        for (n, f) in [("Hell", RASTER_HELL), ("Mittel", RASTER_MITTEL), ("Kräftig", RASTER_KRAEFTIG)] {
                                            if ui.selectable_label(o.raster_farbe == rgb(f), n).clicked() {
                                                o.raster_farbe = rgb(f);
                                            }
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
                                        // Einträge in der Reihenfolge des Originals (TForm4, 0x4a0884–0x4a08b2)
                                        egui::ComboBox::from_id_salt("optfeld").width(150.0).selected_text(schriftfeld_text(o.schriftfeld)).show_ui(ui, |ui| {
                                            for v in SCHRIFTFELD_REIHENFOLGE {
                                                ui.selectable_value(&mut o.schriftfeld, v, schriftfeld_text(v));
                                            }
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
            None => {
                // Rasterfarbe sofort sichtbar, solange der Dialog offen ist
                self.grid_color = farbe(d.o.raster_farbe);
                self.opt_dlg = Some(d);
            }
            Some(false) => self.grid_color = farbe(self.opt.raster_farbe),
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
                self.grid_color = farbe(o.raster_farbe);
                self.opt = d.o;
            }
        }
    }

    /// Starteinstellungen der Optionen auf das Programm anwenden (beim Start).
    pub fn apply_start_options(&mut self) {
        self.opt.migrieren();
        let o = self.opt.clone();
        self.raster_idx = o.raster_idx.min(crate::app::RASTER.len() - 1);
        self.grid = o.gitter;
        self.path_align = o.pfadfang;
        self.auto_conn = o.autoverbindung;
        self.size_frame = o.rahmen_groesse;
        // „Fixierung aktivieren“ wird wie im Original nur gespeichert (TFORM4.Anker, +0xa3a nirgends gelesen):
        // der Knopf Fixierung startet immer gedrückt (DFM Down=True)
        self.fix_active = true;
        self.invert_wheel = o.mausrad_umkehren;
        self.sheet_color = Color32::from_rgb(o.blatt_farbe[0], o.blatt_farbe[1], o.blatt_farbe[2]);
        self.grid_color = farbe(o.raster_farbe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterfarbe_vorgabe_und_alte_einstellungen() {
        let o = Optionen::default();
        assert_eq!(o.raster_farbe, rgb(RASTER_MITTEL));
        assert!(o.variantenrad);
        // mittel liegt zwischen hell und kräftig
        for k in 0..3 {
            assert!(RASTER_KRAEFTIG.to_array()[k] <= o.raster_farbe[k] && o.raster_farbe[k] <= RASTER_HELL.to_array()[k]);
        }
        // gespeicherte Optionen ohne die neuen Felder: Vorgaben
        let alt: Optionen = serde_json::from_str(r#"{"gitter": false}"#).unwrap();
        assert!(!alt.gitter);
        assert_eq!(alt.raster_farbe, rgb(RASTER_MITTEL));
        assert!(alt.variantenrad);
        let j = serde_json::to_string(&Optionen { raster_farbe: rgb(RASTER_HELL), ..Default::default() }).unwrap();
        let back: Optionen = serde_json::from_str(&j).unwrap();
        assert_eq!(back.raster_farbe, rgb(RASTER_HELL));
    }

    #[test]
    fn rasterindex_ohne_kein() {
        assert_eq!(crate::app::RASTER[Optionen::default().raster_idx].0, "10,0mm");
        // alte Einstellungen: Index 6 = 10 mm, 2 = 2,5 mm, 0 = "Kein" → Rasterfang aus
        for (alt, neu, gitter) in [(6, "10,0mm", true), (2, "2,5mm", true), (0, "1,0mm", false), (7, "14,0mm", true)] {
            let mut o: Optionen = serde_json::from_str(&format!(r#"{{"raster_idx": {alt}, "gitter": true}}"#)).unwrap();
            o.migrieren();
            assert_eq!((crate::app::RASTER[o.raster_idx].0, o.gitter), (neu, gitter));
        }
        // neu gespeichert: nur "raster", wird unverändert gelesen
        let j = serde_json::to_string(&Optionen { raster_idx: 1, ..Default::default() }).unwrap();
        assert!(!j.contains("raster_idx"));
        let mut o: Optionen = serde_json::from_str(&j).unwrap();
        o.migrieren();
        assert_eq!(o.raster_idx, 1);
    }

    #[test]
    fn projektvorgaben_fuer_neue_blaetter_und_projekte() {
        let mut app = crate::blatt_tasten::test_app();
        // Hochformat gilt für jedes neue Blatt (0x403ce4), nicht nur für das erste einer neuen Anlage
        app.opt.hochformat = true;
        app.new_sheet();
        assert!(app.project.anlagen[0].sheets.last().unwrap().portrait);
        // Standardschrift und Schriftfeld „aus Datei“ gelten für neue Projekte (0x4055d8)
        let tb = vec![crate::titleblock::TbOwned::Line(0, 0, 100, 0)];
        let f = std::env::temp_dir().join(format!("ep_sfd_test_{}.json", std::process::id()));
        std::fs::write(&f, serde_json::to_string(&tb).unwrap()).unwrap();
        app.opt.schrift = "Arial".into();
        app.opt.schriftfeld = 1;
        app.opt.schriftfeld_datei = f.display().to_string();
        app.new_project();
        let _ = std::fs::remove_file(&f);
        assert_eq!(app.project.schrift, "Arial");
        assert_eq!(app.project.titleblock, tb);
        // Doppelklick-Auswahl in der Reihenfolge des Originals, Vorgabe Zielsprung
        assert_eq!(ZIEL.iter().map(|z| z.1).collect::<Vec<_>>(), ["Eigenschaften", "Zielsprung"]);
        assert_eq!(ZIEL.iter().find(|z| z.0 == Optionen::default().doppelklick_verweis).unwrap().1, "Zielsprung");
    }

    #[test]
    fn strompfade_ab_null() {
        let mut a = crate::model::Anlage { name: "A".into(), sheets: vec![Sheet::new(1)], sammelschienen: vec![], klemmleisten: vec![], klemmen: vec![], schienen: Default::default(), set: AnlSettings::neu(), raw: None };
        crate::rahmen::abgleichen_anlage(&mut a);
        assert_eq!(a.sheets[0].col_label(0), "1");
        a.set.ab_null = true;
        crate::rahmen::abgleichen_anlage(&mut a);
        assert_eq!((a.sheets[0].col_label(0), a.sheets[0].col_label(7)), ("0".to_string(), "7".to_string()));
    }

    #[test]
    fn randvorgabe_wie_original() {
        // neue Anlage: Links 20 mm, die übrigen Ränder 5 mm (Original-Dialog orig_03)
        assert_eq!(Optionen::default().anlage().raender, [50, 50, 200, 50]);
        // ältere gespeicherte Optionen mit der alten Vorgabe (Links 10 mm) → Vorgabe des Originals
        let mut o: Optionen = serde_json::from_str(r#"{"raender": [50, 50, 100, 50]}"#).unwrap();
        o.migrieren();
        assert_eq!((o.raender, o.raender_stand), ([50, 50, 200, 50], 1));
        // selbst gewählte Ränder bleiben, ebenso nach der Umstellung gespeicherte 10 mm
        let mut o: Optionen = serde_json::from_str(r#"{"raender": [50, 50, 150, 50]}"#).unwrap();
        o.migrieren();
        assert_eq!(o.raender, [50, 50, 150, 50]);
        let j = serde_json::to_string(&Optionen { raender: [50, 50, 100, 50], ..Default::default() }).unwrap();
        let mut o: Optionen = serde_json::from_str(&j).unwrap();
        o.migrieren();
        assert_eq!(o.raender, [50, 50, 100, 50]);
    }
}

/// Auswahl „Schriftfeld“ der Programmeinstellungen: gespeicherter Wert (0 Vorgabe, 1 Datei, 2 ohne) in der
/// Reihenfolge der Liste des Originals (TForm4: „Ohne Schriftfeld“, „Vorgabeschriftfeld“, „Schriftfeld aus Datei“).
pub const SCHRIFTFELD_REIHENFOLGE: [usize; 3] = [2, 0, 1];

/// Text eines Eintrags der Auswahl „Schriftfeld“.
pub fn schriftfeld_text(v: usize) -> &'static str {
    match v {
        0 => crate::meldungen::OPT_VORGABESCHRIFTFELD,
        1 => crate::meldungen::OPT_SCHRIFTFELD_DATEI,
        _ => crate::meldungen::OPT_OHNE_SCHRIFTFELD,
    }
}

/// Schriftfeld eines neuen Projekts nach den Projektvorgaben (Original 0x4055d8: Feld 1 = Vorgabeschriftfeld,
/// 2 = Schriftfeld aus Datei 0x4732d4). „Aus Datei“ liest die Schriftfelddatei des Originals (.eff), dazu eine
/// eigene Vorgabedatei (JSON) oder das Schriftfeld eines Projekts (.epr/.epj); ist die Datei unlesbar, gilt die
/// Vorgabe. „Ohne Schriftfeld“ kennt das Modell nicht (leeres Schriftfeld = Vorgabe).
pub fn schriftfeld_neues_projekt(o: &Optionen) -> Option<Vec<crate::titleblock::TbOwned>> {
    if o.schriftfeld == 1 && !o.schriftfeld_datei.trim().is_empty() {
        let p = std::path::Path::new(o.schriftfeld_datei.trim());
        let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let tb = match ext.as_str() {
            "eff" => std::fs::read(p).ok().and_then(|d| crate::schriftfeld_datei::schriftfeld_aus(&d).ok()),
            "json" => std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<Vec<crate::titleblock::TbOwned>>(&s).ok()),
            "epr" => crate::epr::import(p).ok().map(|x| x.titleblock),
            "epj" => std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<crate::model::Project>(&s).ok()).map(|x| x.titleblock),
            _ => None,
        };
        if let Some(tb) = tb.filter(|t| !t.is_empty()) {
            return Some(tb);
        }
    }
    crate::extra::load_default_titleblock()
}
