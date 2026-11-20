//! Diagnose: VSY-Blöcke mit bestimmtem BMK samt Unterblöcken ausgeben.
use elektroplanung::eprfile::{Block, EprFile};
fn hex(d: &[u8]) -> String { d.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join("") }
fn walk(b: &Block, want: &str) {
    if b.is("VSY") {
        let tit = b.kid("TIT").map(|t| String::from_utf8_lossy(&t.data).trim_end_matches('\0').to_string()).unwrap_or_default();
        if tit == want {
            let v: Vec<String> = (0..28).map(|i| b.i32(i).to_string()).collect();
            println!("VSY {tit}: {}", v.join(" "));
            for k in &b.kids { println!("  {} len={} {} '{}'", k.tag_str(), k.data.len(), hex(&k.data[..k.data.len().min(48)]), String::from_utf8_lossy(&k.data).replace('\0', "|")); for kk in &k.kids { println!("    {} {}", kk.tag_str(), hex(&kk.data)); } }
        }
    }
    for k in &b.kids { walk(k, want); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for r in &f.roots { walk(r, &a[2]); }
}
