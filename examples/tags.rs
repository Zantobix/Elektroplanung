//! Diagnose: Häufigkeit der Blockpfade (bis Tiefe n) und ZCN-Typen. cargo run --example tags -- <datei>
use elektroplanung::eprfile::{Block, EprFile};
use std::collections::BTreeMap;
fn walk(b: &Block, path: &str, m: &mut BTreeMap<String, usize>, z: &mut BTreeMap<u8, usize>) {
    let p = format!("{path}/{}", b.tag_str());
    *m.entry(p.clone()).or_default() += 1;
    if b.is("ZCN") && b.data.len() >= 0x50 { *z.entry(b.data[0x3c]).or_default() += 1; }
    for k in &b.kids { walk(k, &p, m, z); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let (mut m, mut z) = (BTreeMap::new(), BTreeMap::new());
    for r in &f.roots { walk(r, "", &mut m, &mut z); }
    for (k, v) in m { println!("{v:6} {k}"); }
    println!("ZCN-Typen: {z:?}");
}
