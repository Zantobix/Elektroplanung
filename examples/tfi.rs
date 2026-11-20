//! Diagnose: Bibliothekseintrag (Symbole.tfi) – Abschnitte nach den Elementen.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lib = elektroplanung::symbols::load_db(std::path::Path::new(&a[1])).unwrap();
    for d in lib.iter().filter(|d| d.name == a[2]).take(3) {
        let r = &d.raw;
        let cnt = i32::from_le_bytes(r[0x14..0x18].try_into().unwrap()) as usize;
        let tp = 0x3c + cnt * 0x50;
        println!("{} {} raw={} elemente={} kopf={:?}", d.key, d.name, r.len(), cnt, &r[..0x3c].chunks(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>());
        let tail = &r[tp..];
        let tl = u32::from_le_bytes(tail[0..4].try_into().unwrap()) as usize;
        let after = &tail[4 + tl..];
        let sl = u32::from_le_bytes(after[0..4].try_into().unwrap()) as usize;
        let rest = &after[4 + sl..];
        println!("  nach Texten+SEI {} Byte:", rest.len());
        for (i, c) in rest.chunks(16).enumerate() {
            let hex: Vec<String> = c.iter().map(|b| format!("{b:02x}")).collect();
            let asc: String = c.iter().map(|b| if (32..127).contains(b) { *b as char } else { '.' }).collect();
            println!("  {:04x}: {:<48} {}", i * 16, hex.join(" "), asc);
        }
        println!("  rest {} Byte: {}", tail.len(), tail.iter().map(|b| if (32..127).contains(b) { (*b as char).to_string() } else { format!("[{b:02x}]") }).collect::<String>());
    }
}
#[allow(dead_code)]
fn _x() {}
