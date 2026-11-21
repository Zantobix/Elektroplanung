//! Eigenschaftendialoge wie im Original (Formular TObjekt_Eigenschaften mit den Teilformularen
//! TSymbol, TTextbox und TSBlatt; Aufbau aus der Formularressource und Bildschirmfotos des Originals).
//! Feste Größe 289 × 405, Beschriftungen fett, unten "Abbruch" und "OK".

use crate::app::App;
use crate::model::*;
use crate::symbols;
use eframe::egui::{self, Color32, RichText, Vec2};

pub struct SymProps {
    pub id: u64,
    pub name: String,
    pub bmk: String,
    pub tx: String,
    pub ty: String,
    pub value: String,
    pub vx: String,
    pub vy: String,
    pub title_align: u8,
    pub drehung: u8,
    /// "Spiegeln Horizontal" im Original (VSY Feld 18 Bit 16)
    pub sp_h: bool,
    /// "Spiegeln Vertikal" im Original (VSY Feld 18 Bit 8)
    pub sp_v: bool,
    pub scale: String,
    pub alt: u16,
    pub alt_items: Vec<String>,
    pub bild: u16,
    pub bild_items: Vec<String>,
    pub rack: String,
    /// Zähler für %C (VSY Feld 38); nur mit „Korrektur“ änderbar (Symbol1LO_IndexkorrekturClick:
    /// LO_Counter.Enabled = LO_Indexkorrektur.Checked)
    pub index: String,
    pub korrektur: bool,
    pub funktion: String,
    pub bauteil: String,
    pub querverweise: bool,
    pub sicherung: bool,
    initial_title: (String, String),
    /// Knopf "Bearbeiten" neben Bauteil gedrückt → nach OK Bauteilzuordnung öffnen
    pub edit_part: bool,
}

pub struct TextProps {
    pub id: u64,
    pub oben: String,
    pub links: String,
    pub hoehe: String,
    pub breite: String,
    pub rahmen: bool,
    /// Zeilenumbruch an der Boxbreite (ZCN Byte 0x19)
    pub umbruch: bool,
    pub fixiert: bool,
    pub align: Align,
    pub drehung: String,
    pub text: String,
    pub pt: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: [u8; 3],
    pub font: String,
    pub font_dlg: Option<FontDlg>,
}

#[derive(Clone)]
pub struct FontDlg {
    pub font: String,
    pub schnitt: usize,
    pub pt: String,
    pub underline: bool,
}

pub struct SheetProps {
    pub r: SheetRef,
    pub name: String,
    pub ort: String,
    pub nummer: String,
    pub subindex: String,
    pub strompfade: String,
    pub portrait: bool,
    /// Blattart (Index in ARTEN = BLT Feld 22 Byte 0) und Zusatz (Klemmplan: 0 = Keine Zuordnung, sonst Leiste)
    pub art: usize,
    pub zusatz: usize,
    /// Klemmleisten der Anlage (Kennung, Titel) für den Zusatz beim Klemmplan
    pub leisten: Vec<(i32, String)>,
}

pub struct AnlProps {
    pub idx: usize,
    pub name: String,
    pub s: AnlSettings,
    /// Texteingaben (Zahlen in mm wie im Original-Dialog)
    pub t: [String; 11],
}

/// Symboleigenschaften im Symboleditor (TSymEigenschaften: Name, Gruppe, Freigaben; Reiter Seiten/Optionen,
/// Stifte/Parameter, Alternativtexte).
pub struct SymEigenProps {
    pub name: String,
    pub group: String,
    pub groups: Vec<String>,
    pub quer: bool,
    pub fuse: bool,
    pub tab: u8,
    pub seiten: Vec<String>,
    pub sel_seite: usize,
    pub optionen: Vec<String>,
    pub sel_option: usize,
    /// eigene Stifte des Symbols (Nummer, Stift), gewählte Zeile und gelöschte Nummern (in Löschreihenfolge)
    pub stifte: Vec<(u32, Stift)>,
    pub sel_stift: usize,
    pub stift_geloescht: Vec<u32>,
    /// Textnummern der Textelemente (Spalten der Alternativtext-Tabelle) und je Satz die Texte
    pub text_cols: Vec<u32>,
    pub alt: Vec<Vec<String>>,
    /// Parameter (GRD › PRA): Nummer, Name, Vorgabe – Reiter „Stifte / Parameter“
    pub params: Vec<(u32, String, String)>,
    pub sel_param: usize,
}

pub enum Props {
    SymEigen(SymEigenProps),
    Symbol(SymProps),
    Text(TextProps),
    Sheet(SheetProps),
    Anlage(AnlProps),
}

fn mm1(v: i32) -> String {
    format!("{:.1}", v as f32 / 10.0).replace('.', ",")
}
fn d1(s: &str, old: i32) -> i32 {
    num(s).map(|v| (v * 10.0).round() as i32).unwrap_or(old)
}

/// Blattformate der Auswahl "Blattformat" (Breite × Höhe in 0,1 mm, Hochformat)
const FORMATE: &[(&str, (i32, i32))] = &[("A4", (2100, 2970)), ("A3", (2970, 4200)), ("A2", (4200, 5940)), ("A1", (5940, 8410))];

const SCHRIFTEN: &[&str] = &["Arial", "Calibri", "Courier New", "Segoe UI", "Tahoma", "Times New Roman", "Verdana"];
const SCHNITTE: &[&str] = &["Standard", "Kursiv", "Fett", "Fett Kursiv"];
const GRADE: &[u32] = &[8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];
const ARTEN: &[&str] = &["Deckblatt", "Inhaltsverzeichnis", "Querverweise", "Schaltplan", "Klemmenplan", "Sicherungsplan", "Artikelliste"];

fn f1(v: f32) -> String {
    format!("{v:.1}").replace('.', ",")
}
fn num(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse().ok()
}
fn d01(v: f32) -> String {
    format!("{}", (v * 10.0).round() as i32)
}

/// Fette Schrift der Beschriftungen (MS Sans Serif fett im Original).
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("bold".into()))
}

/// Beschriftung rechtsbündig, fett (wie die TLabel des Originals); wird nicht abgeschnitten.
fn label(ui: &mut egui::Ui, w: f32, t: &str, enabled: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
    let c = if enabled { Color32::BLACK } else { Color32::from_gray(150) };
    ui.painter().text(rect.right_center(), egui::Align2::RIGHT_CENTER, t, bold(13.0), c);
}

