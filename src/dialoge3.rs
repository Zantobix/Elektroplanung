//! Weitere Dialoge nach den Formularen des Originals, jeweils mit Funktion:
//! Objekteigenschaften für Sammelschiene (TSammelS), Verbindung (TSVerbindung), Tabelle (TSTab),
//! Klemmstelle (TSKlemme), Verweis (TSVerweis), Bild (TBild); Sammelschienen-Einstellungen (TSSchiene),
//! Klemmleiste mit Indexübersicht (TKLeiste), Pfadbeschreibungen (TPfadDialog), Titelgenerator (TTitel),
//! Schriftfeld (TSchriftfeld, TForm10, TSFWahl), PDF-Export mehrerer Projekte (TPExport),
//! Blattnummerierung (TBlattIndex), Anlagenauswahl (TWahl), Benutzermaße (TUserdat),
//! Texteigenschaft (TTextchange), Kontaktspiegel (TSSpiegel), Fehlerliste (TErrliste).

use crate::app::{App, Dialog};
use crate::model::*;
use crate::props::{bold, tool_window};
use crate::symbols;
use eframe::egui::{self, Color32, RichText, Vec2};
use std::path::PathBuf;

pub enum Dlg3 {
    BusbarObj { id: u64, sliste: Vec<String>, sel: usize, einsp: bool, auto: bool, unten: bool, offset: String, tx: String, ty: String, px: String, py: String, laenge: String },
    Schiene { anlage: usize, orig: String, name: String, set: SchieneSet, pots: Vec<String>, stifte: Vec<String> },
    Verbindung { id: u64, art: usize, leitung: usize, leitungen: Vec<String>, opts: u8, text: String },
    /// Tabelle (TSTab), Zustand und Bedienung in tabelle.rs
    Tab(crate::tabelle::TabDlg),
    Klemme { id: u64, leisten: Vec<String>, leiste: usize, titel: String, gr_auto: bool, align: usize, tx: String, ty: String, nummer: String, auto: bool, korr: bool, position: String, posk: bool, ebene: usize, funktion: String, ziel_wechsel: bool },
    Verweis { id: u64, anlagen: Vec<String>, anlage: usize, blatt: usize, link: usize, zu: bool, titel: String },
    Bild { id: u64, grafiken: Vec<String>, grafik: usize, art: usize, oben: String, links: String, hoehe: String, breite: String, rahmen: bool, fixiert: bool },
    KLeiste { anlage: usize, idx: usize, name: String, titel: String, x: String, y: String, anzahl: String, ebenen: usize, nrs: Vec<String> },
    Pfad { r: SheetRef, rows: Vec<String>, grenze: bool },
    Titel { formel: String, dir1: usize, dir2: usize, wahl: usize, hilfe: bool, msg: String },
    Schriftfeld { absolut: bool, hoehe: String },
    NeuesSchriftfeld { anlage: usize, aus_datei: bool, pfad: String },
    SfWahl { quellen: Vec<(String, Vec<crate::titleblock::TbOwned>)>, wahl: usize },
    PExport { liste: Vec<String>, sel: Option<usize>, status: String },
    BlattIndex { anlage: usize, von: String, bis: String, ab: String, plus: String },
    Wahl { path: PathBuf, anlagen: Vec<(String, bool)> },
    UserDat { anlage: usize, breite: String, hoehe: String },
    /// Texteigenschaft (TTextchange): `wert` None = Bezeichner setzen (Texte bzw. BMK der Symbole),
    /// Some(false/true) = Titel bzw. Wert eines Symbols (Eigenschaften eines Symboltextes)
    TextChange { ids: Vec<u64>, text: String, wert: Option<bool> },
    Errliste { lines: Vec<String> },
    /// Listen Export (TListExport): Liste, Dateiformat, Meldung
    ListExport { art: usize, format: usize, msg: String },
}

const DIRS: &[&str] = &["Links nach Rechts", "Rechts nach Links", "Oben nach Unten", "Unten nach Oben"];
const BILD_ARTEN: &[&str] = &["Zentriert", "Eingepasst", "Gestreckt"];
const KL_ALIGN: &[&str] = &["Links", "Mittig", "Rechts"];
const EBENEN: &[&str] = &["1", "2", "3", "4"];

fn num(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse().ok()
}
fn f1(v: f32) -> String {
    format!("{v:.1}").replace('.', ",")
}
fn lbl(ui: &mut egui::Ui, w: f32, t: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
    ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, t, bold(13.0), Color32::BLACK);
}
fn field(ui: &mut egui::Ui, w: f32, t: &str, s: &mut String, fw: f32) {
    ui.horizontal(|ui| {
        lbl(ui, w, t);
        ui.add(egui::TextEdit::singleline(s).desired_width(fw));
    });
}
fn combo(ui: &mut egui::Ui, id: &str, items: &[String], sel: &mut usize, w: f32) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).width(w).selected_text(items.get(*sel).cloned().unwrap_or_default()).show_ui(ui, |ui| {
        for (i, it) in items.iter().enumerate() {
            if ui.selectable_label(*sel == i, it).clicked() {
                *sel = i;
                changed = true;
            }
        }
    });
    changed
}
fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Spaltenweise Liste (TIconTab des Originals): Zellen bearbeiten, Zeile wählen.
fn icon_tab(ui: &mut egui::Ui, id: &str, head: &[&str], rows: &mut [Vec<String>], sel: &mut usize, h: f32) {
    egui::ScrollArea::both().id_salt(id).max_height(h).show(ui, |ui| {
        egui::Grid::new(id).striped(true).num_columns(head.len() + 1).show(ui, |ui| {
            ui.label("");
            for c in head {
                ui.label(RichText::new(*c).strong());
            }
            ui.end_row();
            for (r, row) in rows.iter_mut().enumerate() {
                if ui.selectable_label(*sel == r, format!("{}", r + 1)).clicked() {
                    *sel = r;
                }
                for c in row.iter_mut() {
                    let resp = ui.add(egui::TextEdit::singleline(c).desired_width(90.0));
                    if resp.gained_focus() {
                        *sel = r;
                    }
                }
                ui.end_row();
            }
        });
    });
}

/// Liste als Text (eine Zeile je Eintrag, Spalten durch ';') laden bzw. speichern.
pub fn list_to_text(rows: &[Vec<String>]) -> String {
    rows.iter().map(|r| r.join(";")).collect::<Vec<_>>().join("\r\n")
}
pub fn list_from_text(s: &str, cols: usize) -> Vec<Vec<String>> {
    s.lines()
        .map(|l| {
            let mut v: Vec<String> = l.split(';').map(|c| c.trim().to_string()).collect();
            v.resize(cols.max(1), String::new());
            v
        })
        .collect()
}

/// Titelgenerator: Formel je Objekt auswerten. Variablen: @x laufende Nummer, @S Blattnummer, @P Strompfad,
/// @A Anlage (Kurzform), @T bisheriger Text.
pub fn titel_formel(formel: &str, nr: usize, sheet: u32, pfad: u32, anlage: &str, alt: &str) -> String {
    formel.replace("@x", &nr.to_string()).replace("@S", &sheet.to_string()).replace("@P", &pfad.to_string()).replace("@A", anlage).replace("@T", alt)
}

/// Sortierschlüssel nach Reihenfolge (0 links→rechts, 1 rechts→links, 2 oben→unten, 3 unten→oben).
pub fn order_key(p: P, dir: usize) -> f32 {
    match dir {
        1 => -p.x,
        2 => p.y,
        3 => -p.y,
        _ => p.x,
    }
}

