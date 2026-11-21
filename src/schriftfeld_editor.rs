//! Schriftfeld grafisch bearbeiten (Tools › Schriftfeld bearbeiten, Original Schriftfeldbearbeiten1Click):
//! Linien, Texte (mit Textmakros %Projekt, %Seite …) und das Logo des Schriftfelds in einer Vorschau anzeigen,
//! anklicken, verschieben, an den Griffen in der Größe ändern, hinzufügen und löschen; die Eigenschaften des
//! gewählten Elements (Lage, Text, Ausrichtung, Zeilen) stehen darunter. Koordinaten wie im Original normiert auf
//! 0…10000 in beiden Richtungen des Schriftfeld-Rechtecks (FUS › GRS, `titleblock.rs`).

use crate::model::Align;
use crate::titleblock::{self, TbOwned};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

/// Textmakros des Schriftfelds (titleblock::expand)
pub const MAKROS: &[&str] = &[
    "%Projekt", "%Name", "%Seite", "%Anzahl", "%Max", "%Prev", "%Next", "%Anlage", "%Ort", "%Datum", "%Heute", "%Erstellt", "%Geändert",
    "%Von", "%Status", "%ZNR", "%Owner", "%Kunde",
];

/// Fangraster in Normeinheiten (0,5 % der Breite bzw. Höhe)
pub const RASTER: i32 = 50;

/// Griff beim Ziehen: ganzes Element oder ein Eckpunkt (Linie 0/1; Rechteck 0 = links oben, 1 = rechts oben,
/// 2 = rechts unten, 3 = links unten)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Griff {
    Ganz,
    Punkt(usize),
}

/// Zustand der Vorschau zwischen den Bildern (egui-Speicher)
#[derive(Clone, Default)]
struct Zustand {
    sel: Option<usize>,
    /// Ziehen: Element, Griff, Startlage des Zeigers (Normeinheiten), Element vor dem Ziehen
    drag: Option<(usize, Griff, (f32, f32), TbOwned)>,
    /// senkrechte Dehnung der Vorschau (das Schriftfeld ist sehr flach)
    dehnung: f32,
}

/// Koordinaten eines Elements (x1, y1, x2, y2).
pub fn koord(e: &TbOwned) -> [i32; 4] {
    match e {
        TbOwned::Line(a, b, c, d) | TbOwned::Logo(a, b, c, d) | TbOwned::Text(a, b, c, d, ..) => [*a, *b, *c, *d],
    }
}

pub fn setze_koord(e: &mut TbOwned, k: [i32; 4]) {
    match e {
        TbOwned::Line(a, b, c, d) | TbOwned::Logo(a, b, c, d) | TbOwned::Text(a, b, c, d, ..) => {
            (*a, *b, *c, *d) = (k[0], k[1], k[2], k[3]);
        }
    }
}

fn ist_linie(e: &TbOwned) -> bool {
    matches!(e, TbOwned::Line(..))
}

/// Griffpunkte eines Elements in Normeinheiten.
pub fn griffe(e: &TbOwned) -> Vec<(i32, i32)> {
    let [a, b, c, d] = koord(e);
    if ist_linie(e) {
        vec![(a, b), (c, d)]
    } else {
        vec![(a, b), (c, b), (c, d), (a, d)]
    }
}

/// Auf das Raster fangen.
pub fn fang(v: i32) -> i32 {
    ((v as f32 / RASTER as f32).round() as i32) * RASTER
}

/// Element um (dx, dy) verschieben bzw. einen Griff dorthin ziehen (von der Lage `start` vor dem Ziehen aus).
pub fn ziehen(start: &TbOwned, griff: Griff, dx: i32, dy: i32, raster: bool) -> TbOwned {
    let f = |v: i32| if raster { fang(v) } else { v };
    let mut e = start.clone();
    let [a, b, c, d] = koord(start);
    let k = match griff {
        Griff::Ganz => {
            // linke obere Ecke auf das Raster, Größe bleibt, das Element bleibt im Schriftfeld (0…10000)
            let (lo_x, hi_x) = (-a.min(c), 10000 - a.max(c));
            let (lo_y, hi_y) = (-b.min(d), 10000 - b.max(d));
            let nx = a + (f(a + dx) - a).clamp(lo_x.min(0), hi_x.max(0));
            let ny = b + (f(b + dy) - b).clamp(lo_y.min(0), hi_y.max(0));
            [nx, ny, c + nx - a, d + ny - b]
        }
        Griff::Punkt(i) if ist_linie(start) => {
            let (x, y) = if i == 0 { (a, b) } else { (c, d) };
            let (x, y) = (f(x + dx).clamp(0, 10000), f(y + dy).clamp(0, 10000));
            if i == 0 {
                [x, y, c, d]
            } else {
                [a, b, x, y]
            }
        }
        Griff::Punkt(i) => {
            let mut k = [a, b, c, d];
            let xi = if i == 0 || i == 3 { 0 } else { 2 };
            let yi = if i == 0 || i == 1 { 1 } else { 3 };
            k[xi] = f(k[xi] + dx).clamp(0, 10000);
            k[yi] = f(k[yi] + dy).clamp(0, 10000);
            k
        }
    };
    setze_koord(&mut e, k);
    e
}

