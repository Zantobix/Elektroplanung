//! Suchen über alle Blätter (Bearbeiten › Suchen…, Strg+F) und Fenster „Projekt prüfen“ (Tools) mit Sprung
//! zum Objekt; Rückfrage vor dem Drucken/PDF-Export, wenn die Prüfung Hinweise findet.

use crate::app::{App, View};
use crate::canvas;
use crate::model::*;
use crate::pruefen::{self, Hinweis};
use crate::symbols;
use eframe::egui::{self, Color32, RichText, Vec2};
use std::path::PathBuf;

/// Ein Suchtreffer: Blatt, Objekt, Art des Textes und der gefundene Text.
#[derive(Clone, Debug, PartialEq)]
pub struct Treffer {
    pub sheet: SheetRef,
    pub id: u64,
    pub art: &'static str,
    pub text: String,
}

/// Durchsuchbare Texte eines Objekts, wie sie auf dem Blatt erscheinen: Texte, Tabellenzellen, Verweise,
/// angezeigte BMK, Werte und Symboltexte, Verbindungsbezeichnungen, Sammelschienen und Klemmen.
pub fn objekt_texte(project: &Project, a: &Anlage, s: &Sheet, o: &Obj) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = vec![];
    match &o.kind {
        Kind::Text { text, .. } => v.push(("Text", project.expand_placeholders(text))),
        Kind::Table { cells, .. } => v.extend(cells.iter().map(|c| ("Tabelle", c.clone()))),
        Kind::Verweis { text, .. } => v.push(("Verweis", project.expand_placeholders(text))),
        Kind::Symbol { bmk, value, ext, .. } => {
            v.push(("BMK", symbols::display_bmk(bmk, ext.counter, &canvas::bmk_ctx(a, s))));
            v.push(("Wert", project.expand_placeholders(value)));
            v.extend(ext.texts.iter().map(|(_, t)| ("Symboltext", t.clone())));
        }
        Kind::Wire { bez, leitung, .. } => {
            let name = project.leitungen.get(*leitung).map(|l| l.name.as_str()).unwrap_or("");
            v.extend(bez.iter().map(|(_, t)| ("Verbindung", t.replace("%O", name))));
        }
        Kind::Busbar { name, .. } => v.push(("Sammelschiene", name.clone())),
        Kind::Terminal { leiste, nr, level, tpl, .. } => v.push(("Klemme", crate::epr::terminal_label(tpl, leiste, *nr, *level))),
        _ => {}
    }
    v.retain(|(_, t)| !t.trim().is_empty());
    v
}

/// `query` in allen Anlagen und Blättern suchen (Teiltext; ohne `gross_klein` ohne Beachtung der Schreibweise).
pub fn suchen(project: &Project, query: &str, gross_klein: bool) -> Vec<Treffer> {
    let q = query.trim();
    if q.is_empty() {
        return vec![];
    }
    let norm = |s: &str| if gross_klein { s.to_string() } else { s.to_lowercase() };
    let nq = norm(q);
    let mut out = vec![];
    for (ai, a) in project.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                for (art, t) in objekt_texte(project, a, s, o) {
                    if norm(&t).contains(&nq) {
                        out.push(Treffer { sheet: SheetRef { anlage: ai, sheet: si }, id: o.id, art, text: t.replace('\n', " / ") });
                    }
                }
            }
        }
    }
    out
}

/// Zustand der Fenster dieses Moduls (ein Feld in `App`).
#[derive(Default)]
pub struct SucheState {
    pub suche: Option<SucheDlg>,
    pub pruef: Option<PruefDlg>,
    /// Rückfrage vor der Ausgabe: zurückgehaltener PDF-Auftrag (Datei, drucken) und Zahl der Hinweise
    pub rueckfrage: Option<((PathBuf, bool), usize)>,
    /// Ausgabe nach „trotzdem drucken“ ohne erneute Prüfung zulassen
    pub freigegeben: bool,
    pub bild: Option<crate::bildexport::BildDlg>,
}

