//! Oberfläche: Menü, Werkzeugleisten, Bäume, Blattreiter, Statusleiste, Zeichenfläche und Dialoge.

use crate::app::*;
use crate::canvas::{self, DrawCtx, SheetColors, Xf};
use crate::icons::{self, Icon};
use crate::model::*;
use crate::symbols::{self, SymDef};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Event, FontId, Frame, Key, Margin, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::sync::atomic::Ordering;

const BG: Color32 = Color32::from_rgb(240, 240, 240);
const VIEW_BG: Color32 = Color32::from_rgb(128, 128, 128);
const SEL: Color32 = Color32::from_rgb(220, 0, 0);
const PREVIEW: Color32 = Color32::from_rgb(0, 0, 200);

pub fn tool_button(ui: &mut egui::Ui, icon: Icon, hint: &str, enabled: bool, down: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(27.0, 26.0), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if down {
        p.rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(204, 228, 247));
        p.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0, Color32::from_rgb(0, 84, 153)), StrokeKind::Inside);
    } else if enabled && resp.hovered() {
        p.rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(229, 243, 255));
        p.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0, Color32::from_rgb(204, 232, 255)), StrokeKind::Inside);
    }
    icons::draw(ui, Rect::from_center_size(rect.center(), Vec2::splat(20.0)), icon, enabled);
    if hint.is_empty() {
        resp
    } else {
        resp.on_hover_text(hint)
    }
}

pub fn sep(ui: &mut egui::Ui, w: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::hover());
    let x = r.center().x.round() - 0.5;
    ui.painter().line_segment([Pos2::new(x, r.top() + 2.0), Pos2::new(x, r.bottom() - 2.0)], Stroke::new(1.0, Color32::from_gray(200)));
}

pub fn static_text(ui: &mut egui::Ui, text: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::hover());
    ui.painter().text(Pos2::new(r.left() + 1.0, r.top() + 1.0), Align2::LEFT_TOP, text, FontId::proportional(16.0), Color32::BLACK);
}

pub fn auswahl(ui: &mut egui::Ui, id: &str, w: f32, items: &[String], idx: &mut usize) -> bool {
    let before = *idx;
    let sel = items.get(*idx).cloned().unwrap_or_default();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::hover());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(r.left(), r.top() + 2.0), Vec2::new(w, 22.0))));
    {
        let st = child.style_mut();
        for wv in [&mut st.visuals.widgets.inactive, &mut st.visuals.widgets.hovered, &mut st.visuals.widgets.active, &mut st.visuals.widgets.open] {
            wv.weak_bg_fill = Color32::WHITE;
            wv.bg_fill = Color32::WHITE;
            wv.corner_radius = CornerRadius::ZERO;
            wv.expansion = 0.0;
        }
        st.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(122));
        st.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0, 120, 215));
        st.spacing.button_padding = Vec2::new(3.0, 3.0);
        st.spacing.interact_size.y = 20.0;
        st.override_font_id = Some(FontId::proportional(11.0));
    }
    egui::ComboBox::from_id_salt(id).width(w - 6.0).height(300.0).selected_text(sel).show_ui(&mut child, |ui| {
        for (i, it) in items.iter().enumerate() {
            ui.selectable_value(idx, i, it);
        }
    });
    *idx != before
}

fn star(p: &egui::Painter, c: Pos2, r: f32) {
    let mut pts = vec![];
    for i in 0..10 {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.45 };
        pts.push(Pos2::new(c.x + rr * a.cos(), c.y + rr * a.sin()));
    }
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.0, Color32::from_rgb(120, 90, 0))));
}