fn bt(t: &str) -> RichText {
    RichText::new(t).font(bold(13.0))
}

/// Fenster im Stil eines VCL-Werkzeugfensters (bsToolWindow): kleine Titelzeile links, Schließen-Kreuz rechts,
/// eckiger Rahmen, grauer Hintergrund. Rückgabe true, wenn das Kreuz geklickt wurde.
pub fn tool_window(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], order: egui::Order, add: impl FnOnce(&mut egui::Ui)) -> bool {
    let mut closed = false;
    egui::Window::new(title)
        .id(egui::Id::unique(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(order)
        .fixed_size([size[0], size[1] + 26.0])
        // verschiebbar wie im Original: erscheint mittig, lässt sich an Titelzeile und Hintergrund ziehen
        .movable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .frame(egui::Frame::new().fill(Color32::from_rgb(240, 240, 240)).stroke(egui::Stroke::new(1.0, Color32::from_gray(110))).shadow(egui::epaint::Shadow { offset: [2, 3], blur: 8, spread: 0, color: Color32::from_black_alpha(60) }))
        .show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(size[0], 26.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.text(r.left_center() + Vec2::new(7.0, 0.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(13.0), Color32::BLACK);
            let xr = egui::Rect::from_min_size(egui::pos2(r.right() - 30.0, r.top() + 2.0), Vec2::new(28.0, 22.0));
            let resp = ui.interact(xr, ui.scope_id().with("x"), egui::Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(xr, 0.0, Color32::from_rgb(232, 17, 35));
            }
            let xc = if resp.hovered() { Color32::WHITE } else { Color32::BLACK };
            let c = xr.center();
            for (a, b) in [(Vec2::new(-4.5, -4.5), Vec2::new(4.5, 4.5)), (Vec2::new(-4.5, 4.5), Vec2::new(4.5, -4.5))] {
                ui.painter().line_segment([c + a, c + b], egui::Stroke::new(1.2, xc));
            }
            if resp.clicked() {
                closed = true;
            }
            egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                ui.set_width(size[0] - 12.0);
                // Kontrollkästchen mit deutlichem Rand wie die Windows-Standardelemente
                let v = &mut ui.style_mut().visuals;
                v.widgets.inactive.bg_fill = Color32::WHITE;
                v.widgets.inactive.weak_bg_fill = Color32::from_rgb(225, 225, 225);
                v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_gray(90));
                v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(0, 120, 215));
                v.extreme_bg_color = Color32::WHITE;
                v.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
                v.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
                v.widgets.active.corner_radius = egui::CornerRadius::ZERO;
                add(ui);
            });
        });
    closed
}

fn edit(ui: &mut egui::Ui, s: &mut String, w: f32) -> egui::Response {
    ui.add(egui::TextEdit::singleline(s).desired_width(w).font(egui::FontId::proportional(12.0)))
}

impl App {
    /// Eigenschaften eines Objekts öffnen (Doppelklick / Kontextmenü "Eigenschaften").
    pub fn open_props(&mut self, id: u64) -> bool {
        let Some(o) = self.cur().and_then(|r| self.project.sheet(r)).and_then(|s| s.objects.iter().find(|o| o.id == id)).cloned() else { return false };
        match &o.kind {
            Kind::Symbol { key, bmk, value, title, rot, mirror_h, mirror_v, scale, option, ext, .. } => {
                let def = symbols::find(&self.lib, key);
                let (t, _) = title.unwrap_or((P::default(), false));
                let v = ext.value_off.unwrap_or(P::new(t.x, t.y + 2.8));
                let mut alt_items = vec!["Standard".to_string()];
                if let Some(d) = def {
                    alt_items.extend(d.alt_sets.iter().map(|s| symbols::alt_label(s)));
                }
                let bild_items = def.map(|d| d.options.clone()).filter(|o| !o.is_empty()).unwrap_or_else(|| vec!["Standard".into()]);
                let bauteil = self.project.part_of.get(&id).and_then(|i| self.project.bauteile.get(*i)).map(|b| b.name.clone()).unwrap_or_default();
                let (tx, ty) = (f1(t.x), f1(t.y));
                self.props = Some(Props::Symbol(SymProps {
                    id,
                    name: def.map(|d| d.name.clone()).unwrap_or_else(|| key.clone()),
                    bmk: bmk.clone(),
                    tx: tx.clone(),
                    ty: ty.clone(),
                    value: value.clone(),
                    vx: f1(v.x),
                    vy: f1(v.y),
                    title_align: ext.title_align,
                    drehung: *rot % 4,
                    sp_h: mirror_to_bits(*rot, *mirror_h, *mirror_v).0,
                    sp_v: mirror_to_bits(*rot, *mirror_h, *mirror_v).1,
                    scale: format!("{:.2}", scale).replace('.', ","),
                    alt: ext.alt,
                    alt_items,
                    bild: *option,
                    bild_items,
                    rack: if ext.rack >= 0 { ext.rack.to_string() } else { String::new() },
                    index: if ext.counter > 0 { ext.counter.to_string() } else { String::new() },
                    korrektur: false,
                    funktion: ext.funktion.clone(),
                    bauteil,
                    querverweise: ext.querverweise,
                    sicherung: ext.sicherung,
                    initial_title: (tx, ty),
                    edit_part: false,
                }));
                true
            }
            Kind::Text { pos, text, size, align, vertical, style } => {
                let (mn, mx) = style.rect.unwrap_or_else(|| {
                    let (a, b) = crate::canvas::obj_bounds(&o, &self.lib);
                    let _ = pos;
                    (a, b)
                });
                self.props = Some(Props::Text(TextProps {
                    id,
                    oben: d01(mn.y),
                    links: d01(mn.x),
                    hoehe: d01(mx.y - mn.y),
                    breite: d01(mx.x - mn.x),
                    rahmen: style.frame,
                    umbruch: style.wrap,
                    fixiert: o.fixed,
                    align: *align,
                    drehung: if style.rotation != 0 { style.rotation.to_string() } else if *vertical { "90".into() } else { String::new() },
                    text: text.clone(),
                    pt: (size / 0.3528).round().max(1.0) as u32,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    color: style.color,
                    font: if style.font.is_empty() { "Times New Roman".into() } else { style.font.clone() },
                    font_dlg: None,
                }));
                true
            }
            _ => false,
        }
    }

