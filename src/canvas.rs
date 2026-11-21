//! Zeichnen des Blattes und aller Objekte (Bildschirm-Transformation mm → Pixel).

use crate::model::*;
use crate::symbols::{self, Prim, SymDef};
use crate::titleblock::{self, MacroValues, TbOwned};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy)]
pub struct Xf {
    pub origin: Pos2,
    pub z: f32,
}

impl Xf {
    pub fn s(&self, p: P) -> Pos2 {
        Pos2::new(self.origin.x + p.x * self.z, self.origin.y + p.y * self.z)
    }
    pub fn m(&self, q: Pos2) -> P {
        P::new((q.x - self.origin.x) / self.z, (q.y - self.origin.y) / self.z)
    }
}

pub fn times(px: f32) -> FontId {
    FontId::new(px.max(1.0), FontFamily::Name("times".into()))
}

/// Times in den Schnitten des Originals (Standard, Fett, Kursiv, Fett Kursiv).
pub fn times_styled(px: f32, bold: bool, italic: bool) -> FontId {
    let fam = match (bold, italic) {
        (false, false) => "times",
        (true, false) => "times_b",
        (false, true) => "times_i",
        (true, true) => "times_bi",
    };
    FontId::new(px.max(1.0), FontFamily::Name(fam.into()))
}

/// Text mit Einstellungen wie im Original: Textbox, Ausrichtung je Zeile, Rahmen, Schnitt, Farbe, Drehung.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_box(p: &egui::Painter, xf: Xf, pos: P, text: &str, size_mm: f32, align: Align, vertical: bool, st: &TextStyle, color: Color32, frame_stroke: Stroke) {
    let z = xf.z;
    let font = crate::schriften::font_id(&st.font, size_mm * z, st.bold, st.italic);
    let Some((mn, mx)) = st.rect else {
        let galley = p.layout_no_wrap(text.to_string(), font, color);
        draw_galley(p, xf.s(pos), galley, align, vertical, color);
        return;
    };
    if st.frame {
        let r = Rect::from_min_max(xf.s(mn), xf.s(mx));
        p.rect_stroke(r, 0.0, frame_stroke, egui::StrokeKind::Middle);
    }
    let lines: Vec<String> = if st.wrap {
        // Umbruch: Zeilen an Leerzeichen auf die Boxbreite (hochkant: Boxhöhe) verteilen, 0,5 mm Rand je Seite
        let breite = (if vertical { mx.y - mn.y } else { mx.x - mn.x } - 1.0).max(0.5) * z;
        let mass = |s: &str| p.layout_no_wrap(s.to_string(), font.clone(), color).size().x;
        crate::umbruch::text_umbrechen(text, breite, &mass)
    } else if text.is_empty() {
        vec![String::new()]
    } else {
        text.split('\n').map(String::from).collect()
    };
    let gal: Vec<_> = lines.iter().map(|l| p.layout_no_wrap(l.to_string(), font.clone(), color)).collect();
    let lh = gal.first().map(|g| g.size().y).unwrap_or(size_mm * z);
    let total = lh * gal.len() as f32;
    let (bmn, bmx) = (xf.s(mn), xf.s(mx));
    for (i, g) in gal.into_iter().enumerate() {
        let w = g.size().x;
        let underline = st.underline;
        if vertical {
            // hochkant (von unten nach oben): Zeilen nebeneinander, Ausrichtung entlang der Boxhöhe
            let cx = (bmn.x + bmx.x) / 2.0 - total / 2.0 + i as f32 * lh;
            let y = match align {
                Align::Left => bmx.y,
                Align::Center => (bmn.y + bmx.y) / 2.0 + w / 2.0,
                Align::Right => bmn.y + w,
            };
            p.add(egui::epaint::TextShape::new(Pos2::new(cx, y), g, color).with_angle(-std::f32::consts::FRAC_PI_2));
        } else {
            let x = match align {
                Align::Left => bmn.x,
                Align::Center => (bmn.x + bmx.x) / 2.0 - w / 2.0,
                Align::Right => bmx.x - w,
            };
            let y = (bmn.y + bmx.y) / 2.0 - total / 2.0 + i as f32 * lh;
            if underline && w > 0.0 {
                let uy = y + lh * 0.92;
                p.line_segment([Pos2::new(x, uy), Pos2::new(x + w, uy)], Stroke::new((0.06 * size_mm * z).max(1.0), color));
            }
            p.galley(Pos2::new(x, y), g, color);
        }
    }
}

fn draw_galley(p: &egui::Painter, at: Pos2, galley: std::sync::Arc<egui::Galley>, align: Align, vertical: bool, color: Color32) {
    let sz = galley.size();
    let dx = match align {
        Align::Left => 0.0,
        Align::Center => -sz.x / 2.0,
        Align::Right => -sz.x,
    };
    if vertical {
        let pos = Pos2::new(at.x - sz.y / 2.0, at.y - dx);
        p.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(-std::f32::consts::FRAC_PI_2));
    } else {
        p.galley(Pos2::new(at.x + dx, at.y - sz.y / 2.0), galley, color);
    }
}

pub fn c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Linie mit Stiftart (0 durchgezogen, 1 gestrichelt, 2 Strich/Punkt, 3 gepunktet).
pub fn styled_line(painter: &egui::Painter, pts: &[Pos2], stroke: Stroke, style: u8, z: f32) {
    if pts.len() < 2 {
        return;
    }
    let pattern: &[f32] = match style {
        1 => &[3.0, 1.5],
        2 => &[4.0, 1.0, 0.6, 1.0],
        3 => &[0.6, 1.0],
        _ => {
            painter.add(Shape::line(pts.to_vec(), stroke));
            return;
        }
    };
    let pat: Vec<f32> = pattern.iter().map(|v| (v * z).max(1.5)).collect();
    let mut idx = 0usize;
    let mut left = pat[0];
    let mut on = true;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = a.distance(b);
        if len < 1e-3 {
            continue;
        }
        let dir = (b - a) / len;
        let mut t = 0.0;
        while t < len {
            let step = left.min(len - t);
            if on {
                painter.line_segment([a + dir * t, a + dir * (t + step)], stroke);
            }
            t += step;
            left -= step;
            if left <= 1e-4 {
                idx = (idx + 1) % pat.len();
                left = pat[idx];
                on = idx % 2 == 0;
            }
        }
    }
}

pub fn bezier(a: P, c1: P, c2: P, b: P, n: usize) -> Vec<P> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            P::new(
                u * u * u * a.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * b.x,
                u * u * u * a.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * b.y,
            )
        })
        .collect()
}

pub fn ellipse(a: P, b: P, n: usize) -> Vec<P> {
    let c = P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (rx, ry) = (((b.x - a.x) / 2.0).abs(), ((b.y - a.y) / 2.0).abs());
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            P::new(c.x + rx * t.cos(), c.y + ry * t.sin())
        })
        .collect()
}

pub fn norm_rect(a: P, b: P) -> (P, P) {
    (P::new(a.x.min(b.x), a.y.min(b.y)), P::new(a.x.max(b.x), a.y.max(b.y)))
}

pub fn triangle_pts(a: P, b: P) -> [P; 3] {
    let (mn, mx) = norm_rect(a, b);
    // Spitze in Zugrichtung (a liegt an der Spitzenseite)
    if a.y <= b.y {
        [P::new((mn.x + mx.x) / 2.0, mn.y), P::new(mx.x, mx.y), P::new(mn.x, mx.y)]
    } else {
        [P::new((mn.x + mx.x) / 2.0, mx.y), P::new(mx.x, mn.y), P::new(mn.x, mn.y)]
    }
}

