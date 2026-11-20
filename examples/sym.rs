//! Diagnose: SYM-Block (Projektsymbol) mit Unterblöcken ausgeben.
use elektroplanung::eprfile::{Block, EprFile};
fn show(b: &Block, d: usize) {
    let txt = String::from_utf8_lossy(&b.data).replace('\0', "|");
    let ints: Vec<String> = (0..(b.data.len() / 4).min(24)).map(|i| b.i32(i).to_string()).collect();
    println!("{}{} len={} ints=[{}] txt='{}'", "  ".repeat(d), b.tag_str(), b.data.len(), ints.join(" "), txt.chars().take(90).collect::<String>());
    if !b.is("GRS") { for k in &b.kids { show(k, d + 1); } } else {
        for e in b.data.chunks_exact(0x50) {
            let v: Vec<String> = (0..20).map(|i| i32::from_le_bytes(e[i*4..i*4+4].try_into().unwrap()).to_string()).collect();
            println!("{}  typ={} [{}]", "  ".repeat(d), e[0x3c], v.join(" "));
        }
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for s in f.root("PRJ").unwrap().kids_of("SYM") {
        let name = String::from_utf8_lossy(&s.data[..40]).trim_end_matches('\0').to_string();
        if name.starts_with(&a[2]) { show(s, 0); }
    }
}
