//! Tabelle (Werkzeug „Tabelle“, ZCN Typ 13) wie im Original.
//!
//! Zeichnen nach der Tabellenroutine des Originals (0x4701a4, auch für Listenblätter benutzt):
//! - Zeilenhöhe = Höhe / Zeilen; Schrift (Geviert) = ¾ Zeilenhöhe (Schriftgrad in pt aus der Zeilenhöhe in Pixeln,
//!   gezeichnet wie 96-dpi-Pixel), Kopfzeile fett, Text 1,5 mm vom linken Zellrand, senkrecht mittig.
//! - Ohne „Autobreite“ (oder ohne Inhalt) gleich breite Spalten. Mit „Autobreite“ wird jeder Text aller Zeilen mit
//!   Schrifthöhe = Zeilenhöhe gemessen, je Spalte das Maximum + 3 mm. Passt die Summe in die Breite, bleiben diese
//!   Breiten, die letzte Spalte reicht bis zum rechten Rand (es gibt nur Trennlinien nach den Spalten 1 … n−1);
//!   sonst werden alle Spalten mit dem Faktor Breite/Summe gestaucht (letzte = Rest) und die Schrift mit Faktor × 1,3.
//! - Linien: Rahmen, waagerechte Linien über die ganze Breite, senkrechte über die ganze Höhe.
//! - „Ab“ (ZCN Feld 6): erste angezeigte Datenzeile (mit Überschrift bleibt Zeile 0 die Kopfzeile).
//!
//! Dialog TSTab: Daten, vier Knöpfe (Liste laden/speichern, Alles löschen, Zeile löschen), Überschrift, Autobreite,
//! Fixiert, Spalten, Zeilen, Ab und das Zellenraster (TIconTab). Liste laden/speichern wie im Original als Textdatei
//! (Windows-1252), Spalten durch Tabulator getrennt, Zeilen mit CR LF (Laden: 0x4a69cc/0x490c84, Speichern:
//! 0x4a68fc, TIconTab::Export Art 1). „Zeile löschen“ entfernt die markierte Zeile und hängt unten eine leere an
//! (Zeilenzahl bleibt, 0x4a683c). Bei anderen Daten als „Benutzerdaten“ sind Überschrift, Autobreite, Spalten und
//! Raster gesperrt (0x4a5364).

use crate::canvas::Xf;
use crate::icons::Icon;
use crate::model::P;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Stroke, Vec2};

/// Textabstand vom linken Zellrand in mm (15 × 0,1 mm)
pub const RAND: f32 = 1.5;
/// Autobreite: Zuschlag je Spalte zum längsten Text in mm (30 × 0,1 mm)
pub const ZUSCHLAG: f32 = 3.0;
/// Schrift (Geviert) im Verhältnis zur Zeilenhöhe
pub const SCHRIFT: f32 = 0.75;
/// Größte Zahl der Spalten einer geladenen Liste (Spaltenmaske 32 Bit) und Länge einer Zelle
pub const LADEN_SPALTEN: usize = 32;
pub const LADEN_ZEICHEN: usize = 254;
/// Daten der Tabelle (Tab_Art)
pub const ARTEN: &[&str] = &["Benutzerdaten", "Inhaltsverzeichnis", "Querverweise"];
/// Frage beim Laden einer längeren Liste
pub const FRAGE_ERWEITERN: &str = "Die Tabelle hat nicht genug Zeilen für die geladenen Daten. Soll die Tabelle erweitert werden?";

/// Neue Tabelle wie im Original (Vorgaben 0x41e03a): 2 Spalten, 2 Zeilen, ohne Überschrift, Autobreite,
/// Benutzerdaten, Ab 0, leer.
pub fn neu(a: P, b: P) -> crate::model::Kind {
    crate::model::Kind::Table { a, b, rows: 2, cols: 2, cells: vec![String::new(); 4], header: false, fixed_w: false, art: 0, ab: 0 }
}