/// Weltkoordinaten der Anschlüsse eines Objekts.
pub fn obj_pins(o: &Obj, lib: &[SymDef]) -> Vec<P> {
    match &o.kind {
        Kind::Symbol { key, pos, rot, mirror_h, mirror_v, scale, option, flags, .. } => symbols::find(lib, key)
            .map(|d| {
                d.pins
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| symbols::visible(*option, d.pin_vis.get(*i)) && symbols::flag_visible(*flags, d.pin_info.get(*i).map(|x| &x.1)))
                    .map(|(_, q)| sym_xf(*q, *pos, *rot, *mirror_h, *mirror_v, *scale))
                    .collect()
            })
            .unwrap_or_default(),
        // Verweis: Anschluss an der Pfeilspitze (im Original lassen sich Leitungen anschließen)
        Kind::Terminal { pos, .. } | Kind::Verweis { pos, .. } => vec![*pos],
        Kind::Wire { pts, .. } => {
            let mut v = vec![];
            if let Some(f) = pts.first() {
                v.push(*f);
            }
            if pts.len() > 1 {
                v.push(*pts.last().unwrap());
            }
            v
        }
        _ => vec![],
    }
}

/// Umgebendes Rechteck in mm.
pub fn obj_bounds(o: &Obj, lib: &[SymDef]) -> (P, P) {
    match &o.kind {
        Kind::Line { a, b } | Kind::Rect { a, b } | Kind::Triangle { a, b } | Kind::Circle { a, b } => norm_rect(*a, *b),
        Kind::TextBox { a, b, .. } | Kind::Table { a, b, .. } | Kind::Image { a, b, .. } => norm_rect(*a, *b),
        Kind::Curve { a, c1, c2, b } => {
            let pts = bezier(*a, *c1, *c2, *b, 24);
            pts_bounds(&pts)
        }
        Kind::Wire { pts, .. } => pts_bounds(pts),
        Kind::Busbar { a, x2, .. } => (P::new(a.x, a.y - 0.6), P::new(*x2, a.y + 0.6)),
        // so groß, wie der Text gezeichnet wird (Schriftmaße der Times, Textbox), siehe textmass.rs
        Kind::Text { pos, text, size, align, vertical, style } => crate::textmass::text_umriss(*pos, text, *size, *align, *vertical, style),
        Kind::Poly { pts } => pts_bounds(pts),
        Kind::Symbol { pos, ext: SymExt { bounds: Some((a, b)), .. }, .. } => (pos.add(*a), pos.add(*b)),
        Kind::Symbol { key, pos, rot, mirror_h, mirror_v, scale, option, .. } => match symbols::find(lib, key) {
            Some(d) => {
                let (mn, mx) = symbols::bounds_opt(d, *option);
                let c = [P::new(mn.x, mn.y), P::new(mx.x, mn.y), P::new(mx.x, mx.y), P::new(mn.x, mx.y)];
                let pts: Vec<P> = c.iter().map(|q| sym_xf(*q, *pos, *rot, *mirror_h, *mirror_v, *scale)).collect();
                pts_bounds(&pts)
            }
            None => (P::new(pos.x - 3.0, pos.y - 3.0), P::new(pos.x + 3.0, pos.y + 3.0)),
        },
        Kind::Terminal { pos, .. } => (P::new(pos.x - 1.2, pos.y - 1.2), P::new(pos.x + 1.2, pos.y + 1.2)),
        Kind::Spiegel { pos, rows, .. } => crate::kontaktspiegel::bounds(*pos, rows),
        Kind::Verweis { pos, text_pos, text, dir, vertical, spitze, .. } => {
            // Pfeil (in jeder Drehung ganz) und – falls vorhanden – Text
            let (ta, tb) = pts_bounds(&verweis_triangle(*pos, *dir, *spitze));
            let (ta, tb) = (P::new(ta.x.min(pos.x - 0.5) - 0.3, ta.y.min(pos.y - 0.5) - 0.3), P::new(tb.x.max(pos.x + 0.5) + 0.3, tb.y.max(pos.y + 0.5) + 0.3));
            if text.is_empty() {
                return (ta, tb);
            }
            let t = Obj { id: 0, kind: Kind::Text { pos: verweis_textmitte(*text_pos, *vertical), text: text.clone(), size: VERWEIS_TEXT, align: Align::Center, vertical: *vertical, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
            let (a, b) = obj_bounds(&t, lib);
            (P::new(a.x.min(ta.x), a.y.min(ta.y)), P::new(b.x.max(tb.x), b.y.max(tb.y)))
        }
    }
}

/// Schrifthöhe der Verweistexte in mm (gemessen im Original-Ausdruck, so groß wie BMK)
pub const VERWEIS_TEXT: f32 = 2.5;

/// Pfeil eines Verweises: gefülltes Dreieck (2,4 mm lang, 1,7 mm breit, vermessen im Original-Ausdruck), Spitze in
/// Richtung `dir` (Grad, 0 = rechts, 90 = oben); mit `spitze` liegt die Spitze an `pos`, sonst die Mitte der
/// Grundseite (siehe `crate::epr::verweis`).
pub fn verweis_triangle(pos: P, dir: u16, spitze: bool) -> [P; 3] {
    let a = (dir as f32).to_radians();
    let (ux, uy) = (a.cos(), -a.sin());
    let (len, half) = (2.4, 0.85);
    let (tip, base) = if spitze { (pos, P::new(pos.x - ux * len, pos.y - uy * len)) } else { (P::new(pos.x + ux * len, pos.y + uy * len), pos) };
    [tip, P::new(base.x - uy * half, base.y + ux * half), P::new(base.x + uy * half, base.y - ux * half)]
}

/// Textmitte eines Verweises: Der Textversatz zeigt im Original auf die Mitte der Textoberkante (Ausdruck W2,
/// Seiten 1–4: waagerechte Texte beginnen 0,4 mm unter dem Versatz, hochkant gelesene 0,6 mm rechts davon).
pub fn verweis_textmitte(text_pos: P, vertical: bool) -> P {
    let d = 0.6 * VERWEIS_TEXT;
    if vertical {
        P::new(text_pos.x + d, text_pos.y)
    } else {
        P::new(text_pos.x, text_pos.y + d)
    }
}

pub fn pts_bounds(pts: &[P]) -> (P, P) {
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    for q in pts {
        mn.x = mn.x.min(q.x);
        mn.y = mn.y.min(q.y);
        mx.x = mx.x.max(q.x);
        mx.y = mx.y.max(q.y);
    }
    if pts.is_empty() {
        (P::default(), P::default())
    } else {
        (mn, mx)
    }
}

/// Trefferprüfung in mm mit Toleranz.
pub fn hit(o: &Obj, p: P, tol: f32, lib: &[SymDef]) -> bool {
    let near_poly = |pts: &[P]| pts.windows(2).any(|w| seg_dist(p, w[0], w[1]) <= tol);
    match &o.kind {
        Kind::Line { a, b } => seg_dist(p, *a, *b) <= tol,
        Kind::Curve { a, c1, c2, b } => near_poly(&bezier(*a, *c1, *c2, *b, 32)),
        Kind::Wire { pts, .. } => near_poly(pts),
        Kind::Poly { pts } => {
            let mut v = pts.clone();
            if let Some(f) = pts.first() {
                v.push(*f);
            }
            near_poly(&v)
        }
        Kind::Terminal { pos, .. } => p.dist(*pos) <= 1.2 + tol,
        _ => {
            let (mn, mx) = obj_bounds(o, lib);
            p.x >= mn.x - tol && p.x <= mx.x + tol && p.y >= mn.y - tol && p.y <= mx.y + tol
        }
    }
}

pub struct SheetColors {
    pub sheet: Color32,
    pub grid: Color32,
    /// Ausgabe auf Papier/PDF: ohne Blattumriss (das Original druckt nur den Rahmen)
    pub print: bool,
}

pub struct DrawCtx<'a> {
    pub painter: &'a egui::Painter,
    pub xf: Xf,
    pub lib: &'a [SymDef],
    pub project: &'a Project,
    pub images: &'a std::collections::HashMap<String, egui::TextureHandle>,
    /// Blattangaben zum Auflösen von BMK-Vorlagen
    pub bmk: symbols::BmkCtx,
    /// Anlage des Blattes (Einstellungen der Sammelschienen)
    pub anlage: Option<&'a Anlage>,
}

/// Blattangaben für `DrawCtx::bmk`.
pub fn bmk_ctx(anlage: &Anlage, sheet: &Sheet) -> symbols::BmkCtx {
    symbols::BmkCtx { sheet: sheet.number, ort: sheet.ort.clone(), anlage: if anlage.set.kurz.is_empty() { anlage.name.clone() } else { anlage.set.kurz.clone() } }
}

pub fn draw_sheet_base(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage, colors: &SheetColors, raster: f32, grid_on: bool) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (sw, sh) = sheet.dims();
    let sheet_rect = Rect::from_min_max(xf.s(P::new(0.0, 0.0)), xf.s(P::new(sw, sh)));
    p.rect_filled(sheet_rect, 0.0, colors.sheet);

    // Raster
    if grid_on && raster > 0.0 {
        let mut step = raster;
        while step * z < 6.0 {
            step *= 2.0;
        }
        let gs = Stroke::new(1.0, colors.grid);
        let mut x = GRID_ORIGIN;
        while x <= sw {
            let sx = xf.s(P::new(x, 0.0)).x.round() + 0.5;
            p.line_segment([Pos2::new(sx, sheet_rect.top()), Pos2::new(sx, sheet_rect.bottom())], gs);
            x += step;
        }
        let mut y = GRID_ORIGIN;
        while y <= sh {
            let sy = xf.s(P::new(0.0, y)).y.round() + 0.5;
            p.line_segment([Pos2::new(sheet_rect.left(), sy), Pos2::new(sheet_rect.right(), sy)], gs);
            y += step;
        }
    }

    let black = Stroke::new(1.0_f32.max(0.25 * z), Color32::BLACK);
    let thin = Stroke::new(1.0, Color32::BLACK);
    if !colors.print {
        p.rect_stroke(sheet_rect, 0.0, thin, egui::StrokeKind::Outside);
    }

    // Rahmen
    let (fx0, fy0, fx1, fy1) = sheet.frame();
    let fr = Rect::from_min_max(xf.s(P::new(fx0, fy0)), xf.s(P::new(fx1, fy1)));
    // Deckblatt: ohne Strompfade und Schriftfeld, Rahmen nach Zusatz „mit/ohne Rahmen“ (Anleitung 4.2.1)
    if sheet.blattart() == 0 {
        if sheet.zusatz == 0 {
            p.rect_stroke(fr, 0.0, black, egui::StrokeKind::Middle);
        }
        return;
    }
    p.rect_stroke(fr, 0.0, black, egui::StrokeKind::Middle);

    let th = dc.project.title_h();
    // Strompfad-Spaltenleiste (Klemmplan und Sicherungsübersicht haben keine, wie im Original-Ausdruck; ebenso
    // die übrigen Listenblätter Inhaltsverzeichnis, Querverweise und Artikelliste)
    if matches!(sheet.blattart(), 1 | 2 | 4 | 5 | 6) {
        draw_title_block(dc, sheet, anlage, fx0, fx1, fy1 - th, fy1);
        draw_generated(dc, sheet, anlage);
        return;
    }
    let cols = sheet.columns.max(1);
    let cw = (fx1 - fx0) / cols as f32;
    p.line_segment([xf.s(P::new(fx0, fy0 + COLUMN_BAR_H)), xf.s(P::new(fx1, fy0 + COLUMN_BAR_H))], black);
    for i in 0..cols {
        let x = fx0 + i as f32 * cw;
        if i > 0 {
            p.line_segment([xf.s(P::new(x, fy0)), xf.s(P::new(x, fy0 + COLUMN_BAR_H))], black);
        }
        p.text(
            xf.s(P::new(x + cw / 2.0, fy0 + COLUMN_BAR_H / 2.0 + 0.2)),
            Align2::CENTER_CENTER,
            sheet.col_label(i as usize),
            times(3.6 * z),
            Color32::BLACK,
        );
    }

    // Begrenzungslinien zwischen den Strompfaden (Pfadbeschreibungen)
    if sheet.pfad_linien {
        let gs = Stroke::new(1.0, Color32::from_gray(90));
        for i in 1..cols {
            let x = fx0 + i as f32 * cw;
            p.add(Shape::dashed_line(&[xf.s(P::new(x, fy0 + COLUMN_BAR_H)), xf.s(P::new(x, fy1 - th))], gs, 4.0, 4.0));
        }
    }
    // Schriftfeld
    draw_title_block(dc, sheet, anlage, fx0, fx1, fy1 - th, fy1);
}