impl App {
    fn top_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            if tool_button(ui, Icon::NewProject, "Neues Projekt", true, false).clicked() {
                self.guarded(crate::app::Pending::New);
            }
            if tool_button(ui, Icon::Open, "Projekt laden", true, false).clicked() {
                self.guarded(crate::app::Pending::Open);
            }
            if tool_button(ui, Icon::Save, "Speichern", self.dirty, false).clicked() {
                self.save();
            }
            if tool_button(ui, Icon::SaveProject, "Projekt speichen", self.dirty, false).clicked() {
                self.save();
            }
            if tool_button(ui, Icon::Print, "Drucken", true, false).clicked() {
                self.pdf_req = Some((std::env::temp_dir().join("Elektroplanung_Druck.pdf"), true));
            }
            sep(ui, 5.0);
            let has_sel = !self.selection.is_empty();
            if tool_button(ui, Icon::Cut, "Schneidet alle markierten Objekte aus und fügt sie in die Zwischenablage ein", has_sel, false).clicked() {
                self.copy();
                self.delete_selection();
            }
            if tool_button(ui, Icon::Copy, "Kopiert alle markierten Objekte in die Zwischenablage", has_sel, false).clicked() {
                self.copy();
            }
            if tool_button(ui, Icon::Paste, "Fügt alle Objekte der Zwischenablage in das aktive Blatt ein", !self.clipboard.is_empty(), false).clicked() {
                self.paste();
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::Undo, "Rückgängig", !self.undo.is_empty(), false).clicked() {
                self.do_undo();
            }
            if tool_button(ui, Icon::Redo, "Wiederherstellen", !self.redo.is_empty(), false).clicked() {
                self.do_redo();
            }
            sep(ui, 5.0);
            static_text(ui, "Stift", 33.0);
            let pens: Vec<String> = self.project.stifte.iter().map(|s| s.name.clone()).collect();
            let mut pen = self.pen;
            if auswahl(ui, "stift", 100.0, &pens, &mut pen) {
                self.pen = pen;
                if !self.selection.is_empty() {
                    self.checkpoint();
                    let sel = self.selection.clone();
                    self.with_sheet(|s, _| s.objects.iter_mut().filter(|o| sel.contains(&o.id)).for_each(|o| o.pen = pen));
                }
            }
            if tool_button(ui, Icon::NewPen, "Neuer Stift", true, false).clicked() {
                self.new_pen();
                self.open_pen_dialog(self.pen);
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::Zoom, "Vergrößern/Verkleinern", true, self.zoom_tool).clicked() {
                self.zoom_tool = !self.zoom_tool;
            }
            sep(ui, 5.0);
            static_text(ui, "Rasterfang", 84.0);
            let rs: Vec<String> = RASTER.iter().map(|r| r.0.to_string()).collect();
            let mut ri = self.raster_idx;
            let rr = ui.scope(|ui| auswahl(ui, "raster", 67.0, &rs, &mut ri));
            rr.response.on_hover_text("Rasterauswahl");
            if rr.inner {
                self.raster_idx = ri;
            }
            if tool_button(ui, Icon::Grid, "Rasterfanggitter anzeigen", true, self.grid).clicked() {
                self.grid = !self.grid;
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::PathAlign, "Strompfadausrichtung (Taste S)", true, self.path_align).clicked() {
                self.path_align = !self.path_align;
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::AutoConn, "Automatische Verbindungen (Taste A)", true, self.auto_conn).clicked() {
                self.auto_conn = !self.auto_conn;
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::SizeFrame, "Rahmen zur Größenänderung", true, self.size_frame).clicked() {
                self.size_frame = !self.size_frame;
            }
            sep(ui, 5.0);
            if tool_button(ui, Icon::Fix, "Fixierung aktivieren", true, self.fix_active).clicked() {
                self.fix_active = !self.fix_active;
            }
        });
    }

    fn second_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.add_space(29.0);
            if tool_button(ui, Icon::NewSheet, "Neues Blatt", true, false).clicked() {
                self.new_sheet();
            }
            if tool_button(ui, Icon::NewPotential, "Neues Potential", true, false).clicked() {
                self.new_potential();
            }
            if tool_button(ui, Icon::NewBusbar, "Neue Sammelschiene", true, false).clicked() {
                self.new_busbar();
            }
            sep(ui, 5.0);
            let has_sel = !self.selection.is_empty();
            if tool_button(ui, Icon::Rotate, "Drehen", true, false).clicked() && has_sel {
                self.transform_selection(1);
            }
            if tool_button(ui, Icon::MirrorV, "Vertikal Spiegeln", true, false).clicked() && has_sel {
                self.transform_selection(2);
            }
            if tool_button(ui, Icon::MirrorH, "Horizontal Spiegeln", true, false).clicked() && has_sel {
                self.transform_selection(4);
            }
            sep(ui, 8.0);
            if tool_button(ui, Icon::Group, "Markierte Zeichenobjekte gruppieren", self.selection.len() > 1, false).clicked() {
                self.group_selection(true);
            }
            if tool_button(ui, Icon::Ungroup, "Markierte Gruppe auflösen", has_sel, false).clicked() {
                self.group_selection(false);
            }
            sep(ui, 5.0);
            static_text(ui, "Leitung", 57.0);
            let ls: Vec<String> = self.project.leitungen.iter().map(|l| l.name.clone()).collect();
            let mut li = self.leitung;
            let lr = ui.scope(|ui| auswahl(ui, "leitung", 100.0, &ls, &mut li));
            lr.response.on_hover_text("Leitungsliste");
            if lr.inner {
                self.leitung = li;
                let sel: Vec<u64> = self.selected_objs().iter().filter(|o| matches!(o.kind, Kind::Wire { .. })).map(|o| o.id).collect();
                if !sel.is_empty() {
                    self.checkpoint();
                    self.with_sheet(|s, _| {
                        for o in s.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
                            if let Kind::Wire { leitung, .. } = &mut o.kind {
                                *leitung = li;
                            }
                        }
                    });
                }
            }
            if tool_button(ui, Icon::NewLeitung, "Neue Leitung", true, false).clicked() {
                self.new_leitung();
                self.open_leitung_dialog(self.leitung);
            }
            sep(ui, 5.0);
            static_text(ui, "Klemmleiste", 89.0);
            let ks = self.klemmleisten();
            let mut ki = self.klemme.min(ks.len().saturating_sub(1));
            let kr = ui.scope(|ui| auswahl(ui, "klemme", 105.0, &ks, &mut ki));
            kr.response.on_hover_text("Klemmenliste der aktuellen Anlage");
            if kr.inner {
                self.klemme = ki;
            }
            if tool_button(ui, Icon::NewKlemme, "Neue Klemmleiste", true, false).clicked() {
                self.new_klemmleiste();
            }
        });
    }

    fn left_toolbar(&mut self, ui: &mut egui::Ui) {
        let tools = [
            (Tool::Select, Icon::Select, "Auswahl"),
            (Tool::Klemmstelle, Icon::Klemmstelle, "Neue Klemmstelle"),
            (Tool::Line, Icon::Line, "Neue Linie"),
            (Tool::Curve, Icon::Curve, "Neue Kurve"),
            (Tool::Rect, Icon::Rect, "Neues Rechteck"),
            (Tool::Triangle, Icon::Triangle, "Neues Dreieck"),
            (Tool::Circle, Icon::Circle, "Neuer Kreis"),
            (Tool::Text, Icon::Text, "Neuer Text"),
            (Tool::TextBox, Icon::TextBox, "Neue Textbox"),
            (Tool::Table, Icon::Table, "Neue Tabelle"),
            (Tool::Image, Icon::Image, "Neues Bild"),
        ];
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for (t, ic, hint) in tools {
                if tool_button(ui, ic, hint, true, self.tool == t && self.placing.is_none() && !self.zoom_tool).clicked() {
                    self.tool = t;
                    self.placing = None;
                    self.zoom_tool = false;
                }
            }
        });
    }

    // ------------------------------------------------------------------ Bäume

    fn symbol_tree(&self) -> Vec<TNode> {
        let filter = self.search.to_lowercase();
        let pick = |f: &dyn Fn(&SymDef) -> bool| -> Vec<&SymDef> {
            self.lib.iter().filter(|d| f(d) && (filter.is_empty() || d.name.to_lowercase().contains(&filter))).collect()
        };
        let fav = pick(&|d| d.fav);
        let user = pick(&|d| d.haupt == 200 || d.haupt == 0);
        let std = pick(&|d| d.haupt == 300 || d.haupt == 100);
        vec![
            TNode { id: "sym/none".into(), label: "Keine Zuordnung".into(), bold: false, icon: 0, action: TAction::None, children: vec![] },
            TNode { id: "sym/fav".into(), label: "Favoriten".into(), bold: true, icon: 2, action: TAction::None, children: group_nodes("sym/fav", &fav, 0) },
            TNode { id: "sym/user".into(), label: "Benutzerbauteile".into(), bold: true, icon: 1, action: TAction::None, children: group_nodes("sym/user", &user, 0) },
            TNode { id: "sym/std".into(), label: "Standardbauteile".into(), bold: true, icon: 1, action: TAction::None, children: group_nodes("sym/std", &std, 0) },
        ]
    }

    fn part_tree(&self) -> Vec<TNode> {
        let mk = |id: &str, label: &str, bold: bool, icon: u8| TNode { id: id.into(), label: label.into(), bold, icon, action: TAction::None, children: vec![] };
        let mut user = mk("bt/user", "Benutzerbauteile", true, 1);
        let mut groups: std::collections::BTreeMap<String, Vec<TNode>> = std::collections::BTreeMap::new();
        for (i, b) in self.project.bauteile.iter().enumerate() {
            let g = if b.gruppe.is_empty() { "Projekt".to_string() } else { b.gruppe.clone() };
            groups.entry(g).or_default().push(TNode { id: format!("bt/p/{i}"), label: format!("{} {}", b.name, b.typ).trim().to_string(), bold: false, icon: 0, action: TAction::Bauteil(i), children: vec![] });
        }
        for (g, ch) in groups {
            user.children.push(TNode { id: format!("bt/user/{g}"), label: g, bold: true, icon: 1, action: TAction::None, children: ch });
        }
        vec![mk("bt/none", "Keine Zuordnung", false, 0), mk("bt/fav", "Favoriten", true, 2), user, mk("bt/std", "Standardbauteile", true, 1)]
    }

    /// Projektbaum in der Reihenfolge des Originals: Potentiale, Leitungen, Objektkatalog, Stifte, Grafiken,
    /// Bauteile, Platzhaltergruppen, danach die Anlagen.
    fn project_tree(&self) -> Vec<TNode> {
        let mut out = vec![];
        let grp = |id: &str, label: &str, children: Vec<TNode>| TNode { id: id.into(), label: label.into(), bold: true, icon: 1, action: TAction::None, children };
        let leaf = |id: String, label: String, action: TAction| TNode { id, label, bold: false, icon: 0, action, children: vec![] };
        out.push(grp("prj/pot", "Potentiale", self.project.potentiale.iter().enumerate().map(|(i, l)| leaf(format!("prj/pot/{i}"), l.clone(), TAction::Potential(i))).collect()));
        out.push(grp("prj/ltg", "Leitungen", self.project.leitungen.iter().enumerate().map(|(i, l)| leaf(format!("prj/ltg/{i}"), l.name.clone(), TAction::Leitung(i))).collect()));
        out.push(grp("prj/sym", "Objektkatalog", self.project.symdefs.iter().map(|d| leaf(format!("prj/sym/{}", d.key), d.name.clone(), TAction::Symbol(d.key.clone()))).collect()));
        out.push(grp("prj/stift", "Stifte", self.project.stifte.iter().enumerate().map(|(i, l)| leaf(format!("prj/stift/{i}"), l.name.clone(), TAction::Stift(i))).collect()));
        let mut bilder: Vec<String> = self.project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter_map(|o| match &o.kind {
            Kind::Image { path, .. } => Some(std::path::Path::new(path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| path.clone())),
            _ => None,
        }).collect();
        bilder.retain(|n| !n.starts_with("bld:"));
        bilder.dedup();
        // Bilder des Projekts (aus .epr: PRJ › BLD) zuerst; Doppelklick setzt das Bild auf das Blatt
        let mut nodes: Vec<TNode> = self.project.bilder.iter().enumerate().map(|(i, b)| leaf(format!("prj/bld/{i}"), b.name.clone(), TAction::Bild(i))).collect();
        nodes.extend(bilder.into_iter().enumerate().map(|(i, n)| leaf(format!("prj/img/{i}"), n, TAction::None)));
        out.push(grp("prj/img", "Grafiken", nodes));
        out.push(grp("prj/bt", "Bauteile", self.project.bauteile.iter().enumerate().map(|(i, b)| leaf(format!("prj/bt/{i}"), b.name.clone(), TAction::None)).collect()));
        out.push(grp("prj/ph", "Platzhaltergruppen", self.project.platzhalter.iter().enumerate().map(|(i, g)| leaf(format!("prj/ph/{i}"), g.name.clone(), TAction::Platz(i))).collect()));
        for (ai, a) in self.project.anlagen.iter().enumerate() {
            let leaf = |id: String, label: String| TNode { id, label, bold: false, icon: 0, action: TAction::None, children: vec![] };
            let mk = |id: String, label: String, action: TAction| TNode { id, label, bold: false, icon: 0, action, children: vec![] };
            let ss: Vec<TNode> = a.sammelschienen.iter().enumerate().map(|(i, n)| mk(format!("prj/a{ai}/ss/{i}"), n.clone(), TAction::Schiene(ai, i))).collect();
            let kl: Vec<TNode> = a.klemmleisten.iter().enumerate().map(|(i, n)| mk(format!("prj/a{ai}/kl/{i}"), n.clone(), TAction::Klemmleiste(ai, i))).collect();
            let _ = &leaf;
            let bl: Vec<TNode> = a
                .sheets
                .iter()
                .enumerate()
                .map(|(si, s)| TNode {
                    id: format!("prj/a{ai}/bl/{si}"),
                    label: format!("[{}] {}", s.number, s.name),
                    bold: false,
                    icon: 3,
                    action: TAction::Sheet(SheetRef { anlage: ai, sheet: si }),
                    children: vec![],
                })
                .collect();
            out.push(TNode {
                id: format!("prj/a{ai}"),
                label: a.name.clone(),
                bold: true,
                icon: 4,
                action: TAction::None,
                children: vec![
                    TNode { id: format!("prj/a{ai}/ss"), label: "Sammelschienen".into(), bold: true, icon: 1, action: TAction::None, children: ss },
                    TNode { id: format!("prj/a{ai}/kl"), label: "Klemmleisten".into(), bold: true, icon: 1, action: TAction::None, children: kl },
                    TNode { id: format!("prj/a{ai}/bl"), label: "Blätter".into(), bold: true, icon: 1, action: TAction::None, children: bl },
                ],
            });
        }
        out
    }

    fn draw_tree(&mut self, ui: &mut egui::Ui, nodes: &[TNode]) {
        let top = ui.max_rect().top();
        let mut y = top + 4.0;
        let x = ui.max_rect().left() + 2.0;
        let mut ev = TreeEvents::default();
        let n = nodes.len();
        for (i, node) in nodes.iter().enumerate() {
            self.draw_node(ui, node, 0, x, &mut y, i + 1 == n, &mut ev);
        }
        ui.allocate_space(Vec2::new(ui.available_width().max(10.0), (y - top).max(10.0)));
        if let Some(t) = ev.toggled {
            if !self.tree_open.remove(&t) {
                self.tree_open.insert(t);
            }
        }
        if let Some(n) = ev.clicked {
            self.tree_sel = Some(n.id.clone());
            if let TAction::Symbol(k) = &n.action {
                self.placing = Some(k.clone());
                self.zoom_tool = false;
            }
        }
        if let Some(n) = ev.dbl {
            match &n.action {
                TAction::Sheet(r) => self.open_sheet(*r),
                TAction::Bild(i) => self.place_project_image(*i),
                TAction::Symbol(_) | TAction::None => {}
                _ => self.tree_context(&n, "Eigenschaften"),
            }
        }
        if let Some((n, a)) = ev.ctx {
            self.tree_context(&n, &a);
        }
        if let Some(r) = ev.drag_start {
            self.tree_drag = Some(r);
        }
        if let (Some(src), Some(dst)) = (self.tree_drag, ev.drop_on) {
            self.move_sheet(src.anlage, src.sheet, dst.sheet);
            self.tree_drag = None;
        }
        if ui.input(|i| i.pointer.any_released()) {
            self.tree_drag = None;
        }
    }

    fn draw_node(&self, ui: &mut egui::Ui, node: &TNode, depth: usize, x0: f32, y: &mut f32, last: bool, ev: &mut TreeEvents) {
        let row_h = if node.bold { 15.0 } else { 13.0 };
        let indent = 13.0;
        let lx = x0 + depth as f32 * indent;
        let lc = Stroke::new(1.0, Color32::from_gray(150));
        let p = ui.painter().clone();
        let mid = (*y + row_h / 2.0).round() + 0.5;
        let hx = lx.round() + 3.5;
        let top_y = if depth == 0 && (*y - ui.max_rect().top()) < 5.0 { mid } else { *y - 1.0 };
        p.line_segment([Pos2::new(hx, top_y), Pos2::new(hx, if last { mid } else { *y + row_h })], lc);
        p.line_segment([Pos2::new(hx, mid), Pos2::new(hx + 6.0, mid)], lc);
        let mut tx = hx + 7.0;
        if node.icon > 0 {
            match node.icon {
                3 => {
                    let ir = Rect::from_min_size(Pos2::new(tx, mid - 5.5), Vec2::new(9.0, 11.0));
                    p.rect_filled(ir, 0.0, Color32::WHITE);
                    p.rect_stroke(ir, 0.0, Stroke::new(1.0, Color32::from_gray(80)), StrokeKind::Inside);
                    tx += 11.0;
                }
                4 => {
                    let r2 = Rect::from_min_size(Pos2::new(tx, mid - 5.0), Vec2::new(13.0, 10.0));
                    p.rect_filled(r2, 1.0, Color32::from_rgb(240, 200, 80));
                    p.rect_stroke(r2, 1.0, Stroke::new(1.0, Color32::from_rgb(150, 110, 20)), StrokeKind::Inside);
                    tx += 15.0;
                }
                _ => {
                    let ir = Rect::from_min_size(Pos2::new(tx, mid - 6.0), Vec2::new(10.0, 12.0));
                    p.rect_filled(ir, 0.0, Color32::from_rgb(0, 210, 220));
                    p.rect_stroke(ir, 0.0, Stroke::new(1.0, Color32::from_rgb(0, 50, 110)), StrokeKind::Inside);
                    p.line_segment([Pos2::new(ir.left() + 3.0, ir.top()), Pos2::new(ir.left() + 3.0, ir.bottom())], Stroke::new(1.0, Color32::from_rgb(0, 50, 110)));
                    tx += 11.0;
                    if node.icon == 2 {
                        star(&p, Pos2::new(tx + 5.0, mid), 5.5);
                        tx += 11.0;
                    }
                }
            }
        }
        let font = if node.bold { FontId::new(12.0, egui::FontFamily::Name("bold".into())) } else { FontId::proportional(11.0) };
        let selected = self.tree_sel.as_deref() == Some(node.id.as_str());
        let galley = p.layout_no_wrap(node.label.clone(), font, Color32::BLACK);
        let tr = Rect::from_min_size(Pos2::new(tx + 1.0, mid - galley.size().y / 2.0), galley.size());
        let resp = ui.interact(Rect::from_min_max(Pos2::new(hx, *y), Pos2::new(tr.right() + 4.0, *y + row_h)), ui.scope_id().with(&node.id), Sense::click_and_drag());
        if selected {
            p.rect_filled(tr.expand(1.0), 0.0, Color32::from_rgb(0, 120, 215));
            p.galley(tr.min, galley, Color32::WHITE);
        } else {
            p.galley(tr.min, galley, Color32::BLACK);
        }
        if resp.clicked() || resp.drag_started() {
            if node.bold {
                ev.toggled = Some(node.id.clone());
            }
            ev.clicked = Some(node.clone());
        }
        if resp.double_clicked() {
            ev.dbl = Some(node.clone());
        }
        // Blätter per Ziehen umsortieren (innerhalb der Anlage)
        if let TAction::Sheet(r) = &node.action {
            if resp.drag_started() {
                ev.drag_start = Some(*r);
            }
            if let Some(src) = self.tree_drag {
                if src.anlage == r.anlage && ui.input(|inp| inp.pointer.interact_pos()).is_some_and(|q| resp.rect.contains(q)) {
                    let mark = Rect::from_min_max(Pos2::new(hx, *y), Pos2::new(tr.right() + 4.0, *y + 2.0));
                    p.rect_filled(mark, 0.0, Color32::from_rgb(0, 120, 215));
                    if ui.input(|i| i.pointer.any_released()) {
                        ev.drop_on = Some(*r);
                    }
                }
            }
        }
        // Symbolvorschau als Tooltip (vorschau.rs)
        let resp = match &node.action {
            TAction::Symbol(k) => match symbols::find(&self.lib, k).or_else(|| symbols::find(&self.project.symdefs, k)) {
                Some(d) if self.tree_drag.is_none() => resp.on_hover_ui(|ui| crate::vorschau::symbol_vorschau(ui, d)),
                _ => resp,
            },
            _ => resp,
        };
        resp.context_menu(|ui| {
            ui.style_mut().override_font_id = Some(FontId::proportional(12.0));
            self.tree_menu(ui, node, ev);
        });
        *y += row_h;
        if self.tree_open.contains(&node.id) {
            let start = *y;
            let n = node.children.len();
            for (i, c) in node.children.iter().enumerate() {
                self.draw_node(ui, c, depth + 1, x0, y, i + 1 == n, ev);
            }
            if n > 0 && !last {
                p.line_segment([Pos2::new(hx, start - 1.0), Pos2::new(hx, *y)], lc);
            }
        }
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        // Trennleiste (Breite des Projektexplorers mit der Maus ändern)
        let split = Rect::from_min_max(Pos2::new(full.left() - 3.0, full.top()), Pos2::new(full.left() + 1.0, full.bottom()));
        let sr = ui.interact(split, ui.scope_id().with("splitter"), Sense::drag());
        if sr.hovered() || sr.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if sr.dragged() {
            self.right_w = (self.right_w - sr.drag_delta().x).clamp(150.0, 900.0);
        }
        let tabs = [(RightTab::Projekt, "Projekt", 45.0), (RightTab::Symbole, "Symbole", 52.0), (RightTab::Bauteile, "Bauteile", 50.0)];
        let mut x = full.left() + 2.0;
        let ty = full.top() + 2.0;
        let body = Rect::from_min_max(Pos2::new(full.left() + 2.0, ty + 20.0), Pos2::new(full.right() - 2.0, full.bottom() - 1.0));
        ui.painter().rect_filled(body, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(body, 0.0, Stroke::new(1.0, Color32::from_gray(217)), StrokeKind::Inside);
        for (t, label, w) in tabs {
            let sel = self.right_tab == t;
            let r = if sel { Rect::from_min_size(Pos2::new(x - 1.0, ty - 1.0), Vec2::new(w + 2.0, 22.0)) } else { Rect::from_min_size(Pos2::new(x, ty + 1.0), Vec2::new(w, 19.0)) };
            let resp = ui.interact(r, ui.scope_id().with(label), Sense::click());
            let p = ui.painter();
            p.rect_filled(r, 0.0, if sel { Color32::WHITE } else { Color32::from_rgb(240, 240, 240) });
            p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(217)), StrokeKind::Inside);
            if sel {
                p.line_segment([Pos2::new(r.left() + 1.0, r.bottom() - 0.5), Pos2::new(r.right() - 1.0, r.bottom() - 0.5)], Stroke::new(2.0, Color32::WHITE));
            }
            p.text(Pos2::new(r.center().x, r.center().y), Align2::CENTER_CENTER, label, FontId::proportional(11.0), Color32::BLACK);
            if resp.clicked() {
                self.right_tab = t;
            }
            x += w;
        }
        let inner = body.shrink2(Vec2::new(3.0, 3.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.set_clip_rect(inner);
        egui::ScrollArea::both().auto_shrink([false, false]).show(&mut child, |ui| {
            let nodes = match self.right_tab {
                RightTab::Projekt => self.project_tree(),
                RightTab::Symbole => self.symbol_tree(),
                RightTab::Bauteile => self.part_tree(),
            };
            self.draw_tree(ui, &nodes);
        });
    }

    /// Symboleditor: Reiterleiste der Seiten (Bildvarianten) wie im Original statt der Blattreiter.
    fn page_tabs(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        let Some(e) = &self.sym_edit else { return };
        let names: Vec<String> = if e.options.is_empty() { vec!["Standard".into()] } else { e.options.clone() };
        let cur = if e.cur_flag > 0 { usize::MAX } else { e.option as usize };
        let mut x = full.left() + 31.0;
        let mut clicked = None;
        let mut action: Option<(usize, u8)> = None;
        let mut drop_page: Option<(usize, usize)> = None;
        for (i, n) in names.iter().enumerate() {
            let g = ui.painter().layout_no_wrap(n.clone(), FontId::proportional(11.0), Color32::BLACK);
            let w = g.size().x + 12.0;
            let sel = i == cur;
            let rr = Rect::from_min_size(Pos2::new(x, full.top() + 1.0), Vec2::new(w, if sel { 20.0 } else { 18.0 }));
            let resp = ui.interact(rr, ui.scope_id().with(("pagetab", i)), Sense::click_and_drag());
            if resp.drag_started() {
                self.page_drag = Some(i);
            }
            if let Some(src) = self.page_drag {
                if src != i && ui.input(|inp| inp.pointer.interact_pos()).is_some_and(|q| rr.contains(q)) {
                    ui.painter().rect_filled(Rect::from_min_size(rr.left_top(), Vec2::new(3.0, rr.height())), 0.0, Color32::from_rgb(0, 120, 215));
                    if ui.input(|inp| inp.pointer.any_released()) {
                        drop_page = Some((src, i));
                    }
                }
            }
            let p = ui.painter();
            p.rect_filled(rr, 0.0, if sel { Color32::WHITE } else { BG });
            p.rect_stroke(rr, 0.0, Stroke::new(1.0, Color32::from_gray(217)), StrokeKind::Inside);
            p.galley(Pos2::new(rr.left() + 6.0, rr.top() + 3.0), g, Color32::BLACK);
            if resp.clicked() {
                clicked = Some(i);
            }
            resp.context_menu(|ui| {
                if ui.button("Neue Seite").clicked() {
                    action = Some((i, 0));
                }
                if ui.button("Seite umbenennen").clicked() {
                    action = Some((i, 1));
                }
                if ui.add_enabled(names.len() > 1 && i > 0, egui::Button::new("Seite löschen")).clicked() {
                    action = Some((i, 2));
                }
            });
            x += w + 1.0;
        }
        if ui.input(|i| i.pointer.any_released()) {
            self.page_drag = None;
        }
        let Some(e) = &mut self.sym_edit else { return };
        if let Some((a, b)) = drop_page {
            if e.options.is_empty() {
                e.options.push("Standard".into());
            }
            e.move_page(a as u16, b as u16);
        }
        if let Some(i) = clicked {
            e.option = i as u16;
            e.cur_flag = 0;
            self.selection.clear();
        }
        match action {
            Some((_, 0)) => e.query = Some((0, format!("Seite {}", e.options.len() + 1))),
            Some((i, 1)) => e.query = Some((2 + i as u8, names[i].clone())),
            Some((i, 2)) => {
                let removed = e.delete_page(i as u16);
                self.selection.clear();
                let s = &mut self.sym_edit.as_mut().unwrap().sheet;
                s.objects.retain(|o| !removed.contains(&o.id));
            }
            _ => {}
        }
    }

    fn bottom_row(&mut self, ui: &mut egui::Ui) {
        if self.sym_edit.is_some() {
            return self.page_tabs(ui);
        }
        let full = ui.max_rect();
        let mut x = full.left() + 31.0;
        let mut clicked: Option<usize> = None;
        let mut action: Option<(usize, u8)> = None;
        let mut drop_tab: Option<(usize, usize)> = None;
        // Reiterbereich endet vor dem Suchfeld; passen nicht alle Reiter, blättern zwei Pfeile (wie TTabControl)
        let limit = full.right() - 299.0 - 4.0;
        let labels: Vec<_> = self
            .tabs
            .iter()
            .map(|r| ui.painter().layout_no_wrap(self.project.sheet(*r).map(|s| format!("[{}] {}", s.number, s.name)).unwrap_or_default(), FontId::proportional(11.0), Color32::BLACK))
            .collect();
        let total: f32 = labels.iter().map(|g| g.size().x + 13.0).sum();
        let overflow = x + total > limit;
        if self.tab_first >= self.tabs.len() {
            self.tab_first = 0;
        }
        if self.active < self.tab_first {
            self.tab_first = self.active;
        }
        let arrows_w = if overflow { 34.0 } else { 0.0 };
        let mut last_visible = self.tab_first;
        for (i, r) in self.tabs.clone().iter().enumerate() {
            if i < self.tab_first {
                continue;
            }
            let _ = r;
            let g = labels[i].clone();
            let w = g.size().x + 12.0;
            if overflow && x + w > limit - arrows_w {
                break;
            }
            last_visible = i;
            let sel = i == self.active;
            let rr = Rect::from_min_size(Pos2::new(x, full.top() + 1.0), Vec2::new(w, if sel { 20.0 } else { 18.0 }));
            let resp = ui.interact(rr, ui.scope_id().with(("tab", i)), Sense::click_and_drag());
            let p = ui.painter();
            p.rect_filled(rr, 0.0, if sel { Color32::WHITE } else { BG });
            p.rect_stroke(rr, 0.0, Stroke::new(1.0, Color32::from_gray(217)), StrokeKind::Inside);
            p.galley(Pos2::new(rr.left() + 6.0, rr.top() + 3.0), g, Color32::BLACK);
            if resp.clicked() {
                clicked = Some(i);
            }
            // Reiter ziehen: Blatt innerhalb der Anlage umsortieren (sonst nur die Reiterfolge)
            if resp.drag_started() {
                self.tab_drag = Some(i);
                dbg_log(&format!("tab drag start {i}"));
            }
            if let Some(src) = self.tab_drag {
                if src != i && ui.input(|inp| inp.pointer.interact_pos()).is_some_and(|q| rr.contains(q)) {
                    p.rect_filled(Rect::from_min_size(rr.left_top(), Vec2::new(3.0, rr.height())), 0.0, Color32::from_rgb(0, 120, 215));
                    if ui.input(|inp| inp.pointer.any_released()) {
                        drop_tab = Some((src, i));
                        dbg_log(&format!("tab drop {src} -> {i}"));
                    }
                }
            }
            resp.context_menu(|ui| {
                if ui.button("Blatt schliessen").clicked() {
                    action = Some((i, 0));
                }
                if ui.button("Alle Blätter schließen").clicked() {
                    action = Some((i, 1));
                }
                if ui.button("Alle Blätter dieser Anlage schließen").clicked() {
                    action = Some((i, 2));
                }
                if ui.button("Eigenschaften").clicked() {
                    action = Some((i, 3));
                }
            });
            x += w + 1.0;
        }
        if overflow {
            for k in 0..2usize {
                let rr = Rect::from_min_size(Pos2::new(limit - arrows_w + k as f32 * 17.0, full.top() + 2.0), Vec2::new(16.0, 16.0));
                let resp = ui.interact(rr, ui.scope_id().with(("tabarrow", k)), Sense::click());
                let p = ui.painter();
                p.rect_filled(rr, 0.0, BG);
                p.rect_stroke(rr, 0.0, Stroke::new(1.0, Color32::from_gray(160)), StrokeKind::Inside);
                let c = rr.center();
                let d = if k == 0 { -1.0 } else { 1.0 };
                let tri = vec![Pos2::new(c.x + 2.5 * d, c.y), Pos2::new(c.x - 2.0 * d, c.y - 4.0), Pos2::new(c.x - 2.0 * d, c.y + 4.0)];
                p.add(egui::Shape::convex_polygon(tri, Color32::BLACK, Stroke::NONE));
                if resp.clicked() {
                    if k == 0 {
                        self.tab_first = self.tab_first.saturating_sub(1);
                    } else if last_visible + 1 < self.tabs.len() {
                        self.tab_first += 1;
                    }
                }
            }
        }
        if let Some(i) = clicked {
            self.active = i;
            self.selection.clear();
        }
        if let Some((src, dst)) = drop_tab {
            let (a, b) = (self.tabs[src], self.tabs[dst]);
            if a.anlage == b.anlage {
                self.move_sheet(a.anlage, a.sheet, b.sheet);
            } else {
                let t = self.tabs.remove(src);
                self.tabs.insert(dst, t);
            }
            if a.anlage != b.anlage {
                self.active = dst;
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            self.tab_drag = None;
        }
        if let Some((i, c)) = action {
            let r = self.tabs[i];
            match c {
                0 => {
                    self.tabs.remove(i);
                }
                1 => self.tabs.clear(),
                2 => self.tabs.retain(|t| t.anlage != r.anlage),
                _ => self.open_sheet_props(r),
            }
            self.fix_tabs();
        }
        // Suchfeld (unter dem Projektexplorer)
        let sx = full.right() - 299.0;
        let sr = Rect::from_min_size(Pos2::new(sx, full.top() + 1.0), Vec2::new(94.0, 21.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
        child.style_mut().visuals.extreme_bg_color = Color32::WHITE;
        child
            .add(egui::TextEdit::singleline(&mut self.search).desired_width(88.0).font(FontId::proportional(11.0)))
            .on_hover_text("Suchtext für die Aktuelle Tabelle in der markierten (roter Punkt) Spalte");
        let p = ui.painter();
        p.text(Pos2::new(sx + 101.0, full.top() + 12.0), Align2::CENTER_CENTER, "▾", FontId::proportional(12.0), Color32::BLACK);
        let b1 = Rect::from_min_size(Pos2::new(sx + 108.0, full.top() + 1.0), Vec2::new(22.0, 21.0));
        let b2 = Rect::from_min_size(Pos2::new(sx + 132.0, full.top() + 1.0), Vec2::new(22.0, 21.0));
        for (r, ic) in [(b1, Icon::CaseSensitive), (b2, Icon::Find)] {
            p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(173)), StrokeKind::Inside);
            icons::draw(ui, r.shrink(2.0), ic, true);
        }
        if ui.interact(b2, ui.scope_id().with("find"), Sense::click()).clicked() {
            self.right_tab = RightTab::Symbole;
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        let p = ui.painter();
        p.line_segment([full.left_top(), full.right_top()], Stroke::new(1.0, Color32::from_gray(225)));
        let widths = [130.0, 350.0, 200.0];
        let mut x = full.left();
        let fmt = |v: f32| format!("{:.1}", v).replace('.', ",");
        let texts = [
            format!("X: {}  Y: {:>5}", fmt(self.mouse_mm.x), fmt(self.mouse_mm.y)),
            self.info.clone(),
            if self.project.abgeschlossen { "Projekt ist abgeschlossen".to_string() } else { "Projekt wird bearbeitet".to_string() },
            String::new(),
        ];
        for (i, t) in texts.iter().enumerate() {
            let w = if i < 3 { widths[i] } else { full.right() - x };
            let r = Rect::from_min_size(Pos2::new(x, full.top() + 1.0), Vec2::new(w - 2.0, full.height() - 2.0));
            if i < 3 {
                p.line_segment([Pos2::new(r.right() + 0.5, r.top() + 2.0), Pos2::new(r.right() + 0.5, r.bottom())], Stroke::new(1.0, Color32::from_gray(215)));
            }
            p.with_clip_rect(r).text(Pos2::new(r.left() + 2.0, r.center().y), Align2::LEFT_CENTER, t, FontId::proportional(12.0), Color32::BLACK);
            x += w;
        }
    }

    // ------------------------------------------------------------------ Zeichenfläche

    fn fit(&mut self, area: Rect) {
        // Symboleditor: auf den Inhalt einpassen (das Original füllt die Fläche mit dem Symbol)
        if let Some(e) = &self.sym_edit {
            if !e.sheet.objects.is_empty() {
                let mut mn = P::new(f32::MAX, f32::MAX);
                let mut mx = P::new(f32::MIN, f32::MIN);
                for o in &e.sheet.objects {
                    let (a, b) = canvas::obj_bounds(o, &self.lib);
                    mn = P::new(mn.x.min(a.x), mn.y.min(a.y));
                    mx = P::new(mx.x.max(b.x), mx.y.max(b.y));
                }
                let (w, h) = ((mx.x - mn.x).max(10.0) * 1.3, (mx.y - mn.y).max(10.0) * 1.3);
                let z = ((area.width() - 10.0) / w).min((area.height() - 10.0) / h).clamp(0.5, 80.0);
                let c = P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
                self.view.zoom = z;
                self.view.offset = Vec2::new((area.width() / 2.0 - c.x * z).round(), (area.height() / 2.0 - c.y * z).round());
                return;
            }
        }
        let (sw, sh) = self.sheet().map(|s| s.dims()).unwrap_or((SHEET_W, SHEET_H));
        let z = ((area.width() - 10.0) / sw).min((area.height() - 10.0) / sh).max(0.5);
        self.view.zoom = z;
        self.view.offset = Vec2::new(((area.width() - sw * z) / 2.0).round(), ((area.height() - sh * z) / 2.0).round());
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        // Zeichenfläche mit Bildlaufleisten rechts und unten wie im Original (TScrollBox)
        let full = ui.max_rect();
        let area = Rect::from_min_max(full.min, full.max - Vec2::splat(SCROLLBAR));
        let (resp, painter) = ui.allocate_painter(full.size(), Sense::click_and_drag());
        let resp = resp.with_new_rect(area);
        self.canvas_area = area;
        painter.rect_filled(area, 0.0, VIEW_BG);
        self.scrollbars(ui, &painter, full, area);
        // Blattwechsel: Ansicht des bisherigen Blattes merken, die des neuen wiederherstellen bzw. einpassen
        let cur = self.cur();
        if self.sym_edit.is_none() && cur != self.view_sheet {
            if let Some(old) = self.view_sheet {
                self.sheet_views.insert(old, self.view.clone());
            }
            match cur.and_then(|r| self.sheet_views.get(&r).cloned()) {
                Some(v) => self.view = v,
                None => {
                    self.view.user = false;
                    self.view.last_size = Vec2::ZERO;
                }
            }
            self.view_sheet = cur;
        }
        let portrait = self.sheet().map(|s| s.portrait).unwrap_or(false);
        if !self.view.user && (self.view.last_size != area.size() || self.view.last_portrait != portrait) {
            self.fit(area);
        }
        self.view.last_size = area.size();
        self.view.last_portrait = portrait;
        if self.cur().is_none() {
            return;
        }
        let xf0 = Xf { origin: area.min + self.view.offset, z: self.view.zoom };
        self.handle_canvas_input(ui, &resp, area, xf0);
        let xf = Xf { origin: area.min + self.view.offset, z: self.view.zoom };
        let (sw, sh) = self.sheet().map(|s| s.dims()).unwrap_or((SHEET_W, SHEET_H));
        self.sheet_screen_rect = Rect::from_min_max(xf.s(P::new(0.0, 0.0)), xf.s(P::new(sw, sh)));

        // Bilder laden
        if let Some(s) = self.sheet() {
            let paths: Vec<String> = s
                .objects
                .iter()
                .filter_map(|o| if let Kind::Image { path, .. } = &o.kind { Some(path.clone()) } else { None })
                .filter(|p| !p.is_empty() && !self.images.contains_key(p))
                .collect();
            for pth in paths {
                let img = match pth.strip_prefix("bld:").and_then(|n| n.parse::<i32>().ok()) {
                    Some(nr) => self.project.bilder.iter().find(|b| b.nr == nr).and_then(|b| image::load_from_memory(&b.jpeg).ok()),
                    None => image::open(&pth).ok(),
                };
                if let Some(img) = img {
                    let rgba = img.to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                    let tex = ui.ctx().load_texture(pth.clone(), ci, Default::default());
                    self.images.insert(pth, tex);
                }
            }
        }

        let clip = painter.with_clip_rect(area);
        if let Some(e) = &self.sym_edit {
            self.draw_edit_base(&clip, xf);
            let dc = DrawCtx { painter: &clip, xf, lib: &self.lib, project: &self.project, images: &self.images, bmk: Default::default(), anlage: None };
            for o in &e.sheet.objects {
                if !symbols::visible(e.option, e.vis.get(&o.id)) || !e.flag_shown(o.id) {
                    continue;
                }
                let sel = self.selection.contains(&o.id);
                // Titel-/Wertfeld farbig hinterlegt wie im Original (grün / blau)
                if o.id == crate::extra::EDIT_TITLE_ID || o.id == crate::extra::EDIT_VALUE_ID {
                    let (mn, mx) = canvas::obj_bounds(o, &self.lib);
                    let (mn, mx) = match &o.kind {
                        Kind::Text { style: TextStyle { rect: Some(r), .. }, .. } => *r,
                        _ => (mn, mx),
                    };
                    let c = if o.id == crate::extra::EDIT_TITLE_ID { Color32::from_rgb(0, 128, 0) } else { Color32::from_rgb(0, 0, 255) };
                    clip.rect_filled(Rect::from_min_max(xf.s(mn), xf.s(mx)), 0.0, c);
                }
                canvas::draw_object(&dc, o, if sel { Some(SEL) } else { None });
            }
            let hover = ui.input(|i| i.pointer.hover_pos());
            if let (Drag::Rubber { start }, Some(h)) = (&self.drag, hover) {
                let r = Rect::from_two_pos(xf.s(*start), h);
                clip.add(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.0, Color32::BLACK), 4.0, 3.0));
            }
            if let (Drag::Create { tool, start }, Some(h)) = (&self.drag, hover) {
                if let Some(k) = self.make_kind(*tool, *start, self.snap(xf.m(h))) {
                    canvas::draw_object(&dc, &Obj { id: 0, kind: k, pen: self.pen, fixed: false, group: None, raw: None }, Some(PREVIEW));
                }
            }
            return;
        }
        let Some(r) = self.cur() else { return };
        let Some(anlage) = self.project.anlagen.get(r.anlage) else { return };
        let Some(sheet) = anlage.sheets.get(r.sheet) else { return };
        let dc = DrawCtx { painter: &clip, xf, lib: &self.lib, project: &self.project, images: &self.images, bmk: canvas::bmk_ctx(anlage, sheet), anlage: Some(anlage) };
        let colors = SheetColors { sheet: self.sheet_color, grid: self.grid_color, print: false };
        canvas::draw_sheet_base(&dc, sheet, anlage, &colors, self.raster(), self.grid);
        // Markierung wie im Original: Symbole mit lila Feld um ihren Umriss (6 px links/oben, 8 px rechts/unten),
        // Texte mit lila Feld und rot gestricheltem Rahmen um die Textbox
        for o in sheet.objects.iter().filter(|o| self.selection.contains(&o.id)) {
            let fill = Color32::from_rgb(208, 193, 204);
            match &o.kind {
                Kind::Symbol { .. } => {
                    let (a, b) = canvas::obj_bounds(o, &self.lib);
                    let r = Rect::from_min_max(xf.s(a) - Vec2::splat(6.0), xf.s(b) + Vec2::splat(8.0));
                    clip.rect_filled(r, 0.0, fill);
                }
                Kind::Text { style, .. } => {
                    let (a, b) = style.rect.unwrap_or_else(|| canvas::obj_bounds(o, &self.lib));
                    let r = Rect::from_min_max(xf.s(a), xf.s(b)).expand(2.0);
                    clip.rect_filled(r, 0.0, fill);
                    clip.add(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.0, Color32::from_rgb(223, 0, 11)), 3.0, 2.0));
                }
                _ => {}
            }
        }
        for o in &sheet.objects {
            let sel = self.selection.contains(&o.id);
            canvas::draw_object(&dc, o, if sel { Some(SEL) } else { None });
        }
        for j in canvas::junctions(sheet) {
            clip.circle_filled(xf.s(j), (0.6 * xf.z).max(2.0), Color32::BLACK);
        }
        let hover = ui.input(|i| i.pointer.hover_pos());
        if let Some(hp) = hover {
            if area.contains(hp) && self.tool == Tool::Select && self.placing.is_none() && !self.zoom_tool {
                if let Some(pin) = self.pin_near(xf.m(hp), 6.0 / xf.z, &[]) {
                    clip.circle_stroke(xf.s(pin), 4.0, Stroke::new(1.5, Color32::from_rgb(0, 140, 0)));
                }
            }
        }
        if self.size_frame && self.selection.len() == 1 {
            if let Some(o) = sheet.objects.iter().find(|o| o.id == self.selection[0]) {
                if let Some((a, b)) = o.kind.frame_points() {
                    for q in [a, b, P::new(a.x, b.y), P::new(b.x, a.y)] {
                        let c = xf.s(q);
                        clip.rect_filled(Rect::from_center_size(c, Vec2::splat(7.0)), 0.0, Color32::from_rgb(0, 90, 200));
                        clip.rect_stroke(Rect::from_center_size(c, Vec2::splat(7.0)), 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
                    }
                }
            }
        }
        // Rahmen zur Größenänderung um die ganze Markierung (wie im Original, Griffe 2,4 mm)
        if let Some((mn, mx)) = self.sel_frame() {
            let r = Rect::from_min_max(xf.s(mn), xf.s(mx));
            clip.add(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.0, Color32::from_rgb(0, 90, 200)), 4.0, 3.0));
            let hs = (FRAME_HANDLE * xf.z).max(5.0);
            for q in crate::app::frame_handles(mn, mx) {
                let hr = Rect::from_center_size(xf.s(q), Vec2::splat(hs));
                clip.rect_filled(hr, 0.0, Color32::from_rgb(0, 90, 200));
                clip.rect_stroke(hr, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
            }
        }
        match &self.drag {
            Drag::Rubber { start } => {
                if let Some(h) = hover {
                    let r = Rect::from_two_pos(xf.s(*start), h);
                    clip.add(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.0, Color32::BLACK), 4.0, 3.0));
                }
            }
            Drag::Create { tool, start } => {
                if let Some(h) = hover {
                    if let Some(k) = self.make_kind(*tool, *start, self.snap(xf.m(h))) {
                        canvas::draw_object(&dc, &Obj { id: 0, kind: k, pen: self.pen, fixed: false, group: None, raw: None }, Some(PREVIEW));
                    }
                }
            }
            Drag::Wire { start } => {
                if let Some(h) = hover {
                    let end = self.wire_end(xf.m(h), xf.z);
                    let w = Obj { id: 0, kind: Kind::Wire { pts: route(*start, end), leitung: self.leitung, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None };
                    canvas::draw_object(&dc, &w, Some(PREVIEW));
                }
            }
            _ => {}
        }
        if let (Some(key), Some(h)) = (self.placing.clone(), hover) {
            if area.contains(h) {
                if let Some(def) = symbols::find(&self.lib, &key) {
                    let pin0 = def.pins.first().copied().unwrap_or_default();
                    let pos = self.align_to_path(self.snap(xf.m(h).add(pin0)).sub(pin0));
                    canvas::draw_symbol(&clip, xf, def, pos, 0, false, false, 1.0, 0, Stroke::new((0.25 * xf.z).max(1.0), PREVIEW));
                }
            }
        }
    }

    fn wire_end(&self, m: P, z: f32) -> P {
        self.pin_near(m, 6.0 / z, &[]).unwrap_or_else(|| self.snap(m))
    }

    fn zoom_at(&mut self, at: Pos2, area: Rect, f: f32) {
        let old = self.view.zoom;
        let nz = (old * f).clamp(0.5, 80.0);
        let rel = at - (area.min + self.view.offset);
        self.view.offset += rel - rel * (nz / old);
        self.view.zoom = nz;
        self.view.user = true;
    }

    fn handle_canvas_input(&mut self, ui: &mut egui::Ui, resp: &egui::Response, area: Rect, xf: Xf) {
        let (hover, pressed_l, released_l, pressed_r, pressed_m, down_m, scroll, dbl, shift, ctrl, dragging) = ui.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.button_pressed(PointerButton::Primary),
                i.pointer.button_released(PointerButton::Primary),
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer.button_pressed(PointerButton::Middle),
                i.pointer.button_down(PointerButton::Middle),
                i.smooth_scroll_delta.y,
                i.pointer.button_double_clicked(PointerButton::Primary),
                i.modifiers.shift,
                i.modifiers.command,
                i.pointer.is_decidedly_dragging(),
            )
        });
        // Nur Eingaben, die wirklich auf der Zeichenfläche landen: liegt ein Menü, Untermenü oder Dialog über dem
        // Blatt, gehört der Klick dorthin (sonst löscht der Druck die Markierung und das Menü verschwindet vor dem
        // Loslassen – Optionen/Bildindex im Kontextmenü ließen sich so nicht wählen)
        let blocked = ui.ctx().any_popup_open() || hover.map(|h| ui.ctx().layer_id_at(h) != Some(ui.layer_id())).unwrap_or(true);
        let inside = !blocked && hover.map(|h| area.contains(h)).unwrap_or(false);
        let tol = 5.0 / xf.z;
        if let Some(h) = hover {
            if inside {
                self.mouse_mm = self.snap(xf.m(h));
            }
        }
        // Mausrad: egui verteilt eine Rastung (≈ 50 Punkte) über mehrere Bilder; der Zoom folgt dem Betrag,
        // eine Rastung ≈ 10 % (vorher 15 % je Bild, also mehrere Stufen je Rastung)
        if inside && scroll.abs() > 0.01 {
            // Standard: Rad nach oben vergrößert (Wunsch des Nutzers); "Mausrad umkehren" wie im Original: nach unten vergrößert
            let s = if self.invert_wheel { -scroll } else { scroll };

            self.zoom_at(hover.unwrap(), area, (s * (1.10f32.ln() / 50.0)).exp());
        }
        if inside && pressed_m {
            self.drag = Drag::Pan { last: hover.unwrap() };
        }
        if let Drag::Pan { last } = self.drag.clone() {
            if down_m {
                if let Some(h) = hover {
                    self.view.offset += h - last;
                    self.view.user = true;
                    self.drag = Drag::Pan { last: h };
                }
            } else {
                self.drag = Drag::None;
            }
            return;
        }
        if self.zoom_tool {
            if inside && pressed_l {
                self.zoom_at(hover.unwrap(), area, 1.5);
            }
            if inside && pressed_r {
                self.zoom_at(hover.unwrap(), area, 1.0 / 1.5);
            }
            return;
        }
        let Some(h) = hover else { return };
        let m = xf.m(h);

        // Symbol aus der Bibliothek platzieren (Klick oder Ziehen aus dem Baum)
        if let Some(key) = self.placing.clone() {
            if inside && (pressed_l || (released_l && dragging)) {
                self.place_symbol(key, m);
                self.placing = None;
                self.tool = Tool::Select;
                self.drag = Drag::None;
                return;
            }
            if pressed_r && inside {
                self.placing = None;
            }
            return;
        }

        if inside && dbl && self.tool == Tool::Select {
            if let Some(id) = self.hit_top(m, tol) {
                self.edit_props(id);
                self.drag = Drag::None;
                return;
            }
            // Doppelklick auf freie Blattfläche: Blatteigenschaften (wie im Original)
            if let Some(r) = self.cur() {
                self.open_sheet_props(r);
                self.drag = Drag::None;
                return;
            }
        }

        // Eigentümer › Setzen: das angeklickte Symbol gibt seine Kennzeichnung an die gemerkten Symbole
        if self.owner_pick.is_some() {
            if inside && pressed_l {
                if let Some(id) = self.hit_top(m, tol) {
                    self.owner_picked(id);
                }
                self.drag = Drag::None;
            }
            if inside && pressed_r {
                self.owner_pick = None;
                self.info = "Eigentümer setzen abgebrochen".into();
            }
            return;
        }

        if inside && pressed_l {
            match self.tool {
                Tool::Select => {
                    if let Some((mn, mx)) = self.sel_frame() {
                        let r = (FRAME_HANDLE / 2.0).max(2.5 / xf.z) + 0.5 / xf.z;
                        if let Some(hi) = frame_handles(mn, mx).iter().position(|q| (q.x - m.x).abs() <= r && (q.y - m.y).abs() <= r) {
                            self.checkpoint();
                            let orig = self.selected_objs();
                            self.drag = Drag::Scale { handle: hi, start: (mn, mx), orig };
                            return;
                        }
                    }
                    if self.size_frame && self.selection.len() == 1 {
                        if let Some(o) = self.selected_objs().first().cloned() {
                            if let Some((a, b)) = o.kind.frame_points() {
                                let corners = [a, b, P::new(a.x, b.y), P::new(b.x, a.y)];
                                if let Some(ci) = corners.iter().position(|c| c.dist(m) <= tol * 1.2) {
                                    if !(o.fixed && self.fix_active) {
                                        self.checkpoint();
                                        self.drag = Drag::Resize { id: o.id, corner: ci };
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(pin) = self.pin_near(m, tol, &[]) {
                        let on_selected = self.selected_objs().iter().any(|o| canvas::hit(o, m, tol, &self.lib));
                        if !on_selected {
                            self.drag = Drag::Wire { start: pin };
                            return;
                        }
                    }
                    if let Some(id) = self.hit_top(m, tol) {
                        if shift || ctrl {
                            if let Some(i) = self.selection.iter().position(|s| *s == id) {
                                self.selection.remove(i);
                            } else {
                                self.selection.push(id);
                            }
                        } else if !self.selection.contains(&id) {
                            self.selection = vec![id];
                        }
                        self.expand_groups();
                        self.drag = Drag::Move { start: self.snap(m), applied: P::default(), moved: false };
                    } else {
                        if !shift && !ctrl {
                            self.selection.clear();
                        }
                        self.drag = Drag::Rubber { start: m };
                    }
                }
                Tool::Klemmstelle => {
                    let ks = self.klemmleisten();
                    let leiste = ks.get(self.klemme).cloned().unwrap_or_else(|| "Klemme 1".into());
                    let nr = self.next_terminal(&leiste);
                    let pos = self.snap(m);
                    self.add_obj(Kind::Terminal { pos, leiste, nr, level: 0, text_off: None, tpl: String::new() });
                }
                Tool::Text => {
                    let pos = self.snap(m);
                    self.dialog = Some(Dialog::Text { title: "Neuer Text".into(), value: String::new(), target: TextTarget::NewText(pos) });
                }
                t => self.drag = Drag::Create { tool: t, start: self.snap(m) },
            }
        }

        if let Drag::Move { start, applied, moved } = self.drag.clone() {
            // Gesamtverschiebung seit dem Drücken; mit Strompfadausrichtung richtet das erste Symbol (bzw. die
            // erste Klemmstelle) der Markierung sie auf die Mitte des nächsten Strompfades aus, alle anderen folgen
            let mut total = self.snap(m).sub(start);
            // bloßer Klick ohne Bewegung verschiebt nichts (auch nicht durch die Ausrichtung)
            let still = !moved && total.x.abs() < 1e-4 && total.y.abs() < 1e-4;
            let objs = if still { vec![] } else { self.selected_objs() };
            let movable: Vec<u64> = objs.iter().filter(|o| !(o.fixed && self.fix_active)).map(|o| o.id).collect();
            let reference = self.selection.iter().filter_map(|id| objs.iter().find(|o| o.id == *id && movable.contains(id))).find_map(|o| match &o.kind {
                Kind::Symbol { pos, .. } | Kind::Terminal { pos, .. } => Some(*pos),
                _ => None,
            });
            if let Some(pos) = reference {
                // Lage vor dem Ziehen = jetzige Lage − bisherige Verschiebung
                let p0 = pos.sub(applied);
                let np = self.align_to_path(p0.add(total));
                total.x = np.x - p0.x;
            }
            let d = total.sub(applied);
            if (d.x.abs() > 1e-4 || d.y.abs() > 1e-4) && !movable.is_empty() {
                if !moved {
                    self.checkpoint();
                }
                self.with_sheet(|s, _| s.objects.iter_mut().filter(|o| movable.contains(&o.id)).for_each(|o| o.kind.translate(d)));
                self.drag = Drag::Move { start, applied: total, moved: true };
            }
        }
        if let Drag::Scale { handle, start, orig } = self.drag.clone() {
            let (anchor, sx, sy) = frame_scale(handle, start.0, start.1, self.snap(m));
            let fix = self.fix_active;
            self.with_sheet(|s, _| {
                for o0 in orig.iter().filter(|o| !(o.fixed && fix)) {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == o0.id) {
                        let mut k = o0.kind.clone();
                        k.scale_about(anchor, sx, sy);
                        o.kind = k;
                    }
                }
            });
        }
        if let Drag::Resize { id, corner } = self.drag.clone() {
            let cur = self.snap(m);
            self.with_sheet(|s, _| {
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                    if let Some((a, b)) = o.kind.frame_points() {
                        let (na, nb) = match corner {
                            0 => (cur, b),
                            1 => (a, cur),
                            2 => (P::new(cur.x, a.y), P::new(b.x, cur.y)),
                            _ => (P::new(a.x, cur.y), P::new(cur.x, b.y)),
                        };
                        o.kind.set_frame_points(na, nb);
                    }
                }
            });
        }

        if released_l {
            match self.drag.clone() {
                Drag::Rubber { start } => {
                    let (mn, mx) = canvas::norm_rect(start, m);
                    if mn.dist(mx) > 0.5 {
                        if let Some(s) = self.sheet() {
                            let ids: Vec<u64> = s
                                .objects
                                .iter()
                                .filter(|o| self.selectable(o))
                                .filter(|o| {
                                    let (a, b) = canvas::obj_bounds(o, &self.lib);
                                    a.x >= mn.x && a.y >= mn.y && b.x <= mx.x && b.y <= mx.y
                                })
                                .map(|o| o.id)
                                .collect();
                            for id in ids {
                                if !self.selection.contains(&id) {
                                    self.selection.push(id);
                                }
                            }
                        }
                        self.expand_groups();
                    }
                }
                Drag::Create { tool, start } => {
                    if let Some(k) = self.make_kind(tool, start, self.snap(m)) {
                        let id = self.add_obj(k);
                        if tool == Tool::Image {
                            self.edit_props(id);
                        }
                    }
                }
                Drag::Wire { start } => {
                    let end = self.wire_end(m, xf.z);
                    if end.dist(start) > 0.01 {
                        let l = self.leitung;
                        self.add_obj(Kind::Wire { pts: route(start, end), leitung: l, bez: vec![], opts: 0 });
                    }
                }
                // nach dem Verschieben: freie Anschlüsse automatisch verbinden (gleicher Rückgängig-Schritt)
                Drag::Move { moved: true, .. } if self.auto_conn_eff() => {
                    let ids: Vec<u64> = self.selected_objs().iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })).map(|o| o.id).collect();
                    self.auto_connect_nocp(&ids);
                }
                _ => {}
            }
            self.drag = Drag::None;
        }

        if inside && pressed_r {
            self.mouse_ctx = m;
            if self.tool != Tool::Select {
                self.tool = Tool::Select;
                self.drag = Drag::None;
            } else if let Some(id) = self.hit_top(m, tol) {
                if !self.selection.contains(&id) {
                    self.selection = vec![id];
                    self.expand_groups();
                }
            } else {
                // freie Fläche: Markierung aufheben (Menü "Objekt einfügen / Verweis einfügen")
                self.selection.clear();
            }
        }
        let has_sel = !self.selection.is_empty();
        let objs = self.selected_objs();
        let is_sym = objs.iter().any(|o| matches!(o.kind, Kind::Symbol { .. }));
        let is_text = objs.iter().any(|o| matches!(o.kind, Kind::Text { .. }));
        resp.context_menu(|ui| {
            ui.style_mut().override_font_id = Some(FontId::proportional(12.0));
            self.canvas_menu(ui, has_sel, is_sym, is_text);
        });
    }

    // ------------------------------------------------------------------ Dialoge

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.dialog.take() else { return };
        let mut close = false;
        let mut apply = false;
        let mut sheet_col = self.sheet_color;
        let mut grid_col = self.grid_color;
        let mut inv = self.invert_wheel;
        let mut pick_lib = false;
        egui::Modal::new(egui::Id::unique("dlg")).show(ctx, |ui| {
            ui.style_mut().override_font_id = Some(FontId::proportional(13.0));
            ui.set_min_width(300.0);
            match &mut dlg {
                Dialog::Text { title, value, .. } => {
                    ui.label(egui::RichText::new(title.as_str()).strong());
                    let r = ui.add(egui::TextEdit::multiline(value).desired_rows(3).desired_width(320.0));
                    if !r.has_focus() && !r.lost_focus() {
                        r.request_focus();
                    }
                }
                Dialog::Symbol { bmk, value, option, options, .. } => {
                    ui.label(egui::RichText::new("Symbol – Eigenschaften").strong());
                    egui::Grid::new("sg").num_columns(2).show(ui, |ui| {
                        ui.label("Bezeichnung (BMK):");
                        ui.text_edit_singleline(bmk);
                        ui.end_row();
                        ui.label("Wert:");
                        ui.text_edit_singleline(value);
                        ui.end_row();
                        if !options.is_empty() {
                            ui.label("Option:");
                            let cur = options.get(*option as usize).cloned().unwrap_or_else(|| option.to_string());
                            egui::ComboBox::from_id_salt("symopt").selected_text(cur).show_ui(ui, |ui| {
                                for (i, o) in options.iter().enumerate() {
                                    ui.selectable_value(option, i as u16, o);
                                }
                            });
                            ui.end_row();
                        }
                    });
                }
                Dialog::Project { name, kunde, owner, von, ort, znr, done } => {
                    ui.label(egui::RichText::new("Projekt – Eigenschaften").strong());
                    egui::Grid::new("pg").num_columns(2).show(ui, |ui| {
                        for (l, v) in [("Projekt:", name), ("Kunde:", kunde), ("Eigentümer:", owner), ("Erstellt von:", von), ("Ort:", ort), ("ZNr.:", znr)] {
                            ui.label(l);
                            ui.add(egui::TextEdit::singleline(v).desired_width(260.0));
                            ui.end_row();
                        }
                    });
                    ui.checkbox(done, "Projekt ist abgeschlossen");
                }
                Dialog::Sheet { name, number, columns, .. } => {
                    ui.label(egui::RichText::new("Blatt – Eigenschaften").strong());
                    egui::Grid::new("bg").num_columns(2).show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(name);
                        ui.end_row();
                        ui.label("Blattnummer:");
                        ui.text_edit_singleline(number);
                        ui.end_row();
                        ui.label("Strompfade:");
                        ui.text_edit_singleline(columns);
                        ui.end_row();
                    });
                }
                Dialog::Table { rows, cols, cells, .. } => {
                    ui.label(egui::RichText::new("Tabelle").strong());
                    ui.horizontal(|ui| {
                        ui.label("Zeilen:");
                        ui.add(egui::TextEdit::singleline(rows).desired_width(40.0));
                        ui.label("Spalten:");
                        ui.add(egui::TextEdit::singleline(cols).desired_width(40.0));
                    });
                    let r: usize = rows.parse().unwrap_or(1).clamp(1, 50);
                    let c: usize = cols.parse().unwrap_or(1).clamp(1, 20);
                    cells.resize(r * c, String::new());
                    egui::Grid::new("tg").show(ui, |ui| {
                        for i in 0..r {
                            for j in 0..c {
                                ui.add(egui::TextEdit::singleline(&mut cells[i * c + j]).desired_width(70.0));
                            }
                            ui.end_row();
                        }
                    });
                }
                Dialog::Options => {
                    ui.label(egui::RichText::new("Option").strong());
                    ui.horizontal(|ui| {
                        ui.label("Farbe Blatteditor:");
                        ui.color_edit_button_srgba(&mut sheet_col);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Farbe Rasteranzeige:");
                        ui.color_edit_button_srgba(&mut grid_col);
                    });
                    ui.checkbox(&mut inv, "Mausradfunktion umdrehen");
                    ui.separator();
                    ui.label(self.lib_info.as_str());
                    if ui.button("Symbol-Datenbank (Ordner „Datenbank“) wählen …").clicked() {
                        pick_lib = true;
                    }
                }
                Dialog::Info => {
                    ui.label(egui::RichText::new(format!("Elektroplanung {}", env!("CARGO_PKG_VERSION"))).strong());
                    ui.label("Unabhängiger Nachbau in Rust – nach Version 6.4.2 des Originals (MD Evolution).");
                    ui.label("Schaltpläne mit Blättern, Symbolen, Leitungen und Schriftfeld.");
                    ui.label("Lizenz: GPL-3.0");
                    ui.hyperlink_to("Quelltext und Fehlermeldungen auf GitHub", crate::REPO_URL);
                }
                Dialog::Message(m) => {
                    ui.label(egui::RichText::new("Hinweis").strong());
                    ui.label(m.as_str());
                }
            }
            ui.add_space(8.0);
            let info_only = matches!(dlg, Dialog::Info | Dialog::Message(_) | Dialog::Options);
            ui.horizontal(|ui| {
                if ui.button("    OK    ").clicked() {
                    apply = !info_only;
                    close = true;
                }
                if !info_only && ui.button("Abbrechen").clicked() {
                    close = true;
                }
            });
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
        });
        self.sheet_color = sheet_col;
        self.grid_color = grid_col;
        self.invert_wheel = inv;
        if pick_lib {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                self.load_library(Some(&dir));
            }
        }
        if apply {
            self.apply_dialog(dlg);
        } else if !close {
            self.dialog = Some(dlg);
        }
    }

    // ------------------------------------------------------------------ Menü

    fn menu(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.style_mut().override_font_id = Some(FontId::proportional(12.0));
            ui.menu_button("Datei", |ui| {
                ui.menu_button("Neu", |ui| {
                    if ui.button("Projekt").clicked() {
                        self.guarded(crate::app::Pending::New);
                    }
                    if ui.button("Blatt").clicked() {
                        self.new_sheet();
                    }
                });
                ui.separator();
                if ui.button("Projekt öffnen").clicked() {
                    self.guarded(crate::app::Pending::Open);
                }
                if ui.button("Projekt hinzufügen").clicked() {
                    self.open_add_project();
                }
                if ui.add_enabled(self.dirty, egui::Button::new("Speichern")).clicked() {
                    self.save();
                }
                if ui.button("Projekt Speichern als").clicked() {
                    self.save_as();
                }
                ui.separator();
                if ui.button("Vorlage öffnen").clicked() {
                    self.guarded(crate::app::Pending::Open);
                }
                if ui.button("Vorlage speichern").clicked() {
                    self.save_as();
                }
                ui.separator();
                if ui.button("Schriftfeld speichern").clicked() {
                    self.save_titleblock_default();
                }
                ui.separator();
                // Reihenfolge wie im Original-Menü: Export › PDF (Beta), Bitmapexport, DXF Import
                ui.menu_button("Export", |ui| {
                    if ui.button("PDF (Beta)").clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("Projekt.pdf").save_file() {
                            self.pdf_req = Some((p, false));
                        }
                    }
                });
                // Bitmapexport des Originals: PNG mit wählbarer Auflösung (bildexport.rs)
                if ui.button("Als Bild exportieren…").on_hover_text("Bitmapexport").clicked() {
                    self.open_bild_export();
                }
                if ui.button("DXF Import").clicked() {
                    self.dxf_import();
                }
                ui.separator();
                if !self.recent.is_empty() {
                    ui.separator();
                    for (i, r) in self.recent.clone().iter().enumerate() {
                        let label = format!("{} {}", i + 1, std::path::Path::new(r).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
                        if ui.button(label).on_hover_text(r.as_str()).clicked() {
                            self.guarded(crate::app::Pending::OpenPath(std::path::PathBuf::from(r)));
                        }
                    }
                }
                ui.separator();
                if ui.button("Beenden").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Projekt", |ui| {
                if ui.button("Eigenschaften").clicked() {
                    self.project_props();
                }
                if ui.button("Drucken").clicked() {
                    self.open_druck_dialog();
                }
                if ui.button("Info").clicked() {
                    self.dlg2 = Some(crate::dialoge::Dlg2::Info);
                }
            });
            ui.menu_button("Bearbeiten", |ui| {
                let has = !self.selection.is_empty();
                if ui.add_enabled(has, egui::Button::new("Ausschneiden").shortcut_text("Ctrl+X")).clicked() {
                    self.copy();
                    self.delete_selection();
                }
                if ui.add_enabled(has, egui::Button::new("Kopieren").shortcut_text("Ctrl+C")).clicked() {
                    self.copy();
                }
                if ui.add_enabled(!self.clipboard.is_empty(), egui::Button::new("Einfügen").shortcut_text("Ctrl+V")).clicked() {
                    self.paste();
                }
                if ui.add_enabled(has, egui::Button::new("Löschen").shortcut_text("Del")).clicked() {
                    self.delete_selection();
                }
                // Original: Trennlinie am Ende, Rückgängig/Forwärts nur über Strg+Z / Strg+W und die Werkzeugleiste
                ui.separator();
                if ui.add(egui::Button::new("Suchen…").shortcut_text(crate::kuerzel::belegung(&self.opt.tasten, "suchen"))).clicked() {
                    self.open_suche();
                }
            });
            ui.menu_button("Hinzufügen", |ui| {
                if ui.button("Anlage").on_hover_text("Erweitert das Projekt um eine neue Anlage").clicked() {
                    self.new_anlage();
                }
                if ui.button("Blatt").on_hover_text("Fügt dem Projekt ein neues Blatt hinzu").clicked() {
                    self.new_sheet();
                }
                if ui.button("Symbol (nur Projekt)").on_hover_text("Öffnet die Symbolbearbeitung mit einem neuem Symbol").clicked() {
                    self.open_symbol_editor(None);
                }
                if ui.button("Bauteil (nur Projekt)").clicked() {
                    self.new_bauteil_dialog();
                }
                ui.separator();
                if ui.button("Potential").clicked() {
                    self.new_potential();
                }
                if ui.button("Leitung").clicked() {
                    self.new_leitung();
                }
                if ui.button("Klemmleiste").clicked() {
                    self.new_klemmleiste();
                }
                if ui.button("Sammelschine").clicked() {
                    self.new_busbar();
                }
                if ui.button("Schriftfeld").clicked() {
                    self.open_neues_schriftfeld();
                }
                if ui.button("Grafik").clicked() {
                    self.tool = Tool::Image;
                }
                if ui.button("Platzhaltergruppe").clicked() {
                    self.new_platzhalter();
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui.button("Markierung als Symbol").clicked() {
                    self.selection_to_symbol();
                }
                if ui.button("Ausrichtung").clicked() {
                    self.align_win = true;
                    self.align_ref = 0;
                }
                if ui.button("Strompfadbezeichnung bearbeiten").clicked() {
                    self.open_columns_dialog();
                }
                if ui.button("Textgenerator").clicked() {
                    self.open_titel_dialog();
                }
                if ui.button("BMK neu erzeugen").clicked() {
                    self.xdlg = Some(crate::extra::XDialog::TextGen { scope_project: true });
                }
                if ui.button("Schriftfeld bearbeiten").clicked() {
                    self.open_title_editor();
                }
                if ui.button("Schriftfeldeigenschaften").clicked() {
                    self.open_schriftfeld_eigenschaften();
                }
                if ui.button("Sammelschienenassistent").clicked() {
                    self.open_busbar_assistant();
                }
                if ui.button("Blattnummerierung").clicked() {
                    self.open_blattindex();
                }
                if ui.button("Listen Export").clicked() {
                    self.list_export();
                }
                if ui.button("Projekt prüfen").clicked() {
                    self.open_pruefen();
                }
            });
            ui.menu_button("Extras", |ui| {
                if ui.button("Bibliotheksverwaltung").clicked() {
                    self.show_library = true;
                }
                ui.separator();
                if ui.button("Option").clicked() {
                    self.open_optionen();
                }
            });
            ui.menu_button("Hilfe", |ui| {
                if ui.button("Info").clicked() {
                    self.dialog = Some(Dialog::Info);
                }
            });
        });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.dialog.is_some() || self.xdlg.is_some() || self.opt_dlg.is_some() || self.ask_save.is_some() {
            self.hold_s = false;
            self.hold_a = false;
            self.no_snap = false;
            self.hold_press = [None, None];
            return;
        }
        let eigene = self.opt.tasten.clone();
        // Gehaltene Tasten wie im Original: S kehrt die Strompfadausrichtung, A die automatischen Verbindungen um,
        // solange sie gedrückt sind; Umschalt schaltet den Rasterfang ab. Ein kurzer Druck ohne Setzen/Ziehen
        // schaltet beim Loslassen wie bisher um. Gilt nur für Belegungen ohne Strg/Umschalt/Alt.
        let busy = self.placing.is_some() || matches!(self.drag, Drag::Move { .. } | Drag::Create { .. } | Drag::Wire { .. } | Drag::Resize { .. } | Drag::Scale { .. });
        let hold_keys: [Option<Key>; 2] = ["pfadfang", "autoverb"].map(|id| {
            crate::kuerzel::parse(&crate::kuerzel::belegung(&eigene, id)).filter(|(m, _)| !m.ctrl && !m.shift && !m.alt && !m.command).map(|(_, k)| k)
        });
        let (down, mods_free, shift_down) = ctx.input(|i| {
            let m = i.modifiers;
            (hold_keys.map(|k| k.map(|k| i.key_down(k)).unwrap_or(false)), !m.ctrl && !m.alt && !m.command && !m.shift, m.shift)
        });
        self.no_snap = shift_down;
        let mut toggle = [false, false];
        for n in 0..2 {
            match (self.hold_press[n], down[n]) {
                (None, true) if mods_free => self.hold_press[n] = Some(busy),
                (Some(used), true) => self.hold_press[n] = Some(used || busy),
                (Some(used), false) => {
                    self.hold_press[n] = None;
                    toggle[n] = !used;
                }
                _ => {}
            }
        }
        self.hold_s = self.hold_press[0].is_some();
        self.hold_a = self.hold_press[1].is_some();
        if toggle[0] {
            self.path_align = !self.path_align;
        }
        if toggle[1] {
            self.auto_conn = !self.auto_conn;
        }
        // Befehle mit einstellbarer Belegung (Optionen › Tastenkürzel); S/A ohne Modifikatoren siehe oben
        let fired: Vec<&'static str> = ctx.input(|i| {
            crate::kuerzel::BEFEHLE
                .iter()
                .filter(|b| !(b.id == "pfadfang" && hold_keys[0].is_some() || b.id == "autoverb" && hold_keys[1].is_some()))
                .filter(|b| crate::kuerzel::gedrueckt(i, &eigene, b.id))
                .map(|b| b.id)
                .collect()
        });
        let (arrows, shift, copy, cut, paste) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                [i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)],
                i.modifiers.shift,
                i.events.iter().any(|e| matches!(e, Event::Copy)),
                i.events.iter().any(|e| matches!(e, Event::Cut)),
                i.events.iter().any(|e| matches!(e, Event::Paste(_))) || (cmd && i.key_pressed(Key::V)),
            )
        });
        for id in fired {
            self.run_befehl(id);
        }
        // Esc bricht „Eigentümer setzen“ immer ab (auch bei anderer Belegung von „Werkzeug: Auswahl“)
        if self.owner_pick.is_some() && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.owner_pick = None;
            self.info = "Eigentümer setzen abgebrochen".into();
        }
        if copy {
            self.copy();
        }
        if cut {
            self.copy();
            self.delete_selection();
        }
        if paste {
            self.paste();
        }
        if arrows.iter().any(|b| *b) && !self.selection.is_empty() {
            let st = if shift || self.raster() <= 0.0 { 1.0 } else { self.raster() };
            let d = if arrows[0] {
                P::new(-st, 0.0)
            } else if arrows[1] {
                P::new(st, 0.0)
            } else if arrows[2] {
                P::new(0.0, -st)
            } else {
                P::new(0.0, st)
            };
            self.checkpoint();
            let sel: Vec<u64> = self.selected_objs().iter().filter(|o| !(o.fixed && self.fix_active)).map(|o| o.id).collect();
            self.with_sheet(|s, _| s.objects.iter_mut().filter(|o| sel.contains(&o.id)).for_each(|o| o.kind.translate(d)));
            // mit gehaltener Taste A wie im Original: freie Anschlüsse nach dem Schritt automatisch verbinden
            if self.hold_a {
                if let Some(p) = self.hold_press[1].as_mut() {
                    *p = true;
                }
                let ids: Vec<u64> = self.selected_objs().iter().filter(|o| sel.contains(&o.id) && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })).map(|o| o.id).collect();
                self.auto_connect_nocp(&ids);
            }
        }
    }

    fn handle_shot(&mut self, ctx: &egui::Context) {
        if let Some(r) = self.export_req.take() {
            let rect = self.sheet_screen_rect;
            let ppp = ctx.pixels_per_point();
            ctx.request_screenshot(move |img| {
                let x0 = (rect.left() * ppp).max(0.0) as usize;
                let y0 = (rect.top() * ppp).max(0.0) as usize;
                let x1 = ((rect.right() * ppp) as usize).min(img.size[0]);
                let y1 = ((rect.bottom() * ppp) as usize).min(img.size[1]);
                if x1 > x0 && y1 > y0 {
                    let mut buf = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let c = img.pixels[y * img.size[0] + x];
                            buf.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
                        }
                    }
                    let _ = image::save_buffer(&r, &buf, (x1 - x0) as u32, (y1 - y0) as u32, image::ExtendedColorType::Rgba8);
                }
            });
        }
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
        ctx.request_repaint();
        if shot.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if shot.frames >= 120 && !shot.requested {
            shot.requested = true;
            let path = shot.path.clone();
            let done = shot.done.clone();
            ctx.request_screenshot(move |img| {
                let mut buf = Vec::with_capacity(img.pixels.len() * 4);
                for c in &img.pixels {
                    buf.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
                }
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = image::save_buffer(&path, &buf, img.size[0] as u32, img.size[1] as u32, image::ExtendedColorType::Rgba8) {
                    eprintln!("Bild konnte nicht gespeichert werden: {e}");
                }
                done.store(true, Ordering::SeqCst);
            });
        }
        if shot.frames > 3000 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Baumknoten aus Gruppenpfaden bilden (Gruppen und Symbole alphabetisch wie im Original).
