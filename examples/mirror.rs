//! Diagnose: Symbole mit Spiegelbits (VSY Feld 18 Bit 8 / Bit 16) und ihr Blatt.
use elektroplanung::eprfile::EprFile;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for anl in f.root("PRJ").unwrap().kids_of("ANL") {
        for b in anl.kids_of("BLT") {
            let name: String = b.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
            for v in b.kids_of("VSY") {
                let fl = v.i32(18) & 0x1f;
                if fl & 0x18 != 0 {
                    let t = v.kid("TIT").map(|t| String::from_utf8_lossy(&t.data).trim_end_matches('\0').to_string()).unwrap_or_default();
                    println!("{name}: {t} flags={fl:#x} pos=({},{}) bounds={:?}", v.i32(16), v.i32(17), (32..36).map(|i| v.i32(i)).collect::<Vec<_>>());
                }
            }
        }
    }
}