    /// Dialog "Symboleigenschaften" des Symboleditors öffnen (Knopf mit Werkzeug-Icon).
    pub fn open_sym_eigenschaften(&mut self) {
        let Some(e) = &self.sym_edit else { return };
        let mut groups: Vec<String> = self.lib.iter().map(|d| d.group.join(" / ")).collect();
        groups.sort();
        groups.dedup();
        let mut text_cols: Vec<u32> = e.text_idx.values().copied().filter(|i| *i > 1).collect();
        for set in &e.alt_sets {
            text_cols.extend(set.iter().map(|(i, _)| *i));
        }
        text_cols.sort();
        text_cols.dedup();
        let alt = e.alt_sets.iter().map(|set| text_cols.iter().map(|c| set.iter().find(|(i, _)| i == c).map(|(_, t)| t.clone()).unwrap_or_default()).collect()).collect();
        self.props = Some(Props::SymEigen(SymEigenProps {
            name: e.name.clone(),
            group: e.group.join(" / "),
            groups,
            quer: e.frg.0,
            fuse: e.frg.1,
            tab: 0,
            seiten: e.options.clone(),
            sel_seite: e.option as usize,
            optionen: e.flag_names.clone(),
            sel_option: 0,
            stifte: e.pens.clone(),
            sel_stift: 0,
            stift_geloescht: vec![],
            text_cols,
            alt,
            params: e.params.clone(),
            sel_param: 0,
        }));
    }

    /// Anlagendialog wie im Original (TSAnlage).
    pub fn open_anlage_props(&mut self, idx: usize) {
        let Some(an) = self.project.anlagen.get(idx) else { return };
        let s = an.set.clone();
        let t = [
            s.kurz.clone(),
            s.von.clone(),
            s.znr.clone(),
            s.pfade.to_string(),
            mm1(s.pfad_hoehe),
            s.size_titel.to_string(),
            s.size_wert.to_string(),
            s.size_verbg.to_string(),
            s.size_pfad.to_string(),
            String::new(),
            String::new(),
        ];
        self.props = Some(Props::Anlage(AnlProps { idx, name: an.name.clone(), s, t }));
    }

    pub fn open_sheet_props_dialog(&mut self, r: SheetRef) {
        // Klemmleisten mit Kennung, damit ein Klemmplan seine Leiste im .epr findet
        crate::klemmen::ensure_ids(&mut self.project);
        let leisten: Vec<(i32, String)> = self.project.anlagen.get(r.anlage).map(|a| a.klemmen.iter().map(|k| (k.id, k.name.clone())).collect()).unwrap_or_default();
        if let Some(s) = self.project.sheet(r) {
            self.props = Some(Props::Sheet(SheetProps {
                r,
                name: s.name.clone(),
                ort: s.ort.clone(),
                nummer: s.number.to_string(),
                subindex: s.subindex.to_string(),
                strompfade: if s.col_auto || s.columns == 0 { "-1".into() } else { s.columns.to_string() },
                portrait: s.portrait,
                // 0 heißt bei neuen Blättern „noch nicht festgelegt“ (Schaltplan), bei gelesenen Blättern Deckblatt
                art: if s.blattart() as usize >= ARTEN.len() { 3 } else { s.blattart() as usize },
                zusatz: match s.blattart() {
                    4 => leisten.iter().position(|l| l.0 == s.klm).map(|i| i + 1).unwrap_or(0),
                    // Zusatz der übrigen Blattarten (berechnet.rs): Querverweise speichern 1–3, die Liste zeigt 0–2
                    2 => s.zusatz.saturating_sub(1) as usize,
                    0 | 6 => s.zusatz as usize,
                    _ => 0,
                },
                leisten,
            }));
        }
    }

