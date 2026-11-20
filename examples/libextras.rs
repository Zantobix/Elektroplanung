//! Prüfung: Alternativtexte/Optionen der Bibliothek gegen den SYM-Block desselben Symbols in W2.epr.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lib = elektroplanung::symbols::load_db(std::path::Path::new(&a[1])).unwrap();
    let n = lib.iter().filter(|d| !d.flag_names.is_empty() || !d.alt_sets.is_empty()).count();
    println!("Bibliothek: {} Symbole, {} mit Optionen/Alternativtexten", lib.len(), n);
    for d in lib.iter().filter(|d| d.name == a[2]) {
        println!("{}: Optionen {:?} Alternativtexte {:?}", d.key, d.flag_names, d.alt_sets);
    }
    let p = elektroplanung::epr::import(std::path::Path::new(&a[3])).unwrap();
    for d in p.symdefs.iter().filter(|d| d.name == a[2]) {
        println!("W2 {}: Optionen {:?} Alternativtexte {:?}", d.key, d.flag_names, d.alt_sets);
    }
}
