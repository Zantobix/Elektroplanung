//! Diagnose: Rohdaten eines Unterblocks unter Blöcken mit Kennung TAG ausgeben (cargo run --example rawkid -- <epr> <TAG> <KID>).
use elektroplanung::eprfile::{Block, EprFile};
fn walk(b: &Block, tag: &str, kid: &str) {
    if b.is(tag) {
        for k in b.kids.iter().filter(|k| k.is(kid)) {
            let s: String = k.data.iter().map(|c| if *c == 0 { '|' } else if *c < 32 { '~' } else { *c as char }).collect();
            println!("{} [{}] {}", b.tag_str(), k.data.len(), s.chars().take(600).collect::<String>());
        }
    }
    for k in &b.kids { walk(k, tag, kid); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for r in &f.roots { walk(r, &a[2], &a[3]); }
}