    /// Eigenschaftendialog zeichnen; OK übernimmt, Abbruch verwirft.
    pub fn props_ui(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.props.take() else { return };
        let mut result: Option<bool> = None;
        let anlagen: Vec<String> = self.project.anlagen.iter().map(|a| a.name.clone()).collect();
        let (title, size) = match &p {
            Props::SymEigen(_) => ("Symboleigenschaften", [692.0, 478.0]),
            _ => ("Eigenschaften", [289.0, 405.0]),
        };
        let closed = tool_window(ctx, "props", title, size, egui::Order::Middle, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 6.0);
                match &mut p {
                    Props::Symbol(s) => symbol_ui(ui, s),
                    Props::Text(t) => text_ui(ui, t),
                    Props::Sheet(s) => sheet_ui(ui, s, &anlagen),
                    Props::Anlage(a) => anlage_ui(ui, a),
                    Props::SymEigen(s) => symeigen_ui(ui, s),
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(30.0);
                    if ui.add_sized([96.0, 25.0], egui::Button::new("Abbruch")).clicked() {
                        result = Some(false);
                    }
                    ui.add_space(14.0);
                    if ui.add_sized([96.0, 25.0], egui::Button::new("OK")).clicked() {
                        result = Some(true);
                    }
                });
            });
        if closed {
            result = Some(false);
        }
        // Schriftdialog über dem Textdialog
        if let Props::Text(t) = &mut p {
            if let Some(fd) = t.font_dlg.as_mut() {
                let mut done: Option<bool> = None;
                font_window(ctx, fd, &mut done);
                if let Some(ok) = done {
                    if ok {
                        t.font = fd.font.clone();
                        t.bold = fd.schnitt >= 2;
                        t.italic = fd.schnitt % 2 == 1;
                        t.pt = fd.pt.trim().parse().unwrap_or(t.pt).clamp(1, 400);
                        t.underline = fd.underline;
                    }
                    t.font_dlg = None;
                }
            }
        }
        if let Props::Symbol(sp) = &mut p {
            if sp.edit_part {
                sp.edit_part = false;
                let (id, idx) = (sp.id, self.project.part_of.get(&sp.id).copied().unwrap_or(0));
                self.xdlg = Some(crate::extra::XDialog::PartAssign { obj: id, idx });
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            result = Some(false);
        }
        match result {
            Some(true) => self.apply_props(p),
            Some(false) => {}
            None => self.props = Some(p),
        }
    }

    fn apply_props(&mut self, p: Props) {
        self.checkpoint();
        match p {
            Props::Symbol(s) => {
                let title_changed = (s.tx.clone(), s.ty.clone()) != s.initial_title;
                self.with_sheet(|sh, _| {
                    if let Some(o) = sh.objects.iter_mut().find(|o| o.id == s.id) {
                        if let Kind::Symbol { bmk, value, title, rot, mirror_h, mirror_v, scale, option, ext, .. } = &mut o.kind {
                            *bmk = s.bmk.clone();
                            *value = s.value.clone();
                            if title.is_some() || title_changed {
                                let vert = title.map(|t| t.1).unwrap_or(false);
                                *title = Some((P::new(num(&s.tx).unwrap_or(0.0), num(&s.ty).unwrap_or(0.0)), vert));
                            }
                            if let (Some(x), Some(y)) = (num(&s.vx), num(&s.vy)) {
                                if title.is_some() {
                                    ext.value_off = Some(P::new(x, y));
                                }
                            }
                            ext.title_align = s.title_align;
                            ext.value_align = s.title_align.min(ext.value_align.max(s.title_align));
                            if *option != s.bild {
                                ext.bounds = None;
                            } else if let Some(b) = ext.bounds {
                                let (nh, nv) = mirror_from_bits(s.drehung % 4, s.sp_h, s.sp_v);
                                ext.bounds = Some(retransform_bounds(b, (*rot, *mirror_h, *mirror_v), (s.drehung % 4, nh, nv)));
                            }
                            *rot = s.drehung % 4;
                            let (nh, nv) = mirror_from_bits(s.drehung % 4, s.sp_h, s.sp_v);
                            *mirror_h = nh;
                            *mirror_v = nv;
                            *scale = num(&s.scale).filter(|v| *v > 0.01).unwrap_or(*scale);
                            ext.alt = s.alt;
                            *option = s.bild;
                            ext.rack = s.rack.trim().parse().unwrap_or(-1);
                            if s.korrektur {
                                ext.counter = s.index.trim().parse::<i32>().unwrap_or(0).max(0);
                            }
                            ext.funktion = s.funktion.clone();
                            ext.querverweise = s.querverweise;
                            ext.sicherung = s.sicherung;
                        }
                    }
                });
            }
            Props::Text(t) => {
                let (l, o, b, h) = (num(&t.links).unwrap_or(0.0) / 10.0, num(&t.oben).unwrap_or(0.0) / 10.0, num(&t.breite).unwrap_or(10.0) / 10.0, num(&t.hoehe).unwrap_or(5.0) / 10.0);
                let rotation: i16 = t.drehung.trim().parse().unwrap_or(0);
                let vertical = rotation.rem_euclid(360) == 90 || rotation.rem_euclid(360) == 270;
                let (mn, mx) = (P::new(l, o), P::new(l + b.max(0.1), o + h.max(0.1)));
                self.with_sheet(|sh, _| {
                    if let Some(ob) = sh.objects.iter_mut().find(|x| x.id == t.id) {
                        ob.fixed = t.fixiert;
                        if let Kind::Text { pos, text, size, align, vertical: v, style } = &mut ob.kind {
                            *text = t.text.trim_end_matches('\n').to_string();
                            *size = t.pt as f32 * 0.3528;
                            *align = t.align;
                            *v = vertical;
                            style.frame = t.rahmen;
                            style.wrap = t.umbruch;
                            style.bold = t.bold;
                            style.italic = t.italic;
                            style.underline = t.underline;
                            style.color = t.color;
                            style.rotation = rotation;
                            style.font = if t.font == "Times New Roman" { String::new() } else { t.font.clone() };
                            style.rect = Some((mn, mx));
                            *pos = crate::epr::text_anchor(mn, mx, t.align, vertical);
                        }
                    }
                });
            }
            Props::SymEigen(sp) => {
                if let Some(e) = &mut self.sym_edit {
                    e.name = sp.name.clone();
                    e.group = sp.group.split(" / ").map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                    e.options = if sp.seiten.is_empty() { vec!["Standard".into()] } else { sp.seiten.clone() };
                    if e.option as usize >= e.options.len() {
                        e.option = 0;
                    }
                    e.flag_names = sp.optionen.clone();
                    if e.cur_flag as usize > e.flag_names.len() {
                        e.cur_flag = 0;
                    }
                    let n = e.flag_names.len() as u32;
                    e.flags.retain(|_, f| *f <= n);
                    e.alt_sets = sp
                        .alt
                        .iter()
                        .map(|row| sp.text_cols.iter().zip(row).map(|(i, t)| (*i, t.clone())).collect())
                        .collect();
                    e.frg = (sp.quer, sp.fuse);
                    e.params = crate::parameter::clean_params(&sp.params);
                    // Stifte: gelöschte Nummern nacheinander aus den Elementen nehmen, dann die bearbeitete Liste
                    for nr in &sp.stift_geloescht {
                        let mut pens = std::mem::take(&mut e.pens);
                        let mut el: Vec<(u64, &mut usize)> = e.sheet.objects.iter_mut().map(|o| (o.id, &mut o.pen)).collect();
                        crate::symbol_stifte::loeschen(&mut pens, *nr, &mut el);
                        e.pens = pens;
                    }
                    e.pens = sp.stifte.clone();
                }
                self.sym_frg_default = (sp.quer, sp.fuse);
            }
            Props::Anlage(a) => {
                let t = &a.t;
                if let Some(an) = self.project.anlagen.get_mut(a.idx) {
                    let mut s = a.s.clone();
                    s.kurz = t[0].clone();
                    s.von = t[1].clone();
                    s.znr = t[2].clone();
                    s.pfade = t[3].trim().parse().unwrap_or(s.pfade).clamp(1, 99);
                    s.pfad_hoehe = d1(&t[4], s.pfad_hoehe);
                    s.size_titel = t[5].trim().parse().unwrap_or(s.size_titel);
                    s.size_wert = t[6].trim().parse().unwrap_or(s.size_wert);
                    s.size_verbg = t[7].trim().parse().unwrap_or(s.size_verbg);
                    s.size_pfad = t[8].trim().parse().unwrap_or(s.size_pfad);
                    an.name = a.name.clone();
                    an.set = s;
                    // Strompfade gelten für alle Blätter der Anlage, die keine eigene Angabe haben
                    let pf = an.set.pfade.max(1) as u32;
                    for sh in an.sheets.iter_mut().filter(|sh| sh.col_auto) {
                        sh.columns = pf;
                    }
                }
                let _ = t;
            }
            Props::Sheet(s) => {
                // Regeln des Originals für Nummer 0, Subindex 0 und Deckblatt (blattnummer.rs); im Nachbau
                // angelegte Blätter ohne Art gelten als Schaltplan
                let art = s.art as u8;
                let (n, sub) = match crate::blattnummer::pruefen(&self.project, s.r, &s.nummer, &s.subindex, art) {
                    Ok(v) => v,
                    Err(m) => {
                        self.dialog = Some(crate::app::Dialog::Message(m));
                        return;
                    }
                };
                if let Some(sh) = self.project.sheet_mut(s.r) {
                    sh.name = s.name;
                    sh.ort = s.ort;
                    sh.number = n;
                    sh.subindex = sub;
                    // -1 = Strompfade der Anlage übernehmen (wie im Original)
                    let c: i32 = s.strompfade.trim().parse().unwrap_or(-1);
                    if c > 0 {
                        sh.columns = c.clamp(1, 40) as u32;
                        sh.col_auto = false;
                    } else {
                        sh.col_auto = true;
                    }
                    sh.portrait = s.portrait;
                    // Blattart; beim Klemmplan die gewählte Klemmleiste (BLT Feld 32 ab Bit 8)
                    sh.art = s.art as u8;
                    sh.art_fest = true;
                    sh.klm = if s.art == 4 { s.zusatz.checked_sub(1).and_then(|i| s.leisten.get(i)).map(|l| l.0).unwrap_or(0) } else { 0 };
                    sh.zusatz = match s.art {
                        2 => (s.zusatz.min(2) + 1) as u8,
                        0 | 6 => s.zusatz.min(2) as u8,
                        _ => 0,
                    };
                }
                // Inhaltsverzeichnis, Querverweise, Artikelliste: fehlende Folgeblätter anlegen – wie im Original nach
                // der Rückfrage „Sollen alle nötigen Blätter erzeugt werden?“ (Nein = nur dieses Blatt)
                if !crate::meldungen::folgeblaetter_frage(&self.project, s.r) {
                    self.dirty = true;
                    return;
                }
                match crate::berechnet::folgeblaetter_anlegen(&mut self.project, s.r) {
                    Ok(0) => {}
                    Ok(n) => self.info = format!("{n} Folgeblätter für die Liste angelegt"),
                    Err(m) => self.dialog = Some(crate::app::Dialog::Message(m)),
                }
            }
        }
        self.dirty = true;
    }
}

