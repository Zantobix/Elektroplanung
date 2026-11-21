//! Diagnose: Klemmstellen einer Leiste mit allen Verbindungen, die an ihnen beginnen oder enden.
//! cargo run --example klemmen_netz -- <datei> <anlage-anfang> <leiste>
use elektroplanung::{epr, model::*};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    let an = p.anlagen.iter().find(|x| x.name.starts_with(&a[2])).unwrap();
    for s in &an.sheets {
        let wires: Vec<(usize, &Vec<P>, u8, bool)> = s
            .objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| match &o.kind {
                Kind::Wire { pts, opts, .. } => {
                    let raw = o.raw.as_deref().filter(|r| r.block.is("VBG")).map(|r| r.block.i32(14) & 0x100 != 0).unwrap_or(false);
                    Some((i, pts, *opts, raw))
                }
                _ => None,
            })
            .collect();
        for o in &s.objects {
            let Kind::Terminal { pos, leiste, nr, level, .. } = &o.kind else { continue };
            if leiste != &a[3] {
                continue;
            }
            println!("Blatt {} Klemme {nr}/{level} bei ({:.1},{:.1})", s.number, pos.x, pos.y);
            for (i, pts, opts, br) in &wires {
                let n = pts.len();
                if pts[0].dist(*pos) <= 0.85 || pts[n - 1].dist(*pos) <= 0.85 {
                    let v: Vec<String> = pts.iter().map(|q| format!("({:.1},{:.1})", q.x, q.y)).collect();
                    println!("   Obj {i} bruecke={br} opts={opts:#x} {}", v.join(" "));
                }
            }
        }
    }
}
