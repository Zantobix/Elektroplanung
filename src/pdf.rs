//! PDF-Ausgabe (für "PDF Export" und "Drucken"): Die Blätter werden mit denselben Zeichenroutinen wie auf
//! dem Bildschirm in eine unsichtbare Ebene gezeichnet; die entstandenen Formen werden als Vektor-PDF
//! (Linien, Kurven, Text in den PDF-Standardschriften Times, Helvetica, Courier; `schriften::pdf_art`) geschrieben.
//! Texte mit eigener, installierter TrueType-Schrift werden in dieser Schrift ausgegeben (eingebettete Teilmenge,
//! `pdf_schriften`).

use eframe::egui::{self, epaint::ClippedShape, Color32, Pos2, Rect, Shape};

pub struct Page {
    pub shapes: Vec<ClippedShape>,
    /// Bildschirmlage des Blatt-Ursprungs (0/0 mm) beim Zeichnen
    pub origin: Pos2,
    /// Pixel je mm beim Zeichnen
    pub z: f32,
    pub w_mm: f32,
    pub h_mm: f32,
    /// Verknüpfungen (Verweise, Querverweise der Sammelschienen): Fläche in mm und Zielseite (Index in den Seiten)
    pub links: Vec<Link>,
    /// Pixel der Bildobjekte auf dieser Seite (`pdf_bilder`)
    pub bilder: crate::pdf_bilder::Pixel,
    /// Beim Drucken: Lage des Blattes auf dem Druckerpapier (`druck_seite`); sonst ist die Seite so groß wie das Blatt
    pub platz: Option<crate::druck_seite::Platz>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub mn: crate::model::P,
    pub mx: crate::model::P,
    pub page: usize,
}

/// Formen einer unsichtbaren Ebene einsammeln, die `draw` in diesem Bild gezeichnet hat.
pub fn collect(ctx: &egui::Context, layer: egui::LayerId) -> Vec<ClippedShape> {
    ctx.graphics(|g| g.get(layer).map(|pl| pl.all_entries().cloned().collect()).unwrap_or_default())
}

/// Alle Blätter eines Projekts mit den Bildschirm-Zeichenroutinen zeichnen und als PDF ausgeben.
/// Rückgabe: PDF-Bytes und Anzahl der Blätter.
pub fn render_project(
    ctx: &egui::Context,
    project: &crate::model::Project,
    lib: &[crate::symbols::SymDef],
    images: &std::collections::HashMap<String, egui::TextureHandle>,
) -> (Vec<u8>, usize) {
    render_sheets(ctx, project, lib, images, &|_| true)
}

/// Wie `render_project`, aber nur die Blätter, für die `take` wahr ist (Blattauswahl im Druckdialog).
pub fn render_sheets(
    ctx: &egui::Context,
    project: &crate::model::Project,
    lib: &[crate::symbols::SymDef],
    images: &std::collections::HashMap<String, egui::TextureHandle>,
    take: &dyn Fn(crate::model::SheetRef) -> bool,
) -> (Vec<u8>, usize) {
    render_sheets_platz(ctx, project, lib, images, take, &|_| None)
}