/// Aufteilung einer Tabelle: Spaltenbreiten (mm), Zeilenhöhe, Schriftgröße (Geviert, mm) und je angezeigter
/// Zeile die Datenzeile (Index in die Zellen, zeilenweise).
#[derive(Clone, Debug, PartialEq)]
pub struct Aufteilung {
    pub breiten: Vec<f32>,
    pub zeilenhoehe: f32,
    pub schrift: f32,
    pub datenzeilen: Vec<usize>,
}

/// Datenzeile der angezeigten Zeile `r`: mit Überschrift ist Zeile 0 die Kopfzeile, danach geht es ab Zeile `ab`
/// weiter (ab = 0 wie ab = 1); ohne Überschrift beginnt die Anzeige bei Zeile `ab`.
pub fn datenzeile(r: usize, kopf: bool, ab: usize) -> usize {
    match (kopf, r) {
        (true, 0) => 0,
        (true, _) if ab > 0 => ab + r - 1,
        (true, _) => r,
        (false, _) => ab + r,
    }
}

/// Aufteilung wie im Original. `mass(text, fett, schrift)` = Textbreite in mm.
#[allow(clippy::too_many_arguments)]
pub fn aufteilung(cells: &[String], cols: usize, rows: usize, kopf: bool, auto: bool, ab: usize, breite: f32, hoehe: f32, mass: &dyn Fn(&str, bool, f32) -> f32) -> Aufteilung {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let zh = hoehe / rows as f32;
    let mut schrift = SCHRIFT * zh;
    let mut breiten = vec![breite / cols as f32; cols];
    // Autobreite nur mit Inhalt (das Original speichert leere Tabellen ohne Zellen)
    if auto && cells.iter().any(|c| !c.is_empty()) {
        let mut w = vec![0.0f32; cols];
        for (i, t) in cells.iter().enumerate() {
            let fett = kopf && i < cols;
            w[i % cols] = w[i % cols].max(mass(t, fett, zh));
        }
        w.iter_mut().for_each(|x| *x += ZUSCHLAG);
        let summe: f32 = w.iter().sum();
        let f = if summe > 0.0 { breite / summe } else { 1.0 };
        if f < 1.0 {
            let mut s = 0.0;
            for x in w.iter_mut().take(cols - 1) {
                *x *= f;
                s += *x;
            }
            w[cols - 1] = breite - s;
            schrift *= f * 1.3;
        } else {
            // die letzte Spalte reicht bis zum Rahmen
            let s: f32 = w[..cols - 1].iter().sum();
            w[cols - 1] = breite - s;
        }
        breiten = w;
    }
    let datenzeilen = (0..rows).map(|r| datenzeile(r, kopf, ab)).collect();
    Aufteilung { breiten, zeilenhoehe: zh, schrift, datenzeilen }
}

/// Textbreite in mm mit der Schrift `name` (leer = Times), wie sie auf dem Blatt gezeichnet wird.
pub fn textbreite(name: &str, text: &str, fett: bool, size: f32) -> f32 {
    let m = crate::schriftmass::metrik(name, fett, false);
    crate::textmass::groesse_mit(m.as_deref(), text, size, fett, false).0
}