/// Verweise eines Blattes (Objekt-Id, Text) für die Auswahl "Verweislink".
fn verweis_links(project: &Project, r: SheetRef) -> Vec<(u64, String)> {
    project
        .sheet(r)
        .map(|s| s.objects.iter().filter_map(|o| if let Kind::Verweis { text, .. } = &o.kind { Some((o.id, text.clone())) } else { None }).filter(|(_, t)| !t.is_empty()).collect())
        .unwrap_or_default()
}

impl App {
    /// Eigenschaften der Objektarten ohne eigenen Dialog in props.rs; true = Dialog geöffnet.
    pub fn open_props3(&mut self, id: u64) -> bool {
        let Some(r) = self.cur() else { return false };
        let Some(o) = self.project.sheet(r).and_then(|s| s.objects.iter().find(|o| o.id == id)).cloned() else { return false };
        let anl = &self.project.anlagen[r.anlage];
        self.dlg3 = Some(match &o.kind {
            Kind::Busbar { a, x2, name, label, lage, versatz, .. } => {
                let mut sliste: Vec<String> = anl.sammelschienen.clone();
                if !sliste.contains(name) {
                    sliste.push(name.clone());
                }
                let sel = sliste.iter().position(|s| s == name).unwrap_or(0);
                let set = anl.schienen.get(name).cloned().unwrap_or_default();
                // Positionierung aus OBJ Feld 15 (Bit 1 fest = kein Auto, Bit 0 Unten) und Feld 11 (Y Offset)
                let (auto, unten) = (lage & crate::sammelschiene_lage::FEST == 0, lage & crate::sammelschiene_lage::UNTEN != 0);
                Dlg3::BusbarObj { id, sliste, sel, einsp: set.einspeisung, auto, unten, offset: f1(*versatz), tx: f1(label.x), ty: f1(label.y), px: f1(a.x), py: f1(a.y), laenge: f1(x2 - a.x) }
            }
            Kind::Wire { leitung, bez, opts, .. } => Dlg3::Verbindung {
                id,
                art: (*opts & WIRE_BRUECKE != 0) as usize,
                leitung: *leitung,
                leitungen: self.project.leitungen.iter().map(|l| l.name.clone()).collect(),
                opts: *opts,
                text: bez.first().map(|b| b.1.clone()).unwrap_or_default(),
            },
            Kind::Table { rows, cols, cells, header, fixed_w, art, ab, .. } => Dlg3::Tab(crate::tabelle::TabDlg::neu(id, *rows, *cols, cells, *header, *fixed_w, o.fixed, *art, *ab)),
            Kind::Terminal { leiste, nr, level, text_off, tpl, opt, .. } => {
                let mut leisten = anl.klemmleisten.clone();
                for k in &anl.klemmen {
                    if !leisten.contains(&k.name) {
                        leisten.push(k.name.clone());
                    }
                }
                if !leisten.contains(leiste) {
                    leisten.push(leiste.clone());
                }
                let t = text_off.unwrap_or(P::new(-1.0, -2.0));
                Dlg3::Klemme {
                    id,
                    leiste: leisten.iter().position(|l| l == leiste).unwrap_or(0),
                    leisten,
                    titel: if tpl.is_empty() { "%O%B.%C".into() } else { tpl.clone() },
                    gr_auto: opt.gr_auto,
                    align: opt.align.min(2) as usize,
                    tx: f1(t.x),
                    ty: f1(t.y),
                    nummer: nr.to_string(),
                    auto: opt.auto,
                    korr: opt.korr,
                    position: opt.pos(*nr).to_string(),
                    posk: opt.posk,
                    ebene: (*level as usize).saturating_sub(1),
                    funktion: opt.funktion.clone(),
                    ziel_wechsel: opt.ziel_wechsel,
                }
            }
            Kind::Verweis { text, dir, ziel, .. } => {
                // gesetztes Ziel vorwählen (Anlage, Blatt, Gegenverweis), sonst das eigene Blatt
                let (blatt_ref, gegen) = ziel.as_ref().and_then(|z| self.project.resolve_ziel(z)).unwrap_or((r, None));
                let link = gegen.and_then(|g| verweis_links(&self.project, blatt_ref).iter().position(|(i, _)| *i == g)).map(|i| i + 1).unwrap_or(0);
                Dlg3::Verweis {
                    id,
                    anlagen: self.project.anlagen.iter().map(|a| a.name.clone()).collect(),
                    anlage: blatt_ref.anlage,
                    blatt: blatt_ref.sheet,
                    link,
                    zu: *dir == 0 || *dir == 270,
                    titel: text.clone(),
                }
            }
            Kind::Image { a, b, path, mode, frame } => {
                let mut grafiken: Vec<String> = self.project.bilder.iter().map(|b| format!("bld:{}", b.nr)).collect();
                if !grafiken.contains(path) && !path.is_empty() {
                    grafiken.push(path.clone());
                }
                let (mn, mx) = crate::canvas::norm_rect(*a, *b);
                Dlg3::Bild {
                    id,
                    grafik: grafiken.iter().position(|g| g == path).unwrap_or(0),
                    grafiken,
                    art: match mode {
                        1 => 0,
                        2 => 1,
                        _ => 2,
                    },
                    oben: f1(mn.y),
                    links: f1(mn.x),
                    hoehe: f1(mx.y - mn.y),
                    breite: f1(mx.x - mn.x),
                    rahmen: *frame,
                    fixiert: o.fixed,
                }
            }
            _ => return false,
        });
        true
    }

    pub fn open_schiene_dialog(&mut self, anlage: usize, name: &str) {
        let Some(a) = self.project.anlagen.get(anlage) else { return };
        let set = a.schienen.get(name).cloned().unwrap_or_default();
        self.dlg3 = Some(Dlg3::Schiene {
            anlage,
            orig: name.into(),
            name: name.into(),
            set,
            pots: self.project.potentiale.clone(),
            stifte: self.project.stifte.iter().map(|s| s.name.clone()).collect(),
        });
    }

    pub fn open_kleiste_dialog(&mut self, anlage: usize, idx: usize) {
        let Some(a) = self.project.anlagen.get(anlage) else { return };
        let name = a.klemmleisten.get(idx).cloned().unwrap_or_default();
        let info = a.klemmen.iter().find(|k| k.name == name).cloned().unwrap_or_else(|| KlemmInfo { id: 0, name: name.clone(), nrs: vec![], intern: name.clone(), etagen: 1, titel_off: (0, -50) });
        let mut nrs: Vec<String> = info.nrs.iter().map(|n| n.to_string()).collect();
        // Klemmen, die nur auf den Blättern stehen, ergänzen
        for s in &a.sheets {
            for o in &s.objects {
                if let Kind::Terminal { leiste, nr, .. } = &o.kind {
                    if *leiste == name && !nrs.contains(&nr.to_string()) {
                        nrs.push(nr.to_string());
                    }
                }
            }
        }
        nrs.sort_by_key(|n| n.parse::<u32>().unwrap_or(0));
        // Name (KLM Byte 0–43), Titel (TIT), Titelposition (Feld 10/11, 0,1 mm) und Ebenen (Feld 15) aus der Klemmleiste
        let intern = if info.intern.is_empty() { name.clone() } else { info.intern.clone() };
        let (tx, ty) = if info.id == 0 { (0, -50) } else { info.titel_off };
        self.dlg3 = Some(Dlg3::KLeiste { anlage, idx, name: intern, titel: name, x: f1(tx as f32 / 10.0), y: f1(ty as f32 / 10.0), anzahl: nrs.len().to_string(), ebenen: (info.etagen.max(1) as usize - 1).min(3), nrs });
    }

