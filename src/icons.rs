//! Eigene, einfache Vektor-Symbole für die Werkzeugleisten (20 × 20 px Raster wie im Original).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    NewProject,
    Open,
    Save,
    SaveProject,
    Print,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    NewPen,
    Zoom,
    Grid,
    PathAlign,
    AutoConn,
    SizeFrame,
    Fix,
    NewSheet,
    NewPotential,
    NewBusbar,
    Rotate,
    MirrorV,
    MirrorH,
    Group,
    Ungroup,
    NewLeitung,
    NewKlemme,
    Select,
    Klemmstelle,
    Line,
    Curve,
    Rect,
    Triangle,
    Circle,
    Text,
    TextBox,
    Table,
    Image,
    CaseSensitive,
    Find,
}


/// Index im Original-Bildverzeichnis ImageList1 (aus der Formularressource TFORM1 der Original-Exe:
/// ImageIndex der ToolButtons bzw. der zugeordneten Actions).
pub fn original_index(icon: Icon) -> Option<usize> {
    Some(match icon {
        Icon::Select => 0,
        Icon::Line => 1,
        Icon::Curve => 2,
        Icon::Rect => 3,
        Icon::Triangle => 4,
        Icon::Circle => 5,
        Icon::Text => 6,
        Icon::Klemmstelle => 12,
        Icon::NewSheet => 13,
        Icon::NewProject => 14,
        Icon::Open => 15,
        Icon::Save => 16,
        Icon::SaveProject => 17,
        Icon::Print => 18,
        Icon::Zoom => 19,
        Icon::Cut => 20,
        Icon::Copy => 21,
        Icon::Paste => 22,
        Icon::Undo => 23,
        Icon::Redo => 24,
        Icon::PathAlign => 25,
        Icon::NewBusbar => 26,
        Icon::NewKlemme => 27,
        Icon::AutoConn => 28,
        Icon::Rotate => 29,
        Icon::MirrorV => 30,
        Icon::MirrorH => 31,
        Icon::NewPen => 32,
        Icon::NewLeitung => 33,
        Icon::NewPotential => 34,
        Icon::Image => 35,
        Icon::TextBox => 36,
        Icon::Table => 37,
        Icon::Grid => 38,
        Icon::SizeFrame => 40,
        Icon::Fix => 41,
        Icon::Group => 42,
        Icon::Ungroup => 43,
        Icon::CaseSensitive | Icon::Find => return None,
    })
}

static ORIGINAL_PNGS: [&[u8]; 45] = [
    include_bytes!("../resources/ImageList1.Bitmap_0.png"),
    include_bytes!("../resources/ImageList1.Bitmap_1.png"),
    include_bytes!("../resources/ImageList1.Bitmap_2.png"),
    include_bytes!("../resources/ImageList1.Bitmap_3.png"),
    include_bytes!("../resources/ImageList1.Bitmap_4.png"),
    include_bytes!("../resources/ImageList1.Bitmap_5.png"),
    include_bytes!("../resources/ImageList1.Bitmap_6.png"),
    include_bytes!("../resources/ImageList1.Bitmap_7.png"),
    include_bytes!("../resources/ImageList1.Bitmap_8.png"),
    include_bytes!("../resources/ImageList1.Bitmap_9.png"),
    include_bytes!("../resources/ImageList1.Bitmap_10.png"),
    include_bytes!("../resources/ImageList1.Bitmap_11.png"),
    include_bytes!("../resources/ImageList1.Bitmap_12.png"),
    include_bytes!("../resources/ImageList1.Bitmap_13.png"),
    include_bytes!("../resources/ImageList1.Bitmap_14.png"),
    include_bytes!("../resources/ImageList1.Bitmap_15.png"),
    include_bytes!("../resources/ImageList1.Bitmap_16.png"),
    include_bytes!("../resources/ImageList1.Bitmap_17.png"),
    include_bytes!("../resources/ImageList1.Bitmap_18.png"),
    include_bytes!("../resources/ImageList1.Bitmap_19.png"),
    include_bytes!("../resources/ImageList1.Bitmap_20.png"),
    include_bytes!("../resources/ImageList1.Bitmap_21.png"),
    include_bytes!("../resources/ImageList1.Bitmap_22.png"),
    include_bytes!("../resources/ImageList1.Bitmap_23.png"),
    include_bytes!("../resources/ImageList1.Bitmap_24.png"),
    include_bytes!("../resources/ImageList1.Bitmap_25.png"),
    include_bytes!("../resources/ImageList1.Bitmap_26.png"),
    include_bytes!("../resources/ImageList1.Bitmap_27.png"),
    include_bytes!("../resources/ImageList1.Bitmap_28.png"),
    include_bytes!("../resources/ImageList1.Bitmap_29.png"),
    include_bytes!("../resources/ImageList1.Bitmap_30.png"),
    include_bytes!("../resources/ImageList1.Bitmap_31.png"),
    include_bytes!("../resources/ImageList1.Bitmap_32.png"),
    include_bytes!("../resources/ImageList1.Bitmap_33.png"),
    include_bytes!("../resources/ImageList1.Bitmap_34.png"),
    include_bytes!("../resources/ImageList1.Bitmap_35.png"),
    include_bytes!("../resources/ImageList1.Bitmap_36.png"),
    include_bytes!("../resources/ImageList1.Bitmap_37.png"),
    include_bytes!("../resources/ImageList1.Bitmap_38.png"),
    include_bytes!("../resources/ImageList1.Bitmap_39.png"),
    include_bytes!("../resources/ImageList1.Bitmap_40.png"),
    include_bytes!("../resources/ImageList1.Bitmap_41.png"),
    include_bytes!("../resources/ImageList1.Bitmap_42.png"),
    include_bytes!("../resources/ImageList1.Bitmap_43.png"),
    include_bytes!("../resources/ImageList1.Bitmap_44.png"),
];