fn group_nodes(prefix: &str, defs: &[&SymDef], depth: usize) -> Vec<TNode> {
    let mut groups: std::collections::BTreeMap<String, Vec<&SymDef>> = std::collections::BTreeMap::new();
    let mut leaves: Vec<&SymDef> = vec![];
    for d in defs {
        match d.group.get(depth) {
            Some(g) => groups.entry(g.clone()).or_default().push(d),
            None => leaves.push(d),
        }
    }
    let mut out = vec![];
    for (g, list) in groups {
        let id = format!("{prefix}/{g}");
        let children = group_nodes(&id, &list, depth + 1);
        out.push(TNode { id, label: g, bold: true, icon: 1, action: TAction::None, children });
    }
    leaves.sort_by(|a, b| a.name.cmp(&b.name));
    for d in leaves {
        out.push(TNode { id: format!("{prefix}/sym/{}", d.key), label: d.name.clone(), bold: false, icon: 0, action: TAction::Symbol(d.key.clone()), children: vec![] });
    }
    out
}

impl App {
    /// Kontextmenü des Projektexplorers wie im Original (PopupMenu2, Sichtbarkeit aus PopupMenu2Popup @ 0x4422b4):
    /// feste Reihenfolge, je Knotenart nur die passenden Einträge.
    fn tree_menu(&self, ui: &mut egui::Ui, node: &TNode, ev: &mut TreeEvents) {
        let tab = self.right_tab;
        let id = node.id.as_str();
        // Gruppe im Projektbaum (Feld +0x0c der Gruppe bzw. ihres Elternknotens: 1 Potentiale … 7 Platzhaltergruppen)
        let cat = if tab == RightTab::Projekt {
            [("prj/pot", "Neues Potential"), ("prj/ltg", "Neue Leitung"), ("prj/sym", "Neues Symbol"), ("prj/stift", "Neuer Stift"), ("prj/img", "Neue Grafik"), ("prj/bt", "Neues Bauteil"), ("prj/ph", "Neue Platzhaltergruppe")]
                .iter()
                .find(|(p, _)| id == *p || id.starts_with(&format!("{p}/")))
                .map(|(_, n)| *n)
        } else {
            None
        };
        let is_anlage = tab == RightTab::Projekt && id.starts_with("prj/a") && id.matches('/').count() == 1;
        let is_sym = matches!(node.action, TAction::Symbol(_));
        let is_sheet = matches!(node.action, TAction::Sheet(_));
        let is_part = matches!(node.action, TAction::Bauteil(_));
        let is_entry = !matches!(node.action, TAction::None) || is_anlage;
        let fav = match &node.action {
            TAction::Symbol(k) => symbols::find(&self.lib, k).map(|d| d.fav).unwrap_or(false),
            _ => false,
        };
        fn item(ui: &mut egui::Ui, show: bool, label: &str, pick: &mut Option<String>) {
            if show && ui.button(label).clicked() {
                *pick = Some(label.to_string());
                ui.close();
            }
        }
        let mut pick: Option<String> = None;
        item(ui, is_entry, "Eigenschaften", &mut pick);
        item(ui, is_entry, "Löschen", &mut pick);
        if is_sym && tab == RightTab::Projekt {
            ui.menu_button("In Bibliothek", |ui| {
                for l in ["Standardbereich", "Benutzerbereich"] {
                    if ui.button(l).clicked() {
                        pick = Some(l.to_string());
                        ui.close();
                    }
                }
            });
        }
        item(ui, is_sym, "Symbol bearbeiten", &mut pick);
        item(ui, is_sheet, "Blatt schliessen", &mut pick);
        item(ui, is_sheet, "Alle Blätter dieser Anlage  schließen", &mut pick);
        item(ui, is_part, "Bauteil bearbeiten", &mut pick);
        let lib_tab = tab != RightTab::Projekt && (is_sym || is_part);
        item(ui, lib_tab && !fav, "Zu den Favoriten hinzufügen", &mut pick);
        item(ui, lib_tab && fav, "Aus den Favoriten entfernen", &mut pick);
        if is_sheet || is_anlage {
            ui.menu_button("Reihenfolge", |ui| {
                for l in ["Ganz nach Oben", "Eins nach Oben", "Eins nach Unten", "Ganz nach Unten"] {
                    if ui.button(l).clicked() {
                        pick = Some(l.to_string());
                        ui.close();
                    }
                }
            });
        }
        for l in ["Neues Symbol", "Neue Grafik", "Neue Leitung", "Neues Potential", "Neuer Stift", "Neues Bauteil", "Neue Platzhaltergruppe"] {
            item(ui, cat == Some(l), l, &mut pick);
        }
        if let Some(l) = pick {
            ev.ctx = Some((node.clone(), l));
        }
    }