fn symbol_ui(ui: &mut egui::Ui, s: &mut SymProps) {
    ui.spacing_mut().window_margin = egui::Margin::same(3);
    // Kopfzeilen mit festen Breiten wie im Original (Name 116, Titel/Wert 116, X/Y je 32 px)
    ui.spacing_mut().item_spacing = Vec2::new(3.0, 2.0);
    ui.spacing_mut().interact_size.y = 18.0;
    ui.horizontal(|ui| {
        label(ui, 46.0, "Name", true);
        ui.add_sized([116.0, 20.0], egui::TextEdit::singleline(&mut s.name).interactive(false));
    });
    for (n, txt, x, y) in [("Titel", &mut s.bmk, &mut s.tx, &mut s.ty), ("Wert", &mut s.value, &mut s.vx, &mut s.vy)] {
        ui.horizontal(|ui| {
            label(ui, 46.0, n, true);
            ui.add_sized([116.0, 20.0], egui::TextEdit::singleline(txt));
            ui.label(bt("X"));
            ui.add_sized([34.0, 19.0], egui::TextEdit::singleline(x).font(egui::FontId::proportional(11.0)).margin(egui::Margin::symmetric(2, 2)));
            ui.label(bt("Y"));
            ui.add_sized([34.0, 19.0], egui::TextEdit::singleline(y).font(egui::FontId::proportional(11.0)).margin(egui::Margin::symmetric(2, 2)));
        });
    }
    ui.horizontal(|ui| {
        label(ui, 110.0, "Titelausrichtung", true);
        egui::ComboBox::from_id_salt("tal").width(140.0).selected_text(["Links", "Mittig", "Rechts"][s.title_align.min(2) as usize]).show_ui(ui, |ui| {
            for (i, n) in ["Links", "Mittig", "Rechts"].iter().enumerate() {
                ui.selectable_value(&mut s.title_align, i as u8, *n);
            }
        });
    });
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Transformation").size(11.0));
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(bt("Drehung"));
                egui::ComboBox::from_id_salt("dreh").width(50.0).selected_text(format!("{}°", s.drehung as u32 * 90)).show_ui(ui, |ui| {
                    for i in 0..4u8 {
                        ui.selectable_value(&mut s.drehung, i, format!("{}°", i as u32 * 90));
                    }
                });
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.label(bt("Spiegeln"));
                ui.checkbox(&mut s.sp_h, bt("Horizontal"));
                ui.checkbox(&mut s.sp_v, bt("Vertikal"));
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(bt("Skalierung"));
                edit(ui, &mut s.scale, 60.0);
            });
        });
    });
    egui::Grid::new("sym2").num_columns(2).spacing([4.0, 2.0]).show(ui, |ui| {
        label(ui, 98.0, "Alternativtext", true);
        let cur = s.alt_items.get(s.alt as usize).cloned().unwrap_or_else(|| "Benutzerdefiniert".into());
        egui::ComboBox::from_id_salt("alt").width(150.0).selected_text(cur).show_ui(ui, |ui| {
            for (i, n) in s.alt_items.iter().enumerate() {
                ui.selectable_value(&mut s.alt, i as u16, n);
            }
        });
        ui.end_row();
        let bild_on = s.bild_items.len() > 1;
        label(ui, 98.0, "Bild", bild_on);
        ui.add_enabled_ui(bild_on, |ui| {
            let cur = s.bild_items.get(s.bild as usize).cloned().unwrap_or_else(|| "Standard".into());
            egui::ComboBox::from_id_salt("bild").width(150.0).selected_text(cur).show_ui(ui, |ui| {
                for (i, n) in s.bild_items.iter().enumerate() {
                    ui.selectable_value(&mut s.bild, i as u16, n);
                }
            });
        });
        ui.end_row();
        label(ui, 98.0, "Rackposition", true);
        edit(ui, &mut s.rack, 60.0);
        ui.end_row();
        label(ui, 98.0, "Index", s.korrektur);
        ui.horizontal(|ui| {
            ui.add_enabled(s.korrektur, egui::TextEdit::singleline(&mut s.index).desired_width(40.0));
            ui.checkbox(&mut s.korrektur, bt("Korrektur"));
        });
        ui.end_row();
        label(ui, 98.0, "Funktion", true);
        edit(ui, &mut s.funktion, 150.0);
        ui.end_row();
        label(ui, 98.0, "Bauteil", true);
        ui.horizontal(|ui| {
            ui.add_enabled(false, egui::TextEdit::singleline(&mut s.bauteil).desired_width(126.0).background_color(Color32::from_rgb(255, 255, 225)));
            // Knopf "Bearbeiten" (TSpeedButton LO_Liste): Bauteilzuordnung
            let btn = egui::Button::image(crate::icons::image(crate::icons::Icon::EditBauteil, 20.0));
            if ui.add(btn.min_size(Vec2::new(31.0, 24.0))).on_hover_text("Bearbeiten").clicked() {
                s.edit_part = true;
            }
        });
        ui.end_row();
    });
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Freigaben").size(11.0));
        ui.checkbox(&mut s.querverweise, bt("Querverweise"));
        ui.checkbox(&mut s.sicherung, bt("Sicherungsübersicht"));
    });
}

