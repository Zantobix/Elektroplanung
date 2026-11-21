//! Textmaße ohne Zeichenfläche: Breite und Höhe eines Textes in mm, so wie `canvas` ihn mit Times New Roman
//! zeichnet. egui 0.37 setzt die Schriftgröße = Geviert (units per em) und die Zeilenhöhe = Ober- + Unterlänge +
//! Zeilenabstand (Times New Roman: (1825 + 443 + 87) / 2048 ≈ 1,15 Geviert). Die Zeichenbreiten sind die der Times
//! (Adobe-Metrik, für Times New Roman und Liberation Serif gleich) in 1/1000 Geviert.
//!
//! Umriss, Treffertest und Griffe eines Textes beruhen darauf (`text_umriss`), damit sie auch nach dem Vergrößern
//! zur gezeichneten Schrift passen. Texte mit eigener, installierter Schrift (TextStyle::font) werden mit deren
//! Metrik gemessen (`schriftmass`), Times bleibt der Rückfall.

use crate::model::{Align, TextStyle, P};

/// Zeilenhöhe in Geviert (egui: Ober- + Unterlänge + Zeilenabstand der Times New Roman)
pub const ZEILE: f32 = 1.15;

/// Breite eines Zeichens der Times (Normal) in 1/1000 Geviert.
fn zeichen(c: char) -> u16 {
    const ASCII: [u16; 95] = [
        250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, // ' ' bis '/'
        500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, // '0' bis '?'
        921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, // '@' bis 'O'
        556, 722, 667, 556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500, // 'P' bis '_'
        333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500, // '`' bis 'o'
        500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541, // 'p' bis '~'
    ];
    match c {
        ' '..='~' => ASCII[c as usize - 32],
        'Ä' | 'Ö' | 'Ü' | 'À'..='Æ' | 'Ò'..='Ö' | 'Ù'..='Ý' => 722,
        'ä' | 'à'..='å' => 444,
        'ö' | 'ü' | 'ò'..='õ' | 'ù'..='û' | 'ß' | 'µ' | '€' | '±' => 500,
        'é' | 'è' | 'ê' | 'ë' | 'ç' => 444,
        '°' => 400,
        '²' | '³' | '¹' => 300,
        '×' | '÷' | '¬' => 564,
        '\t' => 1000,
        _ => 600,
    }
}

/// Breite einer Zeile in mm bei der Schriftgröße `size` (mm, Geviert).
pub fn breite(zeile: &str, size: f32, bold: bool, italic: bool) -> f32 {
    let w: u32 = zeile.chars().map(|c| zeichen(c) as u32).sum();
    // Fett ist im Mittel gut 5 % breiter, Kursiv etwas schmaler als Normal
    let f = match (bold, italic) {
        (true, _) => 1.05,
        (false, true) => 0.97,
        _ => 1.0,
    };
    w as f32 / 1000.0 * size * f
}

/// Zeilenhöhe in mm.
pub fn zeilenhoehe(size: f32) -> f32 {
    size * ZEILE
}

/// Größe (Breite der längsten Zeile, Höhe aller Zeilen) eines mehrzeiligen Textes in mm.
pub fn groesse(text: &str, size: f32, bold: bool, italic: bool) -> (f32, f32) {
    groesse_mit(None, text, size, bold, italic)
}

/// Größe mit der Metrik einer Schrift (`None` = Times); der Schrift fehlende Zeichen zählen mit Times-Breite.
pub fn groesse_mit(m: Option<&crate::schriftmass::Metrik>, text: &str, size: f32, bold: bool, italic: bool) -> (f32, f32) {
    let zeilen: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').map(|l| l.trim_end_matches('\r')).collect() };
    let Some(m) = m else {
        let w = zeilen.iter().map(|l| breite(l, size, bold, italic)).fold(0.0, f32::max);
        return (w, zeilen.len() as f32 * zeilenhoehe(size));
    };
    let zeile = |l: &str| -> f32 { l.chars().map(|c| m.zeichen(c).map(|w| w * size).unwrap_or_else(|| breite(c.encode_utf8(&mut [0; 4]), size, bold, italic))).sum() };
    let w = zeilen.iter().map(|l| zeile(l)).fold(0.0, f32::max);
    (w, zeilen.len() as f32 * m.zeile * size)
}

