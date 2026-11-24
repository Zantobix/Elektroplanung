//! Importfenster für DXF (Nachbau-Erweiterung, Nutzer 24.11.2026, siehe `dxf.rs`): Ebenen an/aus mit Farbe und
//! Anzahl, Maßstab (Einheit der Datei oder fest), Einpassen, Gruppieren, Farben als Stifte, Vorschau.

use crate::app::App;
use crate::dxf::{self, Einstellungen, Zeichnung};
use crate::model::*;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

/// Wahl des Maßstabs.
#[derive(Clone, Copy, PartialEq)]
pub enum Massstab {
    Datei,
    Mm,
    Cm,
    M,
    Zoll,
    Faktor,
}

pub struct DxfDialog {
    pub pfad: std::path::PathBuf,
    pub z: Zeichnung,
    pub e: Einstellungen,
    pub wahl: Massstab,
    pub faktor: String,
}

impl DxfDialog {
    pub fn neu(pfad: std::path::PathBuf, z: Zeichnung) -> Self {
        let e = Einstellungen::vorgabe(&z);
        DxfDialog { pfad, z, e, wahl: Massstab::Datei, faktor: "1".into() }
    }

    fn massstab(&self) -> f32 {
        match self.wahl {
            Massstab::Datei => dxf::mm_je_einheit(self.z.einheit),
            Massstab::Mm => 1.0,
            Massstab::Cm => 10.0,
            Massstab::M => 1000.0,
            Massstab::Zoll => 25.4,
            Massstab::Faktor => self.faktor.trim().replace(',', ".").parse::<f32>().ok().filter(|f| *f > 0.0).unwrap_or(1.0),
        }
    }
}