fn text_ui(ui: &mut egui::Ui, t: &mut TextProps) {
    ui.group(|ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Textboxabmessung").size(11.0));
        egui::Grid::new("tb").num_columns(4).spacing([4.0, 4.0]).show(ui, |ui| {
            label(ui, 42.0, "Oben", true);
            edit(ui, &mut t.oben, 70.0);
            label(ui, 58.0, "Höhe", true);
            edit(ui, &mut t.hoehe, 70.0);
            ui.end_row();
            label(ui, 42.0, "Links", true);
            edit(ui, &mut t.links, 70.0);
            label(ui, 58.0, "Breite", true);
            edit(ui, &mut t.breite, 70.0);
            ui.end_row();
        });
    });
    ui.horizontal(|ui| {
        ui.label(bt("Rahmen"));
        ui.checkbox(&mut t.rahmen, "");
        ui.add_space(8.0);
        // im Original immer gesperrt (zeigt nur den gespeicherten Wert)
        ui.add_enabled(false, egui::Label::new(bt("Umbruch")));
        ui.add_enabled(false, egui::Checkbox::new(&mut t.umbruch, ""));
        ui.add_space(8.0);
        ui.label(bt("Fixiert"));
        ui.checkbox(&mut t.fixiert, "");
    });
    ui.horizontal(|ui| {
        label(ui, 112.0, "Textausrichtung", true);
        let names = ["Links", "Mittig", "Rechts"];
        let cur = match t.align {
            Align::Left => 0,
            Align::Center => 1,
            Align::Right => 2,
        };
        egui::ComboBox::from_id_salt("tal").width(90.0).selected_text(names[cur]).show_ui(ui, |ui| {
            ui.selectable_value(&mut t.align, Align::Left, "Links");
            ui.selectable_value(&mut t.align, Align::Center, "Mittig");
            ui.selectable_value(&mut t.align, Align::Right, "Rechts");
        });
    });
    ui.horizontal(|ui| {
        label(ui, 112.0, "Drehung", true);
        // Auswahl wie im Original (TAuswahl T_Drehung: 0°, 90°, 180°, 270°)
        let cur = t.drehung.trim().parse::<i32>().unwrap_or(0).rem_euclid(360);
        egui::ComboBox::from_id_salt("tdreh").width(84.0).selected_text(format!("{cur}°")).show_ui(ui, |ui| {
            for g in [0, 90, 180, 270] {
                if ui.selectable_label(cur == g, format!("{g}°")).clicked() {
                    t.drehung = if g == 0 { String::new() } else { g.to_string() };
                }
            }
        });
        if ui.add_sized([48.0, 19.0], egui::Button::new(RichText::new("Schrift").size(11.0))).clicked() {
            let schnitt = (t.bold as usize) * 2 + t.italic as usize;
            t.font_dlg = Some(FontDlg { font: t.font.clone(), schnitt, pt: t.pt.to_string(), underline: t.underline });
        }
    });
    egui::ScrollArea::vertical().max_height(176.0).show(ui, |ui| {
        ui.add(egui::TextEdit::multiline(&mut t.text).desired_width(270.0).desired_rows(11).font(egui::FontId::proportional(12.0)));
    });
}

