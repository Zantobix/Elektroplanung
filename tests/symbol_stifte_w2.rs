//! Stifte einer Symboldefinition (SYM › GRD › SIT, src/symbol_stifte.rs): aus W2.epr lesen, ändern, im
//! Originalformat speichern und wieder lesen. Fehlt W2.epr (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, eprfile::EprFile, model::*, symbols::Prim};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn stifte_der_elemente(d: &elektroplanung::symbols::SymDef) -> Vec<u8> {
    d.prims.iter().filter_map(|p| match p {
        Prim::Line(.., s) | Prim::Poly(_, s) | Prim::Bezier(.., s) | Prim::Ellipse(.., s) => Some(*s),
        _ => None,
    }).collect()
}

/// SIT-Blöcke des Symbols `name` in der gespeicherten Datei.
fn sit_daten(bytes: &[u8], name: &str) -> Vec<Vec<u8>> {
    let f = EprFile::parse(bytes).unwrap();
    let prj = f.root("PRJ").unwrap();
    let s = prj.kids_of("SYM").find(|s| elektroplanung::symbols::cstr(&s.data[..40]) == name).unwrap();
    s.kid("GRD").unwrap().kids_of("SIT").map(|b| b.data.clone()).collect()
}

#[test]
fn symbol_buss_stifte_lesen_aendern_schreiben() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let orig = std::fs::read(&path).unwrap();
    let d = p.symdefs.iter().find(|d| d.name == "Buss").expect("Symbol Buss");
    let namen: Vec<(u32, &str)> = d.pens.iter().map(|(n, s)| (*n, s.name.as_str())).collect();
    assert_eq!(namen, vec![(4, "Mittel"), (5, "Bus"), (6, "Buss")]);
    assert_eq!(((d.pens[0].1.width * 10.0).round(), d.pens[1].1.color), (4.0, [128, 128, 128]));
    // Elemente nennen ihre Stifte (früher auf 0–3 gekappt)
    let el = stifte_der_elemente(d);
    assert!(el.contains(&4) && el.contains(&5) && el.contains(&6), "{el:?}");
    let sit_vorher = sit_daten(&orig, "Buss");

    // ändern: Stift 5 rot, neuer Stift 7 für das erste Element
    let d = p.symdefs.iter_mut().find(|d| d.name == "Buss").unwrap();
    d.pens[1].1.color = [255, 0, 0];
    d.pens.push((7, Stift { name: "Neu".into(), style: 1, color: [0, 0, 255], width: 0.7 }));
    let i = d.prims.iter().position(|p| matches!(p, Prim::Line(..))).unwrap();
    if let Prim::Line(_, _, s) = &mut d.prims[i] {
        *s = 7;
    }
    let out = std::env::temp_dir().join(format!("ep_symstifte_{}.epr", std::process::id()));
    epr_write::save(&p, &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let q = epr::import(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let d2 = q.symdefs.iter().find(|d| d.name == "Buss").unwrap();
    assert_eq!(d2.pens.len(), 4);
    assert_eq!(d2.pens[1].1.color, [255, 0, 0]);
    assert_eq!(d2.pens[3], (7, Stift { name: "Neu".into(), style: 1, color: [0, 0, 255], width: 0.7 }));
    assert_eq!(stifte_der_elemente(d2)[i], 7);
    // unveränderte Stifte bleiben byteidentisch, geänderte nur in der Farbe (Feld 6)
    let sit = sit_daten(&bytes, "Buss");
    assert_eq!(sit[0], sit_vorher[0]);
    assert_eq!(sit[2], sit_vorher[2]);
    assert_eq!(sit[1][..24], sit_vorher[1][..24]);
    assert_eq!(sit[1][28..], sit_vorher[1][28..]);
    assert_eq!(sit[3][32..36], 7i32.to_le_bytes());
}

#[test]
fn bibliothek_liest_stifte() {
    let Some(dir) = std::env::var("EP_DATENBANK").ok().map(PathBuf::from).filter(|d| d.exists()) else { return };
    let lib = match elektroplanung::symbols::load_db(&dir) {
        Ok(l) => l,
        Err(e) => {
            // Das Original sperrt seine Datenbank, solange es läuft
            eprintln!("Bibliothek nicht lesbar ({e}) – Test übersprungen");
            return;
        }
    };
    let buss = lib.iter().find(|d| d.name == "Buss").expect("Bibliothekssymbol Buss");
    let namen: Vec<&str> = buss.pens.iter().map(|(_, s)| s.name.as_str()).collect();
    assert_eq!(namen, vec!["Mittel", "Bus", "Buss"]);
    // jedes Element mit eigenem Stift findet seinen Stift
    for d in &lib {
        for s in stifte_der_elemente(d).into_iter().filter(|s| *s >= 4) {
            assert!(d.pens.iter().any(|(n, _)| *n == s as u32), "{}: Stift {s} fehlt in {:?}", d.name, d.pens);
        }
    }
}

/// Stift 5 des Symbols "Buss" löschen (wie „Stift Löschen“): sein SIT-Block fällt weg, Stift 6 rückt auf 5,
/// die Elemente mit Stift 5 zeichnen mit dem Standardstift 0.
#[test]
fn symbol_buss_stift_loeschen() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let d = p.symdefs.iter_mut().find(|d| d.name == "Buss").unwrap();
    let vorher = stifte_der_elemente(d);
    let mut nummern: Vec<usize> = vorher.iter().map(|s| *s as usize).collect();
    let mut el: Vec<(u64, &mut usize)> = nummern.iter_mut().enumerate().map(|(i, n)| (i as u64, n)).collect();
    elektroplanung::symbol_stifte::loeschen(&mut d.pens, 5, &mut el);
    let mut k = 0;
    for pr in d.prims.iter_mut() {
        if let Prim::Line(.., s) | Prim::Poly(_, s) | Prim::Bezier(.., s) | Prim::Ellipse(.., s) = pr {
            *s = nummern[k] as u8;
            k += 1;
        }
    }
    let out = std::env::temp_dir().join(format!("ep_symstift_del_{}.epr", std::process::id()));
    epr_write::save(&p, &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let q = epr::import(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let d2 = q.symdefs.iter().find(|d| d.name == "Buss").unwrap();
    let namen: Vec<(u32, &str)> = d2.pens.iter().map(|(n, s)| (*n, s.name.as_str())).collect();
    assert_eq!(namen, vec![(4, "Mittel"), (5, "Buss")]);
    let erwartet: Vec<u8> = vorher.iter().map(|&s| match s { 5 => 0, 6 => 5, s => s }).collect();
    assert_eq!(stifte_der_elemente(d2), erwartet);
    assert_eq!(sit_daten(&bytes, "Buss").len(), 2);
}
