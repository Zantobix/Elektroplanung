//! Alle Blätter einer .epr-Datei als PDF schreiben (ohne Fenster, zum Vergleich mit Ausdrucken des Originals).
//! cargo run --release --example pdf -- <datei.epr> <ziel.pdf>
use elektroplanung::{epr, pdf};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    let mut lib = p.symdefs.clone();
    lib.dedup_by(|a, b| a.key == b.key);
    let (bytes, n) = pdf::render_headless(&p, &lib);
    std::fs::write(&a[2], bytes).unwrap();
    println!("{n} Seiten → {}", a[2]);
}