fn symeigen_ui(ui: &mut egui::Ui, s: &mut SymEigenProps) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                label(ui, 80.0, "Name", true);
                ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut s.name));
            });
            ui.horizontal(|ui| {
                label(ui, 80.0, "Gruppe", true);
                egui::ComboBox::from_id_salt("symgrp").width(194.0).selected_text(s.group.clone()).show_ui(ui, |ui| {
                    for g in s.groups.clone() {
                        ui.selectable_value(&mut s.group, g.clone(), g);
                    }
                });
            });
        });
        ui.add_space(40.0);
        ui.vertical(|ui| {
            ui.checkbox(&mut s.quer, bt("Freigabe für Querverweise"));
            ui.checkbox(&mut s.fuse, bt("Freigabe für Sicherungsübersicht"));
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (i, t) in ["Seiten / Optionen", "Stifte / Parameter", "Alternativtexte"].iter().enumerate() {
            if ui.selectable_label(s.tab == i as u8, *t).clicked() {
                s.tab = i as u8;
            }
        }
    });
    ui.separator();
    let list = |ui: &mut egui::Ui, id: &str, items: &mut Vec<String>, sel: &mut usize, w: f32, del: &str, min: usize| {
        ui.vertical(|ui| {
            ui.set_width(w);
            egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0, Color32::from_gray(160))).show(ui, |ui| {
                ui.set_min_size(Vec2::new(w - 4.0, 270.0));
                egui::ScrollArea::vertical().id_salt(id).max_height(270.0).show(ui, |ui| {
                    for (i, it) in items.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i + 1));
                            let r = ui.add(egui::TextEdit::singleline(it).desired_width(w - 40.0));
                            if r.gained_focus() || r.clicked() {
                                *sel = i;
                            }
                        });
                    }
                });
            });
            if ui.add_enabled(items.len() > min, egui::Button::new(del).min_size(Vec2::new(99.0, 25.0))).clicked() && *sel < items.len() {
                if id == "seiten" && *sel == 0 {
                    // TSymEigenschaften.Button3Click: die erste Seite bleibt
                    crate::meldungen::hinweis(crate::meldungen::T_ACHTUNG, crate::meldungen::SE_ERSTE_SEITE);
                } else {
                    items.remove(*sel);
                    *sel = sel.saturating_sub(1);
                }
            }
        });
    };
    match s.tab {
        0 => {
            ui.horizontal_top(|ui| {
                list(ui, "seiten", &mut s.seiten, &mut s.sel_seite, 280.0, "Seite Löschen", 1);
                ui.add_space(12.0);
                list(ui, "optionen", &mut s.optionen, &mut s.sel_option, 388.0, "Option Löschen", 0);
            });
        }
        1 => {
            ui.horizontal_top(|ui| {
                crate::symbol_stifte::liste_ui(ui, &mut s.stifte, &mut s.sel_stift, &mut s.stift_geloescht);
                ui.add_space(12.0);
                crate::parameter::param_table_ui(ui, &mut s.params, &mut s.sel_param);
            });
        }
        _ => {
            egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0, Color32::from_gray(160))).show(ui, |ui| {
                ui.set_min_size(Vec2::new(680.0, 270.0));
                egui::Grid::new("altgrid").striped(true).show(ui, |ui| {
                    ui.label(bt("Satz"));
                    for c in &s.text_cols {
                        ui.label(bt(&format!("Text {c}")));
                    }
                    ui.end_row();
                    let mut del: Option<usize> = None;
                    for (r, row) in s.alt.iter_mut().enumerate() {
                        // Kontextmenü wie TTextAbfrage: Zeile Löschen
                        ui.label(format!("{}", r + 1)).context_menu(|ui| {
                            if ui.button("Zeile Löschen").clicked() {
                                del = Some(r);
                                ui.close();
                            }
                        });
                        row.resize(s.text_cols.len(), String::new());
                        for cell in row.iter_mut() {
                            ui.add(egui::TextEdit::singleline(cell).desired_width(70.0));
                        }
                        ui.end_row();
                    }
                    if let Some(r) = del {
                        s.alt.remove(r);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(264.0);
                if ui.add_enabled(!s.text_cols.is_empty(), egui::Button::new("Neuer Alternativtext").min_size(Vec2::new(153.0, 25.0))).clicked() {
                    s.alt.push(vec![String::new(); s.text_cols.len()]);
                }
            });
        }
    }
}

fn anlage_ui(ui: &mut egui::Ui, a: &mut AnlProps) {
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
    let row = |ui: &mut egui::Ui, l: &str, v: &mut String, w: f32| {
        ui.horizontal(|ui| {
            label(ui, 84.0, l, true);
            ui.add_sized([w, 20.0], egui::TextEdit::singleline(v));
        });
    };
    row(ui, "Anlagenname", &mut a.name, 190.0);
    row(ui, "Kurzform", &mut a.t[0], 110.0);
    row(ui, "Erstellt von", &mut a.t[1], 190.0);
    row(ui, "Zeichnugs Nr", &mut a.t[2], 190.0);
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Strompfade").size(11.0));
        ui.horizontal(|ui| {
            ui.label(bt("Anzahl"));
            ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut a.t[3]));
            ui.label(bt("Höhe"));
            ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut a.t[4]));
            ui.label(bt("mm"));
            ui.checkbox(&mut a.s.ab_null, bt("Ab '0'"));
        });
    });
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Schriftgrösse für").size(11.0));
        egui::Grid::new("anl_sz").num_columns(4).show(ui, |ui| {
            for (i, (l, j)) in [("Titel", 5), ("Verbindungen", 7), ("Wert", 6), ("Pfadbezeichnung", 8)].iter().enumerate() {
                label(ui, if i % 2 == 0 { 40.0 } else { 104.0 }, l, true);
                ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut a.t[*j]));
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        label(ui, 84.0, "Blattformat", true);
        let cur = FORMATE.iter().find(|f| f.1 == a.s.format).map(|f| f.0.to_string()).unwrap_or_else(|| format!("{} × {}", a.s.format.0 / 10, a.s.format.1 / 10));
        egui::ComboBox::from_id_salt("fmt").width(130.0).selected_text(cur).show_ui(ui, |ui| {
            for (n, f) in FORMATE {
                ui.selectable_value(&mut a.s.format, *f, *n);
            }
        });
        // Benutzermaße (TUserdat): eigene Breite/Höhe in mm
        ui.menu_button("Benutzermaße", |ui| {
            let mut b = format!("{}", a.s.format.0 as f32 / 10.0);
            let mut h = format!("{}", a.s.format.1 as f32 / 10.0);
            ui.horizontal(|ui| {
                ui.label("Breite");
                if ui.add(egui::TextEdit::singleline(&mut b).desired_width(60.0)).changed() {
                    if let Some(v) = num(&b) {
                        a.s.format.0 = (v * 10.0).round() as i32;
                    }
                }
                ui.label("mm");
            });
            ui.horizontal(|ui| {
                ui.label("Hoehe");
                if ui.add(egui::TextEdit::singleline(&mut h).desired_width(60.0)).changed() {
                    if let Some(v) = num(&h) {
                        a.s.format.1 = (v * 10.0).round() as i32;
                    }
                }
                ui.label("mm");
            });
        });
    });
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
        ui.set_width(270.0);
        ui.label(RichText::new("Blattränder in mm (Hochformat)").size(11.0));
        let mut t: Vec<String> = a.s.raender.iter().map(|v| mm1(*v)).collect();
        egui::Grid::new("anl_r").num_columns(4).show(ui, |ui| {
            for (i, l) in ["Oben", "Unten", "Links", "Rechts"].iter().enumerate() {
                label(ui, 44.0, l, true);
                ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut t[i]));
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        for i in 0..4 {
            a.s.raender[i] = d1(&t[i], a.s.raender[i]);
        }
    });
}

