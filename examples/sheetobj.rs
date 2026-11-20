//! Diagnose: je Blatt die OBJ ohne TIT (Sammelschienen) und ZCN Typ 13 ausgeben.
use elektroplanung::eprfile::EprFile;
fn s(d: &[u8]) -> String { d.iter().take_while(|c| **c != 0).map(|c| *c as char).collect() }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = EprFile::read(std::path::Path::new(&a[1])).unwrap();
    for (ai, anl) in f.root("PRJ").unwrap().kids_of("ANL").enumerate() {
        for (bi, b) in anl.kids_of("BLT").enumerate() {
            for o in &b.kids {
                let sms = o.is("OBJ") && o.kid("TIT").is_none();
                let z13 = o.is("ZCN") && o.data.len() >= 0x50 && o.data[0x3c] == 13;
                let bzd = o.is("VBG") && o.kid("BZD").is_some();
                if sms || z13 || bzd {
                    let v: Vec<String> = (0..20).map(|i| o.i32(i).to_string()).collect();
                    let k: Vec<String> = o.kids.iter().map(|k| format!("{}:{}", k.tag_str(), s(&k.data))).collect();
                    println!("A{ai} B{bi} '{}' {} [{}] {:?}", s(&b.data), o.tag_str(), v.join(" "), k);
                }
            }
        }
    }
}
