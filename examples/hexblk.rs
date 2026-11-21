//! Diagnose: Blöcke unter einem Pfad (z. B. PRJ/SIT, PRJ/ANL/BLT) mit Rohdaten (hex) und i32-Feldern ausgeben.
//! cargo run --example hexblk -- <datei.epr> <PFAD> [max]
use elektroplanung::eprfile::{Block, EprFile};
fn walk(b: &Block, path: &str, want: &str, n: &mut usize, max: usize) {
    let p = if path.is_empty() { b.tag_str().to_string() } else { format!("{path}/{}", b.tag_str()) };
    if p == want && *n < max {
        *n += 1;
        let kids: Vec<String> = b.kids.iter().map(|k| k.tag_str().to_string()).collect();
        println!("{p} #{} len={} kids={}", *n, b.data.len(), kids.join(","));
        for (i, c) in b.data.chunks(16).enumerate() {
            let h: Vec<String> = c.iter().map(|x| format!("{x:02x}")).collect();
            let s: String = c.iter().map(|x| if (32..127).contains(x) { *x as char } else { '.' }).collect();
            println!("  {:4x}: {:<48} {s}", i * 16, h.join(" "));
        }
        let v: Vec<String> = (0..b.data.len() / 4).map(|i| format!("{i}={}", b.i32(i))).collect();
        println!("  i32: {}", v.join(" "));
    }
    for k in &b.kids {
        walk(k, &p, want, n, max);
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let max = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let mut n = 0;
    for r in &f.roots {
        walk(r, "", &a[2], &mut n, max);
    }
}