/// Wie `render_sheets`, je Blatt mit seiner Lage auf dem Druckerpapier (Drucken, `druck_seite::einrichten`).
pub fn render_sheets_platz(
    ctx: &egui::Context,
    project: &crate::model::Project,
    lib: &[crate::symbols::SymDef],
    images: &std::collections::HashMap<String, egui::TextureHandle>,
    take: &dyn Fn(crate::model::SheetRef) -> bool,
    platz: &dyn Fn(crate::model::SheetRef) -> Option<crate::druck_seite::Platz>,
) -> (Vec<u8>, usize) {
    use crate::canvas::{self, DrawCtx, SheetColors, Xf};
    let z = 10.0;
    let mut pages = vec![];
    // Bildobjekte: vorhandene Texturen, fehlende (z. B. Bilder anderer Blätter) laden; Pixel für das PDF
    let mut images = images.clone();
    let mut pixel: std::collections::HashMap<String, std::sync::Arc<egui::ColorImage>> = Default::default();
    let mut n = 0usize;
    // ausgegebene Blätter in Seitenreihenfolge (Ziele der Verknüpfungen)
    let refs: Vec<crate::model::SheetRef> = project
        .anlagen
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| (0..a.sheets.len()).map(move |si| crate::model::SheetRef { anlage: ai, sheet: si }))
        .filter(|r| take(*r))
        .collect();
    for (ai, anlage) in project.anlagen.iter().enumerate() {
        for (si, sheet) in anlage.sheets.iter().enumerate() {
            if !take(crate::model::SheetRef { anlage: ai, sheet: si }) {
                continue;
            }
            let layer = egui::LayerId::new(egui::Order::Background, egui::Id::unique(("pdf-page", n)));
            let origin = Pos2::new(-400000.0, -400000.0 + n as f32 * 4000.0);
            let mut bilder = crate::pdf_bilder::Pixel::new();
            for o in &sheet.objects {
                let crate::model::Kind::Image { path, .. } = &o.kind else { continue };
                if path.is_empty() {
                    continue;
                }
                if !pixel.contains_key(path) {
                    if let Some(ci) = crate::bildexport::bild_laden(project, path) {
                        images.entry(path.clone()).or_insert_with(|| ctx.load_texture(path.clone(), ci.clone(), Default::default()));
                        pixel.insert(path.clone(), std::sync::Arc::new(ci));
                    }
                }
                if let (Some(t), Some(ci)) = (images.get(path), pixel.get(path)) {
                    bilder.insert(t.id(), ci.clone());
                }
            }
            let (sw, sh) = sheet.dims();
            let painter = egui::Painter::new(ctx.clone(), layer, Rect::from_min_size(origin, egui::Vec2::new(sw * z, sh * z)));
            let xf = Xf { origin, z };
            let dc = DrawCtx { painter: &painter, xf, lib, project, images: &images, bmk: canvas::bmk_ctx(anlage, sheet), anlage: Some(anlage), bruecken: crate::bruecken::karte(&sheet.objects) };
            let colors = SheetColors { sheet: Color32::WHITE, grid: Color32::WHITE, print: true };
            canvas::draw_sheet_base(&dc, sheet, anlage, &colors, None);
            for o in &sheet.objects {
                canvas::draw_object(&dc, o, None);
            }
            for j in canvas::junctions(sheet) {
                painter.circle_filled(xf.s(j), 0.6 * z, Color32::BLACK);
            }
            let links = crate::verweise::flaechen(project, crate::model::SheetRef { anlage: ai, sheet: si }, lib)
                .into_iter()
                .filter_map(|f| {
                    let ziel = f.ziel.ok()?.blatt;
                    Some(Link { mn: f.mn, mx: f.mx, page: refs.iter().position(|r| *r == ziel)? })
                })
                .collect();
            pages.push(Page { shapes: collect(ctx, layer), origin, z, w_mm: sw, h_mm: sh, links, bilder, platz: platz(crate::model::SheetRef { anlage: ai, sheet: si }) });
            n += 1;
        }
    }
    (build(&pages), n)
}

/// PDF ohne Fenster erzeugen (für MCP-Server und Kommandozeile).
pub fn render_headless(project: &crate::model::Project, lib: &[crate::symbols::SymDef]) -> (Vec<u8>, usize) {
    let ctx = egui::Context::default();
    crate::setup_style(&ctx);
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(4000.0, 4000.0))), ..Default::default() };
    // erster Durchlauf lädt die Schriften
    let _ = ctx.run_ui(input(), |_| {});
    let images = Default::default();
    let mut out = (vec![], 0);
    let _ = ctx.run_ui(input(), |ui| out = render_project(ui.ctx(), project, lib, &images));
    out
}

const PT_PER_MM: f32 = 72.0 / 25.4;

struct W<'a> {
    s: String,
    page: &'a Page,
    /// eingebettete Schriften (Texte mit eigener Schrift, `pdf_schriften`)
    schriften: &'a mut crate::pdf_schriften::Schriften,
    /// eingebettete Bilder (`pdf_bilder`)
    bilder: &'a mut crate::pdf_bilder::Bilder,
}

