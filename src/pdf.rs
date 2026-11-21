//! PDF-Ausgabe (für "PDF Export" und "Drucken"): Die Blätter werden mit denselben Zeichenroutinen wie auf
//! dem Bildschirm in eine unsichtbare Ebene gezeichnet; die entstandenen Formen werden als Vektor-PDF
//! (Linien, Kurven, Text in Times-Roman) geschrieben.

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
    use crate::canvas::{self, DrawCtx, SheetColors, Xf};
    let z = 10.0;
    let mut pages = vec![];
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
            let (sw, sh) = sheet.dims();
            let painter = egui::Painter::new(ctx.clone(), layer, Rect::from_min_size(origin, egui::Vec2::new(sw * z, sh * z)));
            let xf = Xf { origin, z };
            let dc = DrawCtx { painter: &painter, xf, lib, project, images, bmk: canvas::bmk_ctx(anlage, sheet), anlage: Some(anlage) };
            let colors = SheetColors { sheet: Color32::WHITE, grid: Color32::WHITE, print: true };
            canvas::draw_sheet_base(&dc, sheet, anlage, &colors, 0.0, false);
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
            pages.push(Page { shapes: collect(ctx, layer), origin, z, w_mm: sw, h_mm: sh, links });
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
                self.ellipse(e.center, e.radius.x, e.radius.y, e.stroke.width, e.stroke.color, fill);
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
            _ => {}
        }
    }
    fn text(&mut self, t: &egui::epaint::TextShape) {
        let g = &t.galley;
        let size_px = g.job.sections.first().map(|s| s.format.font_id.size).unwrap_or(12.0);
        let size_pt = size_px / self.page.z * PT_PER_MM;
        // Schnitt aus der Schriftfamilie: F1 Times-Roman, F2 Fett, F3 Kursiv, F4 Fett Kursiv
        let fname = match g.job.sections.first().map(|s| &s.format.font_id.family) {
            Some(egui::FontFamily::Name(n)) if &**n == "times_b" => "F2",
            Some(egui::FontFamily::Name(n)) if &**n == "times_i" => "F3",
            Some(egui::FontFamily::Name(n)) if &**n == "times_bi" => "F4",
            _ => "F1",
        };
        let color =t.override_text_color.or_else(|| g.job.sections.first().map(|s| s.format.color)).unwrap_or(t.fallback_color);
        let (sin, cos) = t.angle.sin_cos();
        for row in &g.rows {
            let Some(first) = row.row.glyphs.first() else { continue };
            let text: String = row.row.glyphs.iter().map(|gl| gl.chr).collect();
            if text.trim().is_empty() {
                continue;
            }
            let local = Pos2::new(row.pos.x + first.pos.x, row.pos.y + first.pos.y);
            let world = Pos2::new(t.pos.x + local.x * cos - local.y * sin, t.pos.y + local.x * sin + local.y * cos);
            // PDF dreht gegen den Uhrzeigersinn, der Bildschirm (y nach unten) im Uhrzeigersinn
            let (a, b) = (cos, -sin);
            let x = self.x(world);
            let y = self.y(world);
            self.s.push_str(&format!(
                "BT /{fname} {size_pt:.3} Tf {} rg {a:.5} {b:.5} {:.5} {a:.5} {x:.3} {y:.3} Tm ({}) Tj ET\n",
                Self::color(color),
                -b,
                pdf_string(&text)
            ));
        }
    }
}

/// Text in WinAnsi kodieren und für PDF maskieren.
fn pdf_string(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let b: u32 = match ch {
            '€' => 0x80,
            '„' => 0x84,
            '“' => 0x93,
            '”' => 0x94,
            '–' => 0x96,
            '—' => 0x97,
            c if (c as u32) < 256 => c as u32,
            _ => '?' as u32,
        };
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
    // 1 Katalog, 2 Seitenliste, 3 Schrift, danach je Seite: Seite + Inhalt
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + i * 2)).collect();
    objs.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), n));
    // Objekt 3: Schriftverzeichnis mit den vier Times-Schnitten
    objs.push(
        ["Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic"]
            .iter()
            .enumerate()
            .map(|(i, n)| format!("/F{} << /Type /Font /Subtype /Type1 /BaseFont /{n} /Encoding /WinAnsiEncoding >>", i + 1))
            .fold("<<".to_string(), |a, b| a + " " + &b)
            + " >>",
    );
    // Verknüpfungen als eigene Objekte hinter allen Seiten (Nummern ab 4 + 2n)
    let mut annot_nr = 4 + 2 * n;
    let mut annots: Vec<String> = vec![];
    for (i, pg) in pages.iter().enumerate() {
        let mut w = W { s: String::from("1 J 1 j\n"), page: pg };
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
        let content = w.s;
        let mut refs = vec![];
        for l in pg.links.iter().filter(|l| l.page < n) {
            let (x0, x1) = (l.mn.x * PT_PER_MM, l.mx.x * PT_PER_MM);
            let (y0, y1) = ((pg.h_mm - l.mx.y) * PT_PER_MM, (pg.h_mm - l.mn.y) * PT_PER_MM);
            annots.push(format!("<< /Type /Annot /Subtype /Link /Rect [{x0:.2} {y0:.2} {x1:.2} {y1:.2}] /Border [0 0 0] /Dest [{} 0 R /Fit] >>", 4 + l.page * 2));
            refs.push(format!("{annot_nr} 0 R"));
            annot_nr += 1;
        }
        let annots_entry = if refs.is_empty() { String::new() } else { format!(" /Annots [{}]", refs.join(" ")) };
        objs.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font 3 0 R >> /Contents {} 0 R{annots_entry} >>",
            pg.w_mm * PT_PER_MM,
            pg.h_mm * PT_PER_MM,
            5 + i * 2
        ));
        objs.push(format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content));
    }
    objs.extend(annots);
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
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