/// Umriss (links oben, rechts unten) einer BMK bzw. eines Wertes in mm: `at` = Titel X/Y (VSY Feld 10/11
/// relativ zur Lage), `w`/`h` = Textbreite/-höhe, Ausrichtung 0 Links / 1 Mittig / 2 Rechts (Feld 12 Hi-Word).
/// Waagerecht beginnt der Text oben an `at`; hochkant (Feld 12 Bit 24) liest er von unten nach oben und
/// beginnt bei Links unten an `at` (Original-Ausdruck Seite 1: "-M2.1" unter dem gedrehten Motor).
pub fn title_rect(at: P, w: f32, h: f32, align: u8, vert: bool) -> (P, P) {
    let shift = match align {
        1 => w / 2.0,
        2 => w,
        _ => 0.0,
    };
    if vert {
        let bottom = at.y + shift;
        (P::new(at.x, bottom - w), P::new(at.x + h, bottom))
    } else {
        (P::new(at.x - shift, at.y), P::new(at.x - shift + w, at.y + h))
    }
}

/// Natürliche Sortierung von Kennzeichen ("-F2" vor "-F10").
pub fn natural_key(s: &str) -> Vec<(String, u64)> {
    let mut out = vec![];
    let mut txt = String::new();
    let mut num = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((std::mem::take(&mut txt), num.parse().unwrap_or(0)));
                num.clear();
            }
            txt.push(c);
        }
    }
    out.push((txt, num.parse().unwrap_or(0)));
    out
}

