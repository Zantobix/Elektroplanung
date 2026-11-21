//! Symbolvorschau als Tooltip im Symbolbaum: das Symbol mit derselben Zeichenroutine wie auf dem Blatt,
//! eingepasst in ein Feld von 120 × 120 Punkten, Anschlüsse als kleine rote Punkte.

use crate::canvas::{self, Xf};
use crate::model::P;
use crate::symbols::{self, SymDef};
use eframe::egui::{self, Color32, Stroke, Vec2};

/// Kantenlänge des Vorschaufeldes in Punkten
pub const GROESSE: f32 = 120.0;

/// Maßstab (Punkte je mm) und Ursprung, damit `def` mittig in `rect` passt (Rand 8 Punkte, höchstens 12 Punkte/mm).
pub fn einpassen(def: &SymDef, rect: egui::Rect) -> Xf {
    let (mn, mx) = symbols::bounds(def);
    let (w, h) = ((mx.x - mn.x).max(1.0), (mx.y - mn.y).max(1.0));
    let z = ((rect.width() - 16.0) / w).min((rect.height() - 16.0) / h).clamp(0.5, 12.0);
    let c = P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
    Xf { origin: rect.center() - Vec2::new(c.x * z, c.y * z), z }
}

/// Vorschau zeichnen (Inhalt eines Tooltips).
pub fn symbol_vorschau(ui: &mut egui::Ui, def: &SymDef) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(GROESSE), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, Color32::WHITE);
    p.rect_stroke(rect, 2.0, Stroke::new(1.0, Color32::from_gray(190)), egui::StrokeKind::Inside);
    let xf = einpassen(def, rect);
    let stroke = Stroke::new((0.25 * xf.z).clamp(1.0, 2.0), Color32::BLACK);
    canvas::draw_symbol_full(&p, xf, def, P::default(), 0, false, false, 1.0, 0, 0, 0, &[], &[], stroke);
    for (i, q) in def.pins.iter().enumerate() {
        if symbols::visible(0, def.pin_vis.get(i)) {
            p.circle_filled(xf.s(*q), 2.0, Color32::from_rgb(220, 0, 0));
        }
    }
    ui.label(&def.name);
    if !def.bmk.is_empty() {
        ui.small(format!("BMK-Vorlage {}", def.bmk));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn einpassen_mittig_und_im_feld() {
        let mut d = SymDef::empty("t", "Test", "-K");
        d.prims = vec![symbols::Prim::Line(P::new(-10.0, -5.0), P::new(30.0, 5.0), 0)];
        let r = egui::Rect::from_min_size(egui::pos2(100.0, 200.0), Vec2::splat(GROESSE));
        let xf = einpassen(&d, r);
        let a = xf.s(P::new(-10.0, -5.0));
        let b = xf.s(P::new(30.0, 5.0));
        assert!(r.contains(a) && r.contains(b));
        assert!(((a.x + b.x) / 2.0 - r.center().x).abs() < 0.01 && ((a.y + b.y) / 2.0 - r.center().y).abs() < 0.01);
        assert!((b.x - a.x) > 90.0, "Symbol soll das Feld ausfüllen");
    }
}