/// Tabelle zeichnen: Rahmen, Linien und Zellen (`kopf` = Überschrift fett, `auto` = Autobreite).
#[allow(clippy::too_many_arguments)]
pub fn zeichnen(p: &egui::Painter, xf: Xf, mn: P, mx: P, cells: &[String], cols: usize, rows: usize, kopf: bool, auto: bool, ab: usize, schrift_name: &str, stroke: Stroke, tc: Color32) {
    let cols = cols.max(1);
    let z = xf.z;
    let a = aufteilung(cells, cols, rows, kopf, auto, ab, mx.x - mn.x, mx.y - mn.y, &|t, f, s| textbreite(schrift_name, t, f, s));
    p.rect_stroke(Rect::from_min_max(xf.s(mn), xf.s(mx)), 0.0, stroke, egui::StrokeKind::Middle);
    for r in 1..a.datenzeilen.len() {
        let y = mn.y + r as f32 * a.zeilenhoehe;
        p.line_segment([xf.s(P::new(mn.x, y)), xf.s(P::new(mx.x, y))], stroke);
    }
    let mut x = mn.x;
    for w in &a.breiten[..cols - 1] {
        x += w;
        p.line_segment([xf.s(P::new(x, mn.y)), xf.s(P::new(x, mx.y))], stroke);
    }
    for (r, &d) in a.datenzeilen.iter().enumerate() {
        let fett = kopf && r == 0;
        let top = mn.y + r as f32 * a.zeilenhoehe;
        let mut x = mn.x;
        for (c, w) in a.breiten.iter().enumerate() {
            if let Some(t) = cells.get(d * cols + c).filter(|t| !t.is_empty()) {
                // zu lange Texte werden kleiner geschrieben, bis sie in die Zelle passen
                let tw = textbreite(schrift_name, t, fett, a.schrift);
                let platz = (w - RAND).max(0.1);
                let s = if tw > platz { a.schrift * platz / tw } else { a.schrift };
                let font = crate::schriften::font_id(schrift_name, s * z, fett, false);
                let y = top + (a.zeilenhoehe - s) / 2.0;
                p.text(xf.s(P::new(x + RAND, y)), Align2::LEFT_TOP, t, font, tc);
            }
            x += w;
        }
    }
}

// ---------------------------------------------------------------- Liste laden/speichern

/// Text einer Liste lesen: Windows-1252 (wie das Original), gültiges UTF-8 mit Umlauten wird ebenfalls erkannt.
pub fn text_aus_bytes(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) if !s.is_ascii() => s.trim_start_matches('\u{feff}').to_string(),
        _ => crate::symbols::cstr(&b.iter().map(|&c| if c == 0 { b' ' } else { c }).collect::<Vec<u8>>()),
    }
}

/// Liste wie das Original zerlegen: Zeilen durch LF (CR entfällt), Spalten durch Tabulator, je Zelle höchstens
/// 254 Zeichen, höchstens 32 Spalten.
pub fn liste_lesen(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return vec![];
    }
    text.split('\n')
        .map(|z| z.replace('\r', "").split('\t').take(LADEN_SPALTEN).map(|c| c.chars().take(LADEN_ZEICHEN).collect()).collect())
        .collect()
}

/// Liste wie TIconTab::Export (Art 1): Zellen durch Tabulator, jede Zeile mit CR LF, Windows-1252.
pub fn liste_schreiben(cells: &[Vec<String>]) -> Vec<u8> {
    let mut s = String::new();
    for r in cells {
        s.push_str(&r.iter().map(|c| c.replace(['\t', '\r', '\n'], " ")).collect::<Vec<_>>().join("\t"));
        s.push_str("\r\n");
    }
    crate::eprfile::encode_cp1252(&s)
}

/// Geladene Liste ins Raster übernehmen: mit `erweitern` wächst die Zeilenzahl auf die der Liste; Spalten bleiben.
/// Geladene Zeilen werden in allen Spalten überschrieben (fehlende Zellen leer), weitere Zeilen bleiben.
pub fn liste_uebernehmen(grid: &mut Vec<Vec<String>>, liste: &[Vec<String>], erweitern: bool) {
    let cols = grid.first().map(|r| r.len()).unwrap_or(1).max(1);
    if erweitern && liste.len() > grid.len() {
        grid.resize(liste.len(), vec![String::new(); cols]);
    }
    for (r, zeile) in grid.iter_mut().zip(liste) {
        for (c, z) in r.iter_mut().enumerate() {
            *z = if c < LADEN_SPALTEN { zeile.get(c).cloned().unwrap_or_default() } else { String::new() };
        }
    }
}