/// Zeilen der Sicherungsübersicht einer Anlage: alle Symbole mit Freigabe "Sicherungsübersicht"
/// (VSY Feld 20 Bit 24), sortiert nach Kennzeichen – Spalten Sicherung, Wert, Funktion (Original-Ausdruck Seite 5).
pub fn fuse_rows(anlage: &Anlage) -> Vec<[String; 3]> {
    let mut rows: Vec<[String; 3]> = vec![];
    for s in &anlage.sheets {
        let ctx = bmk_ctx(anlage, s);
        for o in &s.objects {
            if let Kind::Symbol { bmk, value, ext, pos, .. } = &o.kind {
                let bmk = symbols::display_bmk(bmk, ext.counter, &ctx);
                if ext.sicherung && !bmk.is_empty() && !rows.iter().any(|r| r[0] == bmk) {
                    // Funktion des Symbols, sonst die Pfadbeschreibung seines Strompfades (Anleitung 4.2.5)
                    let f = if ext.funktion.trim().is_empty() { crate::klemmplan::pfad_text(s, pos.x) } else { ext.funktion.clone() };
                    rows.push([bmk, value.clone(), f]);
                }
            }
        }
    }
    rows.sort_by_key(|r| natural_key(&r[0]));
    rows
}

/// Inhalt berechneter Blätter: Sicherungsübersicht (Blattart 5) und Klemmplan (Blattart 4). Das Original speichert
/// sie nicht in der Datei, sondern erzeugt sie beim Anzeigen und Drucken aus den Symbolen bzw. Klemmleisten.
pub fn draw_generated(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (fx0, fy0, fx1, fy1) = sheet.frame();
    let stroke = Stroke::new((0.18 * z).max(1.0), Color32::BLACK);
    let text = |at: P, t: &str, em: f32, a: Align2| {
        p.text(xf.s(at), a, t, times(em * z), Color32::BLACK);
    };
    if sheet.art == 5 {
        // Maße aus dem Original-Ausdruck (Seite 5): Zeilen 4,95 mm, Spalten 24,5 / 14,5 mm / Rest
        let (x0, x1) = (fx0 + 4.5, fx1 - 7.5);
        let (y0, y1) = (fy0 + 9.5, fy1 - dc.project.title_h() - 3.0);
        let rh = 4.95;
        let n = ((y1 - y0) / rh).floor().max(2.0) as usize;
        let widths = [24.5, 14.5, (x1 - x0 - 39.0).max(10.0)];
        draw_grid(p, xf, P::new(x0, y0), P::new(x0 + widths.iter().sum::<f32>(), y0 + n as f32 * rh), &widths, &vec![rh; n], stroke);
        // Folgeblätter (direkt folgende Sicherungsübersichten) setzen die Liste fort, Kopfzeile auf jedem Blatt
        let mut rows = vec![["Sicherung".to_string(), "Wert".to_string(), "Funktion".to_string()]];
        rows.extend(fuse_rows(anlage).into_iter().skip(crate::klemmplan::folge_index(anlage, sheet) * n.saturating_sub(1)));
        for (r, row) in rows.iter().take(n).enumerate() {
            let mut x = x0;
            for (c, w) in widths.iter().enumerate() {
                text(P::new(x + 0.8, y0 + (r as f32 + 0.5) * rh), &row[c], 2.6, Align2::LEFT_CENTER);
                x += w;
            }
        }
    } else if sheet.art == 4 {
        // Klemmplan (Original-Ausdruck Seite 12): siehe klemmplan.rs
        crate::klemmplan::draw(dc, sheet, anlage);
    } else {
        // Artikelliste, Inhaltsverzeichnis, Querverweise: siehe berechnet.rs
        crate::berechnet::draw(dc, sheet, anlage);
    }
}

pub fn macro_values(project: &Project, anlage: &Anlage, sheet: &Sheet) -> MacroValues {
    let idx = anlage.sheets.iter().position(|s| std::ptr::eq(s, sheet)).unwrap_or(0);
    let prev = if idx > 0 { anlage.sheets[idx - 1].number.to_string() } else { "---".into() };
    let next = anlage.sheets.get(idx + 1).map(|s| s.number.to_string()).unwrap_or_else(|| "---".into());
    let max = anlage.sheets.iter().map(|s| s.number).max().unwrap_or(1);
    MacroValues {
        projekt: project.name.clone(),
        // wie im Original: Datum des Blattes (BLT), sonst des Projekts
        erstellt: if sheet.erstellt.is_empty() { project.erstellt.clone() } else { sheet.erstellt.clone() },
        geaendert: if sheet.geaendert.is_empty() { project.geaendert.clone() } else { sheet.geaendert.clone() },
        name: sheet.name.clone(),
        von: project.von.clone(),
        seite: sheet.number.to_string(),
        // wie im Original-Ausdruck: Kurzform der Anlage ("UV1"), sonst der Name; Ort des Blattes ("W2"), sonst des Projekts
        anlage: if anlage.set.kurz.trim().is_empty() { anlage.name.clone() } else { anlage.set.kurz.clone() },
        next,
        prev,
        status: if project.abgeschlossen { "Projekt ist abgeschlossen".into() } else { "Projekt wird bearbeitet".into() },
        ort: if sheet.ort.trim().is_empty() { project.ort.clone() } else { sheet.ort.clone() },
        owner: project.owner.clone(),
        kunde: project.kunde.clone(),
        anzahl: anlage.sheets.len().to_string(),
        max: max.to_string(),
        znr: project.znr.clone(),
        datum: today(),
    }
}

fn draw_title_block(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage, x0: f32, x1: f32, y0: f32, y1: f32) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let tp = |x: i32, y: i32| P::new(x0 + x as f32 / 10000.0 * (x1 - x0), y0 + y as f32 / 10000.0 * (y1 - y0));
    let black = Stroke::new(1.0_f32.max(0.25 * z), Color32::BLACK);
    let vals = macro_values(dc.project, anlage, sheet);
    let elems = if dc.project.titleblock.is_empty() { titleblock::default_owned() } else { dc.project.titleblock.clone() };
    for e in elems {
        match e {
            TbOwned::Line(a, b, c, d) => {
                p.line_segment([xf.s(tp(a, b)), xf.s(tp(c, d))], black);
            }
            TbOwned::Logo(a, b, c, d) => {
                let r = Rect::from_min_max(xf.s(tp(a, b)), xf.s(tp(c, d)));
                let hs = Stroke::new(1.0, Color32::from_rgb(90, 80, 80));
                let step = (1.3 * z).max(3.0);
                let clip = p.with_clip_rect(r);
                let mut t = -r.height();
                while t < r.width() {
                    clip.line_segment([Pos2::new(r.left() + t, r.bottom()), Pos2::new(r.left() + t + r.height(), r.top())], hs);
                    clip.line_segment([Pos2::new(r.left() + t, r.top()), Pos2::new(r.left() + t + r.height(), r.bottom())], hs);
                    t += step;
                }
                p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgb(90, 80, 80)), egui::StrokeKind::Inside);
                p.text(r.center(), Align2::CENTER_CENTER, "Logo", times(1.6 * z), Color32::from_rgb(200, 60, 40));
            }
            TbOwned::Text(a, b, c, d, tpl, align, lines) => {
                let text = titleblock::expand(&tpl, &vals);
                let text = text.trim_end_matches('\n').to_string();
                if text.trim().is_empty() {
                    continue;
                }
                let ra = tp(a, b);
                let rb = tp(c, d);
                let h_mm = (rb.y - ra.y).abs();
                let w_mm = (rb.x - ra.x).abs();
                let mut em = 0.72 * h_mm / lines as f32;
                let font = |em: f32| times(em * z);
                let mut galley = p.layout_no_wrap(text.clone(), font(em), Color32::BLACK);
                let avail = (w_mm - 0.4) * z;
                if galley.size().x > avail && avail > 1.0 {
                    em *= avail / galley.size().x;
                    galley = p.layout_no_wrap(text.clone(), font(em), Color32::BLACK);
                }
                let r = Rect::from_min_max(xf.s(ra), xf.s(rb));
                let gx = match align {
                    Align::Left => r.left() + 0.3 * z,
                    Align::Center => r.center().x - galley.size().x / 2.0,
                    Align::Right => r.right() - 0.3 * z - galley.size().x,
                };
                let gy = if lines > 1 { r.top() + 0.3 * z } else { r.center().y - galley.size().y / 2.0 };
                p.galley(Pos2::new(gx, gy), galley, Color32::BLACK);
            }
        }
    }
}