pub struct SucheDlg {
    pub text: String,
    pub gross_klein: bool,
    pub treffer: Vec<Treffer>,
    pub sel: Option<usize>,
    fokus: bool,
    gesucht: Option<(String, bool)>,
}

pub struct PruefDlg {
    pub hinweise: Vec<Hinweis>,
    pub sel: Option<usize>,
}

/// Anlagen- und Blattname für Listen.
fn blatt_namen(p: &Project, r: SheetRef) -> (String, String) {
    let a = p.anlagen.get(r.anlage);
    let s = a.and_then(|a| a.sheets.get(r.sheet));
    (a.map(|a| a.name.clone()).unwrap_or_default(), s.map(|s| format!("{} {}", s.number, s.name)).unwrap_or_default())
}

/// Liste mit Spalten Anlage / Blatt / Art / Text; Rückgabe: angeklickte Zeile.
fn liste(ui: &mut egui::Ui, id: &str, rows: &[(SheetRef, &str, &str)], p: &Project, sel: Option<usize>, height: f32) -> Option<usize> {
    let mut clicked = None;
    let widths = [110.0, 130.0, 105.0];
    egui::ScrollArea::vertical().id_salt(id).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new(id).striped(true).num_columns(4).spacing([6.0, 2.0]).show(ui, |ui| {
            for (i, t) in ["Anlage", "Blatt", "Art", "Text"].iter().enumerate() {
                let rt = RichText::new(*t).strong();
                if i < 3 {
                    ui.add_sized([widths[i], 16.0], egui::Label::new(rt));
                } else {
                    ui.label(rt);
                }
            }
            ui.end_row();
            for (i, (r, art, text)) in rows.iter().enumerate() {
                let (an, bl) = blatt_namen(p, *r);
                let on = sel == Some(i);
                let mut hit = false;
                for (k, t) in [an.as_str(), bl.as_str(), *art].iter().enumerate() {
                    hit |= ui.add_sized([widths[k], 16.0], egui::Button::selectable(on, *t).truncate()).clicked();
                }
                hit |= ui.add(egui::Button::selectable(on, *text)).clicked();
                if hit {
                    clicked = Some(i);
                }
                ui.end_row();
            }
        });
    });
    clicked
}

impl App {
    /// Bearbeiten › Suchen… (Strg+F): Fenster öffnen bzw. das Suchfeld wieder in den Vordergrund holen.
    pub fn open_suche(&mut self) {
        match &mut self.suche.suche {
            Some(d) => d.fokus = true,
            None => self.suche.suche = Some(SucheDlg { text: String::new(), gross_klein: false, treffer: vec![], sel: None, fokus: true, gesucht: None }),
        }
    }

    /// Tools › Projekt prüfen
    pub fn open_pruefen(&mut self) {
        let hinweise = pruefen::pruefen(&self.project, &self.lib);
        self.info = format!("Projekt prüfen: {} Hinweise", hinweise.len());
        self.suche.pruef = Some(PruefDlg { hinweise, sel: None });
    }

    /// Blatt öffnen, Objekt markieren und die Ansicht auf `at` (bzw. die Mitte des Objekts) legen.
    pub fn springe_zu(&mut self, r: SheetRef, id: u64, at: Option<P>) {
        if self.sym_edit.is_some() {
            self.info = "Erst den Symboleditor schließen".into();
            return;
        }
        let Some(s) = self.project.sheet(r) else { return };
        let (sw, sh) = s.dims();
        let portrait = s.portrait;
        let obj = s.objects.iter().find(|o| o.id == id);
        let center = at.or_else(|| {
            obj.map(|o| {
                let (a, b) = canvas::obj_bounds(o, &self.lib);
                P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
            })
        });
        let exists = obj.is_some();
        self.open_sheet(r);
        if exists {
            self.selection = vec![id];
        }
        let area = self.canvas_area;
        let Some(c) = center.filter(|_| area.width() > 20.0 && area.height() > 20.0) else { return };
        // mindestens doppelte Blattgröße, eine stärkere Vergrößerung des Blattes bleibt erhalten
        let fit = ((area.width() - 10.0) / sw).min((area.height() - 10.0) / sh).max(0.5);
        let cur = if self.view_sheet == Some(r) { Some(self.view.zoom) } else { self.sheet_views.get(&r).map(|v| v.zoom) };
        let z = cur.unwrap_or(fit).max(fit * 2.0);
        let v = View {
            zoom: z,
            offset: Vec2::new((area.width() / 2.0 - c.x * z).round(), (area.height() / 2.0 - c.y * z).round()),
            user: true,
            last_size: area.size(),
            last_portrait: portrait,
        };
        if self.view_sheet == Some(r) {
            self.view = v;
        } else {
            self.sheet_views.insert(r, v);
        }
    }

