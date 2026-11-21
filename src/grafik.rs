//! Projektgrafiken wie im Original (Teilformular TGrafik: Name, Vorschau, „Laden“).
//!
//! - Hinzufügen › Grafik / Projektbaum › „Neue Grafik“ (NeueGrafik1Click): Bilddatei wählen; die Grafik wird
//!   als Projektbild angelegt, ihr Name ist der Dateiname (höchstens 20 Zeichen, BLD Byte 0–20).
//! - Projektbaum › Eigenschaften: Name ändern, mit „Laden“ (Grafik1Button1Click) das Bild ersetzen.
//! - Projektbaum › Löschen: Projektbild entfernen, solange es auf keinem Blatt verwendet wird.
//!
//! Projektbilder werden als JPEG gehalten wie im .epr (PRJ › BLD › JPG); andere Formate werden umgewandelt.

use crate::app::{App, Dialog};
use crate::model::*;
use eframe::egui::{self, Vec2};

pub struct GrafikDlg {
    pub idx: usize,
    pub name: String,
    /// neu geladene Bilddaten (JPEG), erst bei OK übernommen
    pub jpeg: Option<Vec<u8>>,
    pub tex: Option<egui::TextureHandle>,
}

/// Bilddatei lesen und als JPEG liefern (JPEG-Dateien unverändert).
pub fn load_as_jpeg(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return Ok(bytes);
    }
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.to_rgb8().write_to(&mut out, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// Name der Grafik aus dem Dateinamen (ohne Pfad und Endung, höchstens 20 Zeichen wie BLD Byte 0–20).
pub fn name_from_path(path: &std::path::Path) -> String {
    let s = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    s.chars().take(20).collect()
}

/// Wird das Projektbild mit dieser Nummer auf einem Blatt verwendet?
pub fn graphic_in_use(p: &Project, nr: i32) -> bool {
    let key = format!("bld:{nr}");
    p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).any(|o| matches!(&o.kind, Kind::Image { path, .. } if *path == key))
}

fn pick_image() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().add_filter("Bilder", &["jpg", "jpeg", "png", "bmp"]).pick_file()
}

impl App {
    /// Hinzufügen › Grafik: Bilddatei wählen und als Projektgrafik anlegen.
    pub fn new_project_graphic(&mut self) {
        let Some(path) = pick_image() else { return };
        match load_as_jpeg(&path) {
            Ok(jpeg) => {
                self.checkpoint();
                let nr = self.project.bilder.iter().map(|b| b.nr).max().unwrap_or(0) + 1;
                self.project.bilder.push(Bild { name: name_from_path(&path), nr, jpeg });
                self.info = format!("Grafik „{}“ angelegt", name_from_path(&path));
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Die Grafik konnte nicht geladen werden: {e}"))),
        }
    }

    /// Projektbaum › Eigenschaften einer Projektgrafik.
    pub fn open_grafik_dialog(&mut self, idx: usize) {
        if let Some(b) = self.project.bilder.get(idx) {
            self.zusatz.grafik = Some(GrafikDlg { idx, name: b.name.clone(), jpeg: None, tex: None });
        }
    }

    /// Projektbaum › Löschen einer Projektgrafik.
    pub fn delete_project_graphic(&mut self, idx: usize) {
        let Some(b) = self.project.bilder.get(idx) else { return };
        if graphic_in_use(&self.project, b.nr) {
            self.dialog = Some(Dialog::Message(format!("Die Grafik „{}“ wird auf einem Blatt verwendet und kann nicht gelöscht werden.", b.name)));
            return;
        }
        self.checkpoint();
        let b = self.project.bilder.remove(idx);
        self.images.remove(&format!("bld:{}", b.nr));
    }

    pub fn grafik_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.zusatz.grafik.take() else { return };
        if d.tex.is_none() {
            let data = d.jpeg.clone().or_else(|| self.project.bilder.get(d.idx).map(|b| b.jpeg.clone())).unwrap_or_default();
            if let Ok(img) = image::load_from_memory(&data) {
                let rgba = img.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                d.tex = Some(ctx.load_texture("grafik_vorschau", ci, Default::default()));
            }
        }
        let mut done: Option<bool> = None;
        let mut load = false;
        let closed = crate::props::tool_window(ctx, "grafikdlg", "Eigenschaften", [289.0, 405.0], egui::Order::Foreground, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Name").font(crate::props::bold(13.0)));
                ui.add(egui::TextEdit::singleline(&mut d.name).char_limit(20).desired_width(200.0));
            });
            let (r, _) = ui.allocate_exact_size(Vec2::new(277.0, 289.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 0.0, egui::Color32::WHITE);
            if let Some(t) = &d.tex {
                let s = t.size_vec2();
                let k = (r.width() / s.x).min(r.height() / s.y).min(1.0);
                let ir = egui::Rect::from_center_size(r.center(), s * k);
                ui.painter().image(t.id(), ir, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            ui.horizontal(|ui| {
                ui.add_space(101.0);
                if ui.add(egui::Button::new("Laden").min_size(Vec2::new(75.0, 25.0))).clicked() {
                    load = true;
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                if ui.add(egui::Button::new("Abbruch").min_size(Vec2::new(75.0, 25.0))).clicked() {
                    done = Some(false);
                }
                ui.add_space(60.0);
                if ui.add(egui::Button::new("OK").min_size(Vec2::new(75.0, 25.0))).clicked() {
                    done = Some(true);
                }
            });
        });
        if load {
            if let Some(path) = pick_image() {
                match load_as_jpeg(&path) {
                    Ok(j) => {
                        d.jpeg = Some(j);
                        d.tex = None;
                        if d.name.trim().is_empty() {
                            d.name = name_from_path(&path);
                        }
                    }
                    Err(e) => self.dialog = Some(Dialog::Message(format!("Die Grafik konnte nicht geladen werden: {e}"))),
                }
            }
        }
        match done {
            Some(true) => {
                self.checkpoint();
                if let Some(b) = self.project.bilder.get_mut(d.idx) {
                    b.name = d.name.trim().chars().take(20).collect();
                    if let Some(j) = d.jpeg.take() {
                        b.jpeg = j;
                        let key = format!("bld:{}", b.nr);
                        self.images.remove(&key);
                    }
                }
            }
            Some(false) => {}
            None if closed => {}
            None => self.zusatz.grafik = Some(d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_aus_datei_hoechstens_20_zeichen() {
        assert_eq!(name_from_path(std::path::Path::new("C:/x/Logo.png")), "Logo");
        assert_eq!(name_from_path(std::path::Path::new("ein_sehr_langer_dateiname_fuer_bild.jpg")).chars().count(), 20);
    }

    #[test]
    fn png_wird_jpeg_und_verwendung_erkannt() {
        let dir = std::env::temp_dir().join("ep_grafik_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.png");
        image::RgbImage::from_pixel(4, 3, image::Rgb([200, 10, 10])).save(&p).unwrap();
        let j = load_as_jpeg(&p).unwrap();
        assert!(j.starts_with(&[0xFF, 0xD8]));
        let mut pr = Project::new_default();
        pr.bilder.push(Bild { name: "t".into(), nr: 3, jpeg: j });
        assert!(!graphic_in_use(&pr, 3));
        pr.anlagen[0].sheets[0].objects.push(Obj { id: 99, pen: 0, fixed: false, group: None, raw: None, kind: Kind::Image { a: P::new(0.0, 0.0), b: P::new(1.0, 1.0), path: "bld:3".into(), mode: 2, frame: false } });
        assert!(graphic_in_use(&pr, 3));
    }
}
