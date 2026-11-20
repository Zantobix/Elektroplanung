//! Diagnose: Unterblöcke von Blättern, die beim Import kein Objekt ergeben.
use elektroplanung::{epr, eprfile::EprFile};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let p = epr::import_file(&f).unwrap();
    for an in &p.anlagen { for s in &an.sheets { if let Some(r) = &s.raw { for (i, m) in r.map.iter().enumerate() { if m.is_none() {
        let k = &r.block.kids[i];
        let v: Vec<String> = (0..20).map(|j| k.i32(j).to_string()).collect();
        println!("{} / {}: {} len={} [{}] kids={:?}", an.name, s.name, k.tag_str(), k.data.len(), v.join(" "), k.kids.iter().map(|c| c.tag_str()).collect::<Vec<_>>());
    } } } } }
}