impl W<'_> {
    fn x(&self, p: Pos2) -> f32 {
        (p.x - self.page.origin.x) / self.page.z * PT_PER_MM
    }
    fn y(&self, p: Pos2) -> f32 {
        (self.page.h_mm - (p.y - self.page.origin.y) / self.page.z) * PT_PER_MM
    }
    fn pt(&mut self, p: Pos2) -> String {
        format!("{:.3} {:.3}", self.x(p), self.y(p))
    }
    fn width(&self, px: f32) -> f32 {
        (px / self.page.z * PT_PER_MM).max(0.2)
    }
    fn color(c: Color32) -> String {
        format!("{:.3} {:.3} {:.3}", c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0)
    }
    fn stroke_poly(&mut self, pts: &[Pos2], closed: bool, width: f32, color: Color32, fill: Option<Color32>) {
        if pts.len() < 2 && fill.is_none() {
            return;
        }
        let mut s = String::new();
        s.push_str(&format!("{} RG {:.3} w ", Self::color(color), self.width(width)));
        if let Some(f) = fill {
            s.push_str(&format!("{} rg ", Self::color(f)));
        }
        for (i, p) in pts.iter().enumerate() {
            let q = self.pt(*p);
            s.push_str(&format!("{q} {} ", if i == 0 { "m" } else { "l" }));
        }
        let op = match (fill.is_some(), width > 0.0 && color.a() > 0) {
            (true, true) => "b",
            (true, false) => "f",
            (false, _) if closed => "s",
            _ => "S",
        };
        s.push_str(op);
        s.push('\n');
        self.s.push_str(&s);
    }
    fn ellipse(&mut self, c: Pos2, rx: f32, ry: f32, width: f32, color: Color32, fill: Option<Color32>) {
        let n = 48;
        let pts: Vec<Pos2> = (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                Pos2::new(c.x + rx * a.cos(), c.y + ry * a.sin())
            })
            .collect();
        self.stroke_poly(&pts, true, width, color, fill);
    }
    fn shape(&mut self, sh: &Shape) {
        match sh {
            Shape::Vec(v) => v.iter().for_each(|s| self.shape(s)),
            Shape::LineSegment { points, stroke } => {
                if stroke.width > 0.0 && stroke.color.a() > 0 {
                    self.stroke_poly(points, false, stroke.width, stroke.color, None)
                }
            }
            Shape::Path(p) => {
                let col = match &p.stroke.color {
                    egui::epaint::ColorMode::Solid(c) => *c,
                    _ => Color32::BLACK,
                };
                let fill = if p.fill.a() > 0 { Some(p.fill) } else { None };
                if (p.stroke.width > 0.0 && col.a() > 0) || fill.is_some() {
                    self.stroke_poly(&p.points, p.closed, if col.a() > 0 { p.stroke.width } else { 0.0 }, col, fill);
                }
            }
            Shape::Circle(c) => {
                let fill = if c.fill.a() > 0 { Some(c.fill) } else { None };
                if (c.stroke.width > 0.0 && c.stroke.color.a() > 0) || fill.is_some() {
                    self.ellipse(c.center, c.radius, c.radius, c.stroke.width, c.stroke.color, fill);
                }
            }
            Shape::Ellipse(e) => {
                let fill = if e.fill.a() > 0 { Some(e.fill) } else { None };
                // ohne Strich und Füllung nichts ausgeben (sonst zeichnete `s` eine Linie in Mindestbreite)
                if (e.stroke.width > 0.0 && e.stroke.color.a() > 0) || fill.is_some() {
                    let w = if e.stroke.color.a() > 0 { e.stroke.width } else { 0.0 };
                    self.ellipse(e.center, e.radius.x, e.radius.y, w, e.stroke.color, fill);
                }
            }
            Shape::Rect(r) => {
                let rr = r.rect;
                let pts = [rr.left_top(), rr.right_top(), rr.right_bottom(), rr.left_bottom()];
                // Weiße Flächen (Blatthintergrund) brauchen auf Papier keine Farbe
                let fill = if r.fill.a() > 0 && r.fill != Color32::WHITE { Some(r.fill) } else { None };
                if fill.is_some() || (r.stroke.width > 0.0 && r.stroke.color.a() > 0) {
                    self.stroke_poly(&pts, true, if r.stroke.color.a() > 0 { r.stroke.width } else { 0.0 }, r.stroke.color, fill);
                }
            }
            Shape::Text(t) => self.text(t),
            // Bildobjekt: Pixel als XObject an der Lage des Textur-Netzes
            Shape::Mesh(m) => {
                if let Some(ci) = self.page.bilder.get(&m.texture_id) {
                    let name = self.bilder.nutzen(m.texture_id, ci);
                    let r = m.calc_bounds();
                    let (x, y) = (self.x(r.left_bottom()), self.y(r.left_bottom()));
                    let (w, h) = (r.width() / self.page.z * PT_PER_MM, r.height() / self.page.z * PT_PER_MM);
                    self.s.push_str(&format!("q {w:.3} 0 0 {h:.3} {x:.3} {y:.3} cm /{name} Do Q\n"));
                }
            }
            _ => {}
        }
    }
    fn text(&mut self, t: &egui::epaint::TextShape) {
        let g = &t.galley;
        let size_px = g.job.sections.first().map(|s| s.format.font_id.size).unwrap_or(12.0);
        let size_pt = size_px / self.page.z * PT_PER_MM;
        // Schrift und Schnitt aus der Schriftfamilie: F1–F4 Times, F5–F8 Helvetica, F9–F12 Courier
        // (je Standard, Fett, Kursiv, Fett Kursiv)
        let (name, bold, italic) = g.job.sections.first().map(|s| crate::schriften::aus_familie(&s.format.font_id.family)).unwrap_or_default();
        let basis = match crate::schriften::pdf_art(&name) {
            crate::schriften::PdfArt::Times => 1,
            crate::schriften::PdfArt::Helvetica => 5,
            crate::schriften::PdfArt::Courier => 9,
        };
        let fname = format!("F{}", basis + bold as usize + 2 * italic as usize);
        // eigene Schrift eingebettet (sonst und für fehlende Zeichen: Standardschrift wie bisher)
        let eigen = self.schriften.schrift(&name, bold, italic);
        let color = t.override_text_color.or_else(|| g.job.sections.first().map(|s| s.format.color)).unwrap_or(t.fallback_color);
        let (sin, cos) = t.angle.sin_cos();
        // PDF dreht gegen den Uhrzeigersinn, der Bildschirm (y nach unten) im Uhrzeigersinn
        let (a, b) = (cos, -sin);
        for row in &g.rows {
            let glyphs = &row.row.glyphs;
            if glyphs.iter().all(|gl| gl.chr.is_whitespace()) {
                continue;
            }
            // Läufe gleicher Schrift: (Glyphe ab, Glyphe bis, eingebettet)
            let mut laeufe: Vec<(usize, usize, Option<usize>)> = vec![];
            for (i, gl) in glyphs.iter().enumerate() {
                // Zeichen außerhalb von WinAnsi (Ω, →, …): eingebettete Ersatzschrift statt „?“
                let k = eigen.filter(|&k| self.schriften.glyphe(k, gl.chr).is_some()).or_else(|| {
                    if winansi(gl.chr).is_none() && !gl.chr.is_whitespace() {
                        self.schriften.ersatz(gl.chr, bold, italic)
                    } else {
                        None
                    }
                });
                match laeufe.last_mut() {
                    Some(l) if l.2 == k => l.1 = i + 1,
                    _ => laeufe.push((i, i + 1, k)),
                }
            }
            for (von, bis, k) in laeufe {
                let first = &glyphs[von];
                let local = Pos2::new(row.pos.x + first.pos.x, row.pos.y + first.pos.y);
                let world = Pos2::new(t.pos.x + local.x * cos - local.y * sin, t.pos.y + local.x * sin + local.y * cos);
                let x = self.x(world);
                let y = self.y(world);
                let (font, text) = match k {
                    // Glyphennummern; Abstände nach den Glyphenlagen von egui (Unterschneidung), in 1/1000 Geviert
                    Some(k) => {
                        let mut s = String::from("[<");
                        let mut soll = 0.0;
                        for (j, gl) in glyphs[von..bis].iter().enumerate() {
                            let Some((gid, w)) = self.schriften.glyphe(k, gl.chr) else { continue };
                            let ist = (gl.pos.x - first.pos.x) / size_px.max(0.01) * 1000.0;
                            let d = soll - ist;
                            if j > 0 && d.abs() >= 0.5 {
                                s.push_str(&format!("> {d:.0} <"));
                                soll -= d.round();
                            }
                            s.push_str(&format!("{gid:04X}"));
                            soll += w;
                        }
                        s.push_str(">] TJ");
                        (crate::pdf_schriften::Schriften::res(k), s)
                    }
                    None => {
                        let text: String = glyphs[von..bis].iter().map(|gl| gl.chr).collect();
                        if text.trim().is_empty() {
                            continue;
                        }
                        (fname.clone(), format!("({}) Tj", pdf_string(&text)))
                    }
                };
                self.s.push_str(&format!("BT /{font} {size_pt:.3} Tf {} rg {a:.5} {b:.5} {:.5} {a:.5} {x:.3} {y:.3} Tm {text} ET\n", Self::color(color), -b));
            }
        }
    }
}

