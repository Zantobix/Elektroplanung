//! Weitere Funktionen nach den Formularen des Originals: Symbolbearbeitung (Leiste "BLeiste"),
//! Stift/Leitung/Potential (TStift, TLeitung, TPotential), Strompfadbezeichnung, Sammelschienenassistent
//! (TSammelS), Textgenerator, Schriftfeld bearbeiten/speichern (TSchriftfeld), Bauteile (TSBauteil),
//! Platzhaltergruppen (TStextliste), Bibliotheksverwaltung (TBiblio), Kontaktspiegel, Verweise, zuletzt
//! geöffnete Projekte und das Kontextmenü des Projektexplorers (PopupMenu2).

use crate::app::*;
use crate::canvas;
use crate::model::*;
use crate::symbols::{self, Prim, SymDef};
use crate::titleblock::{self, TbOwned};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

/// Vergrößerung im Symboleditor (1 mm Symbol = 5 mm auf der Arbeitsfläche) und Mittelpunkt.
pub const EDIT_K: f32 = 5.0;
pub const EDIT_C: P = P::new(148.5, 105.0);

/// Feste Ids und Größe (Symbolmaß, mm) des Titel- und Wertfeldes im Symboleditor.
pub const EDIT_TITLE_ID: u64 = u64::MAX - 1;
pub const EDIT_VALUE_ID: u64 = u64::MAX - 2;
pub const TITLE_BOX: P = P::new(6.0, 3.0);

pub struct SymEdit {
    pub key: String,
    pub name: String,
    pub bmk: String,
    pub group: Vec<String>,
    pub haupt: i64,
    pub fav: bool,
    pub title: Option<P>,
    pub options: Vec<String>,
    pub option: u16,
    pub sheet: Sheet,
    /// Sichtbarkeit (Option, Modus) je Objekt; fehlt = in allen Optionen
    pub vis: HashMap<u64, (u16, u16)>,
    pub prev_raster: usize,
    pub new_option_name: String,
    /// Zusatzoptionen (OPT) des Symbols und je Element die Option (0 = immer sichtbar)
    pub flag_names: Vec<String>,
    pub flags: HashMap<u64, u32>,
    /// in der Liste "Option" gewählte Zusatzoption (0 = keine, dann gilt die Seite `option`)
    pub cur_flag: u32,
    /// Textnummer je Textelement (für Alternativtexte; 1 = BMK-Vorlage)
    pub text_idx: HashMap<u64, u32>,
    pub alt_sets: Vec<Vec<(u32, String)>>,
    /// Namensabfrage "Neue Seite" (0) / "Neue Option" (1) wie TTextAbfrage im Original
    pub query: Option<(u8, String)>,
    /// Freigaben (Querverweise, Sicherungsübersicht) als Vorgabe für neu gesetzte Symbole
    pub frg: (bool, bool),
    /// Textausrichtung (Element Feld 8) und Füllung (Feld 10) je Objekt
    pub modes: HashMap<u64, (u8, bool)>,
    /// unverändert übernommene Parameter und Wertanker des Symbols
    pub params: Vec<(u32, String, String)>,
    pub value_pos: Option<P>,
    /// Anfangslage von Titel- und Wertfeld im Editor (nur verschobene Felder ändern das Symbol)
    pub boxes0: (P, P),
    /// gesetztes Symbol, aus dem der Editor geöffnet wurde, und offene Frage "auf alle N übernehmen?" (symbol_kopie.rs)
    pub instanz: Option<u64>,
    pub frage: Option<usize>,
    /// eigene Stifte des Symbols (SIT, Nummer ab 4); Objekte im Editor tragen die Stiftnummer in `pen`
    pub pens: Vec<(u32, Stift)>,
    /// per „Bezeichner setzen“ geänderte Anschlussbezeichnung: Anschluss-Objekt → Textnummer (bezeichner.rs)
    pub pin_text: HashMap<u64, u32>,
    /// „Bezeichner setzen“ wartet auf den Klick auf einen Text: Anschluss-Objekt
    pub bezeichner_pick: Option<u64>,
    /// Stiftauswahl des Hauptfensters beim Öffnen (der Editor benutzt `App::pen` für die Symbolstifte)
    pub haupt_pen: usize,
}

impl SymEdit {
    /// Seite (Bildvariante) verschieben; Elemente und Anschlüsse behalten ihre Seite (Nummern werden umgerechnet).
    pub fn move_page(&mut self, from: u16, to: u16) {
        let n = self.options.len() as u16;
        if from == to || from >= n || to >= n {
            return;
        }
        let name = self.options.remove(from as usize);
        self.options.insert(to as usize, name);
        let map = |i: u16| -> u16 {
            if i == from {
                to
            } else if from < to && i > from && i <= to {
                i - 1
            } else if to < from && i >= to && i < from {
                i + 1
            } else {
                i
            }
        };
        for v in self.vis.values_mut() {
            v.0 = map(v.0);
        }
        self.option = map(self.option);
    }

    /// Seite (Bildvariante) löschen: Elemente nur dieser Seite entfallen (Rückgabe: ihre Ids), Elemente
    /// späterer Seiten rücken eine Nummer vor. Seite 0 (Grundbild) bleibt immer erhalten.
    pub fn delete_page(&mut self, page: u16) -> Vec<u64> {
        if page == 0 || page as usize >= self.options.len() {
            return vec![];
        }
        self.options.remove(page as usize);
        let mut removed = vec![];
        for (id, v) in self.vis.iter_mut() {
            if v.0 == page && v.1 == 0 {
                removed.push(*id);
            } else if v.0 > page {
                v.0 -= 1;
            }
        }
        self.vis.retain(|id, _| !removed.contains(id));
        if self.option >= page && self.option > 0 {
            self.option -= 1;
        }
        removed
    }

    /// Element in der gewählten Zusatzoption sichtbar? (ohne Auswahl werden alle gezeigt)
    pub fn flag_shown(&self, id: u64) -> bool {
        match self.flags.get(&id).copied().unwrap_or(0) {
            0 => true,
            f => self.cur_flag == 0 || self.cur_flag == f,
        }
    }
}

pub enum XDialog {
    Stift { idx: usize, name: String, style: usize, width: String, color: Color32 },
    Leitung { idx: usize, name: String, color: Color32, quer: String, adern: String, kennz: usize, width: String, style: u8 },
    Potential { idx: usize, name: String },
    Columns { text: String },
    /// Sammelschienenassistent (TS_Assi): L1, L2, L3, N, PE; `auto` = unter die vorhandenen Sammelschienen
    Busbar { pots: Vec<(String, bool)>, auto: bool, y: String, dy: String, x0: String, x1: String },
    TextGen { scope_project: bool },
    Title { elems: Vec<TbOwned> },
    Bauteil { idx: Option<usize>, b: Bauteil },
    PartAssign { obj: u64, idx: usize },
    Platz { idx: usize, name: String, text: String },
    Verweis { text: String, start: bool },
    SymName { name: String, bmk: String },
}

fn to_c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
fn from_c32(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}
/// Ablage für Einstellungen und Benutzerbibliothek (%APPDATA%\Elektroplanung bzw. ~/.config/Elektroplanung;
/// für Tests über EP_APPDIR umlenkbar).
pub fn appdir() -> PathBuf {
    let d = crate::config_dir();
    let _ = std::fs::create_dir_all(&d);
    d
}

impl App {
    // ------------------------------------------------------------------ Symbolbearbeitung