/// Abstand eines Punktes von einer Strecke.
fn abstand(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() < 1e-6 { 0.0 } else { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) };
    p.distance(a + ab * t)
}

/// Element unter dem Punkt `p` (Bildschirm), `s` rechnet Normeinheiten in Bildschirmlage um. Linien gewinnen bei
/// weniger als `tol` Pixel Abstand, sonst das kleinste Rechteck, das den Punkt enthält.
pub fn treffer(elems: &[TbOwned], p: Pos2, s: &dyn Fn(i32, i32) -> Pos2, tol: f32) -> Option<usize> {
    let linie = elems
        .iter()
        .enumerate()
        .filter(|(_, e)| ist_linie(e))
        .map(|(i, e)| {
            let [a, b, c, d] = koord(e);
            (i, abstand(p, s(a, b), s(c, d)))
        })
        .filter(|(_, d)| *d <= tol)
        .min_by(|x, y| x.1.total_cmp(&y.1));
    if let Some((i, _)) = linie {
        return Some(i);
    }
    elems
        .iter()
        .enumerate()
        .filter(|(_, e)| !ist_linie(e))
        .filter_map(|(i, e)| {
            let [a, b, c, d] = koord(e);
            let r = Rect::from_two_pos(s(a, b), s(c, d));
            r.contains(p).then_some((i, r.area()))
        })
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(i, _)| i)
}

/// Neues Element in der Mitte des Schriftfelds.
pub fn neu(art: u8) -> TbOwned {
    match art {
        0 => TbOwned::Line(4000, 5000, 6000, 5000),
        1 => TbOwned::Text(4000, 3000, 6000, 7000, "Text".into(), Align::Center, 1),
        _ => TbOwned::Logo(4500, 1000, 5500, 9000),
    }
}