pub fn pen_stroke(project: &Project, pen: usize, z: f32) -> (Stroke, u8) {
    // im Symboleditor gelten die Stifte des Symbols (symbol_stifte::Ersatz)
    let st = crate::symbol_stifte::ersatz(pen).unwrap_or_else(|| project.stifte.get(pen).cloned());
    let st = st.unwrap_or(Stift { name: String::new(), style: 0, color: [0, 0, 0], width: 0.25 });
    (Stroke::new((st.width.max(0.05) * z).max(1.0), c32(st.color)), st.style)
}

pub fn draw_object(dc: &DrawCtx, o: &Obj, color_override: Option<Color32>) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (mut stroke, style) = pen_stroke(dc.project, o.pen, z);
    if let Some(c) = color_override {
        stroke.color = c;
    }
    let tc = stroke.color;
    let sp = |pts: &[P]| -> Vec<Pos2> { pts.iter().map(|q| xf.s(*q)).collect() };
    match &o.kind {
        Kind::Line { a, b } => styled_line(p, &sp(&[*a, *b]), stroke, style, z),
        Kind::Curve { a, c1, c2, b } => styled_line(p, &sp(&bezier(*a, *c1, *c2, *b, 40)), stroke, style, z),
        Kind::Rect { a, b } => {
            let (mn, mx) = norm_rect(*a, *b);
            let pts = [mn, P::new(mx.x, mn.y), mx, P::new(mn.x, mx.y), mn];
            styled_line(p, &sp(&pts), stroke, style, z);
        }
        Kind::Triangle { a, b } => {
            let t = triangle_pts(*a, *b);
            styled_line(p, &sp(&[t[0], t[1], t[2], t[0]]), stroke, style, z);
        }
        Kind::Circle { a, b } => styled_line(p, &sp(&ellipse(*a, *b, 64)), stroke, style, z),
        Kind::Verweis { pos, text_pos, text, dir, vertical, spitze, .. } => {
            let tri: Vec<Pos2> = verweis_triangle(*pos, *dir, *spitze).iter().map(|q| xf.s(*q)).collect();
            p.add(Shape::convex_polygon(tri, tc, Stroke::NONE));
            let shown = dc.project.expand_placeholders(text);
            draw_text(p, xf.s(verweis_textmitte(*text_pos, *vertical)), &shown, VERWEIS_TEXT * z, Align::Center, *vertical, tc);
        }
        Kind::Text { pos, text, size, align, vertical, style } => {
            // eigene Schriftfarbe, außer bei Markierung/Vorschau
            let col = if color_override.is_none() && style.color != [0, 0, 0] { c32(style.color) } else { tc };
            // ohne eigenen Schriftnamen gilt die Standardschrift des Projekts
            let style = if style.font.is_empty() && !dc.project.schrift.is_empty() {
                std::borrow::Cow::Owned(TextStyle { font: dc.project.schrift.clone(), ..style.clone() })
            } else {
                std::borrow::Cow::Borrowed(style)
            };
            draw_text_box(p, xf, *pos, &dc.project.expand_placeholders(text), *size, *align, *vertical, &style, col, stroke);
        }
        Kind::TextBox { a, b, text, size } => {
            let (mn, mx) = norm_rect(*a, *b);
            let r = Rect::from_min_max(xf.s(mn), xf.s(mx));
            styled_line(p, &sp(&[mn, P::new(mx.x, mn.y), mx, P::new(mn.x, mx.y), mn]), stroke, style, z);
            let job = egui::text::LayoutJob::simple(text.clone(), crate::schriften::font_id(&dc.project.schrift, size * z, false, false), tc, (r.width() - 1.0 * z).max(5.0));
            let g = p.layout_job(job);
            p.with_clip_rect(r).galley(r.min + Vec2::splat(0.5 * z), g, tc);
        }
        Kind::Spiegel { pos, rows, .. } => crate::kontaktspiegel::draw(dc, *pos, rows, stroke),
        Kind::Table { a, b, rows, cols, cells, header, fixed_w, art, ab } => {
            let (mn, mx) = norm_rect(*a, *b);
            let rows = (*rows).max(1);
            // Daten „Inhaltsverzeichnis“/„Querverweise“: Inhalt bei jedem Zeichnen aus dem Projekt (berechnet.rs)
            let berechnet = dc.anlage.and_then(|an| crate::berechnet::tabelle(an, *art, *ab, *header, rows));
            let (cols, cells) = match &berechnet {
                Some((c, z)) => (*c, z),
                None => (*cols, cells),
            };
            let cols = cols.max(1);
            let rh = (mx.y - mn.y) / rows as f32;
            // Schrift wie im Original-Ausdruck (Seite 35: Zeilen 8,1 mm, Schrift ≈ 5 mm), Spalten nach dem längsten Text,
            // die letzte Spalte füllt den Rest
            let em = (rh * 0.62).min(5.0);
            let widths = if *fixed_w { vec![(mx.x - mn.x) / cols as f32; cols] } else { table_widths(p, cells, cols, mx.x - mn.x, em) };
            draw_grid(p, xf, mn, mx, &widths, &vec![rh; rows], stroke);
            let mut x = mn.x;
            for (c, w) in widths.iter().enumerate() {
                for r in 0..rows {
                    if let Some(t) = cells.get(r * cols + c).filter(|t| !t.is_empty()) {
                        let cr = Rect::from_min_max(xf.s(P::new(x, mn.y + r as f32 * rh)), xf.s(P::new(x + w, mn.y + (r + 1) as f32 * rh)));
                        let font = if *header && r == 0 { times_styled(em * z, true, false) } else { times(em * z) };
                        p.with_clip_rect(cr).text(Pos2::new(cr.left() + 0.8 * z, cr.center().y), Align2::LEFT_CENTER, t, font, tc);
                    }
                }
                x += w;
            }
        }
        Kind::Busbar { a, x2, name, label, style: bs, xin, xout, .. } => {
            let b = P::new(*x2, a.y);
            let mut st = stroke;
            st.width = (0.35 * z).max(1.0);
            // Einstellungen der Schiene (TSSchiene): Stift, Querverweise, Pfeile, Einspeisung
            let set = dc.anlage.and_then(|an| an.schienen.get(name)).cloned().unwrap_or_default();
            let line_style = if set.stift > 0 { dc.project.stifte.get(set.stift).map(|s| s.style).unwrap_or(0) } else if *bs != 0 { *bs } else { style };
            styled_line(p, &sp(&[*a, b]), st, line_style, z);
            if set.einspeisung {
                p.rect_filled(Rect::from_center_size(xf.s(*a), Vec2::splat(1.2 * z)), 0.0, tc);
            }
            let (xin, xout) = if set.quer { (xin.clone(), xout.clone()) } else { (String::new(), String::new()) };
            let (xin, xout) = (&xin, &xout);
            let pfeile = set.pfeile;
            if !name.is_empty() {
                let lp = a.add(*label);
                p.text(xf.s(lp), Align2::LEFT_TOP, name, times(2.5 * z), tc);
            }
            // Querverweise: Pfeil ► vor dem Anfang (vom vorigen Blatt) bzw. hinter dem Ende (zum nächsten Blatt)
            let arrow = |tip: P| {
                if pfeile {
                    let tri = vec![xf.s(tip), xf.s(P::new(tip.x - 2.0, tip.y - 0.6)), xf.s(P::new(tip.x - 2.0, tip.y + 0.6))];
                    p.add(Shape::convex_polygon(tri, tc, Stroke::NONE));
                }
            };
            if !xin.is_empty() {
                arrow(*a);
                p.text(xf.s(P::new(a.x - 2.2, a.y)), Align2::RIGHT_CENTER, xin, times(1.8 * z), tc);
            }
            if !xout.is_empty() {
                arrow(P::new(b.x + 2.0, b.y));
                p.text(xf.s(P::new(b.x + 2.4, b.y)), Align2::LEFT_CENTER, xout, times(1.8 * z), tc);
            }
        }
        Kind::Image { a, b, path, mode, frame } => {
            let (mn, mx) = norm_rect(*a, *b);
            let r = Rect::from_min_max(xf.s(mn), xf.s(mx));
            if let Some(tex) = dc.images.get(path) {
                // Gestreckt füllt die Fläche, Eingepasst behält das Seitenverhältnis, Zentriert zeigt die Originalgröße (96 dpi)
                let [iw, ih] = tex.size();
                let (iw, ih) = (iw as f32, ih as f32);
                let ir = match mode {
                    1 => Rect::from_center_size(r.center(), Vec2::new(iw * 25.4 / 96.0 * z, ih * 25.4 / 96.0 * z)),
                    2 => {
                        let k = (r.width() / iw).min(r.height() / ih);
                        Rect::from_center_size(r.center(), Vec2::new(iw * k, ih * k))
                    }
                    _ => r,
                };
                p.with_clip_rect(r).image(tex.id(), ir, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            } else {
                p.line_segment([r.left_top(), r.right_bottom()], Stroke::new(1.0, Color32::GRAY));
                p.line_segment([r.right_top(), r.left_bottom()], Stroke::new(1.0, Color32::GRAY));
                p.text(r.center(), Align2::CENTER_CENTER, "Bild", times(3.0 * z), Color32::DARK_GRAY);
            }
            if *frame || !dc.images.contains_key(path) {
                p.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
            }
        }
        Kind::Poly { pts } => {
            let mut v = sp(pts);
            if let Some(f) = v.first().copied() {
                v.push(f);
            }
            styled_line(p, &v, stroke, style, z);
        }
        Kind::Symbol { key, pos, rot, mirror_h, mirror_v, bmk, value, scale, option, title, flags, ext } => {
            if let Some(def) = symbols::find(dc.lib, key) {
                draw_symbol_full(p, xf, def, *pos, *rot, *mirror_h, *mirror_v, *scale, *option, *flags, ext.alt, &ext.params, &ext.texts, stroke);
                let (mn, mx) = obj_bounds(o, dc.lib);
                if let Some((off, vert)) = title {
                    // Lage aus der Projektdatei (Titel X/Y bzw. Wert X/Y im Eigenschaftendialog), Ausrichtung Links/Mittig/Rechts
                    let place = |txt: &str, at: P, align: u8, size: f32| {
                        let g = p.layout_no_wrap(txt.to_string(), times(size * z), tc);
                        let (mn, mx) = title_rect(at, g.size().x / z, g.size().y / z, align, *vert);
                        if *vert {
                            // gedreht um die linke obere Ecke des Textes: liegt nach −90° links unten im Umriss
                            p.add(egui::epaint::TextShape::new(xf.s(P::new(mn.x, mx.y)), g, tc).with_angle(-std::f32::consts::FRAC_PI_2));
                        } else {
                            p.galley(xf.s(mn), g, tc);
                        }
                    };
                    if !bmk.is_empty() {
                        place(&symbols::display_bmk(bmk, ext.counter, &dc.bmk), pos.add(*off), ext.title_align, 2.5);
                    }
                    if !value.is_empty() {
                        let shown = dc.project.expand_placeholders(value);
                        let at = ext.value_off.map(|v| pos.add(v)).unwrap_or(P::new(pos.x + off.x, pos.y + off.y + 2.8));
                        place(&shown, at, ext.value_align, 2.2);
                    }
                } else {
                    // BMK links vom Symbol (bzw. am Titelanker der Bibliothek, wenn ungedreht)
                    let anchor = match (def.title, rot % 4 == 0 && !mirror_h && !mirror_v) {
                        (Some(t), true) => P::new(pos.x + (t.x * scale).min(mn.x - pos.x - 0.8), pos.y + t.y * scale),
                        _ => P::new(mn.x - 1.0, (mn.y + mx.y) / 2.0),
                    };
                    if !bmk.is_empty() {
                        p.text(xf.s(P::new(anchor.x, anchor.y - if value.is_empty() { 0.0 } else { 1.4 })), Align2::RIGHT_CENTER, bmk, times(2.6 * z), tc);
                    }
                    if !value.is_empty() {
                        p.text(xf.s(P::new(anchor.x, anchor.y + 1.6)), Align2::RIGHT_CENTER, value, times(2.2 * z), tc);
                    }
                }
            } else {
                let r = Rect::from_center_size(xf.s(*pos), Vec2::splat(6.0 * z));
                p.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                p.text(r.center(), Align2::CENTER_CENTER, "?", times(4.0 * z), tc);
            }
        }
        Kind::Wire { pts, leitung, bez, opts } => {
            let l = dc.project.leitungen.get(*leitung);
            let col = color_override.unwrap_or_else(|| l.map(|l| c32(l.color)).unwrap_or(Color32::BLACK));
            let w = l.map(|l| l.width_mm).unwrap_or(0.25);
            let st = if opts & WIRE_BRUECKE != 0 { 1 } else { l.map(|l| l.style).unwrap_or(0).min(3) };
            styled_line(p, &sp(pts), Stroke::new((w * z).max(1.0), col), st, z);
            // Zielführung: Pfeil am Anfang (Links/Oben) bzw. Ende (Rechts/Unten)
            let arrow = |tip: P, from: P| {
                let d = tip.sub(from);
                let len = (d.x * d.x + d.y * d.y).sqrt().max(1e-3);
                let (ux, uy) = (d.x / len, d.y / len);
                let base = P::new(tip.x - ux * 2.0, tip.y - uy * 2.0);
                let tri = vec![xf.s(tip), xf.s(P::new(base.x - uy * 0.6, base.y + ux * 0.6)), xf.s(P::new(base.x + uy * 0.6, base.y - ux * 0.6))];
                p.add(Shape::convex_polygon(tri, col, Stroke::NONE));
            };
            if pts.len() >= 2 {
                if opts & WIRE_ZIEL_A != 0 {
                    arrow(pts[0], pts[1]);
                }
                if opts & WIRE_ZIEL_B != 0 {
                    arrow(pts[pts.len() - 1], pts[pts.len() - 2]);
                }
            }
            // Verbindungsbezeichnungen (Aderfarben usw.) klein über der Leitung; %O = Name der Leitung
            for (at, t) in bez {
                let t = t.replace("%O", l.map(|l| l.name.as_str()).unwrap_or(""));
                let mut job = egui::text::LayoutJob::default();
                let mut fmt = egui::TextFormat::simple(times(1.6 * z), col);
                if opts & WIRE_UNTERSTRICH != 0 {
                    fmt.underline = Stroke::new((0.1 * z).max(0.5), col);
                }
                job.append(&t, 0.0, fmt);
                let g = p.layout_job(job);
                let s0 = xf.s(*at);
                if opts & WIRE_HOCHKANT != 0 {
                    p.add(egui::epaint::TextShape::new(Pos2::new(s0.x - g.size().y, s0.y), g, col).with_angle(-std::f32::consts::FRAC_PI_2));
                } else {
                    p.galley(Pos2::new(s0.x, s0.y - g.size().y - 0.2 * z), g, col);
                }
                // Markierung: Schrägstrich an (bzw. durch) die Leitung beim Text
                if opts & WIRE_MARKIERUNG != 0 {
                    let (dy0, dy1) = if opts & WIRE_DURCH != 0 { (-1.0, 1.0) } else { (-1.0, 0.0) };
                    p.line_segment([xf.s(P::new(at.x - 0.6, at.y - dy0)), xf.s(P::new(at.x + 0.6, at.y - dy1))], Stroke::new((w * z).max(1.0), col));
                }
            }
        }
        Kind::Terminal { pos, nr, text_off, opt, .. } => {
            let c = xf.s(*pos);
            match text_off {
                // aus .epr: Kreis Ø 1,4 mm, Bezeichnung über der Leitung (Original-Ausdruck Seite 18), Ausrichtung
                // und Gruppenautomatik wie im Klemmen-Dialog
                Some(off) => {
                    p.circle_stroke(c, 0.7 * z, stroke);
                    let sheet = dc.anlage.and_then(|a| a.sheets.iter().find(|s| s.objects.iter().any(|x| x.id == o.id)));
                    let t = crate::klemmen::label_on_sheet(sheet, o);
                    let al = [Align2::LEFT_TOP, Align2::CENTER_TOP, Align2::RIGHT_TOP][opt.align.min(2) as usize];
                    p.text(xf.s(pos.add(*off)), al, t, times(2.3 * z), tc);
                }
                None => {
                    p.circle_stroke(c, 1.0 * z, stroke);
                    p.text(xf.s(P::new(pos.x + 1.5, pos.y - 1.2)), Align2::LEFT_CENTER, nr.to_string(), times(2.4 * z), tc);
                }
            }
        }
    }
}