    /// Symbol im Editor öffnen (`None` = neues Projektsymbol).
    pub fn open_symbol_editor(&mut self, key: Option<String>) {
        if self.sym_edit.is_some() {
            return;
        }
        let def = key.as_ref().and_then(|k| symbols::find(&self.lib, k).cloned());
        let titel = crate::meldungen::editor_titel(def.as_ref().map(|d| (d.key.as_str(), d.name.as_str())));
        let def = def.unwrap_or_else(|| {
            let n = self.project.symdefs.iter().filter(|d| d.haupt == 0).count() + 1;
            SymDef {
                key: format!("proj:{}", self.project.alloc_id()),
                name: format!("Symbol {n}"),
                group: vec!["Projekt".into()],
                haupt: 0,
                fav: false,
                bmk: "-A%C".into(),
                prims: vec![],
                pins: vec![],
                title: None,
                vis: vec![],
                pin_vis: vec![],
                pin_info: vec![],
            flag: vec![],
            flag_names: vec![],
            text_idx: vec![],
            alt_sets: vec![],
            raw: vec![],
            frg: (true, false),
            params: vec![],
            value_pos: None,
            text_mode: vec![],
            fill: vec![],
            pens: vec![],
                options: vec!["Standard".into()],
            }
        });
        let mut sheet = Sheet::new(0);
        // Reiterbeschriftung des Originals (0x477cb0), im Nachbau im Fenstertitel
        sheet.name = titel;
        let mut vis = HashMap::new();
        let mut flags: HashMap<u64, u32> = HashMap::new();
        let mut text_idx: HashMap<u64, u32> = HashMap::new();
        let mut modes: HashMap<u64, (u8, bool)> = HashMap::new();
        let mut id: u64 = 1;
        let e = |p: P| P::new(EDIT_C.x + p.x * EDIT_K, EDIT_C.y + p.y * EDIT_K);
        for (i, pr) in def.prims.iter().enumerate() {
            let (kind, pen) = match pr {
                Prim::Line(a, b, s) => (Kind::Line { a: e(*a), b: e(*b) }, *s as usize),
                Prim::Poly(v, s) => (Kind::Poly { pts: v.iter().map(|p| e(*p)).collect() }, *s as usize),
                Prim::Bezier(a, b, c, d, s) => (Kind::Curve { a: e(*a), c1: e(*b), c2: e(*c), b: e(*d) }, *s as usize),
                Prim::Ellipse(a, b, s) => (Kind::Circle { a: e(*a), b: e(*b) }, *s as usize),
                Prim::Arc(c, r, a0, a1) => {
                    let n = 12;
                    let pts: Vec<P> = (0..=n)
                        .map(|k| {
                            let a = (a0 + (a1 - a0) * k as f32 / n as f32).to_radians();
                            e(P::new(c.x + r * a.cos(), c.y + r * a.sin()))
                        })
                        .collect();
                    for w in pts.windows(2) {
                        sheet.objects.push(Obj { id, kind: Kind::Line { a: w[0], b: w[1] }, pen: 0, fixed: false, group: None, raw: None });
                        if let Some(v) = def.vis.get(i) {
                            vis.insert(id, *v);
                        }
                        id += 1;
                    }
                    continue;
                }
                Prim::Text(c, s, size) => (Kind::Text { pos: e(*c), text: s.clone(), size: size * EDIT_K, align: Align::Center, vertical: false, style: Default::default() }, 0),
                Prim::TextBox(a, b, s, al) => {
                    let (mn, mx) = canvas::norm_rect(e(*a), e(*b));
                    let x = match al {
                        Align::Left => mn.x,
                        Align::Center => (mn.x + mx.x) / 2.0,
                        Align::Right => mx.x,
                    };
                    (Kind::Text { pos: P::new(x, (mn.y + mx.y) / 2.0), text: s.replace("\r\n", "\n"), size: (0.72 * (mx.y - mn.y) / s.lines().count().max(1) as f32).min((mx.x - mn.x) / (0.5 * s.lines().map(|l| l.trim_end().chars().count()).max().unwrap_or(1).max(1) as f32)).max(1.0), align: *al, vertical: false, style: Default::default() }, 0)
                }
            };
            sheet.objects.push(Obj { id, kind, pen, fixed: false, group: None, raw: None });
            if let Some(v) = def.vis.get(i) {
                vis.insert(id, *v);
            }
            if let Some(f) = def.flag.get(i).filter(|f| **f > 0) {
                flags.insert(id, *f);
            }
            if let Some(t) = def.text_idx.get(i).filter(|t| **t > 0) {
                text_idx.insert(id, *t);
            }
            modes.insert(id, (def.text_mode.get(i).copied().unwrap_or(0), def.fill.get(i).copied().unwrap_or(false)));
            id += 1;
        }
        for (i, p) in def.pins.iter().enumerate() {
            sheet.objects.push(Obj { id, kind: Kind::Terminal { pos: e(*p), leiste: "Anschluss".into(), nr: i as u32 + 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
            if let Some(v) = def.pin_vis.get(i) {
                vis.insert(id, *v);
            }
            id += 1;
        }
        // Titel- und Wertfeld wie im Symboleditor des Originals (grün/blau): Lage der BMK bzw. des Wertes,
        // verschiebbar; ohne Anker aus der Bibliothek links neben dem Symbol
        // Umriss aller Elemente (auch der nur mit Option sichtbaren) in Symbolmaß
        let (mut mn, mut mx) = (P::new(f32::MAX, f32::MAX), P::new(f32::MIN, f32::MIN));
        for o in &sheet.objects {
            let (a, b) = canvas::obj_bounds(o, &self.lib);
            mn = P::new(mn.x.min((a.x - EDIT_C.x) / EDIT_K), mn.y.min((a.y - EDIT_C.y) / EDIT_K));
            mx = P::new(mx.x.max((b.x - EDIT_C.x) / EDIT_K), mx.y.max((b.y - EDIT_C.y) / EDIT_K));
        }
        if mn.x > mx.x {
            (mn, mx) = (P::new(-3.0, -3.0), P::new(3.0, 3.0));
        }
        // Titel-/Wertanker sind rechtsbündig (BMK endet dort): das Feld liegt links davon
        let tpos = def.title.map(|t| P::new(t.x - TITLE_BOX.x, t.y)).unwrap_or(P::new(mn.x - TITLE_BOX.x - 1.0, mn.y));
        let vpos = def.value_pos.map(|t| P::new(t.x - TITLE_BOX.x, t.y)).unwrap_or(P::new(tpos.x, tpos.y + TITLE_BOX.y + 1.0));
        let _ = mx;
        for (oid, at, label) in [(EDIT_TITLE_ID, tpos, "Titel"), (EDIT_VALUE_ID, vpos, "Wert")] {
            let a = e(at);
            let b = e(at.add(TITLE_BOX));
            let style = TextStyle { frame: true, rect: Some((a, b)), ..Default::default() };
            sheet.objects.push(Obj { id: oid, kind: Kind::Text { pos: crate::epr::text_anchor(a, b, Align::Center, false), text: label.into(), size: TITLE_BOX.y * EDIT_K * 0.6, align: Align::Center, vertical: false, style }, pen: 0, fixed: false, group: None, raw: None });
        }
        if std::env::var("EP_DEBUG").is_ok() { for o in &sheet.objects { eprintln!("{:?} vis={:?}", o.kind, vis.get(&o.id)); } }
        let prev_raster = self.raster_idx;
        self.raster_idx = 1; // 2,5 mm Arbeitsfläche = 0,5 mm im Symbol
        let options = if def.options.is_empty() { vec!["Standard".into()] } else { def.options.clone() };
        self.sym_edit = Some(SymEdit {
            key: def.key.clone(),
            name: def.name.clone(),
            bmk: def.bmk.clone(),
            group: def.group.clone(),
            haupt: def.haupt,
            fav: def.fav,
            title: def.title,
            options,
            option: 0,
            sheet,
            vis,
            prev_raster,
            new_option_name: String::new(),
            flag_names: def.flag_names.clone(),
            flags,
            cur_flag: 0,
            text_idx,
            alt_sets: def.alt_sets.clone(),
            query: None,
            frg: def.frg,
            modes,
            params: def.params.clone(),
            value_pos: def.value_pos,
            boxes0: (e(tpos), e(vpos)),
            instanz: None,
            frage: None,
            pens: def.pens.clone(),
            pin_text: HashMap::new(),
            bezeichner_pick: None,
            haupt_pen: self.pen,
        });
        self.selection.clear();
        self.tool = Tool::Select;
        self.placing = None;
        self.pen = 0; // Stiftnummern im Editor sind die des Symbols
        self.view_before_edit = Some(self.view.clone());
        self.view.user = false;
        self.view.last_size = Vec2::ZERO;
        self.info = "Symbolbearbeitung: Elemente zeichnen, Anschlüsse mit „Neue Klemmstelle“ setzen".into();
    }

    /// Ist ein Objekt im Editor in der gewählten Option sichtbar?
    pub fn edit_visible(&self, id: u64) -> bool {
        match &self.sym_edit {
            Some(e) => symbols::visible(e.option, e.vis.get(&id)) && e.flag_shown(id),
            None => true,
        }
    }

    /// Neue Objekte im Editor gehören zur gewählten Option (Option 0 = alle Optionen).
    pub fn edit_tag_new(&mut self, id: u64) {
        if let Some(e) = &mut self.sym_edit {
            if e.option > 0 {
                e.vis.insert(id, (e.option, 0));
            }
        }
    }

    pub fn symbol_editor_commit(&mut self) {
        let Some(e) = self.sym_edit.take() else { return };
        self.raster_idx = e.prev_raster;
        self.pen = e.haupt_pen;
        let b = |p: P| P::new(((p.x - EDIT_C.x) / EDIT_K * 10.0).round() / 10.0, ((p.y - EDIT_C.y) / EDIT_K * 10.0).round() / 10.0);
        let mut prims = vec![];
        let mut pvis = vec![];
        let mut pflag: Vec<u32> = vec![];
        let mut ptext: Vec<u32> = vec![];
        let mut pmode: Vec<u8> = vec![];
        let mut pfill: Vec<bool> = vec![];
        let mut pins = vec![];
        let mut pin_vis = vec![];
        let mut pin_info: Vec<(u32, u32)> = vec![];
        // freie Textnummern ab der höchsten vergebenen
        let mut next_text = e.text_idx.values().copied().max().unwrap_or(1).max(1) + 1;
        // Stiftnummer des Elements (0–3 eingebaut, ab 4 Stift des Symbols)
        let style = |pen: usize| pen.min(255) as u8;
        let mut title = e.title;
        let mut value_pos = e.value_pos;
        for o in &e.sheet.objects {
            // Titel-/Wertfeld: nur ihre Lage (linke obere Ecke) übernehmen
            if o.id == EDIT_TITLE_ID || o.id == EDIT_VALUE_ID {
                let (mn, _) = canvas::obj_bounds(o, &self.lib);
                let at = match &o.kind {
                    Kind::Text { style: TextStyle { rect: Some((a, _)), .. }, .. } => *a,
                    _ => mn,
                };
                let start = if o.id == EDIT_TITLE_ID { e.boxes0.0 } else { e.boxes0.1 };
                if at.dist(start) > 0.01 {
                    // Anker = rechte obere Ecke des Feldes
                    let anchor = b(at).add(P::new(TITLE_BOX.x, 0.0));
                    if o.id == EDIT_TITLE_ID {
                        title = Some(anchor);
                    } else {
                        value_pos = Some(anchor);
                    }
                }
                continue;
            }
            let v = e.vis.get(&o.id).copied().unwrap_or((0, 1));
            let st = style(o.pen);
            let pr = match &o.kind {
                Kind::Line { a, b: bb } => Some(Prim::Line(b(*a), b(*bb), st)),
                Kind::Poly { pts } => Some(Prim::Poly(pts.iter().map(|p| b(*p)).collect(), st)),
                Kind::Rect { a, b: bb } => {
                    let (p0, p1) = canvas::norm_rect(*a, *bb);
                    Some(Prim::Poly(vec![b(p0), b(P::new(p1.x, p0.y)), b(p1), b(P::new(p0.x, p1.y))], st))
                }
                Kind::Triangle { a, b: bb } => Some(Prim::Poly(canvas::triangle_pts(*a, *bb).iter().map(|p| b(*p)).collect(), st)),
                Kind::Circle { a, b: bb } => Some(Prim::Ellipse(b(*a), b(*bb), st)),
                Kind::Curve { a, c1, c2, b: bb } => Some(Prim::Bezier(b(*a), b(*c1), b(*c2), b(*bb), st)),
                Kind::Wire { pts, .. } => {
                    for w in pts.windows(2) {
                        prims.push(Prim::Line(b(w[0]), b(w[1]), 0));
                        pvis.push(v);
                        pflag.push(e.flags.get(&o.id).copied().unwrap_or(0));
                        ptext.push(0);
                    }
                    None
                }
                Kind::Text { text, align, .. } => {
                    let (mn, mx) = canvas::obj_bounds(o, &self.lib);
                    Some(Prim::TextBox(b(mn), b(mx), text.clone(), *align))
                }
                Kind::Terminal { pos, nr, .. } => {
                    pins.push(b(*pos));
                    pin_vis.push(v);
                    // Anschlussbezeichnung (Textnummer) des bisherigen Anschlusses beibehalten bzw. die per
                    // „Bezeichner setzen“ gewählte
                    let alt = symbols::find(&self.lib, &e.key).and_then(|d| d.pin_info.get((*nr as usize).wrapping_sub(1))).copied().unwrap_or((0, 0));
                    pin_info.push((e.pin_text.get(&o.id).copied().unwrap_or(alt.0), alt.1));
                    None
                }
                _ => None,
            };
            if let Some(pr) = pr {
                let t = if matches!(pr, Prim::TextBox(..)) {
                    match e.text_idx.get(&o.id) {
                        Some(t) => *t,
                        None => {
                            next_text += 1;
                            next_text - 1
                        }
                    }
                } else {
                    0
                };
                prims.push(pr);
                pvis.push(v);
                pflag.push(e.flags.get(&o.id).copied().unwrap_or(0));
                ptext.push(t);
                let (m, f) = e.modes.get(&o.id).copied().unwrap_or((0, false));
                pmode.push(m);
                pfill.push(f);
            }
        }
        let def = SymDef {
            key: e.key.clone(),
            name: e.name.clone(),
            group: e.group.clone(),
            haupt: e.haupt,
            fav: e.fav,
            bmk: e.bmk.clone(),
            prims,
            pins,
            title,
            vis: pvis,
            pin_vis,
            pin_info,
            options: e.options.clone(),
            flag: pflag,
            flag_names: e.flag_names.clone(),
            text_idx: ptext,
            alt_sets: e.alt_sets.clone(),
            raw: vec![],
            frg: e.frg,
            params: e.params.clone(),
            value_pos,
            text_mode: pmode,
            fill: pfill,
            pens: e.pens.clone(),
        };
        self.checkpoint();
        if let Some(d) = self.lib.iter_mut().find(|d| d.key == def.key) {
            *d = def.clone();
        } else {
            self.lib.push(def.clone());
        }
        if let Some(d) = self.project.symdefs.iter_mut().find(|d| d.key == def.key) {
            *d = def.clone();
        } else {
            self.project.symdefs.push(def.clone());
        }
        if def.haupt == 200 {
            self.save_user_library();
        }
        self.selection.clear();
        self.restore_view();
        self.info = format!("Symbol „{}“ übernommen", def.name);
    }

    pub fn symbol_editor_cancel(&mut self) {
        if let Some(e) = self.sym_edit.take() {
            self.raster_idx = e.prev_raster;
            self.pen = e.haupt_pen;
        }
        self.selection.clear();
        self.restore_view();
        self.info.clear();
    }

    /// Ansicht von vor dem Symboleditor wiederherstellen (sonst Blatt einpassen).
    fn restore_view(&mut self) {
        match self.view_before_edit.take() {
            Some(v) => self.view = v,
            None => {
                self.view.user = false;
                self.view.last_size = Vec2::ZERO;
            }
        }
    }

    /// Leiste der Symbolbearbeitung wie "BLeiste" im Original (Reihenfolge aus der Formularressource, eigene Icons):
    /// Drehen, Vertikal/Horizontal Spiegeln | Gruppieren, Auflösen | Name | Stiftliste | Neuer Stift, Neue Seite,
    /// Neue Option | Symboleigenschaften | Übernehmen, Abbruch | Option + Optionsliste, Option ein/aus.
    pub fn bleiste(&mut self, ui: &mut egui::Ui) {
        // Knopf mit Vektor-Icon
        fn gbtn(ui: &mut egui::Ui, icon: crate::icons::Icon, hint: &str, enabled: bool, down: bool) -> egui::Response {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(27.0, 26.0), if enabled { Sense::click() } else { Sense::hover() });
            let p = ui.painter();
            if down {
                p.rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(204, 228, 247));
                p.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0, Color32::from_rgb(0, 84, 153)), egui::StrokeKind::Inside);
            } else if enabled && resp.hovered() {
                p.rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(229, 243, 255));
            }
            crate::icons::draw(ui, Rect::from_center_size(rect.center(), Vec2::splat(20.0)), icon, enabled);
            resp.on_hover_text(hint)
        }
        use crate::icons::Icon as I;
        let ctx = ui.ctx().clone();
        self.symbol_frage_ui(&ctx);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.add_space(29.0);
            let has = !self.selection.is_empty();
            if gbtn(ui, I::Rotate, "Drehen", has, false).clicked() {
                self.transform_selection(1);
            }
            if gbtn(ui, I::MirrorV, "Vertikal Spiegeln", has, false).clicked() {
                self.transform_selection(2);
            }
            if gbtn(ui, I::MirrorH, "Horizontal Spiegeln", has, false).clicked() {
                self.transform_selection(4);
            }
            crate::view::sep(ui, 2.0);
            if gbtn(ui, I::Group, "Markierte Zeichenobjekte gruppieren", self.selection.len() > 1, false).clicked() {
                self.group_selection(true);
            }
            if gbtn(ui, I::Ungroup, "Markierte Gruppe auflösen", has, false).clicked() {
                self.group_selection(false);
            }
            crate::view::sep(ui, 4.0);
            // Label4: Name des Symbols (Klick: Name und BMK-Vorlage)
            let name = self.sym_edit.as_ref().map(|e| e.name.clone()).unwrap_or_default();
            let (r, resp) = ui.allocate_exact_size(Vec2::new(151.0, 26.0), Sense::click());
            ui.painter().text(Pos2::new(r.left() + 3.0, r.center().y), egui::Align2::LEFT_CENTER, &name, FontId::proportional(13.0), Color32::BLACK);
            if resp.on_hover_text("Name und BMK-Vorlage ändern").clicked() {
                if let Some(e) = &self.sym_edit {
                    self.xdlg = Some(XDialog::SymName { name: e.name.clone(), bmk: e.bmk.clone() });
                }
            }
            ui.add_space(1.0);
            // Stifte des Symbols (eingebaute 0–3 und eigene ab 4)
            let pens: Vec<String> = self.edit_stifte().iter().map(|s| s.name.clone()).collect();
            let mut pen = self.pen;
            let sr = ui.scope(|ui| crate::view::auswahl(ui, "symstift", 74.0, &pens, &mut pen));
            sr.response.on_hover_text("Stiftliste");
            if sr.inner {
                self.pen = pen;
                let sel = self.selection.clone();
                if !sel.is_empty() {
                    self.with_sheet(|s, _| s.objects.iter_mut().filter(|o| sel.contains(&o.id)).for_each(|o| o.pen = pen));
                }
            }
            crate::view::sep(ui, 4.0);
            if gbtn(ui, I::NewPen, "Neuer Stift", true, false).clicked() {
                self.neuer_stift(true);
            }
            ui.add_space(6.0);
            if gbtn(ui, I::NewPage, "Neue Seite", true, false).clicked() {
                if let Some(e) = &mut self.sym_edit {
                    e.query = Some((0, format!("Seite {}", e.options.len() + 1)));
                }
            }
            if gbtn(ui, I::NewOption, "Neue Option", true, false).clicked() {
                if let Some(e) = &mut self.sym_edit {
                    e.query = Some((1, format!("Option {}", e.flag_names.len() + 1)));
                }
            }
            crate::view::sep(ui, 4.0);
            // Hinweise wie im Original (0x47689c: Button7/Button8 im Symbolmodus)
            if gbtn(ui, I::EditSymbol, crate::meldungen::SE_EINSTELLUNGEN, true, false).clicked() {
                self.open_sym_eigenschaften();
            }
            crate::view::sep(ui, 4.0);
            if gbtn(ui, I::Apply, crate::meldungen::SE_SYMBOL_UEBERNEHMEN, true, false).clicked() {
                self.symbol_editor_uebernehmen();
                return;
            }
            if gbtn(ui, I::Cancel, "Abbruch", true, false).clicked() {
                self.symbol_editor_cancel();
                return;
            }
            crate::view::sep(ui, 5.0);
            crate::view::static_text(ui, "Option", 51.0);
            // Optionsliste: Seiten (Bildvarianten) und Zusatzoptionen
            let Some(e) = &mut self.sym_edit else { return };
            let mut items: Vec<String> = e.options.iter().map(|o| format!("Seite: {o}")).collect();
            items.extend(e.flag_names.iter().map(|o| format!("Option: {o}")));
            let mut idx = if e.cur_flag > 0 { e.options.len() + e.cur_flag as usize - 1 } else { e.option as usize };
            let or = ui.scope(|ui| crate::view::auswahl(ui, "symoption", 125.0, &items, &mut idx));
            or.response.on_hover_text("Optionsliste");
            if or.inner {
                if idx < e.options.len() {
                    e.option = idx as u16;
                    e.cur_flag = 0;
                } else {
                    e.cur_flag = (idx - e.options.len() + 1) as u32;
                }
            }
            // Option ein/aus: markierte Elemente gehören zur gewählten Seite bzw. Zusatzoption (oder wieder zu allen)
            let sel = self.selection.clone();
            if gbtn(ui, I::OptionToggle, "Option Ein und Ausschalten", has, false).clicked() {
                if let Some(e) = &mut self.sym_edit {
                    for id in sel {
                        if e.cur_flag > 0 {
                            if e.flags.get(&id) == Some(&e.cur_flag) {
                                e.flags.remove(&id);
                            } else {
                                e.flags.insert(id, e.cur_flag);
                            }
                        } else if e.option == 0 || e.vis.get(&id).map(|v| v.0 == e.option && v.1 == 0).unwrap_or(false) {
                            e.vis.remove(&id);
                        } else {
                            e.vis.insert(id, (e.option, 0));
                        }
                    }
                }
            }
        });
        self.editor_query(ui.ctx());
    }

    /// Namensabfrage für "Neue Seite" / "Neue Option" (Original: TTextAbfrage).
    fn editor_query(&mut self, ctx: &egui::Context) {
        let Some(e) = &mut self.sym_edit else { return };
        let Some((kind, text)) = e.query.as_mut() else { return };
        let title = match *kind {
            0 => "Neue Seite",
            1 => "Neue Option",
            _ => "Seite umbenennen",
        };
        let mut done: Option<bool> = None;
        let closed = crate::props::tool_window(ctx, "symquery", title, [260.0, 80.0], egui::Order::Foreground, |ui| {
            ui.label("Name:");
            let r = ui.add(egui::TextEdit::singleline(text).desired_width(240.0));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                done = Some(true);
            }
            ui.horizontal(|ui| {
                if ui.add_sized([80.0, 24.0], egui::Button::new("OK")).clicked() {
                    done = Some(true);
                }
                if ui.add_sized([80.0, 24.0], egui::Button::new("Abbruch")).clicked() {
                    done = Some(false);
                }
            });
        });
        if closed {
            done = Some(false);
        }
        if let Some(ok) = done {
            let (kind, text) = e.query.take().unwrap();
            if ok && !text.trim().is_empty() {
                if kind >= 2 {
                    let i = (kind - 2) as usize;
                    if e.options.is_empty() {
                        e.options.push("Standard".into());
                    }
                    if let Some(o) = e.options.get_mut(i) {
                        *o = text.trim().to_string();
                    }
                } else if kind == 0 {
                    if e.options.is_empty() {
                        e.options.push("Standard".into());
                    }
                    e.options.push(text.trim().to_string());
                    e.option = (e.options.len() - 1) as u16;
                    e.cur_flag = 0;
                } else if e.flag_names.len() < 32 {
                    e.flag_names.push(text.trim().to_string());
                    e.cur_flag = e.flag_names.len() as u32;
                }
            }
        }
    }

    /// Arbeitsfläche der Symbolbearbeitung (weiß, Raster, Achsenkreuz im Symbol-Ursprung).
    pub fn draw_edit_base(&self, p: &egui::Painter, xf: canvas::Xf) {
        let r = Rect::from_min_max(xf.s(P::new(0.0, 0.0)), xf.s(P::new(SHEET_W, SHEET_H)));
        p.rect_filled(r, 0.0, Color32::WHITE);
        let step = RASTER[self.raster_idx].1.max(2.5) * 2.0;
        let gs = Stroke::new(1.0, Color32::from_rgb(232, 232, 240));
        let mut x = EDIT_C.x % step;
        while x <= SHEET_W {
            let sx = xf.s(P::new(x, 0.0)).x;
            p.line_segment([Pos2::new(sx, r.top()), Pos2::new(sx, r.bottom())], gs);
            x += step;
        }
        let mut y = EDIT_C.y % step;
        while y <= SHEET_H {
            let sy = xf.s(P::new(0.0, y)).y;
            p.line_segment([Pos2::new(r.left(), sy), Pos2::new(r.right(), sy)], gs);
            y += step;
        }
        let c = xf.s(EDIT_C);
        let cs = Stroke::new(1.0, Color32::from_rgb(200, 60, 60));
        p.line_segment([Pos2::new(c.x - 12.0, c.y), Pos2::new(c.x + 12.0, c.y)], cs);
        p.line_segment([Pos2::new(c.x, c.y - 12.0), Pos2::new(c.x, c.y + 12.0)], cs);
        p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Outside);
        if let Some(e) = &self.sym_edit {
            let opt = e.options.get(e.option as usize).cloned().unwrap_or_default();
            p.text(r.left_top() + Vec2::new(8.0, 8.0), egui::Align2::LEFT_TOP, format!("Symbolbearbeitung: {}   –   Option: {}", e.name, opt), FontId::proportional(14.0), Color32::DARK_GRAY);
        }
    }

    // ------------------------------------------------------------------ Dialoge zu Stift, Leitung, Potential …

    pub fn open_pen_dialog(&mut self, idx: usize) {
        // im Symboleditor: eigene Stifte des Symbols (ab Nummer 4)
        if self.sym_edit.is_some() && idx >= crate::symbol_stifte::ERSTER as usize {
            if let Some(s) = self.edit_stifte().get(idx).cloned() {
                self.xdlg = Some(XDialog::Stift { idx: usize::MAX - idx, name: s.name, style: s.style as usize, width: format!("{:.2}", s.width).replace('.', ","), color: to_c32(s.color) });
            }
            return;
        }
        if let Some(s) = self.project.stifte.get(idx) {
            self.xdlg = Some(XDialog::Stift { idx, name: s.name.clone(), style: s.style as usize, width: format!("{:.2}", s.width).replace('.', ","), color: to_c32(s.color) });
        }
    }

    pub fn open_leitung_dialog(&mut self, idx: usize) {
        if let Some(l) = self.project.leitungen.get(idx) {
            self.xdlg = Some(XDialog::Leitung {
                idx,
                name: l.name.clone(),
                color: to_c32(l.color),
                quer: format!("{}", l.quer).replace('.', ","),
                adern: l.adern.to_string(),
                kennz: l.kennz as usize,
                width: format!("{:.2}", l.width_mm).replace('.', ","),
                style: l.style,
            });
        }
    }

    pub fn open_columns_dialog(&mut self) {
        self.open_pfad_dialog();
    }

    pub fn open_busbar_assistant(&mut self) {
        // wie im Original: feste Auswahl L1, L2, L3, N, PE (alle angehakt)
        let pots = BUSBAR_NAMES.iter().map(|p| (p.to_string(), true)).collect();
        self.xdlg = Some(XDialog::Busbar { pots, auto: true, y: "40".into(), dy: "10".into(), x0: "25".into(), x1: "282".into() });
    }

    pub fn open_title_editor(&mut self) {
        let elems = if self.project.titleblock.is_empty() { load_default_titleblock().unwrap_or_else(titleblock::default_owned) } else { self.project.titleblock.clone() };
        self.xdlg = Some(XDialog::Title { elems });
    }

    /// Neues Bauteil (Bauteil1Click 0x486728 → 0x4867d4): ohne Gruppenknoten hat es im Original keine Gruppe
    /// (Kennung 0, angezeigt als „Keine Zuordnung“); „Projekt“ ist keine Bauteilgruppe des Originals.
    pub fn new_bauteil_dialog(&mut self) {
        self.xdlg = Some(XDialog::Bauteil { idx: None, b: Bauteil { gruppe: crate::epr_zusatz::KEINE_GRUPPE.into(), ..Default::default() } });
    }

    pub fn new_platzhalter(&mut self) {
        self.checkpoint();
        let n = self.project.platzhalter.len() + 1;
        self.project.platzhalter.push(Platzhalter { name: format!("Platzhaltergruppe {n}"), werte: vec![] });
        let idx = self.project.platzhalter.len() - 1;
        self.xdlg = Some(XDialog::Platz { idx, name: self.project.platzhalter[idx].name.clone(), text: String::new() });
    }

    /// Schriftfeld des Projekts als Vorgabe für neue Projekte speichern ("Schriftfeld speichern").
    pub fn save_titleblock_default(&mut self) {
        let tb = if self.project.titleblock.is_empty() { titleblock::default_owned() } else { self.project.titleblock.clone() };
        let path = appdir().join("schriftfeld.json");
        match serde_json::to_string_pretty(&tb).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string())) {
            Ok(()) => self.info = "Schriftfeld als Vorgabe für neue Projekte gespeichert".into(),
            Err(e) => self.dialog = Some(Dialog::Message(format!("Schriftfeld konnte nicht gespeichert werden: {e}"))),
        }
    }

    /// Hinzufügen › Schriftfeld: Schriftfeld eines anderen Projekts (.epj/.epr) übernehmen.
    pub fn import_titleblock(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Projekt", &["epj", "epr"]).pick_file() else { return };
        let prj = if p.extension().map(|e| e.eq_ignore_ascii_case("epr")).unwrap_or(false) {
            crate::epr::import(&p)
        } else {
            std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Project>(&s).map_err(|e| e.to_string()))
        };
        match prj {
            Ok(x) => {
                self.checkpoint();
                self.project.titleblock = if x.titleblock.is_empty() { titleblock::default_owned() } else { x.titleblock };
                self.info = "Schriftfeld übernommen".into();
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Schriftfeld konnte nicht gelesen werden: {e}"))),
        }
    }

    /// Kontaktspiegel: listet unter einer Spule alle Kontakte mit gleicher Kennzeichnung (Blatt.Strompfad).
    pub fn add_contact_mirror(&mut self) {
        let Some(o) = self.selected_objs().into_iter().find(|o| matches!(o.kind, Kind::Symbol { .. })) else { return };
        let Kind::Symbol { bmk, .. } = &o.kind else { return };
        if bmk.is_empty() {
            return;
        }
        let mut lines = vec![];
        for a in &self.project.anlagen {
            for s in &a.sheets {
                let (fx0, _, fx1, _) = s.frame();
                let cw = (fx1 - fx0) / s.columns.max(1) as f32;
                for x in &s.objects {
                    if x.id == o.id {
                        continue;
                    }
                    if let Kind::Symbol { bmk: b2, key, pos, .. } = &x.kind {
                        if b2 == bmk {
                            let name = symbols::find(&self.lib, key).map(|d| d.name.clone()).unwrap_or_default();
                            let col = (((pos.x - fx0) / cw).floor() as i32).clamp(0, s.columns as i32 - 1) as usize;
                            lines.push(format!("{name}  /{}.{}", s.number, s.col_label(col)));
                        }
                    }
                }
            }
        }
        if lines.is_empty() {
            lines.push("(keine Kontakte)".into());
        }
        let (_, mx) = canvas::obj_bounds(&o, &self.lib);
        let (mn, _) = canvas::obj_bounds(&o, &self.lib);
        let pos = P::new(mn.x, mx.y + 4.0);
        self.add_obj(Kind::Text { pos, text: lines.join("\n"), size: 2.2, align: Align::Left, vertical: false, style: Default::default() });
    }

    /// Verweis (Start/Ziel): Pfeil und Beschriftung.
    pub fn add_reference(&mut self, at: P, start: bool, text: String) {
        self.checkpoint();
        let gid = self.project.alloc_id() + 1_000_000;
        let tri = if start {
            vec![at, P::new(at.x + 3.0, at.y - 1.5), P::new(at.x + 3.0, at.y + 1.5)]
        } else {
            vec![P::new(at.x - 3.0, at.y - 1.5), at, P::new(at.x - 3.0, at.y + 1.5)]
        };
        let id1 = self.project.alloc_id();
        let id2 = self.project.alloc_id();
        let tp = if start { P::new(at.x + 4.0, at.y) } else { P::new(at.x - 4.0, at.y) };
        let al = if start { Align::Left } else { Align::Right };
        self.with_sheet(|s, _| {
            s.objects.push(Obj { id: id1, kind: Kind::Poly { pts: tri }, pen: 0, fixed: false, group: Some(gid), raw: None });
            s.objects.push(Obj { id: id2, kind: Kind::Text { pos: tp, text, size: 2.5, align: al, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: Some(gid), raw: None });
        });
        self.selection = vec![id1, id2];
    }


    /// Textgenerator: Betriebsmittelkennzeichnungen je Präfix neu nummerieren (Blatt, dann Strompfad, dann Höhe).
    pub fn regenerate_bmk(&mut self, whole_project: bool) {
        self.checkpoint();
        let cur = self.cur();
        let mut items: Vec<(usize, usize, f32, f32, u64, String)> = vec![];
        for (ai, a) in self.project.anlagen.iter().enumerate() {
            for (si, s) in a.sheets.iter().enumerate() {
                if !whole_project && cur != Some(SheetRef { anlage: ai, sheet: si }) {
                    continue;
                }
                for o in &s.objects {
                    if let Kind::Symbol { key, pos, .. } = &o.kind {
                        if let Some(d) = symbols::find(&self.lib, key) {
                            if !d.bmk.is_empty() {
                                items.push((ai, si, pos.x, pos.y, o.id, d.bmk.clone()));
                            }
                        }
                    }
                }
            }
        }
        items.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(a.2.partial_cmp(&b.2).unwrap()).then(a.3.partial_cmp(&b.3).unwrap()));
        let mut counters: HashMap<String, u32> = HashMap::new();
        let mut n = 0;
        for (ai, si, _, _, id, tpl) in items {
            let c = counters.entry(tpl.clone()).or_insert(0);
            *c += 1;
            let num = self.project.anlagen[ai].sheets[si].number;
            let new = symbols::expand_bmk(&tpl, *c, num);
            if let Some(o) = self.project.anlagen[ai].sheets[si].objects.iter_mut().find(|o| o.id == id) {
                if let Kind::Symbol { bmk, .. } = &mut o.kind {
                    *bmk = new;
                    n += 1;
                }
            }
        }
        self.info = format!("Textgenerator: {n} Kennzeichnungen erzeugt");
    }

    /// Eigentümer › Setzen wie im Original: die markierten Symbole merken und auf einen Klick auf das
    /// Eigentümersymbol warten (z. B. Spule für einen Kontakt).
    pub fn start_owner_pick(&mut self) {
        let ids: Vec<u64> = self.selected_objs().iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| o.id).collect();
        let Some(r) = self.cur() else { return };
        if ids.is_empty() || self.sym_edit.is_some() {
            return;
        }
        self.owner_pick = Some((r, ids));
        self.placing = None;
        self.drag = Drag::None;
        self.info = "Eigentümer wählen: Symbol anklicken, auch auf einem anderen Blatt (Esc = Abbruch)".into();
    }

    /// Klick im Modus „Eigentümer setzen“: das angeklickte Symbol wird Eigentümer der gemerkten Symbole (VSY › OWR);
    /// sie zeigen danach dessen Kennzeichnung und Lage (`eigentuemer.rs`), ihre eigene Vorlage bleibt stehen.
    /// Kein Symbol oder eines der Ziele → weiter warten.
    pub fn owner_picked(&mut self, owner_id: u64) {
        let Some((tref, targets)) = self.owner_pick.clone() else { return };
        if self.cur() == Some(tref) && targets.contains(&owner_id) {
            return;
        }
        let owner = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == owner_id)).cloned();
        let Some(Kind::Symbol { bmk, ext, .. }) = owner.map(|o| o.kind) else { return };
        let counter = ext.counter;
        self.owner_pick = None;
        let changes = self
            .project
            .sheet(tref)
            .map(|s| s.objects.iter().filter(|o| targets.contains(&o.id)).any(|o| matches!(&o.kind, Kind::Symbol { ext: e, .. } if e.eigentuemer != owner_id)))
            .unwrap_or(false);
        let shown = self.sheet().zip(self.cur()).and_then(|(s, r)| self.project.anlagen.get(r.anlage).map(|a| symbols::display_bmk(&bmk, counter, &canvas::bmk_ctx(a, s))));
        let shown = shown.unwrap_or_else(|| bmk.clone());
        if !changes {
            self.info = format!("Eigentümer {shown}: keine Änderung");
            return;
        }
        self.checkpoint();
        crate::eigentuemer::setzen(&mut self.project, owner_id, &targets);
        if self.cur() == Some(tref) {
            self.selection = targets;
        }
        self.info = format!("Eigentümer {shown} gesetzt");
    }

    // ------------------------------------------------------------------ Bibliothek, Favoriten, zuletzt geöffnet

    pub fn toggle_favorite(&mut self, key: &str) {
        if let Some(d) = self.lib.iter_mut().find(|d| d.key == key) {
            d.fav = !d.fav;
            let on = d.fav;
            self.fav_overrides.insert(key.to_string(), on);
        }
    }

    /// Projektsymbol in die Benutzer-/Standardbibliothek übernehmen ("In Bibliothek").
    pub fn to_library(&mut self, key: &str, user: bool) {
        let Some(mut d) = symbols::find(&self.lib, key).cloned() else { return };
        d.haupt = if user { 200 } else { 300 };
        d.key = format!("user:{}:{}", d.haupt, d.name);
        if d.group.is_empty() || d.group[0] == "Projekt" {
            d.group = vec!["Sonstiges".into()];
        }
        self.lib.retain(|x| x.key != d.key);
        self.lib.push(d.clone());
        self.save_user_library();
        self.info = format!("„{}“ in die Bibliothek übernommen", d.name);
    }

    pub fn delete_library_symbol(&mut self, key: &str) {
        if key.starts_with("user:") || key.starts_with("proj:") {
            self.lib.retain(|d| d.key != key);
            self.project.symdefs.retain(|d| d.key != key);
            self.save_user_library();
        } else {
            self.dialog = Some(Dialog::Message("Symbole der Original-Datenbank werden hier nicht gelöscht (die Datenbank bleibt unverändert).".into()));
        }
    }

    /// Gruppen der Bibliothek bzw. Bauteilgruppen bearbeiten (TBiblio): 0/10 Hinzufügen, 1/11 Löschen, 2/12 Umbenennen.
    /// Symbolgruppen gibt es nur über ihre Symbole: Umbenennen/Löschen betrifft Benutzer- und Projektsymbole
    /// (gelöschte Gruppen wandern nach "Sonstiges"); Hinzufügen merkt die neue Gruppe ohne Symbol (bib_gruppen.rs).
    pub fn library_group_action(&mut self, a: u8, old: &str, new: &str) {
        let split = |g: &str| -> Vec<String> { g.split(" / ").map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect() };
        match a {
            0 if !new.trim().is_empty() => {
                if !crate::bib_gruppen::hinzufuegen(&mut self.lib_gruppen, &self.lib, new) {
                    crate::meldungen::hinweis(crate::meldungen::T_DOPPELT, crate::meldungen::GRUPPE_DOPPELT);
                    return;
                }
                crate::bib_gruppen::speichern(&appdir(), &self.lib_gruppen);
                self.lib_filter = split(new).join(" / ");
                self.info = format!("Gruppe „{}“ angelegt", self.lib_filter);
                return;
            }
            1 | 2 => {
                let target = if a == 1 { vec!["Sonstiges".to_string()] } else { split(new) };
                if target.is_empty() || old.is_empty() {
                    return;
                }
                // gemerkte leere Gruppen folgen (Löschen entfernt sie)
                if crate::bib_gruppen::aendern(&mut self.lib_gruppen, old, if a == 1 { None } else { Some(new) }) {
                    crate::bib_gruppen::speichern(&appdir(), &self.lib_gruppen);
                }
                for d in self.lib.iter_mut().chain(self.project.symdefs.iter_mut()) {
                    if d.group.join(" / ") == old && (d.key.starts_with("user:") || d.key.starts_with("proj:") || d.key.starts_with("epr:")) {
                        d.group = target.clone();
                    }
                }
            }
            10 if self.project.bauteile.iter().any(|b| b.gruppe == new.trim()) => {
                crate::meldungen::hinweis(crate::meldungen::T_DOPPELT, crate::meldungen::GRUPPE_DOPPELT);
                return;
            }
            10 if !new.trim().is_empty() => self.project.bauteile.push(Bauteil { gruppe: new.trim().into(), name: "Neues Bauteil".into(), ..Default::default() }),
            11 | 12 => {
                for b in self.project.bauteile.iter_mut().filter(|b| b.gruppe == old) {
                    b.gruppe = if a == 11 { "Projekt".into() } else { new.trim().into() };
                }
            }
            _ => return,
        }
        self.save_user_library();
        self.dirty = true;
    }

    /// Synchronisierung: Symbole einer anderen Datenbank (Ordner mit Symbole.tfi/Gruppen.tfi) übernehmen.
    /// Neue Symbole kommen in den gewählten Bereich; vorhandene mit anderer Zeichnung werden ersetzt, außer
    /// "Frage bei neuerer Symbolversion" ist gesetzt (dann nur gezählt).
    pub fn library_sync(&mut self) -> String {
        if self.lib_sync.0.trim().is_empty() {
            crate::meldungen::warnung(crate::meldungen::T_ACHTUNG, crate::meldungen::SYNC_KEINE_BIBLIOTHEK);
            return String::new();
        }
        let dir = std::path::PathBuf::from(self.lib_sync.0.trim());
        let other = match symbols::load_db(&dir) {
            Ok(v) => v,
            Err(e) => {
                crate::meldungen::warnung(crate::meldungen::T_GEHT_NICHT, crate::meldungen::SYNC_DATENFORMAT);
                return format!("Bibliothek nicht lesbar: {e}");
            }
        };
        let haupt = if self.lib_sync.3 == 1 { 300 } else { 200 };
        let (mut neu, mut ersetzt, mut offen) = (0, 0, 0);
        for mut d in other {
            match self.lib.iter_mut().find(|x| x.name == d.name && x.group == d.group) {
                Some(x) if x.prims != d.prims => {
                    if self.lib_sync.1 {
                        offen += 1;
                    } else {
                        let key = x.key.clone();
                        *x = d;
                        x.key = key;
                        ersetzt += 1;
                    }
                }
                Some(_) => {}
                None => {
                    d.haupt = haupt;
                    d.key = format!("user:{haupt}:{}", d.name);
                    self.lib.push(d);
                    neu += 1;
                }
            }
        }
        self.save_user_library();
        crate::meldungen::hinweis("Summe", &crate::meldungen::sync_summe(0, ersetzt, neu, 0));
        format!("{neu} neue Symbole, {ersetzt} ersetzt, {offen} neuere Versionen nicht übernommen (Frage)")
    }

    /// Datenbank sichern: Gruppen.tfi, Symbole.tfi und die Benutzerbibliothek in einen Ordner kopieren.
    pub fn library_backup(&mut self) -> String {
        let Some(dest) = rfd::FileDialog::new().pick_folder() else { return String::new() };
        let mut n = 0;
        if let Some(db) = symbols::find_db_dir(None) {
            for f in ["Gruppen.tfi", "Symbole.tfi"] {
                if std::fs::copy(db.join(f), dest.join(f)).is_ok() {
                    n += 1;
                }
            }
        }
        if std::fs::copy(appdir().join("benutzer_symbole.json"), dest.join("benutzer_symbole.json")).is_ok() {
            n += 1;
        }
        format!("{n} Datei(en) nach {} gesichert", dest.display())
    }

    pub fn save_user_library(&self) {
        let own: Vec<&SymDef> = self.lib.iter().filter(|d| d.key.starts_with("user:")).collect();
        if let Ok(s) = serde_json::to_string(&own) {
            let _ = std::fs::write(appdir().join("benutzer_symbole.json"), s);
        }
    }

    pub fn load_user_library(&mut self) {
        self.lib_gruppen = crate::bib_gruppen::laden(&appdir());
        if let Ok(s) = std::fs::read_to_string(appdir().join("benutzer_symbole.json")) {
            if let Ok(v) = serde_json::from_str::<Vec<SymDef>>(&s) {
                for d in v {
                    self.lib.retain(|x| x.key != d.key);
                    self.lib.push(d);
                }
            }
        }
        for (k, on) in &self.fav_overrides {
            if let Some(d) = self.lib.iter_mut().find(|d| &d.key == k) {
                d.fav = *on;
            }
        }
    }

    pub fn remember_recent(&mut self, p: &std::path::Path) {
        let s = p.to_string_lossy().to_string();
        self.recent.retain(|x| x != &s);
        self.recent.insert(0, s);
        self.recent.truncate(5);
    }

    /// Bibliotheksverwaltung (eigenes Fenster).
    pub fn library_window(&mut self, ctx: &egui::Context) {
        if !self.show_library {
            return;
        }
        let mut open = true;
        let mut act: Option<(String, u8)> = None;
        let mut group_act: Option<(u8, String, String)> = None;
        let mut sync_start = false;
        let mut backup = false;
        egui::Window::new("Bibliotheksverwaltung").open(&mut open).default_size([620.0, 460.0]).show(ctx, |ui| {
            // Reiter wie im Original (TBiblio): Symbolgruppen, Bauteile, Synchronisierung, Backup – dazu die Symbolliste
            ui.horizontal(|ui| {
                for (i, t) in ["Symbole", "Symbolgruppen", "Bauteile", "Synchronisierung", "Backup"].iter().enumerate() {
                    if ui.selectable_label(self.lib_tab == i, *t).clicked() {
                        self.lib_tab = i;
                    }
                }
            });
            ui.separator();
            match self.lib_tab {
                1 => {
                    let mut groups: Vec<String> = self.lib.iter().map(|d| d.group.join(" / ")).collect();
                    groups.extend(crate::bib_gruppen::leere(&self.lib_gruppen, &self.lib));
                    groups.sort();
                    groups.dedup();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for g in &groups {
                            let n = self.lib.iter().filter(|d| d.group.join(" / ") == *g).count();
                            if ui.selectable_label(self.lib_filter == *g, format!("{g}  ({n})")).clicked() {
                                // erneuter Klick hebt die Markierung auf (dann: neue Hauptgruppe)
                                self.lib_filter = if self.lib_filter == *g { String::new() } else { g.clone() };
                            }
                        }
                    });
                    // wie TBiblio.Button2Click: neue Gruppe unter der markierten („Hauptgruppe: …“) bzw. als neue
                    // Hauptgruppe; ohne Eingabe heißt sie „Neue Gruppe“
                    let ziel = crate::meldungen::neue_gruppe_ziel(&self.lib_filter);
                    ui.label(egui::RichText::new(&ziel.0).strong());
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.lib_sync.4).hint_text(crate::meldungen::BG_NEUE_GRUPPE));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Hinzufügen").clicked() {
                            group_act = Some((0, self.lib_filter.clone(), ziel.1(&self.lib_sync.4)));
                        }
                        if ui.button("Löschen").clicked() {
                            group_act = Some((1, self.lib_filter.clone(), String::new()));
                        }
                        if ui.button("Umbenennen").clicked() {
                            group_act = Some((2, self.lib_filter.clone(), self.lib_sync.4.clone()));
                        }
                    });
                    return;
                }
                2 => {
                    ui.label(egui::RichText::new("Firmenliste").strong());
                    let mut firmen: Vec<String> = self.project.bauteile.iter().map(|b| b.firma.clone()).filter(|f| !f.is_empty()).collect();
                    firmen.sort();
                    firmen.dedup();
                    egui::ScrollArea::vertical().id_salt("firmen").max_height(110.0).show(ui, |ui| {
                        for f in &firmen {
                            ui.label(f);
                        }
                    });
                    ui.label(egui::RichText::new("Bauteilgruppen").strong());
                    let mut gr: Vec<String> = self.project.bauteile.iter().map(|b| b.gruppe.clone()).collect();
                    gr.sort();
                    gr.dedup();
                    egui::ScrollArea::vertical().id_salt("bgr").max_height(110.0).show(ui, |ui| {
                        for g in &gr {
                            if ui.selectable_label(self.lib_filter == *g, g).clicked() {
                                self.lib_filter = g.clone();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.lib_sync.4);
                        if ui.button("Hinzufügen").clicked() {
                            group_act = Some((10, String::new(), self.lib_sync.4.clone()));
                        }
                        if ui.button("Löschen").clicked() {
                            group_act = Some((11, self.lib_filter.clone(), String::new()));
                        }
                        if ui.button("Umbenennen").clicked() {
                            group_act = Some((12, self.lib_filter.clone(), self.lib_sync.4.clone()));
                        }
                    });
                    return;
                }
                3 => {
                    ui.label("Bibliothek mit der synchronisiert werden soll");
                    ui.text_edit_singleline(&mut self.lib_sync.0);
                    if ui.button("Durchsuchen").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            self.lib_sync.0 = d.display().to_string();
                        }
                    }
                    ui.checkbox(&mut self.lib_sync.1, "Frage bei neuerer Symbolversion");
                    ui.checkbox(&mut self.lib_sync.2, "Frage bei neuen Symbolen");
                    ui.label("Neue Symbole einfügen in");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.lib_sync.3, 0, "Benutzerbereich");
                        ui.radio_value(&mut self.lib_sync.3, 1, "Standardbereich");
                    });
                    if ui.button("Start").clicked() {
                        sync_start = true;
                    }
                    if !self.lib_sync.4.is_empty() {
                        ui.small(self.lib_sync.4.as_str());
                    }
                    return;
                }
                4 => {
                    ui.add_space(30.0);
                    if ui.button("Datenbank sichern").clicked() {
                        backup = true;
                    }
                    if !self.lib_sync.4.is_empty() {
                        ui.small(self.lib_sync.4.as_str());
                    }
                    return;
                }
                _ => {}
            }
            ui.horizontal(|ui| {
                ui.label("Suchen:");
                ui.text_edit_singleline(&mut self.lib_filter);
                ui.label(format!("{} Symbole", self.lib.len()));
            });
            ui.separator();
            let f = self.lib_filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("libgrid").striped(true).num_columns(5).show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Gruppe");
                    ui.strong("Bereich");
                    ui.strong("Favorit");
                    ui.strong("");
                    ui.end_row();
                    for d in self.lib.iter().filter(|d| f.is_empty() || d.name.to_lowercase().contains(&f) || d.group.join("/").to_lowercase().contains(&f)) {
                        ui.label(&d.name);
                        ui.label(d.group.join(" / "));
                        ui.label(match d.haupt {
                            200 => "Benutzer",
                            300 | 100 => "Standard",
                            _ => "Projekt",
                        });
                        let mut fav = d.fav;
                        if ui.checkbox(&mut fav, "").changed() {
                            act = Some((d.key.clone(), 0));
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("Bearbeiten").clicked() {
                                act = Some((d.key.clone(), 1));
                            }
                            if (d.key.starts_with("user:") || d.key.starts_with("proj:")) && ui.small_button("Löschen").clicked() {
                                act = Some((d.key.clone(), 2));
                            }
                            if d.haupt == 0 && ui.small_button("In Benutzerbereich").clicked() {
                                act = Some((d.key.clone(), 3));
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        });
        if !open {
            self.show_library = false;
        }
        if let Some((a, old, new)) = group_act {
            self.library_group_action(a, &old, &new);
        }
        if sync_start {
            self.lib_sync.4 = self.library_sync();
        }
        if backup {
            self.lib_sync.4 = self.library_backup();
        }
        if let Some((k, a)) = act {
            match a {
                0 => self.toggle_favorite(&k),
                1 => {
                    self.show_library = false;
                    self.open_symbol_editor(Some(k));
                }
                2 => {
                    if self.loeschen_bestaetigt(Some(&k)) {
                        self.delete_library_symbol(&k)
                    }
                }
                _ => self.to_library(&k, true),
            }
        }
    }

    // ------------------------------------------------------------------ Kontextmenü Projektexplorer (PopupMenu2)

    pub fn tree_context(&mut self, node: &TNode, action: &str) {
        // Original (Lschen1Click): jedes Löschen im Explorer erst nach Rückfrage
        if action == "Löschen" && !self.loeschen_bestaetigt(if let TAction::Symbol(k) = &node.action { Some(k.as_str()) } else { None }) {
            return;
        }
        match (action, &node.action) {
            ("Eigenschaften", TAction::Sheet(r)) => self.open_sheet_props(*r),
            ("Eigenschaften", TAction::Schiene(a, i)) => {
                let name = self.project.anlagen.get(*a).and_then(|x| x.sammelschienen.get(*i)).cloned().unwrap_or_default();
                self.open_schiene_dialog(*a, &name);
            }
            ("Eigenschaften", TAction::Klemmleiste(a, i)) => self.open_kleiste_dialog(*a, *i),
            ("Eigenschaften", TAction::Stift(i)) => self.open_pen_dialog(*i),
            ("Eigenschaften", TAction::Leitung(i)) => self.open_leitung_dialog(*i),
            ("Eigenschaften", TAction::Potential(i)) => {
                self.open_potential_dialog(*i)
            }
            ("Eigenschaften" | "Bauteil bearbeiten", TAction::Bauteil(i)) => {
                if let Some(b) = self.project.bauteile.get(*i) {
                    self.xdlg = Some(XDialog::Bauteil { idx: Some(*i), b: b.clone() });
                }
            }
            ("Eigenschaften", TAction::Platz(i)) => {
                if let Some(g) = self.project.platzhalter.get(*i) {
                    let text = g.werte.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n");
                    self.xdlg = Some(XDialog::Platz { idx: *i, name: g.name.clone(), text });
                }
            }
            ("Löschen", TAction::Sheet(r)) => {
                let r = *r;
                if self.project.anlagen.get(r.anlage).map(|a| a.sheets.len() > 1).unwrap_or(false) {
                    self.checkpoint();
                    self.project.anlagen[r.anlage].sheets.remove(r.sheet);
                    self.tabs.retain(|t| *t != r);
                    for t in &mut self.tabs {
                        if t.anlage == r.anlage && t.sheet > r.sheet {
                            t.sheet -= 1;
                        }
                    }
                    self.fix_tabs();
                } else {
                    self.dialog = Some(Dialog::Message("Das letzte Blatt einer Anlage kann nicht gelöscht werden.".into()));
                }
            }
            ("Löschen", TAction::Symbol(k)) => self.delete_library_symbol(&k.clone()),
            ("Löschen", TAction::Bauteil(i)) => {
                self.checkpoint();
                let i = *i;
                self.project.bauteile.remove(i);
                self.project.part_of.retain(|_, v| *v != i);
                for v in self.project.part_of.values_mut() {
                    if *v > i {
                        *v -= 1;
                    }
                }
            }
            ("Löschen", TAction::Platz(i)) => {
                self.checkpoint();
                self.project.platzhalter.remove(*i);
            }
            ("Symbol bearbeiten", TAction::Symbol(k)) => self.open_symbol_editor(Some(k.clone())),
            ("Zu den Favoriten hinzufügen" | "Aus den Favoriten entfernen", TAction::Symbol(k)) => self.toggle_favorite(&k.clone()),
            ("Standardbereich", TAction::Symbol(k)) => self.to_library(&k.clone(), false),
            ("Benutzerbereich", TAction::Symbol(k)) => self.to_library(&k.clone(), true),
            ("Blatt schliessen", TAction::Sheet(r)) => {
                let r = *r;
                self.tabs.retain(|t| *t != r);
                self.fix_tabs();
            }
            ("Alle Blätter dieser Anlage  schließen", TAction::Sheet(r)) => {
                let a = r.anlage;
                self.tabs.retain(|t| t.anlage != a);
                self.fix_tabs();
            }
            ("Ganz nach Oben" | "Eins nach Oben" | "Eins nach Unten" | "Ganz nach Unten", TAction::Sheet(r)) => {
                let r = *r;
                let n = self.project.anlagen[r.anlage].sheets.len();
                let to = match action {
                    "Ganz nach Oben" => 0,
                    "Eins nach Oben" => r.sheet.saturating_sub(1),
                    "Eins nach Unten" => (r.sheet + 1).min(n - 1),
                    _ => n - 1,
                };
                if to != r.sheet {
                    self.checkpoint();
                    let s = self.project.anlagen[r.anlage].sheets.remove(r.sheet);
                    self.project.anlagen[r.anlage].sheets.insert(to, s);
                    self.tabs = vec![SheetRef { anlage: r.anlage, sheet: to }];
                    self.active = 0;
                }
            }
            // Anlagenknoten "prj/a<n>": Eigenschaften, Löschen, Reihenfolge
            ("Eigenschaften" | "Löschen" | "Ganz nach Oben" | "Eins nach Oben" | "Eins nach Unten" | "Ganz nach Unten", TAction::None)
                if node.id.starts_with("prj/a") && node.id.matches('/').count() == 1 =>
            {
                let Ok(i) = node.id[5..].parse::<usize>() else { return };
                let n = self.project.anlagen.len();
                match action {
                    "Eigenschaften" => self.open_anlage_props(i),
                    "Löschen" => {
                        if n > 1 {
                            self.checkpoint();
                            self.project.anlagen.remove(i);
                            self.tabs.retain(|t| t.anlage != i);
                            for t in &mut self.tabs {
                                if t.anlage > i {
                                    t.anlage -= 1;
                                }
                            }
                            self.fix_tabs();
                        } else {
                            self.dialog = Some(Dialog::Message("Die letzte Anlage kann nicht gelöscht werden.".into()));
                        }
                    }
                    _ => {
                        let to = match action {
                            "Ganz nach Oben" => 0,
                            "Eins nach Oben" => i.saturating_sub(1),
                            "Eins nach Unten" => (i + 1).min(n - 1),
                            _ => n - 1,
                        };
                        if to != i {
                            self.checkpoint();
                            let a = self.project.anlagen.remove(i);
                            self.project.anlagen.insert(to, a);
                            self.tabs.clear();
                            self.fix_tabs();
                        }
                    }
                }
            }
            ("Neues Symbol", _) => self.open_symbol_editor(None),
            ("Neue Grafik", _) => self.new_project_graphic(),
            ("Eigenschaften", TAction::Bild(i)) => self.open_grafik_dialog(*i),
            ("Löschen", TAction::Bild(i)) => self.delete_project_graphic(*i),
            ("Neue Leitung", _) => {
                self.new_leitung();
                self.open_leitung_dialog(self.leitung);
            }
            ("Neues Potential", _) => {
                self.new_potential();
                let i = self.project.potentiale.len() - 1;
                self.open_potential_dialog(i);
            }
            ("Neuer Stift", _) => self.neuer_stift(false),
            // wie im Original (0x4867d4): Gruppe des angeklickten Gruppenknotens als Vorgabe
            ("Neues Bauteil", _) => {
                let g = node.id.strip_prefix("bt/user/").map(|g| g.to_string());
                self.new_bauteil_dialog();
                if let (Some(g), Some(XDialog::Bauteil { b, .. })) = (g, self.xdlg.as_mut()) {
                    b.gruppe = g;
                }
            }
            ("Neue Platzhaltergruppe", _) => self.new_platzhalter(),
            _ => {}
        }
    }

    // ------------------------------------------------------------------ Dialoge zeichnen

    pub fn extra_dialogs(&mut self, ctx: &egui::Context) {
        self.library_window(ctx);
        let Some(mut d) = self.xdlg.take() else { return };
        let mut ok = false;
        let mut close = false;
        // Bauteilliste: neues Bauteil anlegen und danach gleich dem Symbol zuordnen
        let mut new_part_for: Option<u64> = None;
        let parts: Vec<String> = self.project.bauteile.iter().map(|b| format!("{} {}", b.name, b.typ)).collect();
        // Leitung › Stift › Neu: neuen Stift mit der Darstellung der Leitung anlegen
        let pens = self.project.stifte.clone();
        let bt_firmen = crate::bauteil_darstellung::firmen(&self.project);
        let bt_darst = match &d {
            XDialog::Bauteil { idx: Some(i), .. } => crate::bauteil_darstellung::darstellung(&self.project, &self.lib, *i),
            _ => ("Keins".to_string(), "Standard".to_string()),
        };
        let mut new_pen_ltg = false;
        let sf_mass = {
            let (x0, _, x1, _) = self.sheet().map(|s| s.frame()).unwrap_or((10.0, 10.0, 287.0, 200.0));
            (x1 - x0, self.project.title_h())
        };
        egui::Modal::new(egui::Id::unique("xdlg")).show(ctx, |ui| {
            ui.style_mut().override_font_id = Some(FontId::proportional(13.0));
            ui.set_min_width(320.0);
            match &mut d {
                XDialog::Stift { name, style, width, color, .. } => {
                    ui.strong("Stift");
                    egui::Grid::new("st").num_columns(2).show(ui, |ui| {
                        // TStift: Name höchstens 19 Zeichen
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).char_limit(19));
                        ui.end_row();
                        ui.label("Breite in mm");
                        ui.text_edit_singleline(width);
                        ui.end_row();
                        ui.label("Stiftart");
                        let arts = ["Durchgängig", "Gestrichelt", "Strich Punkt", "Gepunktet"];
                        egui::ComboBox::from_id_salt("stiftart").selected_text(arts[(*style).min(3)]).show_ui(ui, |ui| {
                            for (i, a) in arts.iter().enumerate() {
                                ui.selectable_value(style, i, *a);
                            }
                        });
                        ui.end_row();
                        ui.label("Farbe");
                        ui.color_edit_button_srgba(color);
                        ui.end_row();
                    });
                }
                XDialog::Leitung { name, color, quer, adern, kennz, width, style, .. } => {
                    // Aufbau wie TLeitung: Name (19 Zeichen), Gruppe „Adern“ (Querschnitt mm², Anzahl, Farbe,
                    // Kennzeichnung), Gruppe „Darstellung“ (Stift mit Bildknopf „Neuer Stift“)
                    ui.horizontal(|ui| {
                        ui.add_sized([47.0, 20.0], egui::Label::new(egui::RichText::new("Name").strong()));
                        ui.add(egui::TextEdit::singleline(name).char_limit(19).desired_width(120.0));
                    });
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Adern").strong());
                        egui::Grid::new("lt").num_columns(3).show(ui, |ui| {
                            ui.label(egui::RichText::new("Querschnitt").strong());
                            if ui.add(egui::TextEdit::singleline(quer).desired_width(60.0)).lost_focus() {
                                *quer = crate::dialog_regeln::zahl_text(quer, 2);
                            }
                            ui.label(egui::RichText::new("mm²").strong());
                            ui.end_row();
                            ui.label(egui::RichText::new("Anzahl").strong());
                            if ui.add(egui::TextEdit::singleline(adern).desired_width(60.0)).lost_focus() {
                                *adern = crate::dialog_regeln::zahl_text(adern, 0);
                            }
                            ui.end_row();
                            ui.label(egui::RichText::new("Farbe").strong());
                            ui.color_edit_button_srgba(color);
                            ui.end_row();
                            ui.label(egui::RichText::new("Kennzeichnung").strong());
                            let k = ["Keine", "Zahlen", "Farbig"];
                            egui::ComboBox::from_id_salt("kennz").width(121.0).selected_text(k[(*kennz).min(2)]).show_ui(ui, |ui| {
                                for (i, a) in k.iter().enumerate() {
                                    ui.selectable_value(kennz, i, *a);
                                }
                            });
                            ui.end_row();
                        });
                    });
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Darstellung").strong());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Stift").strong());
                            // Stift wie im Original (leitung_stift.rs)
                            if crate::leitung_stift::stift_ui(ui, &pens, width, style) {
                                new_pen_ltg = true;
                            }
                        });
                    });
                }
                XDialog::Potential { name, .. } => {
                    ui.strong("Potential");
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(name);
                    });
                }
                XDialog::Columns { text } => {
                    ui.strong("Strompfadbezeichnung");
                    ui.label("Bezeichnungen der Strompfade, durch Komma getrennt (leer = fortlaufende Nummern):");
                    ui.add(egui::TextEdit::multiline(text).desired_width(420.0).desired_rows(3));
                }
                XDialog::Busbar { pots, auto, y, dy, x0, x1 } => {
                    ui.strong("Sammelschienenassistent");
                    ui.label("Erzeugen der Sammelschienen:");
                    ui.horizontal_wrapped(|ui| {
                        for (n, on) in pots.iter_mut() {
                            ui.checkbox(on, n.as_str());
                        }
                    });
                    ui.checkbox(auto, "Auto (unter die vorhandenen Sammelschienen des Blattes)");
                    egui::Grid::new("bb").num_columns(2).show(ui, |ui| {
                        ui.label("Startpunkt Y in mm");
                        ui.add_enabled(!*auto, egui::TextEdit::singleline(y));
                        ui.end_row();
                        ui.label("Abstand in mm");
                        ui.text_edit_singleline(dy);
                        ui.end_row();
                        ui.label("Startpunkt X in mm");
                        ui.text_edit_singleline(x0);
                        ui.end_row();
                        ui.label("Ende X in mm");
                        ui.text_edit_singleline(x1);
                        ui.end_row();
                    });
                }
                XDialog::TextGen { scope_project } => {
                    ui.strong("Textgenerator");
                    ui.label("Betriebsmittelkennzeichnungen aus den Vorlagen der Symbole neu erzeugen (Reihenfolge: Blatt, Strompfad, Höhe).");
                    ui.radio_value(scope_project, false, "Nur aktuelles Blatt");
                    ui.radio_value(scope_project, true, "Ganzes Projekt");
                }
                XDialog::Title { elems } => {
                    // grafisch wie im Symboleditor (schriftfeld_editor.rs), Maße vom aktuellen Blatt
                    crate::schriftfeld_editor::ui(ui, elems, sf_mass.0, sf_mass.1);
                }
                XDialog::Bauteil { b, .. } => {
                    ui.strong("Bauteil");
                    egui::Grid::new("bt").num_columns(2).show(ui, |ui| {
                        for (l, v) in [
                            ("Gruppe", &mut b.gruppe),
                            ("Name", &mut b.name),
                            ("Typ", &mut b.typ),
                            ("Hersteller", &mut b.firma),
                            ("Artikel Nr", &mut b.artikel),
                            ("Intern Nr", &mut b.nummer),
                        ] {
                            ui.label(l);
                            if l == "Hersteller" {
                                // Herstellerliste (BFirma): Hersteller der vorhandenen Bauteile zur Auswahl
                                crate::bauteil_darstellung::firma_ui(ui, v, &bt_firmen);
                            } else {
                                ui.text_edit_singleline(v);
                            }
                            ui.end_row();
                        }
                    });
                    // wie TSBauteil: Abmessungen und Darstellung
                    ui.group(|ui| {
                        ui.label("Abmessungen in mm");
                        egui::Grid::new("btm").num_columns(2).show(ui, |ui| {
                            for (l, v) in [("Breite", &mut b.breite), ("Höhe", &mut b.hoehe), ("Tiefe", &mut b.tiefe)] {
                                ui.label(l);
                                ui.text_edit_singleline(v);
                                ui.end_row();
                            }
                        });
                    });
                    ui.group(|ui| {
                        ui.label("Darstellung");
                        // Symbol und Bild der Symbole, denen das Bauteil zugeordnet ist (bauteil_darstellung.rs)
                        ui.horizontal(|ui| {
                            ui.label("Symbol:");
                            ui.label(bt_darst.0.as_str());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Bild");
                            ui.add_enabled(false, egui::Button::new(bt_darst.1.as_str()));
                        });
                    });
                }
                XDialog::PartAssign { obj, idx } => {
                    ui.strong("Bauteil zuordnen");
                    ui.small("Ein Bauteil beschreibt das reale Gerät (Typ, Hersteller, Artikelnummer, Maße); das Symbol ist seine Darstellung im Plan.");
                    if parts.is_empty() {
                        ui.label("Im Projekt gibt es noch keine Bauteile.");
                    } else {
                        egui::ComboBox::from_id_salt("partsel").selected_text(parts.get(*idx).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                            for (i, p) in parts.iter().enumerate() {
                                ui.selectable_value(idx, i, p);
                            }
                        });
                    }
                    if ui.button("Neues Bauteil anlegen …").clicked() {
                        new_part_for = Some(*obj);
                    }
                }
                XDialog::Platz { name, text, .. } => {
                    ui.strong("Platzhaltergruppe");
                    // wie TSTextliste: Name (höchstens 8 Zeichen) und rechts daneben die vier Bildknöpfe Liste
                    // laden/speichern, Alles löschen, Zeile löschen (letzte Zeile)
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).char_limit(8).desired_width(110.0));
                        ui.add_space(12.0);
                        if crate::tabelle::icon_knopf(ui, crate::icons::Icon::Open, "Liste laden") {
                            if let Some(p) = rfd::FileDialog::new().add_filter("Liste", &["txt"]).pick_file() {
                                if let Ok(t) = std::fs::read_to_string(p) {
                                    *text = t.replace(';', "=");
                                }
                            }
                        }
                        if crate::tabelle::icon_knopf(ui, crate::icons::Icon::Save, "Liste speichern") {
                            if let Some(p) = rfd::FileDialog::new().add_filter("Liste", &["txt"]).set_file_name("Platzhalter.txt").save_file() {
                                let _ = std::fs::write(p, text.as_str());
                            }
                        }
                        if crate::tabelle::icon_knopf(ui, crate::icons::Icon::Delete, "Alles löschen") {
                            text.clear();
                        }
                        if crate::tabelle::icon_knopf(ui, crate::icons::Icon::DelZeile, "Zeile löschen") {
                            let mut l: Vec<&str> = text.lines().collect();
                            l.pop();
                            *text = l.join("\n");
                        }
                    });
                    ui.label("Platzhalter je Zeile als Name=Wert; in Texten wird %{Name} ersetzt:");
                    ui.add(egui::TextEdit::multiline(text).desired_rows(6).desired_width(380.0));
                }
                XDialog::Verweis { text, start } => {
                    ui.strong(if *start { "Verweis – Start" } else { "Verweis – Ziel" });
                    ui.horizontal(|ui| {
                        ui.label("Beschriftung (z. B. /3.4 oder =UV1+W2)");
                        ui.text_edit_singleline(text);
                    });
                }
                XDialog::SymName { name, bmk } => {
                    ui.strong("Symbol");
                    egui::Grid::new("sn").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(name);
                        ui.end_row();
                        ui.label("BMK-Vorlage (%S, %R, %C)");
                        ui.text_edit_singleline(bmk);
                        ui.end_row();
                    });
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("    OK    ").clicked() {
                    ok = true;
                }
                if ui.button("Abbrechen").clicked() {
                    close = true;
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
        if new_pen_ltg {
            if let XDialog::Leitung { width, style, .. } = &d {
                self.new_pen();
                let w = width.trim().replace(',', ".").parse::<f32>().unwrap_or(0.25).clamp(0.05, 5.0);
                if let Some(p) = self.project.stifte.last_mut() {
                    p.width = w;
                    p.style = *style;
                }
            }
        }
        if let Some(obj) = new_part_for {
            self.new_bauteil_dialog();
            self.part_assign_after = Some(obj);
        } else if ok {
            self.apply_xdialog(d);
        } else if !close {
            self.xdlg = Some(d);
        } else {
            self.part_assign_after = None;
        }
    }

    fn apply_xdialog(&mut self, d: XDialog) {
        let num = |s: &str, def: f32| s.trim().replace(',', ".").parse::<f32>().unwrap_or(def);
        match d {
            XDialog::Stift { idx, name, style, width, color } if idx > usize::MAX / 2 => {
                // Stift des Symbols im Editor (Kennung usize::MAX − Stiftnummer)
                // neuer Stift („Neuer Stift“) entsteht erst hier, beim OK (stift_neu.rs)
                let nr = (usize::MAX - idx) as u32;
                let s = Stift { name, style: style as u8, width: num(&width, 0.25).clamp(0.05, 5.0), color: from_c32(color) };
                if let Some(e) = self.sym_edit.as_mut() {
                    if crate::stift_neu::symbol_setzen(&mut e.pens, nr, s) {
                        self.pen = nr as usize;
                    }
                }
            }
            XDialog::Stift { idx, name, style, width, color } => {
                self.checkpoint();
                let s = Stift { name, style: style as u8, width: num(&width, 0.25).clamp(0.05, 5.0), color: from_c32(color) };
                if crate::stift_neu::projekt_setzen(&mut self.project.stifte, idx, s) {
                    // neuer Projektstift wird gewählt (im Symboleditor nur in der Hauptleiste)
                    match self.sym_edit.as_mut() {
                        Some(e) => e.haupt_pen = idx,
                        None => self.pen = idx,
                    }
                    self.dirty = true;
                }
            }
            XDialog::Leitung { idx, name, color, quer, adern, kennz, width, style } => {
                self.checkpoint();
                if let Some(l) = self.project.leitungen.get_mut(idx) {
                    l.name = name;
                    l.color = from_c32(color);
                    l.quer = num(&quer, 1.5);
                    l.adern = adern.trim().parse().unwrap_or(3);
                    l.kennz = kennz as u8;
                    l.width_mm = num(&width, 0.25).clamp(0.05, 5.0);
                    l.style = style;
                }
            }
            XDialog::Potential { idx, name } => {
                self.checkpoint();
                if let Some(p) = self.project.potentiale.get_mut(idx) {
                    *p = name;
                }
            }
            XDialog::Columns { text } => {
                let labels: Vec<String> = text.split(',').map(|s| s.trim().to_string()).collect();
                self.checkpoint();
                self.with_sheet(|s, _| {
                    s.col_labels = if labels.iter().all(|l| l.is_empty()) { vec![] } else { labels.clone() };
                    if labels.len() as u32 > s.columns && labels.iter().any(|l| !l.is_empty()) {
                        s.columns = labels.len() as u32;
                    }
                });
            }
            XDialog::Busbar { pots, auto, y, dy, x0, x1 } => {
                let (y, dy, x0, x1) = (num(&y, 40.0), num(&dy, 10.0), num(&x0, 25.0), num(&x1, 282.0));
                // wie im Original (TS_Assi): nur auf Schaltplänen, mindestens eine Schiene gewählt
                if self.sheet().map(|s| !matches!(s.art, 0 | 3)).unwrap_or(true) {
                    crate::meldungen::hinweis(crate::meldungen::T_ACHTUNG, "Sammelschienen können hier nicht eingefügt werden");
                    return;
                }
                if !pots.iter().any(|(_, on)| *on) {
                    crate::meldungen::hinweis("Auswahl", crate::meldungen::KEINE_SAMMELSCHIENE);
                    return;
                }
                self.checkpoint();
                let a = self.cur().map(|r| r.anlage).unwrap_or(0);
                let leitung = self.leitung;
                // Auto: im Abstand unter die unterste vorhandene Sammelschiene des Blattes
                let y = match (auto, self.sheet()) {
                    (true, Some(s)) => busbar_auto_y(s, dy),
                    _ => y,
                };
                let mut k = 0;
                for (name, on) in pots {
                    if !on {
                        continue;
                    }
                    let yy = y + dy * k as f32;
                    k += 1;
                    let id = self.project.alloc_id();
                    let tid = self.project.alloc_id();
                    // vorhandene Sammelschiene gleichen Namens verwenden, sonst anlegen; PE wie im Original
                    // mit Stift "Strich/Punkt"
                    let pen = if name == "PE" { 2 } else { 0 };
                    if let Some(anl) = self.project.anlagen.get_mut(a) {
                        if !anl.sammelschienen.contains(&name) {
                            anl.sammelschienen.push(name.clone());
                            if pen != 0 {
                                anl.schienen.entry(name.clone()).or_default().stift = pen;
                            }
                        }
                    }
                    self.with_sheet(|s, _| {
                        s.objects.push(Obj { id, kind: Kind::Wire { pts: vec![P::new(x0, yy), P::new(x1, yy)], leitung, bez: vec![], opts: 0 }, pen, fixed: false, group: None, raw: None });
                        s.objects.push(Obj { id: tid, kind: Kind::Text { pos: P::new(x0 - 1.0, yy), text: name.clone(), size: 2.5, align: Align::Right, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
                    });
                }
                self.info = format!("{k} Sammelschienen angelegt");
            }
            XDialog::TextGen { scope_project } => self.regenerate_bmk(scope_project),
            XDialog::Title { elems } => {
                self.checkpoint();
                self.project.titleblock = elems;
            }
            XDialog::Bauteil { idx, b } => {
                self.checkpoint();
                match idx {
                    Some(i) => {
                        if let Some(x) = self.project.bauteile.get_mut(i) {
                            *x = b;
                        }
                    }
                    None => {
                        self.project.bauteile.push(b);
                        // aus „Bauteil zuordnen“ heraus angelegt: gleich dem Symbol zuordnen
                        if let Some(obj) = self.part_assign_after.take() {
                            self.project.part_of.insert(obj, self.project.bauteile.len() - 1);
                            self.dirty = true;
                        }
                    }
                }
                self.tree_open.insert("bt/user".into());
                self.tree_open.insert("bt/user/Projekt".into());
            }
            XDialog::PartAssign { obj, idx } => {
                if idx < self.project.bauteile.len() {
                    self.checkpoint();
                    self.project.part_of.insert(obj, idx);
                }
            }
            XDialog::Platz { idx, name, text } => {
                self.checkpoint();
                if let Some(g) = self.project.platzhalter.get_mut(idx) {
                    g.name = name;
                    g.werte = text
                        .lines()
                        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
                        .filter(|(k, _)| !k.is_empty())
                        .collect();
                }
            }
            XDialog::Verweis { text, start } => {
                let at = self.snap(self.mouse_ctx);
                self.add_reference(at, start, text);
            }
            XDialog::SymName { name, bmk } => {
                if let Some(e) = &mut self.sym_edit {
                    e.name = name;
                    e.bmk = bmk;
                }
            }
        }
    }
}

/// Vorgabe-Schriftfeld (von "Schriftfeld speichern"), falls vorhanden.
/// Vorgabeschriftfeld (Datei › Schriftfeld speichern), sonst das Standard-Schriftfeld.
pub fn default_titleblock() -> Vec<TbOwned> {
    load_default_titleblock().unwrap_or_else(titleblock::default_owned)
}

pub fn load_default_titleblock() -> Option<Vec<TbOwned>> {
    let s = std::fs::read_to_string(appdir().join("schriftfeld.json")).ok()?;
    serde_json::from_str(&s).ok()
}

/// Sammelschienen des Assistenten (Original: feste Liste L1, L2, L3, N, PE).
pub const BUSBAR_NAMES: [&str; 5] = ["L1", "L2", "L3", "N", "PE"];

/// Startpunkt Y für "Auto": im Abstand `dy` unter der untersten Sammelschiene des Blattes, ohne Sammelschiene
/// `dy` unter der Strompfadleiste.
pub fn busbar_auto_y(s: &Sheet, dy: f32) -> f32 {
    let lowest = s.objects.iter().filter_map(|o| if let Kind::Busbar { a, .. } = &o.kind { Some(a.y) } else { None }).fold(f32::MIN, f32::max);
    if lowest > f32::MIN {
        lowest + dy
    } else {
        s.frame().1 + COLUMN_BAR_H + dy
    }
}