fn farbe(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

impl App {
    /// Datei › DXF Import: Datei wählen und das Importfenster öffnen.
    pub fn dxf_import(&mut self) {
        // Original (0x49d9a0): ohne geöffnetes Blatt kein Dateidialog
        if self.cur().is_none() {
            return;
        }
        let Some(p) = rfd::FileDialog::new().add_filter("DXF", &["dxf"]).pick_file() else { return };
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| dxf::lesen(&b)) {
            Ok(z) => self.dxf_dlg = Some(DxfDialog::neu(p, z)),
            Err(e) => crate::meldungen::warnung(crate::meldungen::T_FEHLER_X, &format!("{}\n{e}", crate::meldungen::DXF_FEHLER)),
        }
    }

    /// Übernehmen nach den Einstellungen des Fensters.
    pub fn dxf_uebernehmen(&mut self, d: &DxfDialog) {
        let Some(frame) = self.sheet().map(|s| s.frame()) else { return };
        let mut e = d.e.clone();
        e.massstab = d.massstab();
        let teile = dxf::umsetzen(&d.z, &e, frame);
        if teile.is_empty() {
            self.info = "DXF Import: keine Elemente auf den gewählten Ebenen".into();
            return;
        }
        self.checkpoint();
        // Farben: je Ebene und Farbe ein Stift „DXF <Ebene>“ (vorhandene gleichen Namens und gleicher Farbe nutzen)
        let stift_fuer = |p: &mut Project, ebene: &str, c: [u8; 3]| -> usize {
            let name = format!("DXF {ebene}");
            if let Some(i) = p.stifte.iter().position(|s| s.name == name && s.color == c) {
                return i;
            }
            let name = if p.stifte.iter().any(|s| s.name == name) { format!("{name} {:02X}{:02X}{:02X}", c[0], c[1], c[2]) } else { name };
            p.stifte.push(Stift { name, style: 0, color: c, width: 0.25 });
            p.stifte.len() - 1
        };
        let gid = e.gruppieren.then(|| self.project.alloc_id() + 1_000_000);
        let mut ids = vec![];
        for (k, c, ebene) in teile {
            let pen = match c {
                Some(c) => stift_fuer(&mut self.project, &ebene, c),
                None => self.pen,
            };
            let id = self.project.alloc_id();
            self.with_sheet(|s, _| s.objects.push(Obj { id, kind: k, pen, fixed: false, group: gid, raw: None }));
            ids.push(id);
        }
        self.dirty = true;
        self.info = format!("DXF Import: {} Elemente", ids.len());
        self.selection = ids;
    }

    /// Importfenster zeichnen.
    pub fn dxf_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dxf_dlg.take() else { return };
        let mut ok = false;
        let mut weg = false;
        egui::Window::new("DXF Import").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.label(d.pfad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.strong("Ebenen");
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for eb in &d.z.ebenen {
                            let mut an = d.e.ebenen.contains(&eb.name);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut an, "").changed() {
                                    if an {
                                        d.e.ebenen.push(eb.name.clone());
                                    } else {
                                        d.e.ebenen.retain(|n| n != &eb.name);
                                    }
                                }
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                                ui.painter().rect_filled(r, 0.0, farbe(dxf::aci_rgb(eb.farbe)));
                                ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Inside);
                                ui.label(format!("{} ({}){}", eb.name, eb.anzahl, if eb.aus { " – in der Datei aus" } else { "" }));
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.small_button("alle").clicked() {
                            d.e.ebenen = d.z.ebenen.iter().map(|e| e.name.clone()).collect();
                        }
                        if ui.small_button("keine").clicked() {
                            d.e.ebenen.clear();
                        }
                    });
                    ui.add_space(6.0);
                    ui.strong("Maßstab");
                    for (w, t) in [
                        (Massstab::Datei, format!("Einheit der Datei: {}", dxf::einheit_name(d.z.einheit))),
                        (Massstab::Mm, "1 Einheit = 1 mm".into()),
                        (Massstab::Cm, "1 Einheit = 1 cm".into()),
                        (Massstab::M, "1 Einheit = 1 m".into()),
                        (Massstab::Zoll, "1 Einheit = 1 Zoll".into()),
                    ] {
                        ui.radio_value(&mut d.wahl, w, t);
                    }
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut d.wahl, Massstab::Faktor, "mm je Einheit:");
                        ui.add(egui::TextEdit::singleline(&mut d.faktor).desired_width(60.0));
                    });
                    ui.add_space(6.0);
                    ui.checkbox(&mut d.e.einpassen, "zu große Zeichnung in den Rahmen einpassen");
                    ui.checkbox(&mut d.e.gruppieren, "als Gruppe einfügen");
                    ui.checkbox(&mut d.e.farben, "Ebenenfarben als Stifte übernehmen");
                });
                // Vorschau: übernommene Elemente in den Rahmen des Blattes gezeichnet
                let (r, _) = ui.allocate_exact_size(Vec2::new(300.0, 220.0), Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 0.0, Color32::WHITE);
                p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Inside);
                if let Some(frame) = self.sheet().map(|s| s.frame()) {
                    let mut e = d.e.clone();
                    e.massstab = d.massstab();
                    let (w, h) = (frame.2 - frame.0, frame.3 - frame.1);
                    let k = ((r.width() - 8.0) / w).min((r.height() - 8.0) / h);
                    let o = Pos2::new(r.center().x - w * k / 2.0 - frame.0 * k, r.center().y - h * k / 2.0 - frame.1 * k);
                    let s = |q: P| Pos2::new(o.x + q.x * k, o.y + q.y * k);
                    p.rect_stroke(Rect::from_two_pos(s(P::new(frame.0, frame.1)), s(P::new(frame.2, frame.3))), 0.0, Stroke::new(1.0, Color32::LIGHT_GRAY), egui::StrokeKind::Inside);
                    for (kd, c, _) in dxf::umsetzen(&d.z, &e, frame) {
                        let st = Stroke::new(1.0, c.map(farbe).unwrap_or(Color32::BLACK));
                        match kd {
                            Kind::Line { a, b } => {
                                p.line_segment([s(a), s(b)], st);
                            }
                            Kind::Poly { pts } => {
                                let mut v: Vec<Pos2> = pts.iter().map(|q| s(*q)).collect();
                                if let Some(f) = v.first().copied() {
                                    v.push(f);
                                }
                                p.add(egui::Shape::line(v, st));
                            }
                            Kind::Circle { a, b } => {
                                p.circle_stroke(s(P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)), (b.x - a.x).abs() / 2.0 * k, st);
                            }
                            Kind::Text { pos, .. } => {
                                p.circle_filled(s(pos), 1.0, st.color);
                            }
                            _ => {}
                        }
                    }
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!d.e.ebenen.is_empty(), egui::Button::new("Importieren")).clicked() {
                    ok = true;
                }
                if ui.button("Abbrechen").clicked() {
                    weg = true;
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                weg = true;
            }
        });
        if ok {
            self.dxf_uebernehmen(&d);
        } else if !weg {
            self.dxf_dlg = Some(d);
        }
    }
}