    /// Alle Blätter des Projekts als Vektor-PDF ausgeben (und auf Wunsch drucken).
    fn make_pdf(&mut self, ctx: &egui::Context) {
        // vorher Projekt prüfen (Optionen), bei Hinweisen Rückfrage in suche.rs
        if !self.ausgabe_freigeben() {
            return;
        }
        let Some((path, print)) = self.pdf_req.take() else { return };
        let sel = self.print_sel.take();
        let (bytes, n) = crate::pdf::render_sheets(ctx, &self.project, &self.lib, &self.images, &|r| sel.as_ref().map(|v| v.contains(&r)).unwrap_or(true));
        match std::fs::write(&path, bytes) {
            Ok(()) => {
                if print {
                    let printer = self.print_printer.take();
                    match crate::pdf::print_file_to(&path, printer.as_deref()) {
                        Ok(()) => self.info = format!("{n} Blätter an den Drucker übergeben"),
                        Err(e) => self.dialog = Some(Dialog::Message(format!("Drucken nicht möglich: {e}"))),
                    }
                } else {
                    self.info = format!("PDF gespeichert: {} ({n} Blätter)", path.display());
                }
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("PDF konnte nicht geschrieben werden: {e}"))),
        }
    }
}

#[derive(Default)]
struct TreeEvents {
    drag_start: Option<SheetRef>,
    drop_on: Option<SheetRef>,
    clicked: Option<TNode>,
    dbl: Option<TNode>,
    ctx: Option<(TNode, String)>,
    toggled: Option<String>,
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(d) = &self.lib_dir {
            storage.set_string("lib_dir", d.to_string_lossy().to_string());
        }
        let favs: Vec<String> = self.fav_overrides.iter().map(|(k, v)| format!("{k}\t{}", if *v { 1 } else { 0 })).collect();
        storage.set_string("favs", favs.join("\n"));
        storage.set_string("recent", self.recent.join("\n"));
        if let Ok(j) = serde_json::to_string(&self.opt) {
            storage.set_string("optionen", j);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shot(&ctx);
        self.shortcuts(&ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));

