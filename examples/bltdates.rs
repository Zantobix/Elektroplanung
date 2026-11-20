//! Diagnose: Delphi-Datumswerte (f64) an allen 4-Byte-Stellen der BLT-Blöcke.
use elektroplanung::eprfile::EprFile;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for anl in f.root("PRJ").unwrap().kids_of("ANL") {
        let n: String = anl.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
        let mut s = String::new();
        for o in (0..anl.data.len().saturating_sub(7)).step_by(4) {
            let v = f64::from_le_bytes(anl.data[o..o + 8].try_into().unwrap());
            if (40000.0..50000.0).contains(&v) { s += &format!(" @{o}={:.3}", v); }
        }
        println!("ANL {n}:{s}");
        for b in anl.kids_of("BLT").take(3) {
            let n: String = b.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
            let mut s = String::new();
            for o in (0..b.data.len().saturating_sub(7)).step_by(4) {
                let v = f64::from_le_bytes(b.data[o..o + 8].try_into().unwrap());
                if (40000.0..50000.0).contains(&v) { s += &format!(" @{o}={:.3}", v); }
            }
            println!("  BLT {n}:{s}");
        }
    }
}