    pub fn open_pfad_dialog(&mut self) {
        let Some(r) = self.cur() else { return };
        let Some(s) = self.project.sheet(r) else { return };
        self.dlg3 = Some(Dlg3::Pfad { r, rows: (0..s.columns as usize).map(|i| s.col_label(i)).collect(), grenze: s.pfad_linien });
    }

    pub fn open_titel_dialog(&mut self) {
        self.dlg3 = Some(Dlg3::Titel { formel: "-S@x".into(), dir1: 0, dir2: 2, wahl: 0, hilfe: false, msg: String::new() });
    }

    pub fn open_blattindex(&mut self) {
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        let n = self.project.anlagen.get(a).map(|a| a.sheets.len()).unwrap_or(1);
        self.dlg3 = Some(Dlg3::BlattIndex { anlage: a, von: "1".into(), bis: n.to_string(), ab: "1".into(), plus: "1".into() });
    }

    pub fn open_pexport(&mut self) {
        let mut liste = vec![];
        if let Some(p) = &self.path {
            liste.push(p.display().to_string());
        } else {
            liste.push("(aktuelles Projekt)".into());
        }
        self.dlg3 = Some(Dlg3::PExport { liste, sel: None, status: String::new() });
    }

    /// Datei › Projekt hinzufügen: Anlagen auswählen (TWahl), dann übernehmen.
    pub fn open_add_project(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Elektroplanung Projekt", &["epj", "epr"]).pick_file() else { return };
        match crate::mcp::open_project(&p) {
            Ok(prj) => self.dlg3 = Some(Dlg3::Wahl { path: p, anlagen: prj.anlagen.iter().map(|a| (a.name.clone(), true)).collect() }),
            Err(e) => self.dialog = Some(Dialog::Message(format!("Projekt wird gelesen – Fehler:\n{e}"))),
        }
    }

    pub fn open_neues_schriftfeld(&mut self) {
        self.dlg3 = Some(Dlg3::NeuesSchriftfeld { anlage: 0, aus_datei: false, pfad: String::new() });
    }

    pub fn open_schriftfeld_eigenschaften(&mut self) {
        let h = self.project.title_h();
        self.dlg3 = Some(Dlg3::Schriftfeld { absolut: self.project.tb_absolut, hoehe: f1(h) });
    }

    pub fn open_userdat(&mut self, anlage: usize) {
        let f = self.project.anlagen.get(anlage).map(|a| a.set.format).unwrap_or((2100, 2970));
        self.dlg3 = Some(Dlg3::UserDat { anlage, breite: f1(f.0 as f32 / 10.0), hoehe: f1(f.1 as f32 / 10.0) });
    }

    /// Texteigenschaft: einen Text für alle markierten Texte bzw. als BMK aller markierten Symbole setzen.
    pub fn open_textchange(&mut self) {
        let objs = self.selected_objs();
        let first = objs.iter().find_map(|o| match &o.kind {
            Kind::Text { text, .. } => Some(text.clone()),
            Kind::Symbol { bmk, .. } => Some(bmk.clone()),
            _ => None,
        });
        self.dlg3 = Some(Dlg3::TextChange { ids: objs.iter().map(|o| o.id).collect(), text: first.unwrap_or_default(), wert: None });
    }

    /// Kontaktspiegel (TSSpiegel): verknüpftes Objekt unter der Spule, siehe kontaktspiegel.rs.
    pub fn open_spiegel(&mut self) {
        self.spiegel_oeffnen();
    }

