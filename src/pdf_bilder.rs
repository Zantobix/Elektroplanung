//! Bildobjekte im PDF: Die Zeichenroutine malt Bilder als Textur-Netz (`Shape::Mesh`); das PDF setzt an dessen
//! Stelle die Pixel des Bildes als Image-XObject (RGB, Flate-gepackt, Transparenz als SMask). Jedes Bild wird nur
//! einmal eingebettet, auch wenn es auf mehreren Blättern steht.

use eframe::egui::{self, ColorImage, TextureId};
use std::collections::HashMap;
use std::sync::Arc;

/// Pixel der Bildtexturen einer Seite (Textur der Zeichenroutine → Bild)
pub type Pixel = HashMap<TextureId, Arc<ColorImage>>;

/// Die Bilder eines PDF
#[derive(Default)]
pub struct Bilder {
    liste: Vec<Arc<ColorImage>>,
    nach_textur: HashMap<TextureId, usize>,
}

impl Bilder {
    /// Bild zur Textur vormerken; Rückgabe: Name im Ressourcenverzeichnis (`Im1` …).
    pub fn nutzen(&mut self, id: TextureId, bild: &Arc<ColorImage>) -> String {
        let k = *self.nach_textur.entry(id).or_insert_with(|| {
            self.liste.push(bild.clone());
            self.liste.len() - 1
        });
        format!("Im{}", k + 1)
    }

    pub fn anzahl(&self) -> usize {
        self.liste.len()
    }

    /// PDF-Objekte aller Bilder, durchnummeriert ab `erste` (je Bild das XObject, bei Transparenz zusätzlich die
    /// SMask). Rückgabe: Objekte und Inhalt des XObject-Verzeichnisses (`/Im1 20 0 R …`).
    pub fn objekte(&self, erste: usize) -> (Vec<Vec<u8>>, String) {
        let mut objs = vec![];
        let mut verz = String::new();
        for (k, b) in self.liste.iter().enumerate() {
            let n = erste + objs.len();
            let [w, h] = b.size;
            let rgb: Vec<u8> = b.pixels.iter().flat_map(|c| alpha_weg(*c)).collect();
            let transparent = b.pixels.iter().any(|c| c.a() < 255);
            let smask = if transparent { format!("/SMask {} 0 R", n + 1) } else { String::new() };
            objs.push(crate::pdf_schriften::strom(
                &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 {smask}"),
                &rgb,
            ));
            if transparent {
                let a: Vec<u8> = b.pixels.iter().map(|c| c.a()).collect();
                objs.push(crate::pdf_schriften::strom(&format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8"), &a));
            }
            verz.push_str(&format!(" /Im{} {n} 0 R", k + 1));
        }
        (objs, verz)
    }
}

/// Farbe ohne Vormultiplikation (egui speichert vormultipliziertes Alpha)
fn alpha_weg(c: egui::Color32) -> [u8; 3] {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    [r, g, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bild_einmal_eingebettet_mit_maske() {
        let mut ci = ColorImage::new([2, 1], vec![egui::Color32::RED; 2]);
        ci.pixels[1] = egui::Color32::from_rgba_unmultiplied(0, 0, 255, 128);
        let ci = Arc::new(ci);
        let mut b = Bilder::default();
        let id = TextureId::User(7);
        assert_eq!(b.nutzen(id, &ci), "Im1");
        assert_eq!(b.nutzen(id, &ci), "Im1");
        assert_eq!(b.nutzen(TextureId::User(8), &ci), "Im2");
        let (objs, verz) = b.objekte(10);
        // je Bild XObject + SMask
        assert_eq!(objs.len(), 4);
        assert_eq!(verz, " /Im1 10 0 R /Im2 12 0 R");
        let t = String::from_utf8_lossy(&objs[0]);
        assert!(t.contains("/Width 2 /Height 1") && t.contains("/SMask 11 0 R"), "{t}");
    }
}
