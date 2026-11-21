//! Diagnose: .epr lesen und unverändert wieder speichern (Rundreise prüfen). cargo run --example resave -- <ein.epr> <aus.epr>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = elektroplanung::epr::import(std::path::Path::new(&a[1])).unwrap();
    elektroplanung::epr_write::save(&p, std::path::Path::new(&a[2])).unwrap();
    for w in elektroplanung::epr_projekt::nicht_gespeichert(&p) {
        println!("nicht gespeichert: {w}");
    }
}
