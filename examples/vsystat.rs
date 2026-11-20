//! Diagnose: Werteverteilung der VSY-Felder 19..27 und Unterblöcke.
use elektroplanung::eprfile::{Block, EprFile};
use std::collections::BTreeMap;
fn walk(b: &Block, st: &mut BTreeMap<String, BTreeMap<i32, usize>>, kids: &mut BTreeMap<String, usize>, ex: &mut Vec<String>) {
    if b.is("VSY") {
        for i in [12usize, 13, 14, 15, 18, 20, 21, 22, 23] {
            *st.entry(format!("f{i}")).or_default().entry(b.i32(i)).or_default() += 1;
        }
        for k in &b.kids { *kids.entry(k.tag_str().into()).or_default() += 1; }
        if b.i32(21) != 0 || b.kids.iter().any(|k| k.is("ALT") || k.is("SPA") || k.is("FKT") || k.is("WRT")) || std::env::args().nth(2).map(|w| b.kid("TIT").map(|t| t.data.starts_with(w.as_bytes())).unwrap_or(false)).unwrap_or(false) {
            let t = b.kid("TIT").map(|t| String::from_utf8_lossy(&t.data).trim_end_matches('\0').to_string()).unwrap_or_default();
            let ks: Vec<String> = b.kids.iter().map(|k| format!("{}:{}", k.tag_str(), String::from_utf8_lossy(&k.data).replace('\0', "|"))).collect();
            ex.push(format!("{t} f24..39={:?} kids={:?}", (24..40).map(|i| b.i32(i)).collect::<Vec<_>>(), ks));
        }
    }
    for k in &b.kids { walk(k, st, kids, ex); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let (mut st, mut kids, mut ex) = (BTreeMap::new(), BTreeMap::new(), vec![]);
    for r in &f.roots { walk(r, &mut st, &mut kids, &mut ex); }
    for (k, v) in &st { let top: Vec<String> = v.iter().take(12).map(|(a, b)| format!("{a}:{b}")).collect(); println!("{k}: {}", top.join(" ")); }
    println!("kids {kids:?}");
    for e in ex.iter().take(40) { println!("{e}"); }
}
