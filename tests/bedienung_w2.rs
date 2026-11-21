//! Bedienung der Zeichenfläche gegen das Beispielprojekt (W2.epr): Symbol skalieren, Titel/Wert verschieben und
//! "Nur dieses" (Kopie einer Symboldefinition) überstehen Speichern im Originalformat und Laden.
//! Fehlt die Datei (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, groesse, model::*, symbol_kopie};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if p.exists() {
        Some(p)
    } else {
        eprintln!("W2.epr nicht gefunden – Test übersprungen");
        None
    }
}

fn rundreise(p: &Project, name: &str) -> Project {
    let tmp = std::env::temp_dir().join(format!("ep_bedienung_{name}_{}.epr", std::process::id()));
    epr_write::save(p, &tmp).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    q
}

/// Lage im Blatt (Anlage, Blatt, Index) des ersten Symbols mit BMK und Wert.
fn symbol_mit_wert(p: &Project) -> (usize, usize, usize) {
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for (oi, o) in s.objects.iter().enumerate() {
                if matches!(&o.kind, Kind::Symbol { bmk, value, title: Some(_), .. } if !bmk.is_empty() && !value.is_empty()) {
                    return (ai, si, oi);
                }
            }
        }
    }
    panic!("kein Symbol mit Wert");
}

#[test]
fn skalieren_und_beschriftung_ins_originalformat() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, si, oi) = symbol_mit_wert(&p);
    let o = &mut p.anlagen[ai].sheets[si].objects[oi];
    let pos0 = match &o.kind {
        Kind::Symbol { pos, .. } => *pos,
        _ => unreachable!(),
    };
    groesse::skaliere(&mut o.kind, pos0, 1.5, 1.5);
    groesse::setze_beschriftung(&mut o.kind, false, P::new(-12.3, 4.5));
    groesse::setze_beschriftung(&mut o.kind, true, P::new(7.1, -3.2));
    let erwartet = o.kind.clone();
    let q = rundreise(&p, "skalieren");
    let g = &q.anlagen[ai].sheets[si].objects[oi].kind;
    match (g, &erwartet) {
        (Kind::Symbol { pos, scale, title, ext, .. }, Kind::Symbol { pos: p2, scale: s2, title: t2, ext: e2, .. }) => {
            assert!(pos.dist(*p2) < 0.06);
            assert!((scale - s2).abs() < 1e-5, "Maßstab {scale} statt {s2}");
            assert!(title.unwrap().0.dist(t2.unwrap().0) < 0.06);
            assert!(ext.value_off.unwrap().dist(P::new(7.1, -3.2)) < 0.06);
            assert!(e2.value_off.unwrap().dist(P::new(7.1, -3.2)) < 1e-4);
        }
        _ => panic!("Symbol nicht wiedergefunden"),
    }
}

#[test]
fn nur_dieses_symbol_mit_eigener_definition() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    // ein Symboltyp, der mehrfach gesetzt ist
    let keys: Vec<String> = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter_map(|o| if let Kind::Symbol { key, .. } = &o.kind { Some(key.clone()) } else { None }).collect();
    let key = keys.iter().find(|k| symbol_kopie::anzahl(&p, k) >= 3 && p.symdefs.iter().any(|d| &d.key == *k)).unwrap().clone();
    let n = symbol_kopie::anzahl(&p, &key);
    let id = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).find(|o| matches!(&o.kind, Kind::Symbol { key: k, .. } if *k == key)).unwrap().id;
    let mut def = p.symdefs.iter().find(|d| d.key == key).unwrap().clone();
    def.key = "proj:kopie".into();
    def.name = symbol_kopie::kopie_name(&def.name, &[]);
    let prims = def.prims.len();
    let name = def.name.clone();
    p.symdefs.push(def);
    assert!(symbol_kopie::umstellen(&mut p, id, "proj:kopie"));
    let q = rundreise(&p, "kopie");
    // die Kopie hat einen eigenen SYM-Block mit eigenem Namen, nur ein Symbol nutzt sie
    let neu = q.symdefs.iter().find(|d| d.name == name).expect("Kopie im .epr");
    assert!(neu.key.starts_with("epr:") && neu.key != key);
    assert_eq!(neu.prims.len(), prims);
    assert_eq!(symbol_kopie::anzahl(&q, &neu.key), 1);
    assert_eq!(symbol_kopie::anzahl(&q, &key), n - 1);
    // die ursprüngliche Definition bleibt unverändert
    let alt = |pr: &Project| pr.symdefs.iter().find(|d| d.key == key).map(|d| d.prims.clone());
    assert_eq!(alt(&q), alt(&p));
}
