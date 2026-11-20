//! Diagnose: Text-Zeichenelemente (ZCN Typ 6/11) mit allen Feldern.
use elektroplanung::eprfile::{Block, EprFile};
fn walk(b: &Block, n: &mut usize) {
    if b.is("ZCN") && matches!(b.data.get(0x3c), Some(6) | Some(11)) && *n < 30 {
        *n += 1;
        let v: Vec<String> = (0..20).map(|i| b.i32(i).to_string()).collect();
        let ks: Vec<String> = b.kids.iter().map(|k| format!("{}[{}]:{}", k.tag_str(), k.data.len(), String::from_utf8_lossy(&k.data).replace('\0', "|").chars().take(40).collect::<String>())).collect();
        println!("typ={} [{}] {:?}", b.data[0x3c], v.join(" "), ks);
    }
    for k in &b.kids { walk(k, n); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let mut n = 0;
    for r in &f.roots { walk(r, &mut n); }
}
