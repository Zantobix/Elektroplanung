//! Diagnose: berechnete Klemmpläne aller Klemmplan-Blätter einer .epr-Datei ausgeben.
//! cargo run --example klemmplan -- <datei>
use elektroplanung::{epr, klemmplan};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    for an in &p.anlagen {
        for s in an.sheets.iter().filter(|s| s.art == 4) {
            let name = an.klemmen.iter().find(|k| k.id == s.klm).map(|k| k.name.clone()).unwrap_or_default();
            println!("== {} / {} ({name})", an.name, s.name);
            let plan = klemmplan::plan(an, &name, &p.symdefs);
            let f = |z: &Option<klemmplan::Ziel>| z.as_ref().map(|z| format!("{} {} {}", z.kennung, z.kl, z.pfad)).unwrap_or_default();
            for z in &plan.zeilen {
                println!("{:>3}{} | {:<32} | {}", z.nr, if z.level > 0 { (b'A' + z.level - 1) as char } else { ' ' }, f(&z.von), f(&z.nach));
            }
            let br: Vec<Vec<String>> = plan.bruecken.iter().map(|b| b.rows.iter().map(|r| format!("{}{}", plan.zeilen[*r].nr, plan.zeilen[*r].level)).collect()).collect();
            println!("Brücken {br:?}");
        }
    }
}
