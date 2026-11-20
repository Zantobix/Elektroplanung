//! Diagnose: Symbole eines Blattes (globaler Index) mit Symbolname, TIT und Feldern 10–23.
use elektroplanung::eprfile::EprFile;
fn s(d: &[u8]) -> String { d.iter().take_while(|c| **c != 0).map(|c| *c as char).collect() }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let want: usize = a[2].parse().unwrap();
    let prj = f.root("PRJ").unwrap();
    let names: std::collections::HashMap<Vec<u8>, String> = prj.kids_of("SYM").map(|y| (y.data[80..88].to_vec(), s(&y.data))).collect();
    let mut n = 0;
    for anl in prj.kids_of("ANL") { for b in anl.kids_of("BLT") {
        if n == want { for o in b.kids_of("VSY") {
            let v: Vec<String> = (0..40).map(|i| o.i32(i).to_string()).collect();
            let kids: Vec<String> = o.kids.iter().map(|k| format!("{}:{}", k.tag_str(), s(&k.data))).collect();
            println!("{:20} [{}] {:?}", names.get(&o.data[104..112].to_vec()).cloned().unwrap_or_default(), v.join(" "), kids);
        } }
        n += 1;
    } }
}