/// „Zeile löschen“: markierte Zeile entfernen, unten eine leere anhängen (Zeilenzahl bleibt).
pub fn zeile_loeschen(grid: &mut Vec<Vec<String>>, zeile: Option<usize>) {
    let Some(r) = zeile.filter(|r| *r < grid.len()) else { return };
    let cols = grid[r].len();
    grid.remove(r);
    grid.push(vec![String::new(); cols]);
}

/// Zahl aus „Spalten“/„Zeilen“: Werte unter 2 (und Unlesbares) gelten als 1.
pub fn anzahl(s: &str, max: usize) -> usize {
    s.trim().parse::<i64>().unwrap_or(1).clamp(1, max as i64) as usize
}

/// Raster auf `rows` × `cols` bringen (Zeilen/Spalten hinten entfernen oder anhängen).
pub fn raster_groesse(grid: &mut Vec<Vec<String>>, rows: usize, cols: usize) {
    grid.resize(rows.max(1), vec![String::new(); cols.max(1)]);
    for r in grid.iter_mut() {
        r.resize(cols.max(1), String::new());
    }
}

// ---------------------------------------------------------------- Dialog TSTab

/// Zustand des Eigenschaftendialogs einer Tabelle.
pub struct TabDlg {
    pub id: u64,
    pub art: usize,
    pub spalten: String,
    pub zeilen: String,
    pub header: bool,
    pub auto: bool,
    pub fixiert: bool,
    pub ab: String,
    pub cells: Vec<Vec<String>>,
    /// markierte Zeile des Rasters (TIconTab::GetRow)
    pub sel_row: Option<usize>,
    /// Meldung nach Laden/Speichern
    pub msg: String,
}

impl TabDlg {
    /// Dialog für eine Tabelle füllen (Raster nur bei Benutzerdaten wie im Original).
    #[allow(clippy::too_many_arguments)]
    pub fn neu(id: u64, rows: usize, cols: usize, cells: &[String], header: bool, fixed_w: bool, fixiert: bool, art: u8, ab: u32) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let grid = (0..rows).map(|r| (0..cols).map(|c| cells.get(r * cols + c).cloned().unwrap_or_default()).collect()).collect();
        TabDlg { id, art: (art as usize).min(ARTEN.len() - 1), spalten: cols.to_string(), zeilen: rows.to_string(), header, auto: !fixed_w, fixiert, ab: ab.to_string(), cells: grid, sel_row: None, msg: String::new() }
    }

    pub fn benutzerdaten(&self) -> bool {
        self.art == 0
    }

    /// Werte für das Objekt: (Zeilen, Spalten, Zellen zeilenweise, Überschrift, feste Breite, Art, Ab).
    pub fn werte(&self) -> (usize, usize, Vec<String>, bool, bool, u8, u32) {
        let rows = anzahl(&self.zeilen, 500);
        let cols = anzahl(&self.spalten, 50);
        let mut grid = self.cells.clone();
        raster_groesse(&mut grid, rows, cols);
        let ab = self.ab.trim().parse::<i64>().unwrap_or(0).max(0) as u32;
        (rows, cols, grid.into_iter().flatten().collect(), self.header, !self.auto, self.art as u8, ab)
    }

    /// Spalten/Zeilen ins Raster übernehmen (Tab_ColChange, nur bei Benutzerdaten).
    pub fn groesse_uebernehmen(&mut self) {
        if self.benutzerdaten() {
            let (r, c) = (anzahl(&self.zeilen, 500), anzahl(&self.spalten, 50));
            raster_groesse(&mut self.cells, r, c);
            if self.sel_row.is_some_and(|s| s >= r) {
                self.sel_row = Some(r - 1);
            }
        }
    }

    /// Geladene Liste übernehmen; `frage` entscheidet über das Erweitern (Ja/Nein).
    pub fn laden(&mut self, liste: &[Vec<String>], frage: impl FnOnce() -> bool) {
        let erweitern = liste.len() > self.cells.len() && frage();
        liste_uebernehmen(&mut self.cells, liste, erweitern);
        self.zeilen = self.cells.len().to_string();
    }
}