        let pf = Frame::new().fill(BG).inner_margin(Margin::ZERO);
        egui::Panel::top("menu").exact_size(20.0).frame(Frame::new().fill(Color32::WHITE).inner_margin(Margin { left: 2, right: 0, top: 0, bottom: 0 })).show(ui, |ui| {
            self.menu(ui);
        });
        egui::Panel::top("tb2").exact_size(29.0).frame(pf).show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, Color32::from_gray(225)));
            ui.painter().line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, Color32::from_gray(225)));
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(r.min + Vec2::new(0.0, 1.0), r.max)));
            self.top_toolbar(&mut c);
        });
        egui::Panel::bottom("status").exact_size(20.0).frame(pf).show(ui, |ui| {
            self.status_bar(ui);
        });
        egui::Panel::bottom("tabs").exact_size(23.0).frame(pf).show(ui, |ui| {
            self.bottom_row(ui);
        });
        let rw = self.right_w;
        let resp = egui::Panel::right("right")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(rw)
            .frame(pf.inner_margin(Margin { left: 3, right: 0, top: 0, bottom: 0 }))
            .show(ui, |ui| {
                self.right_panel(ui);
            });
        let _ = resp;
        egui::CentralPanel::no_frame().frame(Frame::new().fill(BG)).show(ui, |ui| {
            let full = ui.max_rect();
            let tb3 = Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + 30.0));
            let mut c3 = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(tb3.min + Vec2::new(0.0, 1.0), tb3.max)));
            if self.sym_edit.is_some() {
                self.bleiste(&mut c3);
            } else {
                self.second_toolbar(&mut c3);
            }
            ui.painter().line_segment([Pos2::new(full.left(), tb3.bottom() - 0.5), Pos2::new(full.right(), tb3.bottom() - 0.5)], Stroke::new(1.0, Color32::from_gray(160)));
            let left = Rect::from_min_max(Pos2::new(full.left(), tb3.bottom() - 3.0), Pos2::new(full.left() + 27.0, full.bottom()));
            let mut cl = ui.new_child(egui::UiBuilder::new().max_rect(left));
            self.left_toolbar(&mut cl);
            let area = Rect::from_min_max(Pos2::new(full.left() + 29.0, tb3.bottom()), Pos2::new(full.right() - 2.0, full.bottom() - 1.0));
            let mut ca = ui.new_child(egui::UiBuilder::new().max_rect(area));
            self.canvas(&mut ca);
        });
        self.dialogs(&ctx);
        self.props_ui(&ctx);
        self.align_window(&ctx);
        self.optionen_ui(&ctx);
        self.dialoge2_ui(&ctx);
        let now = ctx.input(|i| i.time);
        self.retry_library(now);
        self.extra_dialogs(&ctx);
        self.dialoge3_ui(&ctx);
        self.ask_save_ui(&ctx);
        self.suche_ui(&ctx);
        self.make_pdf(&ctx);
    }
}