/// Spaltenbreiten einer Tabelle: alle außer der letzten nach dem längsten Text (+ 2 mm), die letzte füllt den Rest.
pub fn table_widths(p: &egui::Painter, cells: &[String], cols: usize, total: f32, em: f32) -> Vec<f32> {
    let measure = |t: &str| p.layout_no_wrap(t.to_string(), times(em * 10.0), Color32::BLACK).size().x / 10.0;
    let mut w: Vec<f32> = (0..cols)
        .map(|c| cells.iter().skip(c).step_by(cols.max(1)).map(|t| measure(t)).fold(0.0, f32::max) + 2.0)
        .collect();
    let used: f32 = w[..cols - 1].iter().sum();
    if used > total * 0.9 || cols == 1 {
        return vec![total / cols as f32; cols];
    }
    w[cols - 1] = total - used;
    w
}

/// Tabellengitter mit den angegebenen Spaltenbreiten und Zeilenhöhen.
pub fn draw_grid(p: &egui::Painter, xf: Xf, mn: P, mx: P, widths: &[f32], heights: &[f32], stroke: Stroke) {
    p.rect_stroke(Rect::from_min_max(xf.s(mn), xf.s(mx)), 0.0, stroke, egui::StrokeKind::Middle);
    let mut x = mn.x;
    for w in &widths[..widths.len().saturating_sub(1)] {
        x += w;
        p.line_segment([xf.s(P::new(x, mn.y)), xf.s(P::new(x, mx.y))], stroke);
    }
    let mut y = mn.y;
    for h in &heights[..heights.len().saturating_sub(1)] {
        y += h;
        p.line_segment([xf.s(P::new(mn.x, y)), xf.s(P::new(mx.x, y))], stroke);
    }
}

