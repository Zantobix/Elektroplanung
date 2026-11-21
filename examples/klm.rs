//! Diagnose: Klemmleisten (KLM › KFO) und Klemmstellen (OBJ Feld 4 = 0x2FFFF) einer .epr-Datei ausgeben.
use elektroplanung::eprfile::{Block, EprFile};
fn s(b: &Block) -> String {
    let kids: Vec<String> = b.kids.iter().map(|k| format!("{}:{:?}", k.tag_str(), String::from_utf8_lossy(&k.data).trim_end_matches('\0'))).collect();
    let v: Vec<String> = (0..b.data.len() / 4).map(|i| format!("{:x}", b.i32(i))).collect();
    format!("len={} [{}] {}", b.data.len(), v.join(" "), kids.join(" "))
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for anl in f.root("PRJ").unwrap().kids_of("ANL") {
        println!("ANL {}", String::from_utf8_lossy(&anl.data[..55]).trim_end_matches('\0'));
        for k in anl.kids_of("KLM") {
            println!(" KLM {}", s(k));
            println!("     bytes {:02x?}", &k.data[..k.data.len().min(80)]);
            for f in k.kids_of("KFO") {
                println!("   KFO {}", s(f));
            }
        }
        for b in anl.kids_of("BLT") {
            let name: String = b.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
            if b.i32(22) & 0xff != 3 {
                println!(" BLT '{name}' art={} f32={:x}", b.i32(22) & 0xff, b.i32(32));
            }
            if a.get(2).map(|s| s == "typen").unwrap_or(false) {
                for o in b.kids_of("OBJ") {
                    println!("  OBJ f4={:x} b14={} f8={} tit={}", o.i32(4), o.data[0x14], o.i32(8), o.kid("TIT").is_some());
                }
                continue;
            }
            for o in b.kids_of("OBJ").filter(|o| o.i32(4) == 0x2ffff) {
                println!("  T '{name}' {}", s(o));
            }
        }
    }
}
