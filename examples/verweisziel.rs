//! Diagnose: Verweise (OBJ) aller Blätter mit ihren Zielkennungen (Feld 8–13) und den Kennungen der Anlagen/Blätter.
use elektroplanung::eprfile::EprFile;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    let name = |d: &[u8]| -> String { d.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect() };
    for anl in f.root("PRJ").unwrap().kids_of("ANL") {
        println!("ANL '{}' id={:08x}/{:08x}", name(&anl.data), anl.i32(34), anl.i32(35));
        for b in anl.kids_of("BLT") {
            println!("  BLT '{}' nr={} id={:08x}/{:08x}", name(&b.data), b.i32(23) >> 8, b.i32(18), b.i32(19));
            for o in b.kids_of("OBJ") {
                if o.i32(4) == 0xffff || o.i32(4) == 0x2ffff { continue; }
                let t = o.kid("TIT").map(|t| String::from_utf8_lossy(&t.data).trim_end_matches('\0').to_string()).unwrap_or_default();
                let v: Vec<String> = (0..o.data.len() / 4).map(|i| format!("{:x}", o.i32(i))).collect();
                println!("    '{t}' [{}]", v.join(" "));
            }
        }
    }
}