impl App {
    /// Projektbild als Bildobjekt (Pfad "bld:<Nummer>") mittig auf das aktuelle Blatt setzen, Breite 80 mm.
    pub fn place_project_image(&mut self, i: usize) {
        let Some(b) = self.project.bilder.get(i) else { return };
        let (w, h) = image::load_from_memory(&b.jpeg).map(|im| (im.width() as f32, im.height() as f32)).unwrap_or((4.0, 3.0));
        let path = format!("bld:{}", b.nr);
        let (sw, sh) = self.sheet().map(|s| s.dims()).unwrap_or((SHEET_W, SHEET_H));
        let bw = 80.0;
        let bh = bw * h / w.max(1.0);
        let a = P::new((sw - bw) / 2.0, (sh - bh) / 2.0);
        self.add_obj(Kind::Image { a, b: P::new(a.x + bw, a.y + bh), path, mode: 2, frame: false });
    }
}

impl App {
    /// Rückfrage "Geänderte Daten Speichern?" (Ja / Nein / Abbrechen) und Schließen des Fensters abfangen.
    pub fn ask_save_ui(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.quit_ok && self.shot.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.ask_save = Some(crate::app::Pending::Quit);
        }
        let Some(p) = self.ask_save.clone() else { return };
        let mut choice: Option<u8> = None;
        let closed = crate::props::tool_window(ctx, "asksave", "Elektroplanung", [300.0, 70.0], egui::Order::Foreground, |ui| {
            ui.label("Geänderte Daten Speichern?");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_sized([86.0, 24.0], egui::Button::new("Ja")).clicked() {
                    choice = Some(0);
                }
                if ui.add_sized([86.0, 24.0], egui::Button::new("Nein")).clicked() {
                    choice = Some(1);
                }
                if ui.add_sized([86.0, 24.0], egui::Button::new("Abbrechen")).clicked() {
                    choice = Some(2);
                }
            });
        });
        if closed {
            choice = Some(2);
        }
        match choice {
            Some(0) => {
                self.ask_save = None;
                self.save();
                // nur weiter, wenn wirklich gespeichert wurde (Speichern als kann abgebrochen werden)
                if !self.dirty {
                    self.run_pending(p.clone());
                }
            }
            Some(1) => {
                self.ask_save = None;
                self.dirty = false;
                self.run_pending(p.clone());
            }
            Some(_) => self.ask_save = None,
            None => {}
        }
        if self.quit_ok && matches!(p, crate::app::Pending::Quit) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Breite der Bildlaufleisten in Punkten (Windows-Standard 17 px)
