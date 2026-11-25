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
    /// `folgt`: das Kürzel ist noch die Vorgabe und folgt dem Namen, bis es selbst geändert wird; `name0` = Name beim
    /// Öffnen.
    Schiene { anlage: usize, orig: String, name: String, set: SchieneSet, pots: Vec<String>, stifte: Vec<String>, folgt: bool, name0: String },
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

const BILD_ARTEN: &[&str] = &["Zentriert", "Eingepasst", "Gestreckt"];
const KL_ALIGN: &[&str] = &["Links", "Mittig", "Rechts"];
/// Ebenen wie im Original „Nummer/Buchstabe“
const EBENEN: &[&str] = &["1/A", "2/B", "3/C", "4/D"];

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
fn field(ui: &mut egui::Ui, w: f32, t: &str, s: &mut String, fw: f32) -> egui::Response {
    ui.horizontal(|ui| {
        lbl(ui, w, t);
        ui.add(egui::TextEdit::singleline(s).desired_width(fw))
    })
    .inner
}

/// Merker „Anfangsfokus gesetzt“ des offenen Dialogs (wird beim Schließen gelöscht).
const ERSTER_FOKUS: &str = "dlg3_erster_fokus";

/// Fokus beim Öffnen einmal ins erste Eingabefeld (Nachzeichnen 24.11.2026, Befund 76/S4: ohne Klick ins Feld ging
/// die Eingabe verloren). Danach nicht mehr – sonst holte jedes Bild den Fokus von anderen Feldern zurück.
/// Der vorhandene Text wird dabei markiert wie bei Windows-Eingabefeldern (AutoSelect): Tippen ersetzt den Vorschlag.
fn erster_fokus(ui: &egui::Ui, r: &egui::Response, text: &str) {
    let id = egui::Id::unique(ERSTER_FOKUS);
    if !ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        r.request_focus();
        let mut st = egui::text_edit::TextEditState::load(ui.ctx(), r.id).unwrap_or_default();
        let alles = egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(text.chars().count()));
        st.cursor.set_char_range(Some(alles));
        st.store(ui.ctx(), r.id);
        ui.data_mut(|d| d.insert_temp(id, true));
    }
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
    icon_tab_nr(ui, id, "", 1, head, rows, sel, h);
}