/// Textur eines Original-Bildes (einmal dekodiert, im egui-Kontext zwischengespeichert).
fn original_texture(ctx: &egui::Context, idx: usize) -> Option<egui::TextureHandle> {
    let id = egui::Id::unique(("orig-icon", idx));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = image::load_from_memory(ORIGINAL_PNGS.get(idx)?).ok()?.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    let t = ctx.load_texture(format!("orig-icon-{idx}"), ci, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

pub fn draw(p: &egui::Painter, r: Rect, icon: Icon, enabled: bool) {
    if let Some(tex) = original_index(icon).and_then(|i| original_texture(p.ctx(), i)) {
        // Originalgröße 20 × 20 px, mittig; gesperrte Knöpfe grau wie die VCL
        let side = 20.0_f32.min(r.width()).min(r.height());
        let rr = Rect::from_center_size(r.center(), Vec2::splat(side));
        let tint = if enabled { Color32::WHITE } else { Color32::from_rgba_unmultiplied(160, 160, 160, 110) };
        p.image(tex.id(), rr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
        return;
    }
    draw_vector(p, r, icon, enabled);
}

fn draw_vector(p: &egui::Painter, r: Rect, icon: Icon, enabled: bool) {
    let c = r.center();
    let s = r.width().min(r.height()) / 20.0;
    let q = |x: f32, y: f32| Pos2::new(c.x + (x - 10.0) * s, c.y + (y - 10.0) * s);
    let k = |col: Color32| if enabled { col } else { Color32::from_gray(170) };
    let black = k(Color32::BLACK);
    let dark = k(Color32::from_rgb(40, 40, 40));
    let blue = k(Color32::from_rgb(20, 60, 170));
    let red = k(Color32::from_rgb(190, 20, 20));
    let yellow = k(Color32::from_rgb(235, 190, 40));
    let gray = k(Color32::from_rgb(120, 120, 120));
    let w = |col: Color32| Stroke::new(1.2 * s.max(0.8), col);
    let w2 = |col: Color32| Stroke::new(2.0 * s.max(0.8), col);
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(q(x0, y0), q(x1, y1));
    let line = |a: (f32, f32), b: (f32, f32), st: Stroke| {
        p.line_segment([q(a.0, a.1), q(b.0, b.1)], st);
    };
    let doc = |x0: f32, y0: f32, x1: f32, y1: f32, col: Color32| {
        p.rect_filled(rect(x0, y0, x1, y1), 0.0, Color32::WHITE);
        p.rect_stroke(rect(x0, y0, x1, y1), 0.0, w(col), StrokeKind::Inside);
    };
    match icon {
        Icon::NewProject | Icon::NewSheet => {
            doc(5.0, 2.5, 15.0, 17.5, dark);
            line((12.0, 2.5), (15.0, 5.5), w(dark));
        }
        Icon::Open => {
            p.rect_filled(rect(2.0, 6.0, 15.0, 16.0), 1.0, yellow);
            p.rect_stroke(rect(2.0, 6.0, 15.0, 16.0), 1.0, w(dark), StrokeKind::Inside);
            p.add(egui::Shape::convex_polygon(
                vec![q(5.0, 9.0), q(18.5, 9.0), q(15.0, 16.0), q(2.0, 16.0)],
                k(Color32::from_rgb(250, 215, 90)),
                w(dark),
            ));
        }
        Icon::Save | Icon::SaveProject => {
            let off = if icon == Icon::SaveProject { 2.0 } else { 0.0 };
            if off > 0.0 {
                p.rect_filled(rect(5.0, 2.0, 18.0, 15.0), 1.0, gray);
            }
            p.rect_filled(rect(2.5 + off * 0.0, 4.0 + off, 15.5, 17.0 + off * 0.5), 1.0, k(Color32::from_rgb(90, 90, 110)));
            p.rect_filled(rect(5.0, 4.5 + off, 13.0, 9.0 + off), 0.0, Color32::WHITE);
            p.rect_filled(rect(6.0, 12.0 + off * 0.5, 12.0, 17.0 + off * 0.5), 0.0, k(Color32::from_gray(200)));
        }
        Icon::Print => {
            doc(6.0, 2.0, 14.0, 8.0, dark);
            p.rect_filled(rect(2.5, 8.0, 17.5, 14.5), 1.0, gray);
            doc(5.0, 12.0, 15.0, 18.0, dark);
            line((6.5, 14.5), (13.5, 14.5), w(dark));
        }
        Icon::Cut => {
            p.circle_stroke(q(6.5, 15.0), 2.2 * s, w(dark));
            p.circle_stroke(q(13.5, 15.0), 2.2 * s, w(dark));
            line((8.0, 13.5), (13.5, 3.0), w(dark));
            line((12.0, 13.5), (6.5, 3.0), w(dark));
        }
        Icon::Copy => {
            doc(3.0, 2.5, 12.0, 13.5, dark);
            doc(8.0, 6.5, 17.0, 17.5, dark);
        }
        Icon::Paste => {
            p.rect_filled(rect(3.0, 4.0, 14.0, 17.0), 1.0, k(Color32::from_rgb(170, 130, 70)));
            p.rect_filled(rect(6.0, 2.5, 11.0, 5.5), 1.0, gray);
            doc(9.0, 8.0, 17.5, 18.0, dark);
        }
        Icon::Undo | Icon::Redo => {
            let flip = if icon == Icon::Redo { -1.0 } else { 1.0 };
            let f = |x: f32| 10.0 + (x - 10.0) * flip;
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 - i as f32 / 12.0);
                    q(f(10.5 + 5.5 * a.cos()), 12.0 - 5.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, w2(blue)));
            p.add(egui::Shape::convex_polygon(
                vec![q(f(2.0), 10.0), q(f(8.5), 10.0), q(f(5.0), 15.0)],
                blue,
                Stroke::NONE,
            ));
        }
        Icon::NewPen => {
            line((4.0, 16.0), (15.0, 5.0), Stroke::new(3.0 * s, blue));
            line((15.0, 5.0), (16.5, 3.5), Stroke::new(3.0 * s, red));
            line((3.0, 17.0), (4.5, 15.5), w(dark));
            line((2.0, 18.0), (9.0, 18.0), w(dark));
        }
        Icon::Zoom => {
            p.circle_stroke(q(8.5, 8.5), 5.0 * s, w2(dark));
            line((12.0, 12.0), (17.0, 17.0), Stroke::new(3.0 * s, dark));
        }
        Icon::Grid => {
            for i in 0..4 {
                let v = 4.0 + i as f32 * 4.0;
                line((v, 2.5), (v, 17.5), w(blue));
                line((2.5, v), (17.5, v), w(blue));
            }
        }
        Icon::PathAlign => {
            p.rect_stroke(rect(3.0, 3.0, 17.0, 17.0), 0.0, w(dark), StrokeKind::Inside);
            line((10.0, 1.5), (10.0, 18.5), w(blue));
            line((1.5, 10.0), (18.5, 10.0), w(blue));
            p.circle_filled(q(10.0, 10.0), 1.6 * s, red);
        }
        Icon::AutoConn => {
            line((3.0, 10.0), (17.0, 10.0), w2(dark));
            line((10.0, 3.0), (10.0, 17.0), w2(dark));
            p.circle_filled(q(10.0, 10.0), 2.0 * s, black);
            for (x, y) in [(3.0, 10.0), (17.0, 10.0), (10.0, 3.0), (10.0, 17.0)] {
                p.circle_filled(q(x, y), 1.4 * s, blue);
            }
        }
        Icon::SizeFrame => {
            p.add(egui::Shape::dashed_line(
                &[q(4.0, 4.0), q(16.0, 4.0), q(16.0, 16.0), q(4.0, 16.0), q(4.0, 4.0)],
                w(dark),
                2.0 * s,
                1.5 * s,
            ));
            for (x, y) in [(4.0, 4.0), (16.0, 4.0), (16.0, 16.0), (4.0, 16.0)] {
                p.rect_filled(Rect::from_center_size(q(x, y), Vec2::splat(3.5 * s)), 0.0, blue);
            }
        }
        Icon::Fix => {
            p.circle_filled(q(10.0, 6.0), 3.5 * s, red);
            line((10.0, 9.0), (10.0, 18.0), w2(dark));
            line((5.0, 9.5), (15.0, 9.5), w2(dark));
        }
        Icon::NewPotential => {
            doc(4.0, 2.5, 13.0, 16.0, dark);
            line((14.0, 14.0), (14.0, 18.5), w2(red));
            line((11.8, 16.3), (16.2, 16.3), w2(red));
            line((6.0, 7.0), (11.0, 7.0), w(blue));
            line((6.0, 10.0), (11.0, 10.0), w(blue));
        }
        Icon::NewBusbar => {
            p.rect_filled(rect(2.0, 9.0, 18.0, 12.0), 0.0, red);
            p.rect_filled(rect(2.0, 13.0, 18.0, 16.0), 0.0, k(Color32::from_rgb(30, 30, 150)));
            line((5.0, 3.0), (5.0, 9.0), w(dark));
            line((15.0, 3.0), (15.0, 9.0), w(dark));
        }
        Icon::Rotate => {
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.3 + 1.5 * i as f32 / 16.0);
                    q(10.0 + 6.0 * a.cos(), 10.0 + 6.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, w2(blue)));
            p.add(egui::Shape::convex_polygon(vec![q(14.0, 1.5), q(18.5, 6.5), q(12.0, 8.0)], blue, Stroke::NONE));
        }
        Icon::MirrorV => {
            line((10.0, 2.0), (10.0, 18.0), Stroke::new(1.0 * s, gray));
            p.add(egui::Shape::convex_polygon(vec![q(8.0, 4.0), q(8.0, 16.0), q(2.0, 16.0)], blue, Stroke::NONE));
            p.add(egui::Shape::convex_polygon(vec![q(12.0, 4.0), q(18.0, 16.0), q(12.0, 16.0)], Color32::TRANSPARENT, w(blue)));
        }
        Icon::MirrorH => {
            line((2.0, 10.0), (18.0, 10.0), Stroke::new(1.0 * s, gray));
            p.add(egui::Shape::convex_polygon(vec![q(4.0, 8.0), q(16.0, 8.0), q(16.0, 2.0)], blue, Stroke::NONE));
            p.add(egui::Shape::convex_polygon(vec![q(4.0, 12.0), q(16.0, 12.0), q(16.0, 18.0)], Color32::TRANSPARENT, w(blue)));
        }
        Icon::Group | Icon::Ungroup => {
            p.rect_stroke(rect(3.0, 3.0, 11.0, 11.0), 0.0, w(blue), StrokeKind::Inside);
            p.circle_stroke(q(13.0, 13.0), 4.0 * s, w(red));
            if icon == Icon::Group {
                p.add(egui::Shape::dashed_line(
                    &[q(1.5, 1.5), q(18.5, 1.5), q(18.5, 18.5), q(1.5, 18.5), q(1.5, 1.5)],
                    w(dark),
                    2.0 * s,
                    1.5 * s,
                ));
            } else {
                line((2.0, 18.0), (18.0, 2.0), w(dark));
            }
        }
        Icon::NewLeitung => {
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    q(2.0 + 16.0 * t, 10.0 + 4.0 * (t * std::f32::consts::TAU).sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, Stroke::new(2.5 * s, blue)));
            p.circle_filled(q(2.0, 10.0), 1.6 * s, dark);
            p.circle_filled(q(18.0, 10.0), 1.6 * s, dark);
        }
        Icon::NewKlemme => {
            for i in 0..3 {
                let x = 3.0 + i as f32 * 5.0;
                p.rect_filled(rect(x, 6.0, x + 4.0, 17.0), 0.0, k(Color32::from_rgb(70, 110, 200)));
                p.rect_stroke(rect(x, 6.0, x + 4.0, 17.0), 0.0, w(dark), StrokeKind::Inside);
                p.circle_filled(q(x + 2.0, 9.0), 1.0 * s, yellow);
            }
            line((2.0, 3.5), (18.0, 3.5), w(red));
        }
        Icon::Select => {
            p.add(egui::Shape::convex_polygon(
                vec![q(4.0, 3.0), q(4.0, 16.0), q(7.5, 12.5), q(10.0, 17.5), q(12.0, 16.5), q(9.5, 11.5), q(14.0, 11.5)],
                Color32::WHITE,
                w(dark),
            ));
        }
        Icon::Klemmstelle => {
            p.circle_stroke(q(12.0, 8.0), 4.0 * s, w2(dark));
            line((12.0, 12.0), (12.0, 18.0), w2(dark));
            line((2.0, 8.0), (8.0, 8.0), w2(dark));
            p.circle_filled(q(3.0, 15.0), 1.8 * s, red);
        }
        Icon::Line => line((3.0, 17.0), (17.0, 3.0), w2(dark)),
        Icon::Curve => {
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.0;
                    q(2.0 + 16.0 * t, 10.0 - 6.0 * (t * std::f32::consts::TAU).sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, w2(dark)));
        }
        Icon::Rect => {
            p.rect_stroke(rect(3.0, 4.0, 17.0, 16.0), 1.0, w2(dark), StrokeKind::Inside);
        }
        Icon::Triangle => {
            p.add(egui::Shape::closed_line(vec![q(10.0, 3.0), q(17.0, 16.5), q(3.0, 16.5)], w2(dark)));
        }
        Icon::Circle => {
            p.circle_stroke(q(10.0, 10.0), 6.5 * s, w2(dark));
        }
        Icon::Text => {
            p.text(q(10.0, 10.5), egui::Align2::CENTER_CENTER, "A", egui::FontId::proportional(17.0 * s), black);
        }
        Icon::TextBox => {
            p.text(q(10.0, 6.5), egui::Align2::CENTER_CENTER, "ABC", egui::FontId::monospace(6.5 * s), dark);
            p.text(q(10.0, 13.5), egui::Align2::CENTER_CENTER, "DEF", egui::FontId::monospace(6.5 * s), dark);
            p.rect_stroke(rect(2.5, 2.5, 17.5, 17.5), 0.0, w(dark), StrokeKind::Inside);
        }
        Icon::Table => {
            p.rect_stroke(rect(3.0, 3.0, 17.0, 17.0), 0.0, w(dark), StrokeKind::Inside);
            line((10.0, 3.0), (10.0, 17.0), w(dark));
            line((3.0, 10.0), (17.0, 10.0), w(dark));
            p.text(q(6.5, 6.5), egui::Align2::CENTER_CENTER, "a", egui::FontId::proportional(5.5 * s), dark);
            p.text(q(13.5, 13.5), egui::Align2::CENTER_CENTER, "b", egui::FontId::proportional(5.5 * s), dark);
        }
        Icon::Image => {
            p.rect_filled(rect(2.0, 6.0, 18.0, 16.0), 2.0, dark);
            p.rect_filled(rect(5.0, 4.0, 10.0, 6.5), 1.0, dark);
            p.circle_filled(q(10.0, 11.0), 3.5 * s, Color32::WHITE);
            p.circle_filled(q(10.0, 11.0), 2.0 * s, dark);
        }
        Icon::CaseSensitive => {
            p.text(q(10.0, 11.0), egui::Align2::CENTER_CENTER, "Aa", egui::FontId::proportional(11.0 * s), blue);
            line((3.0, 17.0), (17.0, 17.0), w(blue));
        }
        Icon::Find => {
            p.circle_filled(q(6.0, 13.0), 4.0 * s, dark);
            p.circle_filled(q(14.0, 13.0), 4.0 * s, dark);
            p.rect_filled(rect(5.0, 4.0, 8.0, 12.0), 0.0, dark);
            p.rect_filled(rect(12.0, 4.0, 15.0, 12.0), 0.0, dark);
            p.rect_filled(rect(8.0, 8.0, 12.0, 11.0), 0.0, dark);
        }
    }
}

/// Glyphe aus den Formularen der Original-Exe (nicht Teil des Repos; Ordner über EP_GLYPHEN, siehe build.rs).
/// Fehlen die Dateien, ist das Ergebnis leer und die Knöpfe zeigen Text.
#[macro_export]
macro_rules! glyph {
    ($f:literal) => {{
        #[cfg(original_glyphen)]
        let b: &[u8] = include_bytes!(concat!(env!("EP_GLYPHEN_DIR"), "/", $f));
        #[cfg(not(original_glyphen))]
        let b: &[u8] = &[];
        b
    }};
}

/// Beliebiges eingebettetes PNG als Textur (im egui-Kontext zwischengespeichert).
pub fn png_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let id = egui::Id::unique(("png", name));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    let t = ctx.load_texture(name.to_string(), ci, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}
