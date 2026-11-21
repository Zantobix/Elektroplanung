//! Leitung › Darstellung › „Stift“ wie im Original (TLeitung: Auswahl SListe und Knopf Taste1 = neuer Stift,
//! Handler Leitung1Taste1Click). Die Leitung übernimmt Breite und Strichart des gewählten Stiftes; der Nachbau
//! speichert diese Werte an der Leitung (LTG Feld 5 Breite, Feld 10 Strichart).

use crate::model::Stift;
use eframe::egui;

/// Stift, der Breite und Strichart der Leitung hat (erster passender).
pub fn stift_index(pens: &[Stift], width_mm: f32, style: u8) -> Option<usize> {
    pens.iter().position(|p| (p.width - width_mm).abs() < 0.005 && p.style == style)
}

fn mm(t: &str) -> f32 {
    t.trim().replace(',', ".").parse().unwrap_or(0.25)
}

/// Zeile „Stift“: Auswahlliste der Projektstifte und Bildknopf „Neuer Stift“. Rückgabe true = Knopf gedrückt.
pub fn stift_ui(ui: &mut egui::Ui, pens: &[Stift], width: &mut String, style: &mut u8) -> bool {
    let cur = stift_index(pens, mm(width), *style);
    let text = cur.and_then(|i| pens.get(i)).map(|p| p.name.clone()).unwrap_or_else(|| format!("Benutzerdefiniert ({} mm)", width.trim()));
    let mut neu = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("ltg_stift").width(150.0).selected_text(text).show_ui(ui, |ui| {
            for (i, p) in pens.iter().enumerate() {
                if ui.selectable_label(cur == Some(i), format!("{} ({:.2} mm)", p.name, p.width).replace('.', ",")).clicked() {
                    *width = format!("{:.2}", p.width).replace('.', ",");
                    *style = p.style;
                }
            }
        });
        // Taste1 (26 × 26, Bildknopf ohne Hinweis) ruft ToolButton7Click = „Neuer Stift“ auf
        if ui.add(egui::Button::image(crate::icons::image(crate::icons::Icon::NewPen, 18.0)).min_size(egui::Vec2::new(26.0, 26.0))).on_hover_text("Neuer Stift").clicked() {
            neu = true;
        }
    });
    neu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stift_nach_breite_und_strichart() {
        let pens = vec![Stift { name: "a".into(), style: 0, color: [0, 0, 0], width: 0.25 }, Stift { name: "PE".into(), style: 2, color: [0, 0, 0], width: 0.5 }];
        assert_eq!(stift_index(&pens, 0.5, 2), Some(1));
        assert_eq!(stift_index(&pens, 0.5, 0), None);
        assert_eq!(mm("0,35"), 0.35);
    }
}