/// Wie `icon_tab`, mit Kopf `nr_kopf` der Nummernspalte und Zählung ab `ab`.
#[allow(clippy::too_many_arguments)]
fn icon_tab_nr(ui: &mut egui::Ui, id: &str, nr_kopf: &str, ab: usize, head: &[&str], rows: &mut [Vec<String>], sel: &mut usize, h: f32) {
    egui::ScrollArea::both().id_salt(id).max_height(h).show(ui, |ui| {
        egui::Grid::new(id).striped(true).num_columns(head.len() + 1).show(ui, |ui| {
            ui.label(RichText::new(nr_kopf).strong());
            for c in head {
                ui.label(RichText::new(*c).strong());
            }
            ui.end_row();
            for (r, row) in rows.iter_mut().enumerate() {
                if ui.selectable_label(*sel == r, format!("{}", r + ab)).clicked() {
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

/// Titelgenerator: Formel für das `k`-te Objekt (ab 0) auswerten wie das Original (0x496e9c, Hilfetext
/// TTitel.Memo1): „@“ und genau ein Zeichen bilden eine Variable; ihr Wert beginnt bei 0 bzw. bei der Zahl in
/// Klammern direkt dahinter („@x(5)“) und steigt je Objekt um eins. Ein „@“ am Ende entfällt.
pub fn titel_formel(formel: &str, k: usize) -> String {
    titel_formel_mit(formel, k, &TitelWerte::default())
}

/// Zusatzwerte des Textgenerators (Erweiterung, Nutzerentscheidung 23.11.2026): `@{Blatt}`, `@{Pfad}`,
/// `@{Anlage}`, `@{Text}` (bisheriger Text). Im Original wäre „@{“ ein Zähler mit dem Zeichen „{“.
#[derive(Default)]
pub struct TitelWerte<'a> {
    pub blatt: u32,
    pub pfad: u32,
    pub anlage: &'a str,
    pub alt: &'a str,
}

/// Wie [`titel_formel`], zusätzlich mit den Werten `@{Blatt}`, `@{Pfad}`, `@{Anlage}`, `@{Text}` (Groß-/
/// Kleinschreibung egal); unbekannte Namen bleiben als Text stehen.
pub fn titel_formel_mit(formel: &str, k: usize, w: &TitelWerte) -> String {
    let c: Vec<char> = formel.chars().collect();
    let mut out = String::new();
    // Startwert je Variable (erste Angabe gilt)
    let mut start: Vec<(char, i64)> = vec![];
    let mut i = 0;
    while i < c.len() {
        if c[i] != '@' {
            out.push(c[i]);
            i += 1;
            continue;
        }
        let Some(&v) = c.get(i + 1) else { break };
        if v == '{' {
            if let Some(e) = c[i + 2..].iter().position(|ch| *ch == '}') {
                let name: String = c[i + 2..i + 2 + e].iter().collect();
                let wert = match name.trim().to_lowercase().as_str() {
                    "blatt" => Some(w.blatt.to_string()),
                    "pfad" => Some(w.pfad.to_string()),
                    "anlage" => Some(w.anlage.to_string()),
                    "text" => Some(w.alt.to_string()),
                    _ => None,
                };
                if let Some(t) = wert {
                    out.push_str(&t);
                    i += e + 3;
                    continue;
                }
            }
        }
        i += 2;
        let mut s0 = None;
        if c.get(i) == Some(&'(') {
            if let Some(e) = c[i..].iter().position(|ch| *ch == ')') {
                s0 = c[i + 1..i + e].iter().collect::<String>().trim().parse::<i64>().ok();
                i += e + 1;
            }
        }
        let st = match start.iter().find(|(n, _)| *n == v) {
            Some((_, w)) => *w,
            None => {
                let w = s0.unwrap_or(0);
                start.push((v, w));
                w
            }
        };
        out.push_str(&(st + k as i64).to_string());
    }
    out
}

/// Einträge „Reihenfolge 1“ wie TTitel (0x4b13b2 ff.) und je Eintrag der Sortierschlüssel für `order_key`.
pub const TITEL_DIR1: [(&str, usize); 4] = [("Links -> Rechts", 0), ("Oben -> Unten", 2), ("Rechts -> Links", 1), ("Unten -> Oben", 3)];

/// Einträge „Reihenfolge 2“ nach Dir1Change (0x4b144c): zu einer waagerechten Reihenfolge 1 die senkrechten
/// und umgekehrt.
pub fn titel_dir2(dir1: usize) -> [(&'static str, usize); 2] {
    if dir1 % 2 == 0 {
        [("Oben -> Unten", 2), ("Unten -> Oben", 3)]
    } else {
        [("Links -> Rechts", 0), ("Rechts -> Links", 1)]
    }
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
                // erster Eintrag wie im Original: `Neue Schiene "L<n>"` (schiene_neu.rs; Wine-Gegenprobe Runde 12)
                sliste.insert(0, crate::schiene_neu::eintrag(anl));
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
                // gesetztes Ziel vorwählen (Anlage, Blatt, Gegenverweis); ohne Ziel die eigene Anlage mit Blatt
                // „Keine Zuordnung“ wie im Original (0x4a468c, verweis_wahl.rs)
                let gesetzt = ziel.as_ref().and_then(|z| self.project.resolve_ziel(z));
                let (blatt_ref, gegen) = gesetzt.unwrap_or((r, None));
                let start = crate::verweis_wahl::ist_start(*dir);
                let link = gegen.and_then(|g| crate::verweis_wahl::link_kandidaten(&self.project, blatt_ref, id, start).iter().position(|k| k.0 == g)).map(|i| i + 1).unwrap_or(0);
                Dlg3::Verweis {
                    id,
                    anlagen: self.project.anlagen.iter().map(|a| a.name.clone()).collect(),
                    anlage: blatt_ref.anlage,
                    blatt: if gesetzt.is_some() { blatt_ref.sheet + 1 } else { 0 },
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
        // neue Schiene mit Vorgaben „Schiene n“/„Ln“: das Kürzel folgt dem Namen (Nachzeichnen B31: Name „N“ eingetragen,
        // auf dem Blatt stand weiter das Kürzel „L4“)
        let folgt = name.strip_prefix('L').is_some_and(|n| set.name == format!("Schiene {n}"));
        self.dlg3 = Some(Dlg3::Schiene {
            anlage,
            orig: name.into(),
            name: name.into(),
            folgt,
            name0: set.name.clone(),
            set,
            // erster Eintrag „Keine Zuordnung“ wie im Original (leeres Potential)
            pots: std::iter::once("Keine Zuordnung".to_string()).chain(self.project.potentiale.iter().cloned()).collect(),
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
        // Spalte „Bezeichnung“ nur mit eigenen Pfadbeschreibungen (0x4b726c: PFB-Liste), Nummern in „Pfade“
        self.dlg3 = Some(Dlg3::Pfad { r, rows: (0..s.columns as usize).map(|i| s.col_labels.get(i).cloned().unwrap_or_default()).collect(), grenze: s.pfad_linien });
    }

    pub fn open_titel_dialog(&mut self) {
        self.dlg3 = Some(Dlg3::Titel { formel: "S@".into(), dir1: 0, dir2: 0, wahl: 0, hilfe: false, msg: String::new() });
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
        let Some(p) = rfd::FileDialog::new().add_filter("Elektroplanungs Projekt", &["epr", "epj"]).add_filter("Elektroplan Vorlage", &["epv"]).pick_file() else { return };
        // wie das Original: Projekt hinzufügen nimmt nur Dateien ab Version 6.0.0
        if crate::vorlage::is_epr_format(&p) {
            if let Some((titel, text)) = crate::meldungen::ladefehler_vorab(&std::fs::read(&p), true) {
                crate::meldungen::warnung(titel, &text);
                return;
            }
        }
        match crate::mcp::open_project(&p) {
            Ok(prj) => self.dlg3 = Some(Dlg3::Wahl { path: p, anlagen: prj.anlagen.iter().map(|a| (a.name.clone(), true)).collect() }),
            Err(e) => crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, &format!("{}\n{e}", crate::meldungen::L_FEHLER)),
        }
    }

    pub fn open_neues_schriftfeld(&mut self) {
        // Original (Schriftfeld1Click): nicht, solange das Schriftfeld im Editor offen ist
        if matches!(self.xdlg, Some(crate::extra::XDialog::Title { .. })) {
            crate::meldungen::warnung(crate::meldungen::T_ACHTUNG, crate::meldungen::SCHRIFTFELD_IN_BEARBEITUNG);
            return;
        }
        // Dateifeld zeigt anfangs „Keine Datei“ wie im Original (TForm10-Konstruktor 0x4ae1b4, In1.Text)
        self.dlg3 = Some(Dlg3::NeuesSchriftfeld { anlage: 0, aus_datei: false, pfad: crate::meldungen::SF_KEINE_DATEI.into() });
    }

    pub fn open_schriftfeld_eigenschaften(&mut self) {
        let h = self.project.title_h();
        self.dlg3 = Some(Dlg3::Schriftfeld { absolut: self.project.tb_absolut, hoehe: f1(h) });
    }

    pub fn open_userdat(&mut self, anlage: usize) {
        let f = self.project.anlagen.get(anlage).map(|a| a.set.format).unwrap_or((2100, 2970));
        self.dlg3 = Some(Dlg3::UserDat { anlage, breite: f1(f.0 as f32 / 10.0), hoehe: f1(f.1 as f32 / 10.0) });
    }

    /// Kontaktspiegel (TSSpiegel): verknüpftes Objekt unter der Spule, siehe kontaktspiegel.rs.
    pub fn open_spiegel(&mut self) {
        self.spiegel_oeffnen();
    }

    /// Dialoge zeichnen und anwenden.
    pub fn dialoge3_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dlg3.take() else {
            ctx.data_mut(|m| m.remove::<bool>(egui::Id::unique(ERSTER_FOKUS)));
            return;
        };
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
            // Überschrift und Höhe je Art wie TTextchange.FormShow 0x4abc10 (Art 1 Symboltitel: Höhe 120 mit Vorschau)
            Dlg3::TextChange { wert: Some(false), .. } => ("Symboltitel", [285.0, 90.0]),
            Dlg3::TextChange { wert: Some(true), .. } => ("Symbolwert", [285.0, 63.0]),
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
                    // Anzeige wie im Original „Name (Kürzel)“, z. B. „Schiene 1 (L1)“
                    let anl = self.cur().and_then(|r| self.project.anlagen.get(r.anlage));
                    let anzeige: Vec<String> = sliste
                        .iter()
                        .map(|k| match anl.and_then(|a| a.schienen.get(k)).map(|s| s.name.clone()).filter(|n| !n.is_empty() && n != k) {
                            Some(n) => format!("{n} ({k})"),
                            None => k.clone(),
                        })
                        .collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 76.0, "S-Schiene");
                        combo(ui, "sms", &anzeige, sel, 150.0);
                        // Knopf "Bearbeiten" (SammelS1NeuClick): Einstellungen der gewählten Schiene
                        let btn = egui::Button::image(crate::icons::image(crate::icons::Icon::EditSammelschiene, 20.0));
                        if ui.add(btn.min_size(Vec2::new(31.0, 24.0))).on_hover_text("Bearbeiten").clicked() {
                            rail_edit = sliste.get(*sel).cloned();
                        }
                    });
                    // Einspeisung ist eine Eigenschaft der Schiene (Knopf „Bearbeiten“); hier wie im Original nur die
                    // Anzeige (Kreis, gefüllt = mit Einspeisung) der gewählten Schiene
                    *einsp = sliste.get(*sel).and_then(|k| anl.and_then(|a| a.schienen.get(k))).is_some_and(|s| s.einspeisung);
                    ui.horizontal(|ui| {
                        ui.add_space(40.0);
                        ui.label(RichText::new("Einspeisung").font(bold(13.0)));
                        let (r, p) = ui.allocate_painter(Vec2::splat(16.0), egui::Sense::hover());
                        let c = r.rect.center();
                        p.circle_stroke(c, 6.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
                        if *einsp {
                            p.circle_filled(c, 3.5, egui::Color32::BLACK);
                        }
                    });
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
                    // mit „Auto“ legt das Programm Startpunkt und Länge fest (im Original dann gesperrt)
                    let frei = !*auto;
                    for (t, a, b, an) in [("Titelposition", tx, ty, true), ("Startpunkt", px, py, frei)] {
                        ui.horizontal(|ui| {
                            lbl(ui, 96.0, t);
                            ui.add_enabled(an, egui::TextEdit::singleline(a).desired_width(70.0));
                            ui.add_enabled(an, egui::TextEdit::singleline(b).desired_width(70.0));
                        });
                    }
                    ui.add_enabled_ui(frei, |ui| field(ui, 96.0, "Länge", laenge, 70.0));
                }
                Dlg3::Schiene { name, set, pots, stifte, folgt, name0, .. } => {
                    // Name = freier Name (SMS Byte 0), Kürzel = Schlüssel der Schiene (TIT, steht auf dem Blatt)
                    let r = field(ui, 70.0, "Name", &mut set.name, 180.0);
                    erster_fokus(ui, &r, &set.name);
                    if *folgt && set.name != *name0 && !set.name.trim().is_empty() {
                        *name = set.name.trim().to_string();
                    }
                    let vor = name.clone();
                    field(ui, 70.0, "Kürzel", name, 180.0);
                    if *name != vor {
                        *folgt = false;
                    }
                    ui.horizontal(|ui| {
                        lbl(ui, 86.0, "Potential");
                        let mut i = if set.potential.is_empty() { 0 } else { pots.iter().skip(1).position(|p| *p == set.potential).map(|i| i + 1).unwrap_or(0) };
                        if combo(ui, "spot", pots, &mut i, 150.0) {
                            set.potential = if i == 0 { String::new() } else { pots[i].clone() };
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
                    // Anzeige wie im Original „Name (Titel)“, z. B. „Klemme 1 (X1)“; gewählt wird weiter der Titel
                    let anl = self.cur().and_then(|r| self.project.anlagen.get(r.anlage));
                    let anzeige: Vec<String> = leisten
                        .iter()
                        .map(|t| match anl.and_then(|a| a.klemmen.iter().find(|k| &k.name == t)) {
                            Some(k) if !k.intern.is_empty() && k.intern != *t => format!("{} ({t})", k.intern),
                            _ => t.clone(),
                        })
                        .collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 90.0, "Klemmleiste");
                        combo(ui, "kleiste", &anzeige, leiste, 170.0);
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
                        // „Automatisch“ sperrt Nummer, Position und Ebene (Original unter Wine, Runde 11)
                        ui.horizontal(|ui| {
                            lbl(ui, 60.0, "Position");
                            ui.add_enabled(!*auto, egui::TextEdit::singleline(position).desired_width(50.0));
                            ui.checkbox(posk, "Korrektur");
                        });
                        ui.horizontal(|ui| {
                            lbl(ui, 60.0, "Ebene");
                            ui.add_enabled_ui(!*auto, |ui| combo(ui, "kebene", &strs(EBENEN), ebene, 60.0));
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
                Dlg3::Verweis { id, anlagen, anlage, blatt, link, zu, titel } => {
                    // Listen wie im Original (verweis_wahl.rs): Eintrag 0 jeweils „Keine Zuordnung“; Verweislink nur
                    // mit Verweisen der Gegenrichtung (Richtung des Verweises beim Öffnen)
                    let blaetter = crate::verweis_wahl::blatt_eintraege(&self.project, *anlage);
                    let start = self.cur().and_then(|r| self.project.sheet(r)).and_then(|s| s.objects.iter().find(|o| o.id == *id)).map(|o| matches!(&o.kind, Kind::Verweis { dir, .. } if crate::verweis_wahl::ist_start(*dir))).unwrap_or(*zu);
                    let kand = match blatt.checked_sub(1) {
                        Some(b) => crate::verweis_wahl::link_kandidaten(&self.project, SheetRef { anlage: *anlage, sheet: b }, *id, start),
                        None => vec![],
                    };
                    let links = crate::verweis_wahl::link_eintraege(&kand);
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
                        // Komfort des Nachbaus: Auswahl übernimmt den Titel des Gegenverweises (Original: kein OnChange)
                        if combo(ui, "vwlink", &links, link, 170.0) && *link > 0 {
                            if let Some(k) = kand.get(*link - 1).filter(|k| !k.1.trim().is_empty()) {
                                *titel = k.1.clone();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(92.0);
                        // im Dialog TObjekt_Eigenschaften heißen VW_Zu/VW_Von „Start“/„Ziel“ (DFM-Überschreibung)
                        if ui.radio(*zu, "Start").clicked() {
                            *zu = true;
                        }
                        if ui.radio(!*zu, "Ziel").clicked() {
                            *zu = false;
                        }
                    });
                    field(ui, 40.0, "Titel", titel, 200.0);
                }
                Dlg3::Bild { grafiken, grafik, art, oben, links, hoehe, breite, rahmen, fixiert, .. } => {
                    let mut neu = false;
                    // Liste wie im Original (0x446845): Namen der Grafiken des Projekts statt der Schlüssel „bld:<Nr>“
                    let namen: Vec<String> = grafiken.iter().map(|g| crate::verweis_wahl::grafik_name(&self.project, g)).collect();
                    ui.horizontal(|ui| {
                        lbl(ui, 80.0, "Grafik");
                        combo(ui, "bgraf", &namen, grafik, 140.0);
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
                    let r = field(ui, 52.0, "Name", name, 200.0);
                    erster_fokus(ui, &r, name);
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
                        combo(ui, "kleb", &strs(&["1", "2", "3", "4"]), ebenen, 50.0);
                    });
                    ui.label(RichText::new("Indexübersicht").font(bold(12.0)));
                    // Spalten wie im Original (0x4830a8): Klemme und „Verwendung auf Blatt“ (nur Anzeige)
                    let anl = self.project.anlagen.get(*anlage);
                    let mut rows: Vec<Vec<String>> = nrs.iter().map(|n| vec![n.clone(), anl.map(|a| crate::klemmen_verwendung::verwendung(a, titel, n)).unwrap_or_default()]).collect();
                    let mut sel = 0;
                    icon_tab(ui, "kidx", &crate::klemmen_verwendung::KOPF, &mut rows, &mut sel, 200.0);
                    *nrs = rows.into_iter().map(|mut r| r.remove(0)).collect();
                }
                Dlg3::Pfad { r, rows, grenze } => {
                    // Spalten „Pfade“ und „Bezeichnung“ (0x4b726c); Pfadnummern ab 0 bei „Ab '0'“ der Anlage
                    let ab = usize::from(!self.project.sheet(*r).is_some_and(|s| s.ab_null));
                    let mut grid: Vec<Vec<String>> = rows.iter().map(|r| vec![r.clone()]).collect();
                    let mut sel = 0;
                    icon_tab_nr(ui, "pfad", "Pfade", ab, &["Bezeichnung"], &mut grid, &mut sel, 290.0);
                    *rows = grid.into_iter().map(|mut r| r.remove(0)).collect();
                    ui.checkbox(grenze, "Begrenzungslinien");
                }
                Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, msg } => {
                    ui.label(RichText::new("Erzeugungsformel").font(bold(12.0)));
                    ui.add(egui::TextEdit::multiline(formel).desired_rows(2).desired_width(280.0));
                    ui.horizontal(|ui| {
                        lbl(ui, 110.0, "Reihenfolge 1");
                        let alt = *dir1;
                        combo(ui, "dir1", &crate::dialoge3::TITEL_DIR1.iter().map(|d| d.0.to_string()).collect::<Vec<_>>(), dir1, 150.0);
                        // Dir1Change: Reihenfolge 2 neu füllen
                        if *dir1 != alt {
                            *dir2 = 0;
                        }
                    });
                    ui.horizontal(|ui| {
                        lbl(ui, 110.0, "Reihenfolge 2");
                        combo(ui, "dir2", &titel_dir2(*dir1).iter().map(|d| d.0.to_string()).collect::<Vec<_>>(), dir2, 150.0);
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
                        // sinngemäß wie TTitel.Memo1
                        ui.small("Der eingegebene Text wird jedem gefundenen Objekt zugewiesen. Dabei werden Variablen durch ihre Werte ersetzt. Eine Variable wird durch das Zeichen '@' eingeleitet und besteht aus genau einem Zeichen. Der Wert einer Variable beginnt bei 0, es sei denn, nach der Variable wird ein Wert in Klammern eingegeben. Die Variablenwerte werden für jedes neue Objekt um eins erhöht. Für Symbole kann gewählt werden, welcher Text erzeugt werden soll. Zusätzlich (Nachbau): @{Blatt} Blattnummer, @{Pfad} Strompfad, @{Anlage} Kurzform der Anlage, @{Text} bisheriger Text.");
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
                    // wie TFORM10.FormShow: „Gesamtes Projekt“, dann die Anlagen (ohne Namen: „[AnlageX ohne Namen]“)
                    let mut items = vec!["Gesamtes Projekt".to_string()];
                    items.extend(crate::verweis_wahl::anlagen_ohne_namen(&self.project));
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
                        // Original Button3Click 0x4ae374: „Schriftfelddatei|*.eff“; Projekte als Erweiterung
                        if let Some(p) = rfd::FileDialog::new().add_filter("Schriftfelddatei", &["eff"]).add_filter("Projekt", &["epj", "epr"]).pick_file() {
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
                Dlg3::TextChange { text, wert, .. } => {
                    let r = field(ui, 40.0, "Text", text, 220.0);
                    erster_fokus(ui, &r, text);
                    // Symboltitel: Label mit den Textvariablen als Platzhalter (Edit1Change 0x4abe54, textvariablen.rs)
                    if *wert == Some(false) {
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(RichText::new(crate::textvariablen::platzhalter(text)).font(bold(13.0)));
                        });
                    }
                }
                Dlg3::Errliste { lines } => {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for l in lines.iter() {
                            ui.label(l);
                        }
                    });
                    // Blöcke ohne Darstellung (z. B. defekter Verweis aus einem alten Nachbau) auf Wunsch entfernen
                    if lines.iter().any(|l| l.contains("ohne Darstellung")) && ui.button("Blöcke ohne Darstellung entfernen").on_hover_text("Entfernt die gemeldeten Blöcke beim nächsten Speichern aus der Datei (Rückgängig möglich)").clicked() {
                        self.checkpoint();
                        let n = crate::epr::fehlbloecke_entfernen(&mut self.project);
                        self.dirty = true;
                        lines.retain(|l| !l.contains("ohne Darstellung"));
                        self.info = if n == 1 { "1 Block ohne Darstellung entfernt".into() } else { format!("{n} Blöcke ohne Darstellung entfernt") };
                    }
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
        if let Some(mut name) = rail_edit {
            if let Some(r) = self.cur() {
                let obj = if let Dlg3::BusbarObj { id, .. } = &d { Some(*id) } else { None };
                let _ = self.apply_dlg3(d);
                // für den Eintrag „Neue Schiene …“ die eben angelegte Schiene des Objekts bearbeiten
                if crate::schiene_neu::ist_neu(&name) {
                    let neu = self.project.sheet(r).and_then(|s| s.objects.iter().find(|o| Some(o.id) == obj)).and_then(|o| if let Kind::Busbar { name, .. } = &o.kind { Some(name.clone()) } else { None });
                    name = neu.unwrap_or_default();
                }
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
            Some(false) => self.neu_verwerfen(),
            None => self.dlg3 = Some(d),
        }
    }

    /// Offenen Dialog wie mit „OK“ anwenden (für Tests und Skripte; gleicher Weg wie der Knopf).
    pub fn dlg3_ok(&mut self) {
        if let Some(d) = self.dlg3.take() {
            if let Some(keep) = self.apply_dlg3(d) {
                self.dlg3 = Some(keep);
            }
        }
    }

    /// Offenen Dialog wie mit „Abbruch“ schließen (neu angelegte Einträge werden zurückgenommen).
    pub fn dlg3_abbrechen(&mut self) {
        self.dlg3 = None;
        self.neu_verwerfen();
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
                let mut name = sliste.get(sel).cloned().unwrap_or_default();
                // `Neue Schiene "L<n>"` gewählt: Schiene anlegen und das Objekt daran hängen (0x405e08)
                if crate::schiene_neu::ist_neu(&name) {
                    match r.and_then(|r| self.project.anlagen.get_mut(r.anlage)) {
                        Some(a) => name = crate::schiene_neu::anlegen(a),
                        None => return None,
                    }
                }
                // Einspeisung wird im Schienen-Dialog („Bearbeiten“) geändert, hier nur angezeigt
                let _ = einsp;
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
                // Ziel setzen; mit Verweislink wird der Gegenverweis wie im Original zurückverknüpft.
                // Blatt „Keine Zuordnung“ (Index 0): Verweis ohne Ziel, bisheriger Partner wird gelöst
                let start = self.project.find_obj(id).map(|(_, o)| matches!(&o.kind, Kind::Verweis { dir, .. } if crate::verweis_wahl::ist_start(*dir))).unwrap_or(zu);
                let Some(b) = blatt.checked_sub(1) else {
                    crate::verweise::loesen(&mut self.project, id);
                    self.with_sheet(|s, _| {
                        if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                            if let Kind::Verweis { text, dir, .. } = &mut o.kind {
                                *text = titel.clone();
                                if crate::verweis_wahl::ist_start(*dir) != zu {
                                    *dir = (*dir + 180) % 360;
                                }
                            }
                        }
                    });
                    return None;
                };
                let ziel_ref = SheetRef { anlage, sheet: b };
                let gegen = link.checked_sub(1).and_then(|i| crate::verweis_wahl::link_kandidaten(&self.project, ziel_ref, id, start).get(i).map(|k| k.0));
                // Original (0x48056c): ist der Gegenverweis schon mit einem anderen verbunden, erst fragen (Nein = nichts verknüpfen)
                let bestaetigt = !crate::meldungen::zielverweis_belegt(&self.project, id, gegen) || crate::meldungen::frage(crate::meldungen::T_SICHER_F, crate::meldungen::ZIELVERWEIS_BELEGT);
                if bestaetigt && self.project.sheet(ziel_ref).is_some() {
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
                    // leere Zeilen = Pfadnummer (col_label); ganz leer = keine eigenen Beschreibungen
                    s.col_labels = if rows.iter().all(|t| t.trim().is_empty()) { vec![] } else { rows };
                    s.pfad_linien = grenze;
                }
            }
            Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, .. } => {
                let n = self.apply_titel(&formel, dir1, dir2, wahl);
                if n == 0 {
                    crate::meldungen::hinweis(crate::meldungen::T_TEXTGENERATOR, crate::meldungen::TEXTGENERATOR_LEER);
                }
                return Some(Dlg3::Titel { formel, dir1, dir2, wahl, hilfe, msg: format!("{n} Objekt(e) beschriftet") });
            }
            Dlg3::Schriftfeld { absolut, hoehe } => {
                // Das Modell führt das Schriftfeld relativ; beim Umschalten rechnet das .epr-Schreiben um wie das
                // Original. Bleibt es absolut und ändert sich die Höhe, behalten die Elemente wie im Original ihre
                // Lage in mm (relativ umgerechnet).
                let alt_h = (self.project.title_h() * 10.0).round() as i32;
                let war_absolut = self.project.tb_absolut;
                let alt_hoehe = self.project.tb_hoehe;
                self.project.tb_absolut = absolut;
                self.project.tb_hoehe = num(&hoehe).unwrap_or(TITLE_H).clamp(5.0, 60.0);
                let neu_h = (self.project.title_h() * 10.0).round() as i32;
                // Original (0x47b828): bei geänderter Höhe fragen, ob die Zeichenobjekte mitgehen (Ja = anpassen,
                // Nein = Lage in mm behalten, Abbrechen = nichts ändern)
                let antwort = if alt_h != neu_h {
                    crate::meldungen::zeigen(crate::meldungen::T_ANPASSEN, crate::meldungen::SE_HOEHE_FRAGE, crate::meldungen::Knoepfe::JaNeinAbbrechen, false)
                } else {
                    crate::meldungen::Antwort::Ja
                };
                if antwort == crate::meldungen::Antwort::Abbrechen {
                    self.project.tb_absolut = war_absolut;
                    self.project.tb_hoehe = alt_hoehe;
                    return None;
                }
                if (war_absolut || absolut) && alt_h != neu_h && antwort == crate::meldungen::Antwort::Nein {
                    let tb = if self.project.titleblock.is_empty() { crate::titleblock::default_owned() } else { self.project.titleblock.clone() };
                    self.project.titleblock = crate::titleblock::nach_relativ(&crate::titleblock::nach_absolut(&tb, alt_h), neu_h);
                }
            }
            Dlg3::NeuesSchriftfeld { aus_datei, pfad, .. } => {
                if aus_datei && crate::schriftfeld_datei::ist_eff(std::path::Path::new(pfad.trim())) {
                    // Schriftfelddatei des Originals (0x4732d4)
                    match std::fs::read(pfad.trim()) {
                        // Überschrift = Meldung des Originals, Text = Systemmeldung (FormatMessage)
                        Err(e) => crate::meldungen::warnung(crate::meldungen::SF_DATEI_OEFFNEN, &e.to_string()),
                        Ok(d) => match crate::schriftfeld_datei::schriftfeld_aus(&d) {
                            Ok(tb) => {
                                self.checkpoint();
                                self.project.titleblock = tb;
                            }
                            Err(m) => crate::meldungen::warnung(crate::schriftfeld_datei::T_FEHLER, m),
                        },
                    }
                } else if aus_datei {
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
                // Prüfungen und Rückfrage wie das Original (0x49c834); Fehler: Dialog bleibt offen
                let Some(a) = self.project.anlagen.get(anlage) else {
                    crate::meldungen::warnung(crate::meldungen::T_ACHTUNG, crate::meldungen::NN_KEINE_ANLAGE);
                    return Some(Dlg3::BlattIndex { anlage, von, bis, ab, plus });
                };
                let nummern: Vec<(u32, u8)> = a.sheets.iter().map(|s| (s.number, s.subindex)).collect();
                match crate::meldungen::neunummerierung(&nummern, &von, &bis, &ab, &plus) {
                    Err(m) => {
                        crate::meldungen::warnung(crate::meldungen::T_ACHTUNG, m);
                        return Some(Dlg3::BlattIndex { anlage, von, bis, ab, plus });
                    }
                    Ok((neu, reihenfolge)) => {
                        if reihenfolge && !crate::meldungen::frage_ok(crate::meldungen::T_ACHTUNG, crate::meldungen::NN_FRAGE) {
                            return Some(Dlg3::BlattIndex { anlage, von, bis, ab, plus });
                        }
                        for (s, n) in self.project.anlagen[anlage].sheets.iter_mut().zip(neu) {
                            s.number = n;
                        }
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
        let k1 = TITEL_DIR1.get(dir1).map(|d| d.1).unwrap_or(0);
        let k2 = titel_dir2(dir1).get(dir2).map(|d| d.1).unwrap_or(2);
        let kurz = self.project.anlagen[r.anlage].set.kurz.clone();
        let Some(s) = self.project.sheet_mut(r) else { return 0 };
        let blatt = s.number;
        let mut idx: Vec<usize> = (0..s.objects.len())
            .filter(|i| (sel.is_empty() || sel.contains(&s.objects[*i].id)) && matches!(s.objects[*i].kind, Kind::Symbol { .. } | Kind::Text { .. }))
            .collect();
        let pos = |o: &Obj| match &o.kind {
            Kind::Symbol { pos, .. } | Kind::Text { pos, .. } => *pos,
            _ => P::default(),
        };
        idx.sort_by(|a, b| {
            let (pa, pb) = (pos(&s.objects[*a]), pos(&s.objects[*b]));
            order_key(pa, k1).partial_cmp(&order_key(pb, k1)).unwrap().then(order_key(pa, k2).partial_cmp(&order_key(pb, k2)).unwrap())
        });
        let pfade: Vec<u32> = idx.iter().map(|i| crate::epr::column_at(s, pos(&s.objects[*i]).x)).collect();
        for (n, (i, pfad)) in idx.iter().zip(pfade).enumerate() {
            let werte = |alt: &str| titel_formel_mit(formel, n, &TitelWerte { blatt, pfad, anlage: &kurz, alt });
            match &mut s.objects[*i].kind {
                Kind::Symbol { bmk, value, ext, .. } => match wahl {
                    1 => *value = werte(value),
                    // Alternativtext: benutzerdefinierter Text Nr. 2 der Instanz (erster Symboltext nach der BMK)
                    2 => {
                        let alt = ext.texts.iter().find(|(k, _)| *k == 2).map(|(_, t)| t.clone()).unwrap_or_default();
                        let t = werte(&alt);
                        ext.texts.retain(|(k, _)| *k != 2);
                        ext.texts.push((2, t));
                    }
                    _ => *bmk = werte(bmk),
                },
                Kind::Text { text, .. } => *text = werte(text),
                _ => {}
            }
        }
        idx.len()
    }
}
