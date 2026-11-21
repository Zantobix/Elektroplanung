//! Diagnose: alle Texte eines Blattes mit Lage, Größe, Textbox und Einstellungen.
//! cargo run --example texte -- <datei> <blattnummer> [<mindestgröße mm>]
use elektroplanung::{epr, model::*};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    let nr: u32 = a[2].parse().unwrap();
    let min: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    for an in &p.anlagen {
        for s in an.sheets.iter().filter(|s| s.number == nr) {
            println!("== {} / {} ({}) Anlage von {:?}, Projekt von {:?}", an.name, s.name, s.number, an.set.von, p.von);
            for o in &s.objects {
                if let Kind::Text { pos, text, size, align, vertical, style } = &o.kind {
                    if *size < min {
                        continue;
                    }
                    let r = style.rect.map(|(a, b)| format!("box ({:.2},{:.2})–({:.2},{:.2})", a.x, a.y, b.x, b.y)).unwrap_or_default();
                    println!("{:?} pos ({:.2},{:.2}) size {:.2} {:?} hoch={} wrap={} rahmen={} {r}", text, pos.x, pos.y, size, align, vertical, style.wrap, style.frame);
                }
            }
        }
    }
}