    /// Vor `make_pdf`: bei eingeschalteter Option das Projekt prüfen und bei Hinweisen den Auftrag zurückhalten.
    /// Rückgabe true = Ausgabe darf laufen.
    pub fn ausgabe_freigeben(&mut self) -> bool {
        if self.pdf_req.is_none() || std::mem::take(&mut self.suche.freigegeben) || !self.opt.pruefen_vor_druck {
            return true;
        }
        let sel = self.print_sel.clone();
        let n = pruefen::pruefen(&self.project, &self.lib).iter().filter(|h| sel.as_ref().is_none_or(|v| v.contains(&h.sheet))).count();
        if n == 0 {
            return true;
        }
        if let Some(req) = self.pdf_req.take() {
            self.suche.rueckfrage = Some((req, n));
        }
        false
    }

    /// Fenster Suchen, Projekt prüfen, Rückfrage vor der Ausgabe und Bildexport.
    pub fn suche_ui(&mut self, ctx: &egui::Context) {
        self.suche_fenster(ctx);
        self.pruef_fenster(ctx);
        self.rueckfrage_fenster(ctx);
        self.bild_export_ui(ctx);
    }

    fn suche_fenster(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.suche.suche.take() else { return };
        let mut jump: Option<usize> = None;
        let mut neu = false;
        let closed = crate::props::tool_window(ctx, "suche", "Suchen", [640.0, 380.0], egui::Order::Middle, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Suchen nach").font(crate::props::bold(13.0)));
                let r = ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut d.text));
                if d.fokus {
                    r.request_focus();
                    d.fokus = false;
                }
                // Eingabe: nächster Treffer
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !d.treffer.is_empty() {
                    jump = Some(d.sel.map(|s| (s + 1) % d.treffer.len()).unwrap_or(0));
                    r.request_focus();
                }
                ui.checkbox(&mut d.gross_klein, "Groß-/Kleinschreibung beachten");
                neu |= ui.button("Suchen").clicked();
            });
            ui.label(format!("{} Treffer (Texte, BMK, Werte, Verbindungsbezeichnungen, Klemmen)", d.treffer.len()));
            ui.add_space(2.0);
            let rows: Vec<(SheetRef, &str, &str)> = d.treffer.iter().map(|t| (t.sheet, t.art, t.text.as_str())).collect();
            if let Some(i) = liste(ui, "suche_liste", &rows, &self.project, d.sel, 300.0) {
                jump = Some(i);
            }
        });
        if neu || d.gesucht.as_ref() != Some(&(d.text.clone(), d.gross_klein)) {
            d.treffer = suchen(&self.project, &d.text, d.gross_klein);
            d.gesucht = Some((d.text.clone(), d.gross_klein));
            d.sel = None;
        }
        if let Some(t) = jump.and_then(|i| d.treffer.get(i).cloned().map(|t| (i, t))) {
            d.sel = Some(t.0);
            self.springe_zu(t.1.sheet, t.1.id, None);
        }
        if !closed && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.suche.suche = Some(d);
        }
    }

    fn pruef_fenster(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.suche.pruef.take() else { return };
        let mut jump: Option<usize> = None;
        let mut again = false;
        let titel = format!("Projekt prüfen – {} Hinweise", d.hinweise.len());
        let closed = crate::props::tool_window(ctx, "pruefen", &titel, [640.0, 380.0], egui::Order::Middle, |ui| {
            ui.horizontal(|ui| {
                ui.label("Offene Anschlüsse (kein Leitungsende, Anschluss oder Klemme näher als 0,05 mm) und doppelte BMK je Anlage");
            });
            ui.horizontal(|ui| {
                again = ui.button("Erneut prüfen").clicked();
                if d.hinweise.is_empty() {
                    ui.colored_label(Color32::from_rgb(0, 120, 0), "Keine Hinweise");
                }
            });
            ui.add_space(2.0);
            let rows: Vec<(SheetRef, &str, &str)> = d.hinweise.iter().map(|h| (h.sheet, h.art.name(), h.text.as_str())).collect();
            if let Some(i) = liste(ui, "pruef_liste", &rows, &self.project, d.sel, 300.0) {
                jump = Some(i);
            }
        });
        if again {
            d.hinweise = pruefen::pruefen(&self.project, &self.lib);
            d.sel = None;
        }
        if let Some((i, h)) = jump.and_then(|i| d.hinweise.get(i).cloned().map(|h| (i, h))) {
            d.sel = Some(i);
            self.springe_zu(h.sheet, h.id, Some(h.pos));
        }
        if !closed {
            self.suche.pruef = Some(d);
        }
    }

    fn rueckfrage_fenster(&mut self, ctx: &egui::Context) {
        let Some((req, n)) = self.suche.rueckfrage.take() else { return };
        let print = req.1;
        let mut choice: Option<u8> = None;
        let closed = crate::props::tool_window(ctx, "pruef_frage", "Elektroplanung", [330.0, 70.0], egui::Order::Foreground, |ui| {
            ui.label(format!("Es gibt {n} Hinweise – trotzdem {}?", if print { "drucken" } else { "exportieren" }));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_sized([100.0, 24.0], egui::Button::new(if print { "Trotzdem drucken" } else { "Exportieren" })).clicked() {
                    choice = Some(0);
                }
                if ui.add_sized([100.0, 24.0], egui::Button::new("Anzeigen")).clicked() {
                    choice = Some(1);
                }
                if ui.add_sized([100.0, 24.0], egui::Button::new("Abbrechen")).clicked() {
                    choice = Some(2);
                }
            });
        });
        match (closed, choice) {
            (false, Some(0)) => {
                self.pdf_req = Some(req);
                self.suche.freigegeben = true;
            }
            (true, _) | (false, Some(_)) => {
                self.print_sel = None;
                self.print_printer = None;
                if choice == Some(1) {
                    self.open_pruefen();
                }
            }
            (false, None) => self.suche.rueckfrage = Some((req, n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suche_texte_bmk_und_verbindungen() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.ort = "W2".into();
        s.objects.push(Obj { id: 1, kind: Kind::Text { pos: P::new(10.0, 10.0), text: "Küche\nLicht".into(), size: 2.5, align: Align::Left, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj {
            id: 2,
            kind: Kind::Symbol { key: "x".into(), pos: P::new(50.0, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-Q1+%P".into(), value: "B16".into(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
            pen: 0,
            fixed: false,
            group: None,
            raw: None,
        });
        s.objects.push(Obj { id: 3, kind: Kind::Wire { pts: vec![P::new(0.0, 0.0), P::new(5.0, 0.0)], leitung: 0, bez: vec![(P::new(1.0, 0.0), "sw".into())], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        let t = suchen(&p, "LICHT", false);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].id, t[0].art, t[0].text.as_str()), (1, "Text", "Küche / Licht"));
        assert!(suchen(&p, "LICHT", true).is_empty());
        // angezeigte BMK mit aufgelöstem Ort
        let t = suchen(&p, "q1+w2", false);
        assert_eq!((t.len(), t[0].art), (1, "BMK"));
        assert_eq!(suchen(&p, "b16", false)[0].art, "Wert");
        assert_eq!(suchen(&p, "SW", false)[0].art, "Verbindung");
        assert!(suchen(&p, "  ", false).is_empty());
    }
}
