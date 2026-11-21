//! Diagnose: Textfelder des Schriftfelds mit Rechteck in mm (Breite/Höhe des Schriftfelds als Argumente).
//! cargo run --example schriftfeld_texte -- <datei> [<breite mm> <höhe mm>]
use elektroplanung::{epr, titleblock::TbOwned};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    let w: f32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(195.0);
    let h: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(15.0);
    let tb = if p.titleblock.is_empty() { elektroplanung::titleblock::default_owned() } else { p.titleblock.clone() };
    for e in tb {
        if let TbOwned::Text(x0, y0, x1, y1, tpl, align, lines) = e {
            let (ax, ay, bx, by) = (x0 as f32 * w / 1e4, y0 as f32 * h / 1e4, x1 as f32 * w / 1e4, y1 as f32 * h / 1e4);
            println!("{tpl:?} ({ax:.2},{ay:.2})–({bx:.2},{by:.2}) b {:.2} h {:.2} {align:?} zeilen {lines}", bx - ax, by - ay);
        }
    }
}