    /// Dialoge zeichnen und anwenden.
    pub fn dialoge3_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dlg3.take() else { return };
        let mut done: Option<bool> = None;
        let (title, size): (&str, [f32; 2]) = match &d {
            Dlg3::BusbarObj { .. } | Dlg3::Verbindung { .. } | Dlg3::Klemme { .. } | Dlg3::Verweis { .. } | Dlg3::Bild { .. } => ("Eigenschaften", [300.0, 400.0]),
            Dlg3::Tab(_) => ("Eigenschaften", [420.0, 420.0]),
            Dlg3::Schiene { .. } => ("Sammelschiene", [289.0, 300.0]),
            Dlg3::KLeiste { .. } => ("Klemmleiste", [300.0, 420.0]),
            Dlg3::Pfad { .. } => ("Pfadbeschreibungen", [354.0, 376.0]),
            Dlg3::Titel { .. } => ("Titelgenerator", [300.0, 260.0]),
            Dlg3::Schriftfeld { .. } => ("Schriftfeldeigenschaften", [180.0, 154.0]),
            Dlg3::NeuesSchriftfeld { .. } => ("Neues Schriftfeld", [264.0, 184.0]),
            Dlg3::SfWahl { .. } => ("Schriftfeldauswahl", [300.0, 98.0]),
            Dlg3::PExport { .. } => ("Exporteinstellung", [420.0, 325.0]),
            Dlg3::BlattIndex { .. } => ("Blattnummerierung", [358.0, 200.0]),
            Dlg3::Wahl { .. } => ("Auswahl", [275.0, 307.0]),
            Dlg3::UserDat { .. } => ("Benutzermaße", [180.0, 100.0]),
            Dlg3::TextChange { .. } => ("Texteigenschaft", [285.0, 63.0]),
            Dlg3::Errliste { .. } => ("Fehler in den Daten", [620.0, 264.0]),
            Dlg3::ListExport { .. } => ("Listen Export", [420.0, 566.0]),
        };
        let leitungen_n = self.project.leitungen.len();
        let mut rail_edit: Option<String> = None;
        // Klemme › Bauteil "Bearbeiten": Bauteilzuordnung öffnen (Objekt-Id)
        let mut part_edit: Option<u64> = None;
        let closed = tool_window(ctx, "dlg3", title, size, egui::Order::Middle, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 5.0);
            match &mut d {
                Dlg3::BusbarObj { sliste, sel, einsp, auto, unten, offset, tx, ty, px, py, laenge, .. } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 76.0, "S-Schiene");
                        combo(ui, "sms", sliste, sel, 150.0);
                        // Knopf "Bearbeiten" (SammelS1NeuClick): Einstellungen der gewählten Schiene
                        let btn = egui::Button::image(crate::icons::image(crate::icons::Icon::EditSammelschiene, 20.0));
                        if ui.add(btn.min_size(Vec2::new(31.0, 24.0))).on_hover_text("Bearbeiten").clicked() {
                            rail_edit = sliste.get(*sel).cloned();
                        }
                    });
                    ui.checkbox(einsp, RichText::new("Einspeisung").font(bold(13.0)));
                    ui.group(|ui| {
                        ui.label(RichText::new("Positionierung").font(bold(12.0)));
                        ui.horizontal(|ui| {
                            ui.checkbox(auto, "Auto");
                            lbl(ui, 60.0, "Y Offset");
                            ui.add(egui::TextEdit::singleline(offset).desired_width(50.0));
                            ui.label("mm");
                        });
                        ui.checkbox(unten, "Unten");
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 96.0, "");
                        ui.label("X in mm");
                        ui.add_space(30.0);
                        ui.label("Y in mm");
                    });
                    for (t, a, b) in [("Titelposition", tx, ty), ("Startpunkt", px, py)] {
                        ui.horizontal(|ui| {
                            lbl(ui, 96.0, t);
                            ui.add(egui::TextEdit::singleline(a).desired_width(70.0));
                            ui.add(egui::TextEdit::singleline(b).desired_width(70.0));
                        });
                    }
                    field(ui, 96.0, "Länge", laenge, 70.0);
                }
                Dlg3::Schiene { name, set, pots, stifte, .. } => {
                    field(ui, 70.0, "Name", name, 180.0);
                    field(ui, 70.0, "Kürzel", &mut set.kurz, 180.0);
                    ui.horizontal(|ui| {
                        lbl(ui, 86.0, "Potential");
                        let mut i = pots.iter().position(|p| *p == set.potential).unwrap_or(0);
                        if combo(ui, "spot", pots, &mut i, 150.0) {
                            set.potential = pots[i].clone();
                        }
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 86.0, "Stift");
                        combo(ui, "sstift", stifte, &mut set.stift, 150.0);
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.checkbox(&mut set.quer, "Querverweise");
                            ui.checkbox(&mut set.pfeile, "mit Pfeilen");
                            ui.checkbox(&mut set.klemmen, "Klemmen");
                            ui.checkbox(&mut set.schleife, "Durchschleifung");
                            ui.checkbox(&mut set.autoende, "Autoende");
                        });
                        ui.checkbox(&mut set.einspeisung, RichText::new("Einspeisung").font(bold(13.0)));
                    });
                }
                Dlg3::Verbindung { art, leitung, leitungen, opts, text, .. } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 56.0, "Art");
                        combo(ui, "vart", &strs(&["Verbindung", "Brücke"]), art, 180.0);
                    });
                    let mut neu = false;
                    ui.horizontal(|ui| {
                        lbl(ui, 56.0, "Leitung");
                        combo(ui, "vltg", leitungen, leitung, 180.0);
                        neu = ui.button("Neu").clicked();
                    });
                    if neu {
                        leitungen.push(format!("Leitung {}", leitungen_n + 1));
                        *leitung = leitungen.len() - 1;
                    }
                    fn flag(ui: &mut egui::Ui, opts: &mut u8, bit: u8, t: &str) {
                        let mut v = *opts & bit != 0;
                        if ui.checkbox(&mut v, t).changed() {
                            *opts = if v { *opts | bit } else { *opts & !bit };
                        }
                    }
                    flag(ui, opts, WIRE_ZIEL_A, "Zielführung Links/Oben");
                    flag(ui, opts, WIRE_ZIEL_B, "Zielführung Rechts/Unten");
                    flag(ui, opts, WIRE_BRUECKE, "Verbindung ist Brücke");
                    ui.group(|ui| {
                        ui.label(RichText::new("Bezeichnung (%O = Leitung)").font(bold(12.0)));
                        field(ui, 40.0, "Text", text, 180.0);
                        flag(ui, opts, WIRE_HOCHKANT, "Hochkant");
                        flag(ui, opts, WIRE_MARKIERUNG, "Markierung");
                        flag(ui, opts, WIRE_UNTERSTRICH, "Unterstrichen");
                        let mut durch = *opts & WIRE_DURCH != 0;
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            if ui.radio(!durch, "An Markierung").clicked() {
                                durch = false;
                            }
                            if ui.radio(durch, "Durch Markierung").clicked() {
                                durch = true;
                            }
                        });
                        *opts = if durch { *opts | WIRE_DURCH } else { *opts & !WIRE_DURCH };
                    });
                    if *art == 1 {
                        *opts |= WIRE_BRUECKE;
                    }
                }
                Dlg3::Tab(t) => crate::tabelle::dialog_ui(ui, t),
                Dlg3::Klemme { id, leisten, leiste, titel, gr_auto, align, tx, ty, nummer, auto, korr, position, posk, ebene, funktion, ziel_wechsel } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Klemmleiste");
                        combo(ui, "kleiste", leisten, leiste, 170.0);
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Titel").font(bold(12.0)));
                        ui.checkbox(gr_auto, "Gruppenautomatik");
                        field(ui, 40.0, "Text", titel, 120.0);
                        ui.horizontal(|ui| {
                            lbl(ui, 80.0, "Ausrichtung");
                            combo(ui, "kalign", &strs(KL_ALIGN), align, 80.0);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Position X");
                            ui.add(egui::TextEdit::singleline(tx).desired_width(40.0));
                            ui.label("Y");
                            ui.add(egui::TextEdit::singleline(ty).desired_width(40.0));
                        });
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Klemmenindex").font(bold(12.0)));
                        ui.checkbox(auto, "Automatisch");
                        ui.horizontal(|ui| {
                            lbl(ui, 60.0, "Nummer");
                            ui.add_enabled(!*auto, egui::TextEdit::singleline(nummer).desired_width(50.0));
                            ui.checkbox(korr, "Korrektur");
                        });
                        ui.horizontal(|ui| {
                            lbl(ui, 60.0, "Position");
                            ui.add(egui::TextEdit::singleline(position).desired_width(50.0));
                            ui.checkbox(posk, "Korrektur");
                        });
                        ui.horizontal(|ui| {
                            lbl(ui, 60.0, "Ebene");
                            combo(ui, "kebene", &strs(EBENEN), ebene, 60.0);
                        });
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Klemmenplan").font(bold(12.0)));
                        ui.checkbox(ziel_wechsel, "Ziel Wechsel");
                    });
                    field(ui, 60.0, "Funktion", funktion, 180.0);
                    // Bauteil (TIn Klemm_Bauteil, gesperrt) mit Knopf "Bearbeiten" (Klemm_Liste) wie beim Symbol
                    ui.horizontal(|ui| {
                        lbl(ui, 76.0, "Bauteil");
                        let mut bt = self.project.part_of.get(id).and_then(|i| self.project.bauteile.get(*i)).map(|b| b.name.clone()).unwrap_or_default();
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut bt).desired_width(150.0).background_color(egui::Color32::from_rgb(255, 255, 225)));
                        let btn = egui::Button::image(crate::icons::image(crate::icons::Icon::EditBauteil, 20.0));
                        if ui.add(btn.min_size(Vec2::new(31.0, 24.0))).on_hover_text("Bearbeiten").clicked() {
                            part_edit = Some(*id);
                        }
                    });
                }
                Dlg3::Verweis { anlagen, anlage, blatt, link, zu, titel, .. } => {
                    let blaetter: Vec<String> = self.project.anlagen.get(*anlage).map(|a| a.sheets.iter().map(|s| format!("[{}] {}", s.number, s.name)).collect()).unwrap_or_default();
                    // Eintrag 0 = nur das Blatt, danach die Verweise des Zielblattes
                    let links: Vec<String> = std::iter::once("(nur Blatt)".to_string())
                        .chain(verweis_links(&self.project, SheetRef { anlage: *anlage, sheet: *blatt }).into_iter().map(|(_, t)| t))
                        .collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Anlage");
                        if combo(ui, "vwanl", anlagen, anlage, 170.0) {
                            *blatt = 0;
                            *link = 0;
                        }
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Blatt");
                        if combo(ui, "vwblt", &blaetter, blatt, 170.0) {
                            *link = 0;
                        }
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Verweislink");
                        if combo(ui, "vwlink", &links, link, 170.0) && *link > 0 {
                            if let Some(t) = links.get(*link) {
                                *titel = t.clone();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(92.0);
                        if ui.radio(*zu, "Zu").clicked() {
                            *zu = true;
                        }
                        if ui.radio(!*zu, "Von").clicked() {
                            *zu = false;
                        }
                    });
                    field(ui, 40.0, "Titel", titel, 200.0);
                }
                Dlg3::Bild { grafiken, grafik, art, oben, links, hoehe, breite, rahmen, fixiert, .. } => {
                    let mut neu = false;
                    ui.horizontal(|ui| {
                        lbl(ui, 80.0, "Grafik");
                        combo(ui, "bgraf", grafiken, grafik, 140.0);
                        neu = ui.button("Neu").clicked();
                    });
                    if neu {
                        if let Some(p) = rfd::FileDialog::new().add_filter("Bilder", &["png", "jpg", "jpeg", "bmp"]).pick_file() {
                            grafiken.push(p.to_string_lossy().to_string());
                            *grafik = grafiken.len() - 1;
                        }
                    }
                    ui.horizontal(|ui| {
                        lbl(ui, 80.0, "Ausrichtung");
                        combo(ui, "bart", &strs(BILD_ARTEN), art, 140.0);
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Bildabmessung").font(bold(12.0)));
                        ui.horizontal(|ui| {
                            ui.label("Oben");
                            ui.add(egui::TextEdit::singleline(oben).desired_width(60.0));
                            ui.label("Höhe");
                            ui.add(egui::TextEdit::singleline(hoehe).desired_width(60.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Links");
                            ui.add(egui::TextEdit::singleline(links).desired_width(60.0));
                            ui.label("Breite");
                            ui.add(egui::TextEdit::singleline(breite).desired_width(60.0));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(rahmen, "Rahmen");
                        ui.checkbox(fixiert, "Fixieren");
                    });
                }
                Dlg3::KLeiste { anlage, name, titel, x, y, anzahl, ebenen, nrs, .. } => {
                    ui.horizontal(|ui| {
                        lbl(ui, 52.0, "Anlage");
                        ui.label(self.project.anlagen.get(*anlage).map(|a| a.name.clone()).unwrap_or_default());
                    });
                    field(ui, 52.0, "Name", name, 200.0);
                    ui.horizontal(|ui| {
                        lbl(ui, 52.0, "Titel");
                        ui.add(egui::TextEdit::singleline(titel).desired_width(110.0));
                        ui.label("X");
                        ui.add(egui::TextEdit::singleline(x).desired_width(36.0));
                        ui.label("Y");
                        ui.add(egui::TextEdit::singleline(y).desired_width(36.0));
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 52.0, "Anzahl");
                        let r = ui.add(egui::TextEdit::singleline(anzahl).desired_width(50.0));
                        if r.lost_focus() {
                            let n = anzahl.trim().parse::<usize>().unwrap_or(nrs.len()).min(999);
                            while nrs.len() < n {
                                let next = nrs.iter().filter_map(|v| v.parse::<u32>().ok()).max().unwrap_or(0) + 1;
                                nrs.push(next.to_string());
                            }
                            nrs.truncate(n);
                        }
                        lbl(ui, 60.0, "Ebenen");
                        combo(ui, "kleb", &strs(EBENEN), ebenen, 50.0);
                    });
                    ui.label(RichText::new("Indexübersicht").font(bold(12.0)));
                    let mut rows: Vec<Vec<String>> = nrs.iter().map(|n| vec![n.clone()]).collect();
                    let mut sel = 0;
                    icon_tab(ui, "kidx", &["Nr"], &mut rows, &mut sel, 200.0);
                    *nrs = rows.into_iter().map(|mut r| r.remove(0)).collect();
                }
                Dlg3::Pfad { rows, grenze, .. } => {
                    let mut grid: Vec<Vec<String>> = rows.iter().map(|r| vec![r.clone()]).collect();
                    let mut sel = 0;
                    icon_tab(ui, "pfad", &["Bezeichnung"], &mut grid, &mut sel, 290.0);
                    *rows = grid.into_iter().map(|mut r| r.remove(0)).collect();
                    ui.checkbox(grenze, "Begrenzungslinien");
                }
                Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, msg } => {
                    ui.label(RichText::new("Erzeugungsformel").font(bold(12.0)));
                    ui.add(egui::TextEdit::multiline(formel).desired_rows(2).desired_width(280.0));
                    ui.horizontal(|ui| {
                        lbl(ui, 110.0, "Reihenfolge 1");
                        combo(ui, "dir1", &strs(DIRS), dir1, 150.0);
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 110.0, "Reihenfolge 2");
                        combo(ui, "dir2", &strs(DIRS), dir2, 150.0);
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Textauswahl bei Symbolen").font(bold(12.0)));
                        ui.horizontal(|ui| {
                            for (i, t) in ["Titel", "Wert", "Alternativtext"].iter().enumerate() {
                                if ui.radio(*wahl == i, *t).clicked() {
                                    *wahl = i;
                                }
                            }
                        });
                    });
                    if *hilfe {
                        ui.small("Der eingegebene Text wird jedem markierten Objekt zugewiesen. Variablen beginnen mit '@': @x laufende Nummer, @S Blatt, @P Strompfad, @A Anlage, @T bisheriger Text.");
                    }
                    if !msg.is_empty() {
                        ui.small(msg.as_str());
                    }
                }
                Dlg3::Schriftfeld { absolut, hoehe } => {
                    ui.group(|ui| {
                        ui.label(RichText::new("Maßangaben").font(bold(12.0)));
                        if ui.radio(!*absolut, "Relativ").clicked() {
                            *absolut = false;
                        }
                        if ui.radio(*absolut, "Absolut").clicked() {
                            *absolut = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 40.0, "Höhe");
                        ui.add(egui::TextEdit::singleline(hoehe).desired_width(60.0));
                        ui.label("mm");
                    });
                }
                Dlg3::NeuesSchriftfeld { anlage, aus_datei, pfad } => {
                    let mut items = vec!["Alle Anlagen".to_string()];
                    items.extend(self.project.anlagen.iter().map(|a| a.name.clone()));
                    ui.horizontal(|ui| {
                        lbl(ui, 40.0, "Für");
                        combo(ui, "sfanl", &items, anlage, 190.0);
                    });
                    if ui.radio(!*aus_datei, "Vorgabeschriftfeld").clicked() {
                        *aus_datei = false;
                    }
                    if ui.radio(*aus_datei, "Aus Datei").clicked() {
                        *aus_datei = true;
                    }
                    ui.add_enabled(*aus_datei, egui::TextEdit::singleline(pfad).desired_width(230.0));
                    if ui.add_enabled(*aus_datei, egui::Button::new("Suchen")).clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("Projekt", &["epj", "epr"]).pick_file() {
                            *pfad = p.display().to_string();
                        }
                    }
                }
                Dlg3::SfWahl { quellen, wahl } => {
                    let items: Vec<String> = quellen.iter().map(|q| q.0.clone()).collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 110.0, "Schriftfeld von");
                        combo(ui, "sfw", &items, wahl, 160.0);
                    });
                }
                Dlg3::PExport { liste, sel, status } => {
                    ui.label(RichText::new("Projektliste").font(bold(12.0)));
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for (i, p) in liste.iter().enumerate() {
                            if ui.selectable_label(*sel == Some(i), p).clicked() {
                                *sel = Some(i);
                            }
                        }
                    });
                    let mut act = 0;
                    ui.horizontal(|ui| {
                        if ui.button("Hinzufügen").clicked() {
                            act = 1;
                        }
                        if ui.button("Entfernen").clicked() {
                            act = 2;
                        }
                        if ui.button("Start").clicked() {
                            act = 3;
                        }
                    });
                    if !status.is_empty() {
                        ui.small(status.as_str());
                    }
                    match act {
                        1 => {
                            if let Some(ps) = rfd::FileDialog::new().add_filter("Projekt", &["epj", "epr"]).pick_files() {
                                liste.extend(ps.into_iter().map(|p| p.display().to_string()));
                            }
                        }
                        2 => {
                            if let Some(i) = sel.take() {
                                if i < liste.len() {
                                    liste.remove(i);
                                }
                            }
                        }
                        3 => *status = self.batch_pdf(liste),
                        _ => {}
                    }
                }
                Dlg3::BlattIndex { anlage, von, bis, ab, plus } => {
                    let items: Vec<String> = self.project.anlagen.iter().map(|a| a.name.clone()).collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 80.0, "Anlage");
                        combo(ui, "bianl", &items, anlage, 200.0);
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Blattauswahl").font(bold(12.0)));
                        ui.horizontal(|ui| {
                            lbl(ui, 40.0, "Von");
                            ui.add(egui::TextEdit::singleline(von).desired_width(50.0));
                            lbl(ui, 60.0, "Bis");
                            ui.add(egui::TextEdit::singleline(bis).desired_width(50.0));
                        });
                    });
                    ui.group(|ui| {
                        ui.label(RichText::new("Neue Nummerierung").font(bold(12.0)));
                        ui.horizontal(|ui| {
                            lbl(ui, 40.0, "Von");
                            ui.add(egui::TextEdit::singleline(ab).desired_width(50.0));
                            lbl(ui, 60.0, "Abstand");
                            ui.add(egui::TextEdit::singleline(plus).desired_width(50.0));
                        });
                    });
                }
                Dlg3::Wahl { anlagen, .. } => {
                    ui.label(RichText::new("Welche Anlagen sollen hinzugefügt werden").font(bold(12.0)));
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (n, on) in anlagen.iter_mut() {
                            ui.checkbox(on, n.as_str());
                        }
                    });
                    if ui.button("Alle/Keine").clicked() {
                        let all = anlagen.iter().all(|a| a.1);
                        anlagen.iter_mut().for_each(|a| a.1 = !all);
                    }
                }
                Dlg3::UserDat { breite, hoehe, .. } => {
                    for (t, v) in [("Breite", breite), ("Hoehe", hoehe)] {
                        ui.horizontal(|ui| {
                            lbl(ui, 50.0, t);
                            ui.add(egui::TextEdit::singleline(v).desired_width(60.0));
                            ui.label("mm");
                        });
                    }
                }
                Dlg3::TextChange { text, .. } => field(ui, 40.0, "Text", text, 220.0),
                Dlg3::Errliste { lines } => {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for l in lines.iter() {
                            ui.label(l);
                        }
                    });
                }
                Dlg3::ListExport { art, format, msg } => {
                    if let Some(m) = crate::listen::ui(ui, &self.project, art, format) {
                        *msg = m;
                    }
                    if !msg.is_empty() {
                        ui.small(msg.as_str());
                    }
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let is_titel = matches!(d, Dlg3::Titel { .. });
                let only_ok = matches!(d, Dlg3::Errliste { .. } | Dlg3::PExport { .. } | Dlg3::ListExport { .. });
                if !only_ok && ui.add_sized([90.0, 24.0], egui::Button::new(if is_titel { "Schließen" } else { "Abbruch" })).clicked() {
                    done = Some(false);
                }
                if is_titel {
                    if ui.add_sized([60.0, 24.0], egui::Button::new("Hilfe")).clicked() {
                        if let Dlg3::Titel { hilfe, .. } = &mut d {
                            *hilfe = !*hilfe;
                        }
                    }
                    if ui.add_sized([90.0, 24.0], egui::Button::new("Übernehmen")).clicked() {
                        done = Some(true);
                    }
                } else if ui.add_sized([90.0, 24.0], egui::Button::new(if matches!(d, Dlg3::ListExport { .. }) { "Beenden" } else if only_ok { "Schließen" } else { "OK" })).clicked() {
                    done = Some(!only_ok);
                }
            });
        });
        if closed {
            done = Some(false);
        }
        // Sammelschienen-Objekt › Bearbeiten: Objekt übernehmen, dann die Schiene bearbeiten
        if let Some(obj) = part_edit {
            let idx = self.project.part_of.get(&obj).copied().unwrap_or(0);
            self.xdlg = Some(crate::extra::XDialog::PartAssign { obj, idx });
        }
        if let Some(name) = rail_edit {
            if let Some(r) = self.cur() {
                let _ = self.apply_dlg3(d);
                self.open_schiene_dialog(r.anlage, &name);
                return;
            }
        }
        match done {
            Some(true) => {
                // Titelgenerator bleibt offen (Übernehmen), alle anderen schließen
                if let Some(keep) = self.apply_dlg3(d) {
                    self.dlg3 = Some(keep);
                }
            }
            Some(false) => {}
            None => self.dlg3 = Some(d),
        }
    }

    /// PDFs mehrerer Projekte erzeugen (neben der Projektdatei, Endung .pdf). Rückgabe: Statuszeile.
    fn batch_pdf(&mut self, liste: &[String]) -> String {
        let mut ok = 0;
        let mut errs = vec![];
        for p in liste {
            let is_cur = self.path.as_ref().map(|c| c.display().to_string() == *p).unwrap_or(false);
            let (prj, out) = if p.starts_with('(') {
                // ungespeichertes aktuelles Projekt: Zieldatei erfragen
                let Some(out) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("Projekt.pdf").save_file() else { continue };
                (self.project.clone(), out)
            } else if is_cur {
                // aktuelles Projekt mit seinem Bearbeitungsstand, PDF neben der Projektdatei
                (self.project.clone(), PathBuf::from(p).with_extension("pdf"))
            } else {
                let path = PathBuf::from(p);
                match crate::mcp::open_project(&path) {
                    Ok(x) => (x, path.with_extension("pdf")),
                    Err(e) => {
                        errs.push(format!("{p}: {e}"));
                        continue;
                    }
                }
            };
            let mut lib = self.lib.clone();
            lib.extend(prj.symdefs.iter().filter(|d| symbols::find(&self.lib, &d.key).is_none()).cloned());
            let (bytes, _) = crate::pdf::render_headless(&prj, &lib);
            match std::fs::write(&out, bytes) {
                Ok(()) => ok += 1,
                Err(e) => errs.push(format!("{}: {e}", out.display())),
            }
        }
        if errs.is_empty() {
            format!("{ok} PDF-Datei(en) erzeugt")
        } else {
            format!("{ok} erzeugt, Fehler: {}", errs.join("; "))
        }
    }

    /// Dialog übernehmen; Rückgabe: Dialog, der offen bleiben soll.
    fn apply_dlg3(&mut self, d: Dlg3) -> Option<Dlg3> {
        self.checkpoint();
        self.dirty = true;
        let r = self.cur();
        match d {
            Dlg3::BusbarObj { id, sliste, sel, einsp, auto, unten, offset, tx, ty, px, py, laenge } => {
                let name = sliste.get(sel).cloned().unwrap_or_default();
                if let Some(r) = r {
                    self.project.anlagen[r.anlage].schienen.entry(name.clone()).or_default().einspeisung = einsp;
                }
                let off = num(&offset).unwrap_or(0.0);
                let abstand = self.opt.sms_abstand;
                let schriftfeld = self.project.title_h();
                let pfad = r.and_then(|r| self.project.anlagen.get(r.anlage)).map(|a| a.set.pfad_hoehe as f32 / 10.0).unwrap_or(COLUMN_BAR_H);
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Busbar { a, x2, name: n, label, lage, versatz, .. } = &mut o.kind {
                            *n = name.clone();
                            *label = P::new(num(&tx).unwrap_or(label.x), num(&ty).unwrap_or(label.y));
                            a.x = num(&px).unwrap_or(a.x);
                            a.y = num(&py).unwrap_or(a.y);
                            *x2 = a.x + num(&laenge).unwrap_or(*x2 - a.x).max(1.0);
                            // Positionierung wie im Original: Auto = Stapel des Blattes (sammelschiene_lage.rs)
                            *lage = (*lage & !3) | if auto { 0 } else { crate::sammelschiene_lage::FEST } | if unten { crate::sammelschiene_lage::UNTEN } else { 0 };
                            *versatz = off;
                        }
                    }
                    if auto {
                        let (oben, unten) = crate::sammelschiene_lage::stapel_anfang(s, pfad, schriftfeld);
                        let lagen = crate::sammelschiene_lage::stapeln(s, abstand, oben, unten);
                        crate::sammelschiene_lage::anwenden(s, &lagen);
                    }
                });
                crate::epr::update_busbar_xrefs(&mut self.project);
            }
            Dlg3::Schiene { anlage, orig, name, set, .. } => {
                if let Some(a) = self.project.anlagen.get_mut(anlage) {
                    a.schienen.remove(&orig);
                    a.schienen.insert(name.clone(), set);
                    if orig != name {
                        for s in a.sammelschienen.iter_mut().filter(|s| **s == orig) {
                            *s = name.clone();
                        }
                        for sh in &mut a.sheets {
                            for o in &mut sh.objects {
                                if let Kind::Busbar { name: n, .. } = &mut o.kind {
                                    if *n == orig {
                                        *n = name.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Dlg3::Verbindung { id, leitung, leitungen, opts, text, .. } => {
                while self.project.leitungen.len() < leitungen.len() {
                    let n = self.project.leitungen.len();
                    self.project.leitungen.push(Leitung { name: leitungen[n].clone(), color: [0, 0, 0], width_mm: 0.25, quer: 1.5, adern: 3, kennz: 0, style: 0, id: 0 });
                }
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Wire { pts, leitung: l, bez, opts: op } = &mut o.kind {
                            *l = leitung;
                            *op = opts;
                            match (bez.first_mut(), text.is_empty()) {
                                (Some(b), false) => b.1 = text.clone(),
                                (Some(_), true) => {
                                    bez.remove(0);
                                }
                                (None, false) => {
                                    // Bezeichnung in die Mitte des längsten Abschnitts
                                    let seg = pts.windows(2).max_by(|a, b| a[0].dist(a[1]).partial_cmp(&b[0].dist(b[1])).unwrap());
                                    if let Some(w) = seg {
                                        bez.push((P::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0), text.clone()));
                                    }
                                }
                                (None, true) => {}
                            }
                        }
                    }
                });
            }
            Dlg3::Tab(t) => {
                // Zeilen, Spalten, Zellen, Überschrift, Autobreite, Daten (ZCN Feld 9 Byte 0–2) und „Ab“ (Feld 6)
                let (nr, nc, zellen, kopf, fest, daten, ab_wert) = t.werte();
                let fixiert = t.fixiert;
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == t.id) {
                        o.fixed = fixiert;
                        if let Kind::Table { rows, cols, cells, header, fixed_w, art, ab, .. } = &mut o.kind {
                            (*rows, *cols, *cells, *header, *fixed_w, *art, *ab) = (nr, nc, zellen, kopf, fest, daten, ab_wert);
                        }
                    }
                });
            }
            Dlg3::Klemme { id, leisten, leiste, titel, gr_auto, align, tx, ty, nummer, auto, korr, position, posk, ebene, funktion, ziel_wechsel } => {
                let name = leisten.get(leiste).cloned().unwrap_or_default();
                let mehr = self.cur().and_then(|r| self.project.anlagen.get(r.anlage)).and_then(|a| a.klemmen.iter().find(|k| k.name == name)).map(|k| k.etagen > 1).unwrap_or(false);
                let ein = crate::klemmen::Eingabe { leiste: name, nummer, auto, korr, position, posk, ebene: if mehr || ebene > 0 { ebene as u8 + 1 } else { 0 } };
                crate::klemmen::apply_index(&mut self.project, id, &ein);
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Terminal { text_off, tpl, opt, .. } = &mut o.kind {
                            *text_off = Some(P::new(num(&tx).unwrap_or(-1.0), num(&ty).unwrap_or(-2.0)));
                            *tpl = titel.clone();
                            opt.gr_auto = gr_auto;
                            opt.align = align.min(2) as u8;
                            opt.funktion = funktion.trim().to_string();
                            opt.ziel_wechsel = ziel_wechsel;
                        }
                    }
                });
            }
            Dlg3::Verweis { id, anlage, blatt, link, zu, titel, .. } => {
                // Ziel setzen; mit Verweislink wird der Gegenverweis wie im Original zurückverknüpft
                let ziel_ref = SheetRef { anlage, sheet: blatt };
                let gegen = link.checked_sub(1).and_then(|i| verweis_links(&self.project, ziel_ref).get(i).map(|(g, _)| *g));
                if self.project.sheet(ziel_ref).is_some() {
                    let msg = crate::verweise::verknuepfen(&mut self.project, id, ziel_ref, gegen);
                    if let Some(m) = msg {
                        self.info = m;
                    }
                }
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Verweis { text, dir, .. } = &mut o.kind {
                            *text = titel.clone();
                            let is_zu = *dir == 0 || *dir == 270;
                            if is_zu != zu {
                                *dir = (*dir + 180) % 360;
                            }
                        }
                    }
                });
            }
            Dlg3::Bild { id, grafiken, grafik, art, oben, links, hoehe, breite, rahmen, fixiert } => {
                let path = grafiken.get(grafik).cloned().unwrap_or_default();
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        o.fixed = fixiert;
                        if let Kind::Image { a, b, path: pa, mode, frame } = &mut o.kind {
                            let (l, t) = (num(&links).unwrap_or(a.x), num(&oben).unwrap_or(a.y));
                            *a = P::new(l, t);
                            *b = P::new(l + num(&breite).unwrap_or(10.0).max(1.0), t + num(&hoehe).unwrap_or(10.0).max(1.0));
                            *pa = path.clone();
                            *mode = match art {
                                0 => 1,
                                1 => 2,
                                _ => 0,
                            };
                            *frame = rahmen;
                        }
                    }
                });
            }
            Dlg3::KLeiste { anlage, idx, name: intern, titel, x, y, nrs, ebenen, .. } => {
                // Titel = Name der Leiste in den Klemmstellen (%O); ohne Titel gilt der Name
                let name = if titel.trim().is_empty() { intern.trim().to_string() } else { titel.trim().to_string() };
                crate::klemmen::ensure_ids(&mut self.project);
                if let Some(a) = self.project.anlagen.get_mut(anlage) {
                    let old = a.klemmleisten.get(idx).cloned().unwrap_or_default();
                    if let Some(k) = a.klemmleisten.get_mut(idx) {
                        *k = name.clone();
                    }
                    let n: Vec<u32> = nrs.iter().filter_map(|n| n.trim().parse().ok()).collect();
                    let off = ((num(&x).unwrap_or(0.0) * 10.0).round() as i32, (num(&y).unwrap_or(-5.0) * 10.0).round() as i32);
                    if let Some(k) = a.klemmen.iter_mut().find(|k| k.name == old) {
                        k.name = name.clone();
                        k.intern = intern.trim().to_string();
                        k.nrs = n;
                        k.etagen = ebenen as i32 + 1;
                        k.titel_off = off;
                    }
                    for sh in &mut a.sheets {
                        for o in &mut sh.objects {
                            if let Kind::Terminal { leiste, level, .. } = &mut o.kind {
                                if *leiste == old {
                                    *leiste = name.clone();
                                    if ebenen == 0 {
                                        *level = 0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Dlg3::Pfad { r, rows, grenze } => {
                if let Some(s) = self.project.sheet_mut(r) {
                    let def: Vec<String> = (0..rows.len()).map(|i| (i + 1).to_string()).collect();
                    s.col_labels = if rows == def { vec![] } else { rows };
                    s.pfad_linien = grenze;
                }
            }
            Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, .. } => {
                let n = self.apply_titel(&formel, dir1, dir2, wahl);
                return Some(Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, msg: format!("{n} Objekt(e) beschriftet") });
            }
            Dlg3::Schriftfeld { absolut, hoehe } => {
                // Das Modell führt das Schriftfeld relativ; beim Umschalten rechnet das .epr-Schreiben um wie das
                // Original. Bleibt es absolut und ändert sich die Höhe, behalten die Elemente wie im Original ihre
                // Lage in mm (relativ umgerechnet).
                let alt_h = (self.project.title_h() * 10.0).round() as i32;
                let war_absolut = self.project.tb_absolut;
                self.project.tb_absolut = absolut;
                self.project.tb_hoehe = num(&hoehe).unwrap_or(TITLE_H).clamp(5.0, 60.0);
                let neu_h = (self.project.title_h() * 10.0).round() as i32;
                if war_absolut && absolut && alt_h != neu_h {
                    let tb = if self.project.titleblock.is_empty() { crate::titleblock::default_owned() } else { self.project.titleblock.clone() };
                    self.project.titleblock = crate::titleblock::nach_relativ(&crate::titleblock::nach_absolut(&tb, alt_h), neu_h);
                }
            }
            Dlg3::NeuesSchriftfeld { aus_datei, pfad, .. } => {
                if aus_datei {
                    match crate::mcp::open_project(std::path::Path::new(pfad.trim())) {
                        Ok(x) => {
                            let mut quellen = vec![];
                            if !x.titleblock.is_empty() {
                                quellen.push((format!("Projekt {}", x.name), x.titleblock.clone()));
                            }
                            quellen.push(("Standardschriftfeld".into(), crate::titleblock::default_owned()));
                            return Some(Dlg3::SfWahl { quellen, wahl: 0 });
                        }
                        Err(e) => self.dialog = Some(Dialog::Message(format!("Schriftfeld konnte nicht gelesen werden: {e}"))),
                    }
                } else {
                    self.project.titleblock = crate::extra::default_titleblock();
                }
            }
            Dlg3::SfWahl { quellen, wahl } => {
                if let Some((_, tb)) = quellen.into_iter().nth(wahl) {
                    self.project.titleblock = tb;
                }
            }
            Dlg3::PExport { .. } | Dlg3::Errliste { .. } | Dlg3::ListExport { .. } => {}
            Dlg3::BlattIndex { anlage, von, bis, ab, plus } => {
                let (v, b) = (von.trim().parse::<usize>().unwrap_or(1).max(1), bis.trim().parse::<usize>().unwrap_or(usize::MAX));
                let (start, step) = (ab.trim().parse::<u32>().unwrap_or(1).max(1), plus.trim().parse::<u32>().unwrap_or(1).max(1));
                if let Some(a) = self.project.anlagen.get_mut(anlage) {
                    for (k, s) in a.sheets.iter_mut().enumerate().filter(|(k, _)| *k + 1 >= v && *k + 1 <= b) {
                        s.number = start + (k + 1 - v) as u32 * step;
                    }
                }
            }
            Dlg3::Wahl { path, anlagen } => {
                if let Ok(mut prj) = crate::mcp::open_project(&path) {
                    let keep: Vec<bool> = anlagen.iter().map(|a| a.1).collect();
                    let mut i = 0;
                    prj.anlagen.retain(|_| {
                        i += 1;
                        keep[i - 1]
                    });
                    self.add_project(prj);
                }
            }
            Dlg3::UserDat { anlage, breite, hoehe } => {
                if let Some(a) = self.project.anlagen.get_mut(anlage) {
                    a.set.format = ((num(&breite).unwrap_or(210.0) * 10.0) as i32, (num(&hoehe).unwrap_or(297.0) * 10.0) as i32);
                }
            }
            Dlg3::TextChange { ids, text, wert } => {
                self.with_sheet(|s, _| {
                    for o in s.objects.iter_mut().filter(|o| ids.contains(&o.id)) {
                        match &mut o.kind {
                            Kind::Text { text: t, .. } => *t = text.clone(),
                            Kind::Symbol { value, .. } if wert == Some(true) => *value = text.clone(),
                            Kind::Symbol { bmk, .. } => *bmk = text.clone(),
                            _ => {}
                        }
                    }
                });
            }
        }
        None
    }

    /// Titelgenerator auf die markierten Objekte (sonst alle Symbole des Blattes) anwenden; Rückgabe: Anzahl.
    pub fn apply_titel(&mut self, formel: &str, dir1: usize, dir2: usize, wahl: usize) -> usize {
        let Some(r) = self.cur() else { return 0 };
        let sel = self.selection.clone();
        let kurz = self.project.anlagen[r.anlage].set.kurz.clone();
        let Some(s) = self.project.sheet_mut(r) else { return 0 };
        let number = s.number;
        let mut idx: Vec<usize> = (0..s.objects.len())
            .filter(|i| (sel.is_empty() || sel.contains(&s.objects[*i].id)) && matches!(s.objects[*i].kind, Kind::Symbol { .. } | Kind::Text { .. }))
            .collect();
        let pos = |o: &Obj| match &o.kind {
            Kind::Symbol { pos, .. } | Kind::Text { pos, .. } => *pos,
            _ => P::default(),
        };
        idx.sort_by(|a, b| {
            let (pa, pb) = (pos(&s.objects[*a]), pos(&s.objects[*b]));
            order_key(pa, dir1).partial_cmp(&order_key(pb, dir1)).unwrap().then(order_key(pa, dir2).partial_cmp(&order_key(pb, dir2)).unwrap())
        });
        let pfade: Vec<u32> = idx.iter().map(|i| crate::epr::column_at(s, pos(&s.objects[*i]).x)).collect();
        for (n, (i, pf)) in idx.iter().zip(pfade).enumerate() {
            match &mut s.objects[*i].kind {
                Kind::Symbol { bmk, value, ext, .. } => match wahl {
                    1 => *value = titel_formel(formel, n + 1, number, pf, &kurz, value),
                    // Alternativtext: benutzerdefinierter Text Nr. 2 der Instanz (erster Symboltext nach der BMK)
                    2 => {
                        let old = ext.texts.iter().find(|(k, _)| *k == 2).map(|(_, t)| t.clone()).unwrap_or_default();
                        let t = titel_formel(formel, n + 1, number, pf, &kurz, &old);
                        ext.texts.retain(|(k, _)| *k != 2);
                        ext.texts.push((2, t));
                    }
                    _ => *bmk = titel_formel(formel, n + 1, number, pf, &kurz, bmk),
                },
                Kind::Text { text, .. } => *text = titel_formel(formel, n + 1, number, pf, &kurz, text),
                _ => {}
            }
        }
        idx.len()
    }
}