/// Zeichen in WinAnsi (Kodierung der PDF-Standardschriften); `None`, wenn es dort fehlt.
fn winansi(ch: char) -> Option<u8> {
    // 0x80–0x9F: Sonderbelegung von Windows-1252 (Lücken bei 0x81, 0x8D, 0x8F, 0x90, 0x9D)
    const C1: [char; 32] = [
        '€', '\0', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\0', 'Ž', '\0', '\0', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\0', 'ž', 'Ÿ',
    ];
    match ch as u32 {
        0x20..=0x7e | 0xa0..=0xff => Some(ch as u8),
        _ => C1.iter().position(|&c| c == ch && c != '\0').map(|i| 0x80 + i as u8),
    }
}

/// Text in WinAnsi kodieren und für PDF maskieren.
fn pdf_string(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let b = winansi(ch).unwrap_or(b'?') as u32;
        match b {
            0x28 => out.push_str("\\("),
            0x29 => out.push_str("\\)"),
            0x5c => out.push_str("\\\\"),
            32..=126 => out.push(char::from_u32(b).unwrap()),
            _ => out.push_str(&format!("\\{:03o}", b)),
        }
    }
    out
}

pub fn build(pages: &[Page]) -> Vec<u8> {
    let mut objs: Vec<String> = vec![];
    // 1 Katalog, 2 Seitenliste, 3 Schrift, danach je Seite: Seite + Inhalt, Verknüpfungen, eingebettete Schriften
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + i * 2)).collect();
    objs.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), n));
    // Objekt 3: Schriftverzeichnis mit den Standardschriften Times, Helvetica und Courier in je vier Schnitten
    // (die eingebetteten Schriften kommen nach den Seiten hinzu)
    let standard =
        [
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Times-BoldItalic",
            "Helvetica",
            "Helvetica-Bold",
            "Helvetica-Oblique",
            "Helvetica-BoldOblique",
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
        ]
            .iter()
            .enumerate()
            .map(|(i, n)| format!("/F{} << /Type /Font /Subtype /Type1 /BaseFont /{n} /Encoding /WinAnsiEncoding >>", i + 1))
            .fold("<<".to_string(), |a, b| a + " " + &b);
    objs.push(String::new());
    let mut schriften = crate::pdf_schriften::Schriften::default();
    let mut bilder = crate::pdf_bilder::Bilder::default();
    // Verknüpfungen als eigene Objekte hinter allen Seiten (Nummern ab 4 + 2n)
    let mut annot_nr = 4 + 2 * n;
    let mut annots: Vec<String> = vec![];
    for (i, pg) in pages.iter().enumerate() {
        let mut w = W { s: String::from("1 J 1 j\n"), page: pg, schriften: &mut schriften, bilder: &mut bilder };
        let page_rect = Rect::from_min_max(pg.origin, Pos2::new(pg.origin.x + pg.w_mm * pg.z, pg.origin.y + pg.h_mm * pg.z));
        for cs in &pg.shapes {
            let clip = cs.clip_rect.intersect(page_rect.expand(2.0));
            let clipped = clip.width() < page_rect.width() - 1.0 || clip.height() < page_rect.height() - 1.0;
            if clipped {
                let a = w.pt(clip.left_bottom());
                w.s.push_str(&format!(
                    "q {a} {:.3} {:.3} re W n\n",
                    clip.width() / pg.z * PT_PER_MM,
                    clip.height() / pg.z * PT_PER_MM
                ));
            }
            w.shape(&cs.shape);
            if clipped {
                w.s.push_str("Q\n");
            }
        }
        // Drucken: Blatt mit Maßstab und Versatz aufs Papier setzen, auf die bedruckbare Fläche beschnitten
        let (seite_w, seite_h, lage) = match &pg.platz {
            Some(p) => (p.papier_w, p.papier_h, (p.s, p.x, p.y)),
            None => (pg.w_mm, pg.h_mm, (1.0, 0.0, 0.0)),
        };
        let content = match &pg.platz {
            Some(p) => {
                let (x0, y0, x1, y1) = p.clip;
                format!(
                    "q {:.3} {:.3} {:.3} {:.3} re W n\n{} 0 0 {} {:.3} {:.3} cm\n{}Q\n",
                    x0 * PT_PER_MM,
                    (p.papier_h - y1) * PT_PER_MM,
                    (x1 - x0) * PT_PER_MM,
                    (y1 - y0) * PT_PER_MM,
                    p.s,
                    p.s,
                    p.x * PT_PER_MM,
                    (p.papier_h - p.y - p.s * pg.h_mm) * PT_PER_MM,
                    w.s
                )
            }
            None => w.s,
        };
        let mut refs = vec![];
        for l in pg.links.iter().filter(|l| l.page < n) {
            let (s, ox, oy) = lage;
            let (x0, x1) = ((ox + s * l.mn.x) * PT_PER_MM, (ox + s * l.mx.x) * PT_PER_MM);
            let (y0, y1) = ((seite_h - oy - s * l.mx.y) * PT_PER_MM, (seite_h - oy - s * l.mn.y) * PT_PER_MM);
            annots.push(format!("<< /Type /Annot /Subtype /Link /Rect [{x0:.2} {y0:.2} {x1:.2} {y1:.2}] /Border [0 0 0] /Dest [{} 0 R /Fit] >>", 4 + l.page * 2));
            refs.push(format!("{annot_nr} 0 R"));
            annot_nr += 1;
        }
        let annots_entry = if refs.is_empty() { String::new() } else { format!(" /Annots [{}]", refs.join(" ")) };
        objs.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font 3 0 R{{XOBJ}} >> /Contents {} 0 R{annots_entry} >>",
            seite_w * PT_PER_MM,
            seite_h * PT_PER_MM,
            5 + i * 2
        ));
        objs.push(format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content));
    }
    objs.extend(annots);
    let (font_objs, eintraege) = schriften.objekte(objs.len() + 1);
    objs[2] = eintraege.iter().fold(standard, |a, b| a + " " + b) + " >>";
    let (bild_objs, verz) = bilder.objekte(objs.len() + font_objs.len() + 1);
    // Bilder für alle Seiten in einem gemeinsamen XObject-Verzeichnis
    let xobj = if bilder.anzahl() > 0 { format!(" /XObject <<{verz} >>") } else { String::new() };
    for o in objs.iter_mut().skip(3).step_by(2).take(n) {
        *o = o.replace("{XOBJ}", &xobj);
    }
    let mut objs: Vec<Vec<u8>> = objs.into_iter().map(String::into_bytes).collect();
    objs.extend(font_objs);
    objs.extend(bild_objs);
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", o).as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", objs.len() + 1, xref).as_bytes());
    out
}