pub fn draw_text(p: &egui::Painter, at: Pos2, text: &str, px: f32, align: Align, vertical: bool, color: Color32) {
    let galley = p.layout_no_wrap(text.to_string(), times(px), color);
    let sz = galley.size();
    let dx = match align {
        Align::Left => 0.0,
        Align::Center => -sz.x / 2.0,
        Align::Right => -sz.x,
    };
    if vertical {
        // Hochkant: von unten nach oben gelesen, Grundpunkt unten
        let pos = Pos2::new(at.x - sz.y / 2.0, at.y - dx);
        p.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(-std::f32::consts::FRAC_PI_2));
    } else {
        p.galley(Pos2::new(at.x + dx, at.y - sz.y / 2.0), galley, color);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_symbol(p: &egui::Painter, xf: Xf, def: &SymDef, pos: P, rot: u8, mh: bool, mv: bool, scale: f32, option: u16, stroke: Stroke) {
    draw_symbol_ext(p, xf, def, pos, rot, mh, mv, scale, option, 0, 0, stroke)
}

/// Symbol zeichnen mit Bild (`option`), Zusatzoptionen (`flags`, Bitmaske) und Alternativtext-Satz (`alt`, 0 = Standard).
#[allow(clippy::too_many_arguments)]
pub fn draw_symbol_ext(p: &egui::Painter, xf: Xf, def: &SymDef, pos: P, rot: u8, mh: bool, mv: bool, scale: f32, option: u16, flags: u32, alt: u16, stroke: Stroke) {
    draw_symbol_full(p, xf, def, pos, rot, mh, mv, scale, option, flags, alt, &[], &[], stroke)
}

/// Wie `draw_symbol_ext`, zusätzlich mit den Parameterwerten der Instanz (%<Name> in Symboltexten).
#[allow(clippy::too_many_arguments)]
pub fn draw_symbol_full(p: &egui::Painter, xf: Xf, def: &SymDef, pos: P, rot: u8, mh: bool, mv: bool, scale: f32, option: u16, flags: u32, alt: u16, params: &[(u32, String)], custom: &[(u32, String)], stroke: Stroke) {
    let alt_set = if alt > 0 { def.alt_sets.get(alt as usize - 1) } else { None };
    let t = |q: P| xf.s(sym_xf(q, pos, rot, mh, mv, scale));
    let z = xf.z * scale;
    for (i, pr) in def.prims.iter().enumerate() {
        if !symbols::visible(option, def.vis.get(i)) || !symbols::flag_visible(flags, def.flag.get(i)) {
            continue;
        }
        // Stift des Elements: eingebaute Stiftart oder eigener Stift des Symbols (Breite, Art, Farbe)
        let (stroke, art) = match pr {
            Prim::Line(.., st) | Prim::Poly(_, st) | Prim::Bezier(.., st) | Prim::Ellipse(.., st) => crate::symbol_stifte::strich(def, *st, stroke, xf.z),
            _ => (stroke, 0),
        };
        match pr {
            Prim::Line(a, b, _) => styled_line(p, &[t(*a), t(*b)], stroke, art, z),
            Prim::Poly(v, _) if def.fill.get(i).copied().unwrap_or(false) => {
                p.add(Shape::convex_polygon(v.iter().map(|q| t(*q)).collect(), stroke.color, stroke));
            }
            Prim::Ellipse(a, b, _) if def.fill.get(i).copied().unwrap_or(false) => {
                p.add(Shape::convex_polygon(ellipse(*a, *b, 40).into_iter().map(t).collect(), stroke.color, stroke));
            }
            Prim::Poly(v, _) => {
                let mut pts: Vec<Pos2> = v.iter().map(|q| t(*q)).collect();
                if let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                styled_line(p, &pts, stroke, art, z);
            }
            Prim::Bezier(a, b, c, d, _) => {
                let pts: Vec<Pos2> = bezier(*a, *b, *c, *d, 24).into_iter().map(t).collect();
                styled_line(p, &pts, stroke, art, z);
            }
            Prim::Ellipse(a, b, _) => {
                let pts: Vec<Pos2> = ellipse(*a, *b, 40).into_iter().map(t).collect();
                styled_line(p, &pts, stroke, art, z);
            }
            Prim::TextBox(a, b, s, al) => {
                // Alternativtext: Ersatz für diese Textnummer im gewählten Satz
                let idx = def.text_idx.get(i).copied().unwrap_or(0);
                let s = alt_set.and_then(|set| set.iter().find(|(n, _)| *n == idx && idx > 0)).map(|(_, t)| t).unwrap_or(s);
                // benutzerdefinierter Text der Instanz (VSY › ALT) hat Vorrang
                let s = custom.iter().find(|(n, _)| *n == idx && idx > 0).map(|(_, t)| t).unwrap_or(s);
                let s = &symbols::expand_text(s, def, params, option);
                if s.is_empty() {
                    continue;
                }
                // Schrift aus der Höhe der Textbox je Zeile (wie das Schriftfeld); Feld 8 des Elements:
                // 2 = Text dreht mit dem Symbol, sonst bleibt er waagerecht (Mittelpunkt und Box werden transformiert)
                let mode = def.text_mode.get(i).copied().unwrap_or(0);
                let lines: Vec<&str> = s.split('\n').map(|l| l.trim_end_matches('\r')).collect();
                let n = lines.len().max(1) as f32;
                let (ua, ub) = norm_rect(*a, *b);
                let mut em = 0.72 * (ub.y - ua.y).abs().max(0.5) * scale / n;
                // breitere Texte werden wie im Original auf die Boxbreite verkleinert ("PE" statt "N" im Symbol "Verbindung")
                let bw = (ub.x - ua.x).abs() * scale;
                let widest = lines.iter().map(|l| p.layout_no_wrap(l.to_string(), times(em * xf.z), stroke.color).size().x / xf.z).fold(0.0, f32::max);
                if widest > bw && bw > 0.3 {
                    em *= bw / widest;
                }
                let font = times(em * xf.z);
                if mode == 2 && (rot % 4 != 0 || mh || mv) {
                    // Zeilen im Symbolsystem anordnen und als gedrehten Text zeichnen
                    // Drehsinn wie sym_xf (Drehung 1 = 90° im Uhrzeigersinn auf dem Bildschirm)
                    let ang = (rot % 4) as f32 * std::f32::consts::FRAC_PI_2;
                    let (sa, ca) = ang.sin_cos();
                    for (li, l) in lines.iter().enumerate() {
                        let cy = ua.y + (ub.y - ua.y) * (li as f32 + 0.5) / n;
                        let g = p.layout_no_wrap(l.to_string(), font.clone(), stroke.color);
                        let (gw, gh) = (g.size().x, g.size().y);
                        let wl = gw / xf.z / scale;
                        // Mitte des Textes im Symbolsystem je nach Ausrichtung
                        let cx = match al {
                            Align::Left => ua.x + wl / 2.0,
                            Align::Center => (ua.x + ub.x) / 2.0,
                            Align::Right => ub.x - wl / 2.0,
                        };
                        let c = xf.s(sym_xf(P::new(cx, cy), pos, rot, mh, mv, scale));
                        // TextShape dreht um die linke obere Ecke: Ecke = Mitte − gedrehte halbe Größe
                        let (hx, hy) = (gw / 2.0, gh / 2.0);
                        let tl = Pos2::new(c.x - (hx * ca - hy * sa), c.y - (hx * sa + hy * ca));
                        p.add(egui::epaint::TextShape::new(tl, g, stroke.color).with_angle(ang));
                    }
                    continue;
                }
                let pa = sym_xf(*a, pos, rot, mh, mv, scale);
                let pb = sym_xf(*b, pos, rot, mh, mv, scale);
                let (mn, mx) = norm_rect(pa, pb);
                let (x, anchor) = match al {
                    Align::Left => (mn.x, Align2::LEFT_CENTER),
                    Align::Center => ((mn.x + mx.x) / 2.0, Align2::CENTER_CENTER),
                    Align::Right => (mx.x, Align2::RIGHT_CENTER),
                };
                for (li, l) in lines.iter().enumerate() {
                    let cy = mn.y + (mx.y - mn.y) * (li as f32 + 0.5) / n;
                    p.text(xf.s(P::new(x, cy)), anchor, l, font.clone(), stroke.color);
                }
            }
            Prim::Arc(c, r, a0, a1) => {
                let n = 16;
                let pts: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                        t(P::new(c.x + r * a.cos(), c.y + r * a.sin()))
                    })
                    .collect();
                p.add(Shape::line(pts, stroke));
            }
            Prim::Text(c, s, size) => {
                p.text(t(*c), Align2::CENTER_CENTER, s, times(size * z), stroke.color);
            }
        }
    }
}