/// Vorschau und Bearbeitung des Schriftfelds; `elems` wird direkt geändert. `w_mm`/`h_mm` = Maße des
/// Schriftfelds auf dem aktuellen Blatt (für die Anzeige in mm).
pub fn ui(ui: &mut egui::Ui, elems: &mut Vec<TbOwned>, w_mm: f32, h_mm: f32) {
    let id = egui::Id::unique("schriftfeld_editor");
    let mut z: Zustand = ui.data_mut(|d| d.get_temp::<Zustand>(id)).unwrap_or_default();
    if z.dehnung <= 0.0 {
        z.dehnung = 4.0;
    }
    if z.sel.is_some_and(|s| s >= elems.len()) {
        z.sel = None;
    }
    ui.set_min_width(880.0);
    ui.strong("Schriftfeld bearbeiten");
    ui.horizontal(|ui| {
        if ui.button("Neue Linie").clicked() {
            elems.push(neu(0));
            z.sel = Some(elems.len() - 1);
        }
        if ui.button("Neuer Text").clicked() {
            elems.push(neu(1));
            z.sel = Some(elems.len() - 1);
        }
        if ui.button("Neues Logo").clicked() {
            elems.push(neu(2));
            z.sel = Some(elems.len() - 1);
        }
        if ui.add_enabled(z.sel.is_some(), egui::Button::new("Löschen")).clicked() {
            if let Some(s) = z.sel.take() {
                elems.remove(s);
            }
        }
        ui.add_space(12.0); // kein Trennstrich: er dehnt sich in der Zeile auf die volle Höhe und lässt das Fenster wachsen
        ui.label("Höhe ×");
        ui.add(egui::Slider::new(&mut z.dehnung, 1.0..=8.0).step_by(0.5));
        ui.add_space(12.0); // kein Trennstrich: er dehnt sich in der Zeile auf die volle Höhe und lässt das Fenster wachsen
        if ui.button("Standard-Schriftfeld").on_hover_text("Standard-Schriftfeld wiederherstellen").clicked() {
            *elems = titleblock::default_owned();
            z.sel = None;
        }
    });
    // Vorschau: Breite fest, Höhe nach Seitenverhältnis × Dehnung
    let w_px = 860.0;
    let h_px = (w_px * h_mm / w_mm.max(1.0) * z.dehnung).clamp(40.0, 320.0);
    let (resp, p) = ui.allocate_painter(Vec2::new(w_px + 20.0, h_px + 20.0), Sense::click_and_drag());
    let r = Rect::from_min_size(resp.rect.min + Vec2::splat(10.0), Vec2::new(w_px, h_px));
    let s = |x: i32, y: i32| Pos2::new(r.left() + x as f32 / 10000.0 * r.width(), r.top() + y as f32 / 10000.0 * r.height());
    let norm = |q: Pos2| ((q.x - r.left()) / r.width() * 10000.0, (q.y - r.top()) / r.height() * 10000.0);
    p.rect_filled(resp.rect, 0.0, Color32::from_gray(225));
    p.rect_filled(r, 0.0, Color32::WHITE);
    // Raster (10 %) zur Orientierung
    for i in 1..10 {
        let x = r.left() + r.width() * i as f32 / 10.0;
        p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], Stroke::new(1.0, Color32::from_gray(238)));
    }
    p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Outside);
    let sel_col = Color32::from_rgb(223, 0, 11);
    for (i, e) in elems.iter().enumerate() {
        let gew = z.sel == Some(i);
        let [a, b, c, d] = koord(e);
        match e {
            TbOwned::Line(..) => {
                p.line_segment([s(a, b), s(c, d)], Stroke::new(if gew { 2.5 } else { 1.0 }, if gew { sel_col } else { Color32::BLACK }));
            }
            TbOwned::Logo(..) => {
                let rr = Rect::from_two_pos(s(a, b), s(c, d));
                p.rect_stroke(rr, 0.0, Stroke::new(1.0, if gew { sel_col } else { Color32::from_rgb(90, 80, 80) }), egui::StrokeKind::Inside);
                p.text(rr.center(), Align2::CENTER_CENTER, "Logo", FontId::proportional(11.0), Color32::from_rgb(200, 60, 40));
            }
            TbOwned::Text(_, _, _, _, t, al, lines) => {
                let rr = Rect::from_two_pos(s(a, b), s(c, d));
                p.rect_stroke(rr, 0.0, Stroke::new(1.0, if gew { sel_col } else { Color32::from_rgb(170, 190, 230) }), egui::StrokeKind::Inside);
                let text = t.trim_end_matches('\n');
                let mut px = (0.72 * rr.height() / (*lines).max(1) as f32).clamp(6.0, 22.0);
                let font = |px: f32| crate::canvas::times(px);
                let mut g = p.layout_no_wrap(text.to_string(), font(px), Color32::BLACK);
                if g.size().x > rr.width() - 2.0 && rr.width() > 4.0 {
                    px = (px * (rr.width() - 2.0) / g.size().x).max(5.0);
                    g = p.layout_no_wrap(text.to_string(), font(px), Color32::BLACK);
                }
                let x = match al {
                    Align::Left => rr.left() + 1.0,
                    Align::Center => rr.center().x - g.size().x / 2.0,
                    Align::Right => rr.right() - 1.0 - g.size().x,
                };
                let y = if *lines > 1 { rr.top() + 1.0 } else { rr.center().y - g.size().y / 2.0 };
                let col = if text.contains('%') { Color32::from_rgb(0, 0, 160) } else { Color32::BLACK };
                p.with_clip_rect(rr.expand(1.0)).galley(Pos2::new(x, y), g, col);
            }
        }
        if gew {
            for (x, y) in griffe(e) {
                p.rect_filled(Rect::from_center_size(s(x, y), Vec2::splat(7.0)), 0.0, sel_col);
            }
        }
    }
    // Bedienung: Klick wählt, Ziehen verschiebt bzw. zieht einen Griff (Umschalt: ohne Raster)
    let raster = !ui.input(|i| i.modifiers.shift);
    if resp.drag_started() {
        if let Some(q) = resp.interact_pointer_pos() {
            let am_griff = z.sel.and_then(|si| griffe(&elems[si]).iter().position(|(x, y)| s(*x, *y).distance(q) <= 6.0).map(|k| (si, Griff::Punkt(k))));
            let start = am_griff.or_else(|| treffer(elems, q, &s, 5.0).map(|i| (i, Griff::Ganz)));
            if let Some((i, g)) = start {
                z.sel = Some(i);
                z.drag = Some((i, g, norm(q), elems[i].clone()));
            }
        }
    }
    if let (Some((i, g, (x0, y0), orig)), Some(q)) = (&z.drag, resp.interact_pointer_pos()) {
        if resp.dragged() {
            let (x, y) = norm(q);
            if let Some(e) = elems.get_mut(*i) {
                *e = ziehen(orig, *g, (x - x0).round() as i32, (y - y0).round() as i32, raster);
            }
        }
    }
    if resp.drag_stopped() {
        z.drag = None;
    }
    if resp.clicked() {
        z.sel = resp.interact_pointer_pos().and_then(|q| treffer(elems, q, &s, 5.0));
    }
    // Tastatur: Entf löscht, Pfeile schieben um ein Rasterfeld
    if let Some(si) = z.sel {
        let (del, dx, dy) = ui.input(|i| {
            let k = |key| i.key_pressed(key);
            (k(egui::Key::Delete), k(egui::Key::ArrowRight) as i32 - k(egui::Key::ArrowLeft) as i32, k(egui::Key::ArrowDown) as i32 - k(egui::Key::ArrowUp) as i32)
        });
        let editing = ui.ctx().memory(|m| m.focused().is_some());
        if !editing {
            if del {
                elems.remove(si);
                z.sel = None;
            } else if dx != 0 || dy != 0 {
                let e = ziehen(&elems[si], Griff::Ganz, dx * RASTER, dy * RASTER, false);
                elems[si] = e;
            }
        }
    }
    ui.small(format!(
        "Schriftfeld {:.0} × {:.0} mm · Klick wählt, Ziehen verschiebt, rote Griffe ändern die Größe · Raster 0,5 % (Umschalt: frei) · Entf löscht, Pfeiltasten schieben",
        w_mm, h_mm
    ));
    ui.separator();
    // Eigenschaften des gewählten Elements
    ui.horizontal_top(|ui| {
        egui::ScrollArea::vertical().id_salt("sf_liste").max_height(170.0).min_scrolled_height(170.0).show(ui, |ui| {
            ui.set_width(260.0);
            // eigene senkrechte Anordnung: die Zeile außen (horizontal_top) würde die Einträge nebeneinander legen
            ui.vertical(|ui| for (i, e) in elems.iter().enumerate() {
                let name = match e {
                    TbOwned::Line(..) => format!("{:>2}  Linie", i + 1),
                    TbOwned::Logo(..) => format!("{:>2}  Logo", i + 1),
                    TbOwned::Text(_, _, _, _, t, ..) => format!("{:>2}  Text „{}“", i + 1, t.trim_end().replace('\n', " ")),
                };
                if ui.selectable_label(z.sel == Some(i), name).clicked() {
                    z.sel = Some(i);
                }
            });
        });
        ui.add_space(12.0); // kein Trennstrich: er dehnt sich in der Zeile auf die volle Höhe und lässt das Fenster wachsen
        ui.vertical(|ui| {
            let Some(si) = z.sel else {
                ui.weak("Kein Element gewählt");
                return;
            };
            let e = &mut elems[si];
            let mut k = koord(e);
            let pct = |ui: &mut egui::Ui, v: &mut i32, l: &str| {
                ui.label(l);
                let mut f = *v as f32 / 100.0;
                if ui.add(egui::DragValue::new(&mut f).range(0.0..=100.0).speed(0.1).max_decimals(2).suffix(" %")).changed() {
                    *v = (f * 100.0).round() as i32;
                }
            };
            let linie = ist_linie(e);
            egui::Grid::new("sf_koord").num_columns(4).show(ui, |ui| {
                pct(ui, &mut k[0], if linie { "Start X" } else { "Links" });
                pct(ui, &mut k[1], if linie { "Start Y" } else { "Oben" });
                ui.end_row();
                pct(ui, &mut k[2], if linie { "Ende X" } else { "Rechts" });
                pct(ui, &mut k[3], if linie { "Ende Y" } else { "Unten" });
                ui.end_row();
            });
            if k != koord(e) {
                setze_koord(e, k);
            }
            let [a, b, c, d] = koord(e);
            ui.small(format!("= {:.1} / {:.1} mm bis {:.1} / {:.1} mm", a as f32 / 10000.0 * w_mm, b as f32 / 10000.0 * h_mm, c as f32 / 10000.0 * w_mm, d as f32 / 10000.0 * h_mm));
            if let TbOwned::Text(_, _, _, _, t, al, lines) = e {
                ui.horizontal(|ui| {
                    ui.label("Text");
                    ui.add(egui::TextEdit::multiline(t).desired_rows(2).desired_width(300.0));
                    egui::ComboBox::from_id_salt("sf_makro").selected_text("Makro einfügen").width(110.0).show_ui(ui, |ui| {
                        for m in MAKROS {
                            if ui.selectable_label(false, *m).clicked() {
                                t.push_str(m);
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Ausrichtung");
                    for (a, n) in [(Align::Left, "Links"), (Align::Center, "Mittig"), (Align::Right, "Rechts")] {
                        ui.radio_value(al, a, n);
                    }
                    ui.add_space(12.0); // kein Trennstrich: er dehnt sich in der Zeile auf die volle Höhe und lässt das Fenster wachsen
                    ui.label("Zeilen");
                    // wie das Original höchstens 8 Zeilen (FUS › GRS Feld 9)
                    ui.add(egui::DragValue::new(lines).range(1..=8));
                });
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(id, z));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verschieben_und_griffe() {
        let t = TbOwned::Text(1000, 2000, 3000, 6000, "%Seite".into(), Align::Center, 1);
        // ganz verschieben: Größe bleibt, linke obere Ecke auf dem Raster
        assert_eq!(koord(&ziehen(&t, Griff::Ganz, 1012, -488, true)), [2000, 1500, 4000, 5500]);
        // Griff rechts unten
        assert_eq!(koord(&ziehen(&t, Griff::Punkt(2), 260, 9000, true)), [1000, 2000, 3250, 10000]);
        // Linie: Endpunkt frei (ohne Raster)
        let l = TbOwned::Line(0, 5000, 10000, 5000);
        assert_eq!(koord(&ziehen(&l, Griff::Punkt(1), -1234, 7, false)), [0, 5000, 8766, 5007]);
        assert_eq!(griffe(&t).len(), 4);
        assert_eq!(griffe(&l), vec![(0, 5000), (10000, 5000)]);
        // nicht über den Rand hinaus verschieben
        assert_eq!(koord(&ziehen(&t, Griff::Ganz, -5000, 9000, true)), [0, 6000, 2000, 10000]);
        assert_eq!(koord(&ziehen(&l, Griff::Ganz, 300, 0, true)), [0, 5000, 10000, 5000]);
        assert_eq!(fang(74), 50);
        assert_eq!(fang(76), 100);
    }

    #[test]
    fn treffer_linie_vor_rechteck() {
        let el = vec![TbOwned::Text(0, 0, 10000, 10000, "a".into(), Align::Left, 1), TbOwned::Text(2000, 2000, 4000, 4000, "b".into(), Align::Left, 1), TbOwned::Line(0, 5000, 10000, 5000)];
        // 1000 × 100 Pixel
        let s = |x: i32, y: i32| Pos2::new(x as f32 / 10.0, y as f32 / 100.0);
        assert_eq!(treffer(&el, Pos2::new(300.0, 30.0), &s, 5.0), Some(1));
        assert_eq!(treffer(&el, Pos2::new(800.0, 52.0), &s, 5.0), Some(2));
        assert_eq!(treffer(&el, Pos2::new(800.0, 80.0), &s, 5.0), Some(0));
        assert_eq!(treffer(&el, Pos2::new(1200.0, 80.0), &s, 5.0), None);
    }

    /// Vorschau ohne Fenster zeichnen (keine Panik, Elemente bleiben unverändert ohne Eingabe).
    #[test]
    fn vorschau_zeichnen() {
        let ctx = egui::Context::default();
        crate::setup_style(&ctx);
        let mut el = titleblock::default_owned();
        let vorher = el.clone();
        let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 900.0))), ..Default::default() };
        for _ in 0..2 {
            let _ = ctx.run_ui(input(), |ui| super::ui(ui, &mut el, 277.0, 15.0));
        }
        assert_eq!(el, vorher);
    }

    #[test]
    fn neue_elemente() {
        assert!(matches!(neu(0), TbOwned::Line(..)));
        assert!(matches!(neu(1), TbOwned::Text(_, _, _, _, ref t, Align::Center, 1) if t == "Text"));
        assert!(matches!(neu(2), TbOwned::Logo(..)));
    }
}
