//! Diagnose: OBJ-Blöcke (Verweise) eines Blattes mit Text ausgeben.
use elektroplanung::eprfile::EprFile;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for anl in f.root("PRJ").unwrap().kids_of("ANL") {
        for b in anl.kids_of("BLT") {
            let name: String = b.data.iter().take(40).take_while(|c| **c != 0).map(|c| *c as char).collect();
            if name != a[2] { continue; }
            for o in b.kids_of("OBJ") {
                let t = o.kid("TIT").map(|t| String::from_utf8_lossy(&t.data).trim_end_matches('\0').to_string()).unwrap_or_default();
                let v: Vec<String> = (0..16).map(|i| o.i32(i).to_string()).collect();
                let kids: Vec<&str> = o.kids.iter().map(|k| k.tag_str()).collect();
                println!("'{t}' [{}] kids={kids:?}", v.join(" "));
            }
        }
    }
}