/// Umriss (links oben, rechts unten) eines Textes in mm, wie `canvas::draw_text_box` ihn zeichnet: ohne Textbox
/// an `pos` ausgerichtet (waagerecht mittig zur Höhe, hochkant von `pos` nach oben), mit Textbox in ihr nach
/// Ausrichtung und senkrecht mittig. Mit Textbox zählt die Box selbst mit (Rahmen, Markierung des Originals).
pub fn text_umriss(pos: P, text: &str, size: f32, align: Align, vertical: bool, st: &TextStyle) -> (P, P) {
    umriss_mit(crate::schriftmass::metrik(&st.font, st.bold, st.italic).as_deref(), pos, text, size, align, vertical, st)
}

/// Wie `text_umriss`, mit der Metrik einer Schrift (`None` = Times).
pub fn umriss_mit(m: Option<&crate::schriftmass::Metrik>, pos: P, text: &str, size: f32, align: Align, vertical: bool, st: &TextStyle) -> (P, P) {
    let (mut w, mut h) = groesse_mit(m, text, size, st.bold, st.italic);
    if let (true, Some((a, b))) = (st.wrap, st.rect) {
        // mit Umbruch: Maße der umbrochenen Zeilen (wie canvas::draw_text_box)
        let bw = if vertical { (a.y - b.y).abs() } else { (a.x - b.x).abs() };
        let br = |s: &str| groesse_mit(m, s, size, st.bold, st.italic).0;
        let zeilen = crate::umbruch::text_umbrechen(text, (bw - 1.0).max(0.5), &br);
        w = zeilen.iter().map(|l| br(l)).fold(0.0, f32::max);
        h = zeilen.len().max(1) as f32 * groesse_mit(m, "", size, st.bold, st.italic).1;
    }
    let Some((a, b)) = st.rect else {
        let dx = match align {
            Align::Left => 0.0,
            Align::Center => -w / 2.0,
            Align::Right => -w,
        };
        return if vertical { (P::new(pos.x - h / 2.0, pos.y - dx - w), P::new(pos.x + h / 2.0, pos.y - dx)) } else { (P::new(pos.x + dx, pos.y - h / 2.0), P::new(pos.x + dx + w, pos.y + h / 2.0)) };
    };
    let (mn, mx) = (P::new(a.x.min(b.x), a.y.min(b.y)), P::new(a.x.max(b.x), a.y.max(b.y)));
    let c = P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
    let (tmn, tmx) = if vertical {
        let (y0, y1) = match align {
            Align::Left => (mx.y - w, mx.y),
            Align::Center => (c.y - w / 2.0, c.y + w / 2.0),
            Align::Right => (mn.y, mn.y + w),
        };
        (P::new(c.x - h / 2.0, y0), P::new(c.x + h / 2.0, y1))
    } else {
        let (x0, x1) = match align {
            Align::Left => (mn.x, mn.x + w),
            Align::Center => (c.x - w / 2.0, c.x + w / 2.0),
            Align::Right => (mx.x - w, mx.x),
        };
        (P::new(x0, c.y - h / 2.0), P::new(x1, c.y + h / 2.0))
    };
    (P::new(mn.x.min(tmn.x), mn.y.min(tmn.y)), P::new(mx.x.max(tmx.x), mx.y.max(tmx.y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breiten_der_times() {
        // "Raum 2" = R 667 + a 444 + u 500 + m 778 + Leer 250 + 2 500 = 3139/1000 Geviert
        assert!((breite("Raum 2", 10.0, false, false) - 31.39).abs() < 1e-3);
        assert!(breite("Ä", 1.0, false, false) > breite("a", 1.0, false, false));
        let (w, h) = groesse("ab\ncde", 2.0, false, false);
        assert!((w - 2.0 * 1.388).abs() < 1e-3 && (h - 2.0 * 2.0 * ZEILE).abs() < 1e-4);
    }

    /// Eine breitere Schrift (synthetisch: die dicktengleiche egui-Schrift Hack) ergibt einen breiteren Umriss als
    /// Times; Zeichen, die ihr fehlen, zählen mit Times-Breite.
    #[test]
    fn umriss_in_eigener_schrift() {
        let defs = eframe::egui::FontDefinitions::default();
        let d = defs.font_data.get("Hack").unwrap();
        let m = crate::schriftmass::Metrik::aus_daten(std::sync::Arc::new(d.bytes().to_vec()), d.index).unwrap();
        let st = TextStyle::default();
        let (a, b) = umriss_mit(None, P::new(0.0, 0.0), "iiii", 10.0, Align::Left, false, &st);
        let (c, e) = umriss_mit(Some(&m), P::new(0.0, 0.0), "iiii", 10.0, Align::Left, false, &st);
        assert!((b.x - a.x - 4.0 * 2.78).abs() < 1e-3);
        assert!(e.x - c.x > 1.8 * (b.x - a.x), "{a:?} {b:?} / {c:?} {e:?}");
        assert!((e.y - c.y - m.zeile * 10.0).abs() < 1e-4);
        // fehlendes Zeichen: Times-Breite
        let (w, _) = groesse_mit(Some(&m), "\u{10FFFD}", 10.0, false, false);
        assert!((w - breite("\u{10FFFD}", 10.0, false, false)).abs() < 1e-4);
        // nicht installierte Schrift: Times
        let st = TextStyle { font: "Gibt Es Nicht 4711".into(), ..Default::default() };
        assert_eq!(text_umriss(P::new(0.0, 0.0), "iiii", 10.0, Align::Left, false, &st), (a, b));
    }

    /// Mit installierter Arial (Windows): Umriss nach Arial-Maßen, passend zur von egui gezeichneten Breite.
    #[test]
    fn umriss_in_arial_wie_gezeichnet() {
        use eframe::egui;
        if crate::schriftmass::metrik("Arial", false, false).is_none() {
            return;
        }
        let text = "Wiiiiide Unterverteilung 230V";
        let st = TextStyle { font: "Arial".into(), ..Default::default() };
        let (a, b) = text_umriss(P::new(0.0, 0.0), text, 10.0, Align::Left, false, &st);
        let (ta, tb) = text_umriss(P::new(0.0, 0.0), text, 10.0, Align::Left, false, &TextStyle::default());
        assert!(((b.x - a.x) - (tb.x - ta.x)).abs() > 1.0, "anderer Umriss als Times");
        // gezeichnet bei 10 px je mm
        let ctx = egui::Context::default();
        crate::setup_style(&ctx);
        let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(2000.0))), ..Default::default() };
        let _ = ctx.run_ui(input(), |_| {});
        let mut g = egui::Vec2::ZERO;
        let _ = ctx.run_ui(input(), |ui| {
            g = ui.painter().layout_no_wrap(text.into(), crate::schriften::font_id("Arial", 100.0, false, false), egui::Color32::BLACK).size() / 10.0;
        });
        assert!(((b.x - a.x) - g.x).abs() < 0.02 * g.x, "Breite {} gezeichnet {}", b.x - a.x, g.x);
        assert!(((b.y - a.y) - g.y).abs() < 0.03 * g.y, "Höhe {} gezeichnet {}", b.y - a.y, g.y);
    }

    #[test]
    fn umriss_mit_und_ohne_textbox() {
        let st = TextStyle::default();
        // links ausgerichtet, ohne Box: beginnt an pos, senkrecht mittig
        let (a, b) = text_umriss(P::new(10.0, 20.0), "Raum 2", 10.0, Align::Left, false, &st);
        assert!(a.dist(P::new(10.0, 20.0 - 5.75)) < 1e-3 && b.dist(P::new(41.39, 25.75)) < 1e-3);
        // kleine Box: der gezeichnete Text ragt hinaus und zählt mit
        let st = TextStyle { rect: Some((P::new(10.0, 15.0), P::new(20.0, 25.0))), ..Default::default() };
        let (a, b) = text_umriss(P::new(10.0, 20.0), "Raum 2", 10.0, Align::Left, false, &st);
        assert!(a.dist(P::new(10.0, 14.25)) < 1e-3 && b.dist(P::new(41.39, 25.75)) < 1e-3);
    }
}