/// PDF drucken. Windows: Standardprogramm (Verb "print"), sonst öffnen. Linux/macOS: `lp` (CUPS), sonst
/// mit dem Standardprogramm öffnen (xdg-open/open).
#[cfg(not(windows))]
pub fn print_file(path: &std::path::Path) -> Result<(), String> {
    print_file_to(path, None)
}

#[cfg(not(windows))]
pub fn print_file_to(path: &std::path::Path, printer: Option<&str>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("lp");
    if let Some(p) = printer {
        cmd.args(["-d", p]);
    }
    if cmd.arg(path).status().map(|s| s.success()).unwrap_or(false) {
        return Ok(());
    }
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(opener).arg(path).status().map_err(|e| format!("Drucken nicht möglich (lp/{opener}): {e}"))?;
    Ok(())
}

/// Einstellungen eines Druckers öffnen (Windows: Druckeinstellungen über printui, sonst die CUPS-Weboberfläche).
pub fn printer_settings(name: Option<&str>) {
    #[cfg(windows)]
    {
        let mut args = vec!["printui.dll,PrintUIEntry".to_string(), "/e".to_string()];
        match name {
            Some(n) => args.extend(["/n".to_string(), n.to_string()]),
            None => args.push("/n".into()),
        }
        if name.is_none() {
            // ohne Namen: Standarddrucker
            args = vec!["printui.dll,PrintUIEntry".into(), "/e".into(), "/y".into()];
        }
        let _ = std::process::Command::new("rundll32").args(&args).spawn();
    }
    #[cfg(not(windows))]
    {
        let url = match name {
            Some(n) => format!("http://localhost:631/printers/{n}"),
            None => "http://localhost:631/printers/".into(),
        };
        let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(url).spawn();
    }
}