fn sheet_ui(ui: &mut egui::Ui, s: &mut SheetProps, anlagen: &[String]) {
    egui::Grid::new("blt").num_columns(2).spacing([4.0, 8.0]).show(ui, |ui| {
        label(ui, 90.0, "Anlage", true);
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("anl").width(185.0).selected_text(anlagen.get(s.r.anlage).cloned().unwrap_or_default()).show_ui(ui, |_| {});
        });
        ui.end_row();
        label(ui, 90.0, "Art", true);
        egui::ComboBox::from_id_salt("art").width(145.0).selected_text(ARTEN.get(s.art).copied().unwrap_or(ARTEN[3])).show_ui(ui, |ui| {
            for (i, a) in ARTEN.iter().enumerate() {
                if ui.selectable_value(&mut s.art, i, *a).changed() {
                    s.zusatz = 0;
                }
            }
        });
        ui.end_row();
        // Zusatz wie im Original (Texte der Exe): Klemmenplan = Klemmleiste („Keine Zuordnung“), Deckblatt mit/ohne
        // Rahmen, Querverweise 1–3 Tabellen, Artikelliste Summenliste/Summe+BMK/Einzelliste
        let items: Vec<String> = match s.art {
            4 => std::iter::once("Keine Zuordnung".to_string()).chain(s.leisten.iter().map(|l| l.1.clone())).collect(),
            0 => vec!["mit Rahmen".into(), "ohne Rahmen".into()],
            2 => vec!["1 Tabelle".into(), "2 Tabellen".into(), "3 Tabellen".into()],
            6 => vec!["Summenliste".into(), "Summe+BMK".into(), "Einzelliste".into()],
            _ => vec![],
        };
        let mit_zusatz = !items.is_empty();
        label(ui, 90.0, "Zusatz", mit_zusatz);
        ui.add_enabled_ui(mit_zusatz, |ui| {
            let text = items.get(s.zusatz).cloned().unwrap_or_default();
            egui::ComboBox::from_id_salt("zus").width(145.0).selected_text(text).show_ui(ui, |ui| {
                for (i, t) in items.iter().enumerate() {
                    ui.selectable_value(&mut s.zusatz, i, t);
                }
            });
        });
        ui.end_row();
        label(ui, 90.0, "Auto+", false);
        ui.add_enabled_ui(false, |ui| {
            let mut a = true;
            ui.checkbox(&mut a, bt("(bei Darstellung)"));
        });
        ui.end_row();
        label(ui, 90.0, "Bezeichnung", true);
        edit(ui, &mut s.name, 180.0);
        ui.end_row();
        label(ui, 90.0, "Ort", true);
        edit(ui, &mut s.ort, 180.0);
        ui.end_row();
        label(ui, 90.0, "Blattnummer", true);
        edit(ui, &mut s.nummer, 115.0);
        ui.end_row();
        label(ui, 90.0, "Subindex", true);
        edit(ui, &mut s.subindex, 115.0);
        ui.end_row();
        label(ui, 90.0, "Strompfade", true);
        edit(ui, &mut s.strompfade, 115.0);
        ui.end_row();
        label(ui, 90.0, "Ausrichtung", true);
        egui::ComboBox::from_id_salt("ausr").width(135.0).selected_text(if s.portrait { "Hochformat" } else { "Querformat" }).show_ui(ui, |ui| {
            ui.selectable_value(&mut s.portrait, true, "Hochformat");
            ui.selectable_value(&mut s.portrait, false, "Querformat");
        });
        ui.end_row();
    });
}

/// Schriftdialog wie der Windows-Dialog "Schriftart" des Originals (Schriftart, Schriftschnitt, Schriftgrad, Beispiel).
fn font_window(ctx: &egui::Context, fd: &mut FontDlg, done: &mut Option<bool>) {
    let closed = tool_window(ctx, "fontdlg", "Schriftart", [420.0, 300.0], egui::Order::Foreground, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label("Schriftart:");
                    ui.add(egui::TextEdit::singleline(&mut fd.font).desired_width(140.0));
                    egui::ScrollArea::vertical().id_salt("fl").max_height(90.0).show(ui, |ui| {
                        // installierte Schriften (ohne Schriftordner die feste Auswahl)
                        let mut liste = crate::schriften::familien_namen();
                        if liste.is_empty() {
                            liste = SCHRIFTEN.iter().map(|s| s.to_string()).collect();
                        }
                        for f in &liste {
                            if ui.selectable_label(fd.font.eq_ignore_ascii_case(f), f).clicked() {
                                fd.font = f.clone();
                            }
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Schriftschnitt:");
                    let mut cur = SCHNITTE[fd.schnitt.min(3)].to_string();
                    ui.add_enabled(false, egui::TextEdit::singleline(&mut cur).desired_width(100.0));
                    for (i, n) in SCHNITTE.iter().enumerate() {
                        let rt = match i {
                            1 => RichText::new(*n).italics(),
                            2 => RichText::new(*n).strong(),
                            3 => RichText::new(*n).strong().italics(),
                            _ => RichText::new(*n),
                        };
                        if ui.selectable_label(fd.schnitt == i, rt).clicked() {
                            fd.schnitt = i;
                        }
                    }
                });
                ui.vertical(|ui| {
                    ui.label("Schriftgrad:");
                    ui.add(egui::TextEdit::singleline(&mut fd.pt).desired_width(50.0));
                    egui::ScrollArea::vertical().id_salt("gl").max_height(90.0).show(ui, |ui| {
                        for g in GRADE {
                            if ui.selectable_label(fd.pt.trim() == g.to_string(), g.to_string()).clicked() {
                                fd.pt = g.to_string();
                            }
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.set_min_size(Vec2::new(140.0, 80.0));
                    ui.label("Effekte");
                    ui.checkbox(&mut fd.underline, "Unterstrichen");
                });
                let font = crate::schriften::font_id(&fd.font, 26.0, fd.schnitt >= 2, fd.schnitt % 2 == 1);
                ui.add_space(20.0);
                ui.label(RichText::new("AaBbYyZz").font(font));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(200.0);
                if ui.add_sized([90.0, 24.0], egui::Button::new("OK")).clicked() {
                    *done = Some(true);
                }
                if ui.add_sized([90.0, 24.0], egui::Button::new("Abbrechen")).clicked() {
                    *done = Some(false);
                }
            });
        });
    if closed {
        *done = Some(false);
    }
}