/// Knopf mit eigenem Icon (23 × 22 wie die TSpeedButtons) und dem Hinweis des Originals.
fn icon_knopf(ui: &mut egui::Ui, icon: Icon, hint: &str) -> bool {
    ui.add(egui::Button::image(crate::icons::image(icon, 16.0)).min_size(Vec2::new(23.0, 22.0))).on_hover_text(hint).clicked()
}

fn beschriftung(ui: &mut egui::Ui, w: f32, t: &str, enabled: bool) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
    let col = if enabled { Color32::BLACK } else { Color32::GRAY };
    ui.painter().text(r.right_center(), Align2::RIGHT_CENTER, t, crate::props::bold(13.0), col);
}

/// Eingabefeld (TIn); Änderungen gelten beim Verlassen oder mit Eingabe.
fn eingabe(ui: &mut egui::Ui, s: &mut String, enabled: bool) -> bool {
    let r = ui.add_enabled(enabled, egui::TextEdit::singleline(s).desired_width(44.0));
    r.lost_focus()
}

/// Dialog zeichnen (Inhalt des Fensters „Eigenschaften“).
pub fn dialog_ui(ui: &mut egui::Ui, d: &mut TabDlg) {
    let mut knopf = 0u8;
    ui.horizontal(|ui| {
        beschriftung(ui, 50.0, "Daten", true);
        let mut art = d.art;
        egui::ComboBox::from_id_salt("tab_art").width(137.0).selected_text(ARTEN[art]).show_ui(ui, |ui| {
            for (i, t) in ARTEN.iter().enumerate() {
                ui.selectable_value(&mut art, i, *t);
            }
        });
        d.art = art;
        ui.spacing_mut().item_spacing.x = 1.0;
        if icon_knopf(ui, Icon::Open, "Liste laden") {
            knopf = 1;
        }
        if icon_knopf(ui, Icon::Save, "Liste speichern") {
            knopf = 2;
        }
        if icon_knopf(ui, Icon::Delete, "Alles löschen") {
            knopf = 3;
        }
        if icon_knopf(ui, Icon::DelZeile, "Zeile löschen") {
            knopf = 4;
        }
    });
    let benutzer = d.benutzerdaten();
    let fett = |t: &str| RichText::new(t).font(crate::props::bold(13.0));
    let mut groesse = false;
    ui.horizontal(|ui| {
        ui.add_enabled(benutzer, egui::Checkbox::new(&mut d.header, fett("Überschrift")));
        ui.add_space(8.0);
        beschriftung(ui, 60.0, "Spalten", benutzer);
        groesse |= eingabe(ui, &mut d.spalten, benutzer);
    });
    ui.horizontal(|ui| {
        ui.add_enabled(benutzer, egui::Checkbox::new(&mut d.auto, fett("Autobreite")));
        ui.add_space(14.0);
        beschriftung(ui, 60.0, "Zeilen", true);
        groesse |= eingabe(ui, &mut d.zeilen, true);
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut d.fixiert, fett("Fixiert"));
        ui.add_space(30.0);
        beschriftung(ui, 60.0, "Ab", true);
        eingabe(ui, &mut d.ab, true);
    });
    if groesse {
        d.groesse_uebernehmen();
    }
    ui.add_enabled_ui(benutzer, |ui| raster(ui, d));
    if !d.msg.is_empty() {
        ui.label(&d.msg);
    }
    match knopf {
        1 => {
            if let Some(pfad) = rfd::FileDialog::new().add_filter("Textdatei", &["txt"]).pick_file() {
                match std::fs::read(&pfad) {
                    Ok(b) => {
                        let liste = liste_lesen(&text_aus_bytes(&b));
                        if !liste.is_empty() {
                            d.laden(&liste, || {
                                rfd::MessageDialog::new()
                                    .set_title("Frage?")
                                    .set_description(FRAGE_ERWEITERN)
                                    .set_buttons(rfd::MessageButtons::YesNo)
                                    .show()
                                    == rfd::MessageDialogResult::Yes
                            });
                        }
                        d.msg.clear();
                    }
                    Err(e) => d.msg = format!("Liste nicht lesbar: {e}"),
                }
            }
        }
        2 => {
            if let Some(pfad) = rfd::FileDialog::new().add_filter("Textdatei (Tabulator)", &["txt"]).set_file_name("Tabelle.txt").save_file() {
                d.msg = match std::fs::write(&pfad, liste_schreiben(&d.cells)) {
                    Ok(()) => String::new(),
                    Err(e) => format!("Liste nicht gespeichert: {e}"),
                };
            }
        }
        3 => d.cells.iter_mut().flatten().for_each(|c| c.clear()),
        4 => zeile_loeschen(&mut d.cells, d.sel_row),
        _ => {}
    }
}