/// Name des Standarddruckers: Windows Win32_Printer.Default, sonst `lpstat -d` (CUPS, Ausgabe mit LC_ALL=C:
/// „system default destination: NAME“).
pub fn default_printer() -> Option<String> {
    #[cfg(windows)]
    let out = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Printer -Filter 'Default=true').Name"]).output();
    #[cfg(not(windows))]
    let out = std::process::Command::new("lpstat").arg("-d").env("LC_ALL", "C").output();
    let text = String::from_utf8_lossy(&out.ok()?.stdout).trim().to_string();
    if cfg!(windows) {
        return (!text.is_empty()).then_some(text);
    }
    // CUPS-Namen enthalten keine Leerzeichen; „no system default destination“ ergibt so keinen Namen
    let name = text.rsplit_once(": ").map(|(_, n)| n.trim().to_string())?;
    (!name.is_empty() && !name.contains(' ')).then_some(name)
}

/// Druckernamen des Systems: Windows Get-Printer, sonst `lpstat -e` (CUPS).
pub fn printers() -> Vec<String> {
    #[cfg(windows)]
    let out = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "Get-Printer | Select-Object -ExpandProperty Name"]).output();
    #[cfg(not(windows))]
    let out = std::process::Command::new("lpstat").arg("-e").output();
    out.map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()).unwrap_or_default()
}

/// PDF mit dem Standardprogramm von Windows drucken (Verb "print"); klappt das nicht, wird es geöffnet.
#[cfg(windows)]
pub fn print_file(path: &std::path::Path) -> Result<(), String> {
    let p = path.to_string_lossy().replace('\'', "''");
    let ok = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &format!("Start-Process -FilePath '{p}' -Verb Print")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        return Ok(());
    }
    std::process::Command::new("cmd").args(["/C", "start", "", &path.to_string_lossy()]).status().map_err(|e| e.to_string())?;
    Ok(())
}

/// PDF an einen bestimmten Drucker übergeben (Windows: Verb "PrintTo"); ohne Namen an den Standarddrucker.
#[cfg(windows)]
pub fn print_file_to(path: &std::path::Path, printer: Option<&str>) -> Result<(), String> {
    let Some(pr) = printer else { return print_file(path) };
    let p = path.to_string_lossy().replace('\'', "''");
    let q = pr.replace('\'', "''");
    let ok = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &format!("Start-Process -FilePath '{p}' -Verb PrintTo -ArgumentList '\"{q}\"'")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        print_file(path)
    }
}
