//! Diagnose: Verbindungen (VBG) eines Blattes (globaler Index) mit Feldern 0–14 und Punkten.
use elektroplanung::eprfile::EprFile;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let want: usize = a[2].parse().unwrap();
    let mut n = 0;
    for anl in f.root("PRJ").unwrap().kids_of("ANL") { for b in anl.kids_of("BLT") {
        if n == want { for o in b.kids_of("VBG") {
            let v: Vec<String> = (0..15).map(|i| format!("{:08x}", o.i32(i))).collect();
            let pts: Vec<String> = o.kids_of("VBP").map(|p| format!("({},{})", p.i32(0), p.i32(1))).collect();
            println!("[{}] {}", v.join(" "), pts.join(" "));
        } }
        n += 1;
    } }
}
