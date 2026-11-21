//! Bildexport (Datei › Als Bild exportieren…): das aktuelle Blatt oder alle Blätter der Anlage als PNG mit
//! 150/300/600 dpi. Das Original hat dafür nur den Menüeintrag „Bitmapexport“ ohne eigenes Einstellungsformular
//! (TPEXPORT ist die Projektliste der Exporteinstellung, TLISTEXPORT der Listenexport).
//!
//! Ablauf: Das Blatt wird mit den Zeichenroutinen der Zeichenfläche in eine unsichtbare Ebene gezeichnet
//! (Maßstab = Bildpunkte je mm), die Formen werden von epaint in Dreiecke zerlegt (mit Kantenglättung durch
//! weiche Ränder) und hier in Software gerastert – Text über die Glyphen des Schriftatlas.

use crate::app::App;
use crate::canvas::{self, DrawCtx, SheetColors, Xf};
use crate::model::*;
use crate::symbols::SymDef;
use eframe::egui::{self, epaint, Color32, ColorImage, Pos2, Rect, TextureId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Wählbare Auflösungen in dpi (Vorgabe 300)
pub const DPI: [u32; 3] = [150, 300, 600];

/// Einstellungen des Dialogs
pub struct BildDlg {
    /// alle Blätter der Anlage statt nur des aktuellen
    pub alle: bool,
    pub dpi_idx: usize,
    /// Kantenglättung
    pub glatt: bool,
}

impl Default for BildDlg {
    fn default() -> Self {
        BildDlg { alle: false, dpi_idx: 1, glatt: true }
    }
}

/// Bildgröße eines Blattes in Pixeln.
pub fn pixel_groesse(sheet: &Sheet, dpi: u32) -> (usize, usize) {
    let (w, h) = sheet.dims();
    let k = dpi as f32 / 25.4;
    ((w * k).round().max(1.0) as usize, (h * k).round().max(1.0) as usize)
}

/// Bild eines Bildobjekts laden (Projektbild "bld:<Nummer>" oder Datei), wie die Zeichenfläche.
fn bild_laden(project: &Project, path: &str) -> Option<ColorImage> {
    let img = match path.strip_prefix("bld:").and_then(|n| n.parse::<i32>().ok()) {
        Some(nr) => project.bilder.iter().find(|b| b.nr == nr).and_then(|b| image::load_from_memory(&b.jpeg).ok()),
        None => image::open(path).ok(),
    }?;
    let rgba = img.to_rgba8();
    Some(ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

/// Ein Blatt als Bild rastern. `images` = Texturen der Bildobjekte (fehlende werden geladen).
pub fn blatt_bild(
    ctx: &egui::Context,
    project: &Project,
    lib: &[SymDef],
    images: &mut HashMap<String, egui::TextureHandle>,
    r: SheetRef,
    dpi: u32,
    glatt: bool,
) -> Option<image::RgbImage> {
    let anlage = project.anlagen.get(r.anlage)?;
    let sheet = anlage.sheets.get(r.sheet)?;
    // Bildobjekte: Textur für die Zeichenroutine, Pixel für das Rastern
    let mut pixel: HashMap<TextureId, ColorImage> = HashMap::new();
    for o in &sheet.objects {
        let Kind::Image { path, .. } = &o.kind else { continue };
        if path.is_empty() {
            continue;
        }
        if let Some(ci) = bild_laden(project, path) {
            let tex = images.entry(path.clone()).or_insert_with(|| ctx.load_texture(path.clone(), ci.clone(), Default::default()));
            pixel.insert(tex.id(), ci);
        }
    }
    let z = dpi as f32 / 25.4;
    let (w, h) = pixel_groesse(sheet, dpi);
    let (sw, sh) = sheet.dims();
    // weit außerhalb des Fensters zeichnen (wie die PDF-Ausgabe), danach die Ebene wieder leeren
    let layer = egui::LayerId::new(egui::Order::Background, egui::Id::unique(("bild-export", r.anlage, r.sheet)));
    let origin = Pos2::new(-200000.0, -200000.0);
    let painter = egui::Painter::new(ctx.clone(), layer, Rect::from_min_size(origin, egui::Vec2::new(sw * z, sh * z)));
    let xf = Xf { origin, z };
    let dc = DrawCtx { painter: &painter, xf, lib, project, images, bmk: canvas::bmk_ctx(anlage, sheet), anlage: Some(anlage) };
    let colors = SheetColors { sheet: Color32::WHITE, grid: Color32::WHITE, print: true };
    canvas::draw_sheet_base(&dc, sheet, anlage, &colors, None);
    for o in &sheet.objects {
        canvas::draw_object(&dc, o, None);
    }
    for j in canvas::junctions(sheet) {
        painter.circle_filled(xf.s(j), 0.6 * z, Color32::BLACK);
    }
    let shapes = crate::pdf::collect(ctx, layer);
    ctx.graphics_mut(|g| *g.entry(layer) = Default::default());
    let (atlas, atlas_size) = ctx.fonts(|f| (f.image(), f.font_image_size()));
    let opts = epaint::TessellationOptions { feathering: glatt, feathering_size_in_pixels: 1.0, ..Default::default() };
    let prims = epaint::Tessellator::new(ctx.pixels_per_point(), opts, atlas_size, vec![]).tessellate_shapes(shapes);
    let mut tex: HashMap<TextureId, &ColorImage> = pixel.iter().map(|(k, v)| (*k, v)).collect();
    tex.insert(TextureId::default(), &atlas);
    Some(rastern(&prims, origin, w, h, &tex))
}

/// Texel bilinear abtasten (vormultipliziertes RGBA, 0…255).
fn abtasten(img: &ColorImage, u: f32, v: f32) -> [f32; 4] {
    let [iw, ih] = img.size;
    if iw == 0 || ih == 0 {
        return [255.0; 4];
    }
    let x = u * iw as f32 - 0.5;
    let y = v * ih as f32 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |xi: f32, yi: f32| {
        let c = img.pixels[(yi.clamp(0.0, ih as f32 - 1.0) as usize) * iw + xi.clamp(0.0, iw as f32 - 1.0) as usize];
        [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
    };
    let (a, b, c, d) = (at(x0, y0), at(x0 + 1.0, y0), at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0));
    let mut o = [0.0; 4];
    for k in 0..4 {
        o[k] = (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy;
    }
    o
}

/// Dreiecksnetze auf weißen Grund rastern. `origin` = Lage des Bildpunktes (0, 0) in Punkten (1 Punkt = 1 Pixel).
/// Netze mit unbekannter Textur werden übergangen.
pub fn rastern(prims: &[epaint::ClippedPrimitive], origin: Pos2, w: usize, h: usize, tex: &HashMap<TextureId, &ColorImage>) -> image::RgbImage {
    let mut buf = vec![255u8; w * h * 3];
    let edge = |a: (f32, f32), b: (f32, f32), p: (f32, f32)| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    for cp in prims {
        let epaint::Primitive::Mesh(m) = &cp.primitive else { continue };
        let Some(img) = tex.get(&m.texture_id) else { continue };
        let clip = cp.clip_rect;
        let cx0 = ((clip.min.x - origin.x).floor().max(0.0) as usize).min(w);
        let cy0 = ((clip.min.y - origin.y).floor().max(0.0) as usize).min(h);
        let cx1 = ((clip.max.x - origin.x).ceil().max(0.0) as usize).min(w);
        let cy1 = ((clip.max.y - origin.y).ceil().max(0.0) as usize).min(h);
        if cx0 >= cx1 || cy0 >= cy1 {
            continue;
        }
        for t in m.indices.chunks_exact(3) {
            let v = [&m.vertices[t[0] as usize], &m.vertices[t[1] as usize], &m.vertices[t[2] as usize]];
            let p: Vec<(f32, f32)> = v.iter().map(|q| (q.pos.x - origin.x, q.pos.y - origin.y)).collect();
            let area = edge(p[0], p[1], p[2]);
            if area.abs() < 1e-9 {
                continue;
            }
            let x0 = (p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor().max(cx0 as f32)) as usize;
            let x1 = (p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil().min(cx1 as f32)).max(0.0) as usize;
            let y0 = (p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor().max(cy0 as f32)) as usize;
            let y1 = (p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil().min(cy1 as f32)).max(0.0) as usize;
            if x0 >= x1 || y0 >= y1 {
                continue;
            }
            let col: Vec<[f32; 4]> = v.iter().map(|q| [q.color.r() as f32, q.color.g() as f32, q.color.b() as f32, q.color.a() as f32]).collect();
            // Weißer Texel des Atlas (alle UV = 0): Abtasten sparen
            let plain = v.iter().all(|q| q.uv == epaint::WHITE_UV) && m.texture_id == TextureId::default();
            for y in y0..y1 {
                let py = y as f32 + 0.5;
                for x in x0..x1 {
                    let pp = (x as f32 + 0.5, py);
                    let b0 = edge(p[1], p[2], pp) / area;
                    let b1 = edge(p[2], p[0], pp) / area;
                    let b2 = 1.0 - b0 - b1;
                    if b0 < 0.0 || b1 < 0.0 || b2 < 0.0 {
                        continue;
                    }
                    let mut c = [0.0f32; 4];
                    for k in 0..4 {
                        c[k] = col[0][k] * b0 + col[1][k] * b1 + col[2][k] * b2;
                    }
                    if !plain {
                        let u = v[0].uv.x * b0 + v[1].uv.x * b1 + v[2].uv.x * b2;
                        let vv = v[0].uv.y * b0 + v[1].uv.y * b1 + v[2].uv.y * b2;
                        let s = abtasten(img, u, vv);
                        for k in 0..4 {
                            c[k] = c[k] * s[k] / 255.0;
                        }
                    }
                    let a = (c[3] / 255.0).clamp(0.0, 1.0);
                    if a <= 0.0 && c[0] <= 0.0 && c[1] <= 0.0 && c[2] <= 0.0 {
                        continue;
                    }
                    let i = (y * w + x) * 3;
                    for k in 0..3 {
                        buf[i + k] = (c[k] + buf[i + k] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    image::RgbImage::from_raw(w as u32, h as u32, buf).expect("Puffergröße")
}

/// CRC-32 (PNG-Blockprüfsumme).
fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for b in data {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// PNG kodieren und die Auflösung (pHYs, Pixel je Meter) eintragen, damit das Bild in Originalgröße druckt.
pub fn png_bytes(img: &image::RgbImage, dpi: u32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut out = vec![];
    image::codecs::png::PngEncoder::new(&mut out).write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    // Signatur (8) + IHDR (4 Länge + 4 Typ + 13 Daten + 4 CRC) = 33
    if out.len() > 33 && &out[12..16] == b"IHDR" {
        let ppm = (dpi as f64 / 0.0254).round() as u32;
        let mut chunk = b"pHYs".to_vec();
        chunk.extend_from_slice(&ppm.to_be_bytes());
        chunk.extend_from_slice(&ppm.to_be_bytes());
        chunk.push(1);
        let mut ins = 9u32.to_be_bytes().to_vec();
        ins.extend_from_slice(&chunk);
        ins.extend_from_slice(&crc32(&chunk).to_be_bytes());
        out.splice(33..33, ins);
    }
    Ok(out)
}

/// Dateiname je Blatt beim Export aller Blätter: "<Name>_<Blattnummer>.png" (bei gleicher Nummer mit Index).
pub fn datei_je_blatt(base: &Path, nummern: &[u32]) -> Vec<PathBuf> {
    let stem = base.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Blatt".into());
    let dir = base.parent().map(Path::to_path_buf).unwrap_or_default();
    nummern
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let doppelt = nummern.iter().filter(|m| *m == n).count() > 1;
            let name = if doppelt { format!("{stem}_{n}_{}.png", i + 1) } else { format!("{stem}_{n}.png") };
            dir.join(name)
        })
        .collect()
}

/// Blatt ohne Fenster rastern (Tests, Kommandozeile).
pub fn render_headless(project: &Project, lib: &[SymDef], r: SheetRef, dpi: u32, glatt: bool) -> Option<image::RgbImage> {
    let ctx = egui::Context::default();
    crate::setup_style(&ctx);
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(1000.0, 800.0))), ..Default::default() };
    // erster Durchlauf lädt die Schriften
    let _ = ctx.run_ui(input(), |_| {});
    let mut images = HashMap::new();
    let mut out = None;
    let _ = ctx.run_ui(input(), |ui| out = blatt_bild(ui.ctx(), project, lib, &mut images, r, dpi, glatt));
    out
}

impl App {
    /// Datei › Als Bild exportieren…
    pub fn open_bild_export(&mut self) {
        if self.cur().is_none() {
            self.info = "Kein Blatt geöffnet".into();
            return;
        }
        self.suche.bild = Some(BildDlg::default());
    }

    pub fn bild_export_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.suche.bild.take() else { return };
        let Some(cur) = self.cur() else { return };
        let anlage_name = self.project.anlagen.get(cur.anlage).map(|a| a.name.clone()).unwrap_or_default();
        let groesse = self.sheet().map(|s| pixel_groesse(s, DPI[d.dpi_idx.min(2)])).unwrap_or((0, 0));
        let mut go = false;
        let mut abbrechen = false;
        let bold = crate::props::bold(13.0);
        let closed = crate::props::tool_window(ctx, "bildexport", "Als Bild exportieren", [340.0, 160.0], egui::Order::Middle, |ui| {
            ui.radio_value(&mut d.alle, false, egui::RichText::new("Aktuelles Blatt").font(bold.clone()));
            ui.radio_value(&mut d.alle, true, egui::RichText::new(format!("Alle Blätter der Anlage „{anlage_name}“ (je Blatt eine Datei)")).font(bold.clone()));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Auflösung").font(bold.clone()));
                egui::ComboBox::from_id_salt("bild_dpi").width(90.0).selected_text(format!("{} dpi", DPI[d.dpi_idx.min(2)])).show_ui(ui, |ui| {
                    for (i, v) in DPI.iter().enumerate() {
                        ui.selectable_value(&mut d.dpi_idx, i, format!("{v} dpi"));
                    }
                });
                ui.label(format!("{} × {} Pixel", groesse.0, groesse.1));
            });
            ui.checkbox(&mut d.glatt, egui::RichText::new("Kantenglättung").font(bold.clone()));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                go = ui.add_sized([110.0, 24.0], egui::Button::new("Exportieren…")).clicked();
                abbrechen = ui.add_sized([110.0, 24.0], egui::Button::new("Abbrechen")).clicked();
            });
        });
        if closed || abbrechen {
            return;
        }
        if !go {
            self.suche.bild = Some(d);
            return;
        }
        let sheets: Vec<usize> = if d.alle { (0..self.project.anlagen[cur.anlage].sheets.len()).collect() } else { vec![cur.sheet] };
        let vorschlag = self.sheet().map(|s| format!("Blatt {}.png", s.number)).unwrap_or_else(|| "Blatt.png".into());
        let Some(base) = rfd::FileDialog::new().add_filter("PNG-Bild", &["png"]).set_file_name(vorschlag).save_file() else {
            self.suche.bild = Some(d);
            return;
        };
        let files = if d.alle {
            let nummern: Vec<u32> = sheets.iter().map(|si| self.project.anlagen[cur.anlage].sheets[*si].number).collect();
            datei_je_blatt(&base, &nummern)
        } else {
            vec![base.clone()]
        };
        let dpi = DPI[d.dpi_idx.min(2)];
        let mut n = 0;
        for (si, f) in sheets.iter().zip(&files) {
            let r = SheetRef { anlage: cur.anlage, sheet: *si };
            let Some(img) = blatt_bild(ctx, &self.project, &self.lib, &mut self.images, r, dpi, d.glatt) else { continue };
            match png_bytes(&img, dpi).and_then(|b| std::fs::write(f, b).map_err(|e| e.to_string())) {
                Ok(()) => n += 1,
                Err(e) => {
                    self.dialog = Some(crate::app::Dialog::Message(format!("Bild konnte nicht geschrieben werden: {e}")));
                    return;
                }
            }
        }
        self.info = if n == 1 { format!("Bild gespeichert: {} ({dpi} dpi)", files[0].display()) } else { format!("{n} Bilder gespeichert in {} ({dpi} dpi)", base.parent().map(|p| p.display().to_string()).unwrap_or_default()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_mit_aufloesung() {
        let img = image::RgbImage::from_pixel(3, 2, image::Rgb([10, 20, 30]));
        let b = png_bytes(&img, 300).unwrap();
        assert_eq!(&b[37..41], b"pHYs");
        let back = image::load_from_memory(&b).unwrap().to_rgb8();
        assert_eq!(back.dimensions(), (3, 2));
        assert_eq!(back.get_pixel(1, 1).0, [10, 20, 30]);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn dateinamen_je_blatt() {
        let f = datei_je_blatt(Path::new("x/Plan.png"), &[1, 2, 2]);
        let n: Vec<String> = f.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(n, ["Plan_1.png", "Plan_2_2.png", "Plan_2_3.png"]);
    }

    #[test]
    fn blatt_mit_linie_und_text_rastern() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(Obj { id: 1, kind: Kind::Line { a: P::new(20.0, 100.0), b: P::new(120.0, 100.0) }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj { id: 2, kind: Kind::Text { pos: P::new(150.0, 60.0), text: "MMMM".into(), size: 8.0, align: Align::Left, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        let r = SheetRef { anlage: 0, sheet: 0 };
        let img = render_headless(&p, &[], r, 150, true).unwrap();
        assert_eq!(img.dimensions(), (1754, 1240));
        let k = 150.0 / 25.4;
        let dark = |x0: f32, y0: f32, x1: f32, y1: f32| {
            let mut n = 0;
            for y in (y0 * k) as u32..(y1 * k) as u32 {
                for x in (x0 * k) as u32..(x1 * k) as u32 {
                    if img.get_pixel(x, y).0[0] < 128 {
                        n += 1;
                    }
                }
            }
            n
        };
        // Linie (0,25 mm ≈ 1,5 Pixel) bei y = 100 mm
        assert!(dark(30.0, 99.0, 110.0, 101.0) > 100, "Linie fehlt");
        // Text rechts daneben (Glyphen aus dem Schriftatlas)
        assert!(dark(150.0, 54.0, 190.0, 66.0) > 200, "Text fehlt");
        // leere Fläche bleibt weiß
        assert_eq!(dark(30.0, 120.0, 110.0, 140.0), 0);
        // Kantenglättung: Zwischentöne an der Linie
        let grau = (0..img.height()).map(|y| img.get_pixel((60.0 * k) as u32, y).0[0]).filter(|v| *v > 20 && *v < 235).count();
        assert!(grau > 0, "keine Zwischentöne");
    }

    #[test]
    fn dreieck_rastern() {
        let mut m = epaint::Mesh::default();
        m.colored_vertex(Pos2::new(0.0, 0.0), Color32::BLACK);
        m.colored_vertex(Pos2::new(10.0, 0.0), Color32::BLACK);
        m.colored_vertex(Pos2::new(0.0, 10.0), Color32::BLACK);
        m.add_triangle(0, 1, 2);
        let prims = vec![epaint::ClippedPrimitive { clip_rect: Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(10.0)), primitive: epaint::Primitive::Mesh(m) }];
        let atlas = ColorImage::filled([2, 2], Color32::WHITE);
        let tex: HashMap<TextureId, &ColorImage> = [(TextureId::default(), &atlas)].into_iter().collect();
        let img = rastern(&prims, Pos2::ZERO, 10, 10, &tex);
        assert_eq!(img.get_pixel(1, 1).0, [0, 0, 0]);
        assert_eq!(img.get_pixel(9, 9).0, [255, 255, 255]);
    }
}
