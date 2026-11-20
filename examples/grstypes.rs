//! Diagnose: Elementtypen in Symbolen (SYM/GRD/GRS) und Schriftfeld (FUS/GRS); Elemente mit Typ >= 8 ausgeben.
use elektroplanung::eprfile::{i32_at, EprFile};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let prj = f.root("PRJ").unwrap();
    let mut cnt = std::collections::BTreeMap::new();
    let mut show = |name: String, d: &[u8]| for e in d.chunks_exact(0x50) {
        *cnt.entry(e[0x3c]).or_insert(0) += 1;
        if e[0x3c] >= 8 && e[0x3c] != 9 { let v: Vec<i32> = (0..20).map(|i| i32_at(e, i*4)).collect(); println!("{name} typ {} {:?}", e[0x3c], v); }
    };
    for s in prj.kids_of("SYM") { let n: String = s.data.iter().take(40).take_while(|c| **c!=0).map(|c| *c as char).collect();
        if let Some(g) = s.kid("GRD") { for r in g.kids_of("GRS") { show(n.clone(), &r.data); } } }
    if let Some(fu) = prj.kid("FUS") { for r in fu.kids_of("GRS") { show("FUS".into(), &r.data); } }
    println!("{cnt:?}");
}