/// Zellenraster (TIconTab): Spaltennummern, Zeilennummern mit Markierung der aktuellen Zeile, Zellen zum
/// Bearbeiten. Pfeil hoch/runter und Eingabe wechseln die Zeile, Tabulator die Zelle.
fn raster(ui: &mut egui::Ui, d: &mut TabDlg) {
    let cols = d.cells.first().map(|r| r.len()).unwrap_or(1).max(1);
    let nr_w = 26.0;
    let w = ((ui.available_width() - nr_w - 12.0) / cols as f32 - 4.0).clamp(36.0, 140.0);
    let zelle_id = |r: usize, c: usize| egui::Id::unique(("tab_zelle", r, c));
    let mut ziel: Option<(usize, usize)> = None;
    egui::ScrollArea::both().id_salt("tab_raster").max_height(ui.available_height().max(120.0) - 4.0).show(ui, |ui| {
        egui::Grid::new("tab_raster_g").striped(true).spacing(Vec2::new(4.0, 3.0)).num_columns(cols + 1).show(ui, |ui| {
            ui.allocate_exact_size(Vec2::new(nr_w, 14.0), egui::Sense::hover());
            for c in 0..cols {
                ui.label(RichText::new((c + 1).to_string()).strong());
            }
            ui.end_row();
            let n = d.cells.len();
            for r in 0..n {
                let markiert = d.sel_row == Some(r);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(nr_w, 20.0), egui::Sense::click());
                if markiert {
                    ui.painter().rect_filled(rect, 2.0, ui.visuals().selection.bg_fill);
                }
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, (r + 1).to_string(), egui::FontId::proportional(13.0), ui.visuals().text_color());
                if resp.clicked() {
                    d.sel_row = Some(r);
                }
                for c in 0..cols {
                    let id = zelle_id(r, c);
                    let resp = ui.add(egui::TextEdit::singleline(&mut d.cells[r][c]).id(id).desired_width(w));
                    if resp.gained_focus() || resp.has_focus() {
                        d.sel_row = Some(r);
                    }
                    if resp.has_focus() {
                        let (hoch, runter) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
                        if hoch && r > 0 {
                            ziel = Some((r - 1, c));
                        } else if runter && r + 1 < n {
                            ziel = Some((r + 1, c));
                        }
                    } else if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && r + 1 < n {
                        ziel = Some((r + 1, c));
                    }
                }
                ui.end_row();
            }
        });
    });
    if let Some((r, c)) = ziel {
        d.sel_row = Some(r);
        ui.memory_mut(|m| m.request_focus(zelle_id(r, c)));
    }
}