const SCROLLBAR: f32 = 17.0;

impl App {
    /// Bildlaufleisten: Bereich = Blatt mit Rand ∪ sichtbare Fläche; Schieber ziehen, Leiste klicken = Seite weiter,
    /// Pfeile = kleiner Schritt.
    fn scrollbars(&mut self, ui: &mut egui::Ui, p: &egui::Painter, full: Rect, area: Rect) {
        let (sw, sh) = if self.sym_edit.is_some() { (SHEET_W, SHEET_H) } else { self.sheet().map(|s| s.dims()).unwrap_or((SHEET_W, SHEET_H)) };
        let z = self.view.zoom;
        let sheet = Rect::from_min_size(area.min + self.view.offset, Vec2::new(sw * z, sh * z)).expand(20.0);
        let content = sheet.union(area);
        let track_bg = Color32::from_gray(240);
        let thumb_c = Color32::from_gray(192);
        let thumb_h = Color32::from_gray(166);
        p.rect_filled(Rect::from_min_max(area.right_bottom(), full.max), 0.0, track_bg);
        for vertical in [false, true] {
            let track = if vertical {
                Rect::from_min_max(Pos2::new(area.right(), area.top()), Pos2::new(full.right(), area.bottom()))
            } else {
                Rect::from_min_max(Pos2::new(area.left(), area.bottom()), Pos2::new(area.right(), full.bottom()))
            };
            p.rect_filled(track, 0.0, track_bg);
            let len = |r: Rect| if vertical { r.height() } else { r.width() };
            let lo = |r: Rect| if vertical { r.top() } else { r.left() };
            // Pfeilknöpfe an den Enden
            let ab = SCROLLBAR;
            let inner = if vertical { Rect::from_min_max(track.min + Vec2::new(0.0, ab), track.max - Vec2::new(0.0, ab)) } else { Rect::from_min_max(track.min + Vec2::new(ab, 0.0), track.max - Vec2::new(ab, 0.0)) };
            let total = len(content).max(1.0);
            let tl = len(inner);
            let thumb_len = (tl * len(area) / total).clamp(18.0, tl);
            let pos = (lo(area) - lo(content)) / total * tl;
            let thumb = if vertical {
                Rect::from_min_size(Pos2::new(inner.left() + 2.0, inner.top() + pos), Vec2::new(inner.width() - 4.0, thumb_len))
            } else {
                Rect::from_min_size(Pos2::new(inner.left() + pos, inner.top() + 2.0), Vec2::new(thumb_len, inner.height() - 4.0))
            };
            let id = ui.scope_id().with(("scroll", vertical));
            let tr = ui.interact(thumb, id.with("thumb"), Sense::drag());
            let k = total / tl.max(1.0);
            let mut shift = 0.0f32;
            if tr.dragged() {
                shift = if vertical { tr.drag_delta().y } else { tr.drag_delta().x } * k;
            }
            p.rect_filled(thumb, 0.0, if tr.hovered() || tr.dragged() { thumb_h } else { thumb_c });
            let rr = ui.interact(inner, id.with("track"), Sense::click());
            if rr.clicked() && !tr.hovered() {
                if let Some(h) = rr.interact_pointer_pos() {
                    let before = if vertical { h.y < thumb.top() } else { h.x < thumb.left() };
                    shift = if before { -len(area) * 0.9 } else { len(area) * 0.9 };
                }
            }
            for (i, end) in [(0, true), (1, false)] {
                let br = if vertical {
                    if end { Rect::from_min_size(track.min, Vec2::splat(ab)) } else { Rect::from_min_size(Pos2::new(track.left(), track.bottom() - ab), Vec2::splat(ab)) }
                } else if end {
                    Rect::from_min_size(track.min, Vec2::splat(ab))
                } else {
                    Rect::from_min_size(Pos2::new(track.right() - ab, track.top()), Vec2::splat(ab))
                };
                let b = ui.interact(br, id.with(("arrow", i)), Sense::click());
                let c = br.center();
                let d = 3.5;
                let tri = match (vertical, end) {
                    (true, true) => vec![Pos2::new(c.x, c.y - d), Pos2::new(c.x - d, c.y + d * 0.6), Pos2::new(c.x + d, c.y + d * 0.6)],
                    (true, false) => vec![Pos2::new(c.x, c.y + d), Pos2::new(c.x + d, c.y - d * 0.6), Pos2::new(c.x - d, c.y - d * 0.6)],
                    (false, true) => vec![Pos2::new(c.x - d, c.y), Pos2::new(c.x + d * 0.6, c.y + d), Pos2::new(c.x + d * 0.6, c.y - d)],
                    (false, false) => vec![Pos2::new(c.x + d, c.y), Pos2::new(c.x - d * 0.6, c.y - d), Pos2::new(c.x - d * 0.6, c.y + d)],
                };
                p.add(egui::Shape::convex_polygon(tri, if b.hovered() { Color32::BLACK } else { Color32::from_gray(96) }, Stroke::NONE));
                if b.clicked() {
                    shift = if end { -40.0 } else { 40.0 };
                }
            }
            if shift != 0.0 {
                // Inhalt verschiebt sich entgegen der Leiste, begrenzt auf den Bereich
                let max_back = lo(area) - lo(content);
                let max_fwd = (lo(content) + len(content)) - (lo(area) + len(area));
                let s = shift.clamp(-max_back, max_fwd);
                if vertical {
                    self.view.offset.y -= s;
                } else {
                    self.view.offset.x -= s;
                }
                self.view.user = true;
            }
        }
    }
}