/// Verbindungspunkte: Leitungsenden, die auf einer anderen Leitung liegen, oder Treffpunkte von ≥ 3 Enden.
pub fn junctions(sheet: &Sheet) -> Vec<P> {
    let wires: Vec<&Vec<P>> = sheet
        .objects
        .iter()
        .filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None })
        .collect();
    let mut out: Vec<P> = vec![];
    let mut ends: Vec<P> = vec![];
    for w in &wires {
        if let (Some(f), Some(l)) = (w.first(), w.last()) {
            ends.push(*f);
            ends.push(*l);
        }
    }
    for (i, w) in wires.iter().enumerate() {
        for e in [w.first(), w.last()].into_iter().flatten() {
            for (j, o) in wires.iter().enumerate() {
                if i == j {
                    continue;
                }
                let on_mid = o.windows(2).any(|s| seg_dist(*e, s[0], s[1]) < 0.05)
                    && o.first().map(|f| f.dist(*e) > 0.05).unwrap_or(true)
                    && o.last().map(|l| l.dist(*e) > 0.05).unwrap_or(true);
                if on_mid && !out.iter().any(|q| q.dist(*e) < 0.05) {
                    out.push(*e);
                }
            }
        }
    }
    for e in &ends {
        let n = ends.iter().filter(|q| q.dist(*e) < 0.05).count();
        if n >= 3 && !out.iter().any(|q| q.dist(*e) < 0.05) {
            out.push(*e);
        }
    }
    out
}