/// Lage des Textes einer Zelle in mm (links oben der Schrift, ohne Verkleinerung zu langer Texte).
pub fn textlage(a: &Aufteilung, mn: P, r: usize, c: usize) -> Pos2 {
    let x = mn.x + a.breiten[..c].iter().sum::<f32>() + RAND;
    let y = mn.y + r as f32 * a.zeilenhoehe + (a.zeilenhoehe - a.schrift) / 2.0;
    Pos2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Messung wie mit fester Zeichenbreite: 0,5 Geviert je Zeichen, fett 10 % breiter
    fn m(t: &str, fett: bool, s: f32) -> f32 {
        t.chars().count() as f32 * 0.5 * s * if fett { 1.1 } else { 1.0 }
    }
    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn leere_tabelle_gleich_breite_spalten() {
        // ohne Inhalt keine 3-mm-Spalten (früher doppelte Linien links)
        let a = aufteilung(&[], 3, 3, true, true, 0, 90.0, 30.0, &m);
        assert_eq!(a.breiten, vec![30.0; 3]);
        assert_eq!(a.zeilenhoehe, 10.0);
        assert!((a.schrift - 7.5).abs() < 1e-4);
        let a = aufteilung(&v(&["", "", ""]), 3, 1, false, true, 0, 90.0, 10.0, &m);
        assert_eq!(a.breiten, vec![30.0; 3]);
    }

    #[test]
    fn autobreite_letzte_spalte_fuellt() {
        // Zeilenhöhe 8 mm: gemessen mit 8 mm Schrift, „Nr“ fett = 2 × 0,5 × 8 × 1,1 = 8,8 + 3
        let a = aufteilung(&v(&["Nr", "Name", "1", "Lampe"]), 2, 2, true, true, 0, 100.0, 16.0, &m);
        assert!((a.breiten[0] - 11.8).abs() < 1e-3, "{:?}", a.breiten);
        assert!((a.breiten.iter().sum::<f32>() - 100.0).abs() < 1e-3);
        assert!((a.schrift - 6.0).abs() < 1e-4);
    }

    #[test]
    fn autobreite_zu_breit_wird_gestaucht() {
        let a = aufteilung(&v(&["AAAAAAAAAA", "BBBBBBBBBB"]), 2, 1, false, true, 0, 40.0, 10.0, &m);
        // je 10 × 0,5 × 10 + 3 = 53 → Faktor 40/106
        let f = 40.0 / 106.0;
        assert!((a.breiten[0] - 53.0 * f).abs() < 1e-3);
        assert!((a.breiten.iter().sum::<f32>() - 40.0).abs() < 1e-3);
        assert!((a.schrift - 7.5 * f * 1.3).abs() < 1e-3);
    }

    #[test]
    fn ohne_autobreite_gleich() {
        let a = aufteilung(&v(&["sehr langer Text", "x"]), 2, 1, false, false, 0, 50.0, 10.0, &m);
        assert_eq!(a.breiten, vec![25.0, 25.0]);
    }

    #[test]
    fn ab_waehlt_datenzeilen() {
        assert_eq!((0..3).map(|r| datenzeile(r, true, 0)).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!((0..3).map(|r| datenzeile(r, true, 4)).collect::<Vec<_>>(), vec![0, 4, 5]);
        assert_eq!((0..3).map(|r| datenzeile(r, false, 2)).collect::<Vec<_>>(), vec![2, 3, 4]);
        let a = aufteilung(&[], 1, 3, true, false, 4, 10.0, 30.0, &m);
        assert_eq!(a.datenzeilen, vec![0, 4, 5]);
    }

    #[test]
    fn textlage_links_und_mittig() {
        let a = aufteilung(&[], 2, 2, false, false, 0, 40.0, 20.0, &m);
        let p = textlage(&a, P::new(10.0, 10.0), 1, 1);
        assert!((p.x - 31.5).abs() < 1e-4);
        assert!((p.y - (20.0 + 1.25)).abs() < 1e-4);
    }

    #[test]
    fn liste_lesen_wie_original() {
        let l = liste_lesen("a\tb\r\nc\td\r\n");
        assert_eq!(l, vec![v(&["a", "b"]), v(&["c", "d"])]);
        // ohne Zeilenende am Schluss, leere Zeile, Zelle auf 254 Zeichen gekürzt
        let lang = "x".repeat(300);
        let l = liste_lesen(&format!("1\n\n{lang}"));
        assert_eq!(l.len(), 3);
        assert_eq!(l[1], v(&[""]));
        assert_eq!(l[2][0].len(), 254);
        assert!(liste_lesen("").is_empty());
        // Windows-1252
        assert_eq!(text_aus_bytes(b"\xc4rger\t\x80"), "Ärger\t€");
        assert_eq!(text_aus_bytes("Ärger".as_bytes()), "Ärger");
    }

    #[test]
    fn liste_schreiben_tabulator_crlf() {
        let b = liste_schreiben(&[v(&["Nr", "Ä"]), v(&["1", ""])]);
        assert_eq!(b, b"Nr\t\xc4\r\n1\t\r\n");
        assert_eq!(liste_lesen(&text_aus_bytes(&b)), vec![v(&["Nr", "Ä"]), v(&["1", ""])]);
    }

    #[test]
    fn laden_mit_und_ohne_erweitern() {
        let liste = vec![v(&["a", "b", "c"]), v(&["d"]), v(&["e", "f"])];
        let mut d = TabDlg::neu(1, 2, 2, &v(&["1", "2", "3", "4"]), false, false, false, 0, 0);
        d.laden(&liste, || false);
        assert_eq!(d.cells, vec![v(&["a", "b"]), v(&["d", ""])]);
        assert_eq!(d.zeilen, "2");
        let mut d = TabDlg::neu(1, 2, 2, &[], false, false, false, 0, 0);
        d.laden(&liste, || true);
        assert_eq!(d.cells.len(), 3);
        assert_eq!(d.zeilen, "3");
        assert_eq!(d.cells[2], v(&["e", "f"]));
        // kürzere Liste: übrige Zeilen bleiben, Frage entfällt
        let mut d = TabDlg::neu(1, 3, 1, &v(&["x", "y", "z"]), false, false, false, 0, 0);
        d.laden(&[v(&["a"])], || panic!("keine Frage"));
        assert_eq!(d.cells, vec![v(&["a"]), v(&["y"]), v(&["z"])]);
    }

    #[test]
    fn zeile_loeschen_haengt_leere_an() {
        let mut g = vec![v(&["1", "a"]), v(&["2", "b"]), v(&["3", "c"])];
        zeile_loeschen(&mut g, Some(1));
        assert_eq!(g, vec![v(&["1", "a"]), v(&["3", "c"]), v(&["", ""])]);
        zeile_loeschen(&mut g, None);
        zeile_loeschen(&mut g, Some(7));
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn groesse_und_werte() {
        let mut d = TabDlg::neu(1, 2, 2, &v(&["a", "b", "c", "d"]), true, false, false, 0, 0);
        d.spalten = "3".into();
        d.zeilen = "0".into();
        d.groesse_uebernehmen();
        assert_eq!(d.cells, vec![v(&["a", "b", ""])]);
        let (rows, cols, cells, header, fixed, art, ab) = d.werte();
        assert_eq!((rows, cols, header, fixed, art, ab), (1, 3, true, false, 0, 0));
        assert_eq!(cells, v(&["a", "b", ""]));
        // andere Daten: Raster bleibt beim Ändern der Zeilen, erst „OK“ passt an
        let mut d = TabDlg::neu(1, 2, 1, &v(&["a", "b"]), true, false, false, 1, 3);
        d.zeilen = "4".into();
        d.groesse_uebernehmen();
        assert_eq!(d.cells.len(), 2);
        let (rows, _, cells, _, _, art, ab) = d.werte();
        assert_eq!((rows, cells.len(), art, ab), (4, 4, 1, 3));
    }
}