impl App {
    /// Befehl eines Tastenkürzels ausführen.
    pub fn run_befehl(&mut self, id: &str) {
        let has_sel = !self.selection.is_empty();
        let center = self.canvas_area.center();
        match id {
            "neu" => self.guarded(crate::app::Pending::New),
            "oeffnen" => self.guarded(crate::app::Pending::Open),
            "speichern" => self.save(),
            "speichern_als" => self.save_as(),
            "drucken" => self.open_druck_dialog(),
            "suchen" => self.open_suche(),
            "rueck" => self.do_undo(),
            "vor" | "vor2" => self.do_redo(),
            "loeschen" => self.delete_selection(),
            "alles" => {
                if let Some(s) = self.sheet() {
                    self.selection = s.objects.iter().filter(|o| self.selectable(o)).map(|o| o.id).collect();
                }
            }
            "eigenschaften" => {
                if let Some(id) = self.selection.first().copied() {
                    self.edit_props(id);
                }
            }
            "drehen" if has_sel => self.transform_selection(1),
            "spiegel_v" if has_sel => self.transform_selection(2),
            "spiegel_h" if has_sel => self.transform_selection(4),
            "gruppieren" if self.selection.len() > 1 => self.group_selection(true),
            "aufloesen" if has_sel => self.group_selection(false),
            "symbol_bearbeiten" => {
                let key = self.selected_objs().iter().find_map(|o| if let Kind::Symbol { key, .. } = &o.kind { Some(key.clone()) } else { None });
                if let Some(k) = key {
                    self.open_symbol_editor(Some(k));
                }
            }
            "einpassen" => {
                self.view.user = false;
                self.view.last_size = Vec2::ZERO;
            }
            "zoom_in" if self.canvas_area.is_positive() => self.zoom_at(center, self.canvas_area, 1.25),
            "zoom_out" if self.canvas_area.is_positive() => self.zoom_at(center, self.canvas_area, 0.8),
            "blatt_neu" => self.new_sheet(),
            "blatt_vor" if !self.tabs.is_empty() => {
                self.active = (self.active + 1) % self.tabs.len();
                self.selection.clear();
            }
            "blatt_zurueck" if !self.tabs.is_empty() => {
                self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
                self.selection.clear();
            }
            "pfadfang" => self.path_align = !self.path_align,
            "autoverb" => self.auto_conn = !self.auto_conn,
            "gitter" => self.grid = !self.grid,
            "w_auswahl" => {
                if self.owner_pick.take().is_some() {
                    self.info = "Eigentümer setzen abgebrochen".into();
                }
                self.placing = None;
                self.tool = Tool::Select;
                self.drag = Drag::None;
                self.zoom_tool = false;
            }
            "w_linie" => self.tool = Tool::Line,
            "w_rechteck" => self.tool = Tool::Rect,
            "w_kreis" => self.tool = Tool::Circle,
            "w_text" => self.tool = Tool::Text,
            "w_klemme" => self.tool = Tool::Klemmstelle,
            _ => {}
        }
    }
}
fn dbg_log(t: &str) {
    if let Ok(p) = std::env::var("EP_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{t}");
        }
    }
}
