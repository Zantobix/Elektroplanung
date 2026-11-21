//! Diagnose: Projekt ohne Fenster als PDF ausgeben (wie Datei › Export › PDF) und auf Wunsch jedes Blatt zum
//! Vergleich als PNG (Bildexport).
//! cargo run --release --example pdf_export -- <datei.epr> <ausgabe.pdf> [png-präfix [dpi]]
use elektroplanung::{bildexport, epr, model::SheetRef, pdf, symbols};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    // Bibliothek wie beim PDF-Export des Programms: Datenbank + Symbole des Projekts
    let mut lib = symbols::find_db_dir(None).and_then(|d| symbols::load_db(&d).ok()).unwrap_or_default();
    println!("Bibliothek: {} Symbole", lib.len());
    let fehlen: Vec<_> = p.symdefs.iter().filter(|d| symbols::find(&lib, &d.key).is_none()).cloned().collect();
    lib.extend(fehlen);
    let (bytes, n) = pdf::render_headless(&p, &lib);
    std::fs::write(&a[2], &bytes).unwrap();
    println!("{}: {n} Seiten, {} Byte", a[2], bytes.len());
    if let Some(prefix) = a.get(3) {
        let dpi: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
        let mut k = 0;
        for (ai, an) in p.anlagen.iter().enumerate() {
            for si in 0..an.sheets.len() {
                k += 1;
                if let Some(img) = bildexport::render_headless(&p, &lib, SheetRef { anlage: ai, sheet: si }, dpi, true) {
                    img.save(format!("{prefix}-{k:02}.png")).unwrap();
                }
            }
        }
    }
}
