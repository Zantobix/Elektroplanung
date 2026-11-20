//! Diagnose: zwei .epr-Dateien blockweise vergleichen (gleiche Baumstruktur vorausgesetzt).
use elektroplanung::eprfile::{Block, EprFile};
fn flat<'a>(b: &'a Block, path: String, out: &mut Vec<(String, &'a Block)>) {
    let p = format!("{path}/{}", b.tag_str());
    out.push((p.clone(), b));
    for k in &b.kids { flat(k, p.clone(), out); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (fa, fb) = (EprFile::read(a[1].as_ref()).unwrap(), EprFile::read(a[2].as_ref()).unwrap());
    let (mut la, mut lb) = (vec![], vec![]);
    for r in &fa.roots { flat(r, String::new(), &mut la); }
    for r in &fb.roots { flat(r, String::new(), &mut lb); }
    println!("Blöcke: {} / {}", la.len(), lb.len());
    let (mut i, mut j, mut shown) = (0, 0, 0);
    while i < la.len() && j < lb.len() && shown < 40 {
        let (pa, ba) = &la[i]; let (pb, bb) = &lb[j];
        if pa != pb {
            println!("STRUKTUR {i}: {pa}  vs  {pb}");
            if lb.get(j + 1).map(|x| &x.0) == Some(pa) { println!("  + neu: {pb} {:?}", String::from_utf8_lossy(&bb.data)); j += 1; shown += 1; continue; }
            if la.get(i + 1).map(|x| &x.0) == Some(pb) { println!("  - weg: {pa}"); i += 1; shown += 1; continue; }
            break;
        }
        if ba.data != bb.data {
            shown += 1;
            let n = ba.data.len().max(bb.data.len()) / 4;
            let d: Vec<String> = (0..n).filter(|k| ba.i32(*k) != bb.i32(*k)).map(|k| format!("[{k}] {} -> {}", ba.i32(k), bb.i32(k))).collect();
            println!("{pa}: {}", d.join(", "));
            if ba.data.len() != bb.data.len() || d.is_empty() { println!("  text: {:?} -> {:?}", String::from_utf8_lossy(&ba.data), String::from_utf8_lossy(&bb.data)); }
        }
        i += 1; j += 1;
    }
}
