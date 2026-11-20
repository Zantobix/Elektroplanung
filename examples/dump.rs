//! Diagnose: Felder von Blöcken einer .epr-Datei ausgeben. cargo run --example dump -- <datei> <TAG> [von] [bis]
use elektroplanung::eprfile::{Block, EprFile};
fn walk(b: &Block, path: &str, tag: &str, from: usize, to: usize) {
    let p = format!("{path}/{}", b.tag_str());
    if b.is(tag) {
        let name: String = b.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
        let v: Vec<String> = (from..to).map(|i| b.i32(i).to_string()).collect();
        let kids: Vec<String> = b.kids.iter().map(|k| k.tag_str().to_string()).collect();
        println!("{p} len={} '{name}' [{from}..{to}] {} kids={}", b.data.len(), v.join(" "), kids.join(","));
    }
    for k in &b.kids {
        walk(k, &p, tag, from, to);
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let from = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let to = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(16);
    for r in &f.roots {
        walk(r, "", &a[2], from, to);
    }
}
