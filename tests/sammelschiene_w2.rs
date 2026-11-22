//! Lage der Sammelschienen im Beispielprojekt (W2.epr): die Stapelregel der Zeichenroutine des Originals ergibt
//! genau die gespeicherten Y-Lagen; „Nach Unten“ schreibt Feld 15/11 zurück. Fehlt die Datei (EP_SAMPLE), werden
//! die Tests übersprungen.

use elektroplanung::{epr, epr_write, model::*, sammelschiene_lage as sl};
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

#[test]
fn stapel_ergibt_gespeicherte_lagen() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let mut n = 0;
    let mut unten = 0;
    for a in &p.anlagen {
        let pfad = a.set.pfad_hoehe as f32 / 10.0;
        for s in &a.sheets {
            let (oben, u) = sl::stapel_anfang(s, pfad, p.title_h());
            for (id, y) in sl::stapeln(s, 5.0, oben, u) {
                let o = s.objects.iter().find(|o| o.id == id).unwrap();
                let Kind::Busbar { a: pa, name, lage, .. } = &o.kind else { unreachable!() };
                assert!((pa.y - y).abs() < 0.01, "{} / {name}: gespeichert {} gerechnet {y}", s.name, pa.y);
                n += 1;
                unten += (lage & sl::UNTEN != 0) as usize;
            }
        }
    }
    assert!(n > 50 && unten > 10, "{n} Schienen, {unten} unten");
}

#[test]
fn nach_oben_wird_zurueckgeschrieben() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    // PE auf „Sicherungen -F1 und -F2“ liegt unten mit 2,5 mm Versatz
    let (ai, si) = p.anlagen.iter().enumerate().find_map(|(ai, a)| a.sheets.iter().position(|s| s.name == "Sicherungen -F1 und -F2").map(|si| (ai, si))).unwrap();
    let s = &mut p.anlagen[ai].sheets[si];
    let i = s.objects.iter().position(|o| matches!(&o.kind, Kind::Busbar { name, .. } if name == "PE")).unwrap();
    assert!(matches!(s.objects[i].kind, Kind::Busbar { lage: 1, versatz, .. } if versatz == 2.5));
    sl::umschalten(&mut s.objects[i].kind);
    let pfad = p.anlagen[ai].set.pfad_hoehe as f32 / 10.0;
    let th = p.title_h();
    let s = &mut p.anlagen[ai].sheets[si];
    let (oben, unten) = sl::stapel_anfang(s, pfad, th);
    let lagen = sl::stapeln(s, 5.0, oben, unten);
    sl::anwenden(s, &lagen);
    let id = s.objects[i].id;
    let tmp = std::env::temp_dir().join("ep_sammelschiene_test.epr");
    epr_write::save(&p, &tmp).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let o = q.anlagen[ai].sheets[si].objects.iter().find(|o| matches!(&o.kind, Kind::Busbar { name, .. } if name == "PE")).unwrap();
    let Kind::Busbar { a, lage, versatz, .. } = &o.kind else { unreachable!() };
    assert_eq!((*lage, *versatz), (0, 0.0));
    let soll = lagen.iter().find(|(i, _)| *i == id).unwrap().1;
    assert!((a.y - soll).abs() < 0.01);
    // Rohblock: Feld 15 Byte 0 = 0, Feld 11 Wort 0 = 0
    let b = &o.raw.as_ref().unwrap().block;
    assert_eq!((b.data[0x3c], b.data[0x2c], b.data[0x2d]), (0, 0, 0));
}

/// Neue Schiene wie im Original (Gegenprobe Wine 22.11.2026, `fixtures/x_schiene_kuerzel.epr`): freier Name
/// „Schiene 1“ in SMS Byte 0, Kürzel „L1“ im TIT; beides übersteht Speichern und Laden, beschriftet wird mit „L1“.
#[test]
fn neue_schiene_name_und_kuerzel() {
    let mut p = Project::new_default();
    p.anlagen[0].sammelschienen.push("L1".into());
    p.anlagen[0].schienen.insert("L1".into(), SchieneSet { name: "Schiene 1".into(), pfeile: false, ..Default::default() });
    let tmp = std::env::temp_dir().join("ep_schiene_kuerzel_test.epr");
    epr_write::save(&p, &tmp).unwrap();
    let roh = std::fs::read(&tmp).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let i = roh.windows(4).position(|w| w == b"\x99SMS").expect("SMS-Block");
    assert_eq!(&roh[i + 8..i + 17], b"Schiene 1");
    assert_eq!(&roh[i + 17..i + 18], b"\0");
    assert_eq!(q.anlagen[0].sammelschienen, vec!["L1".to_string()]);
    assert_eq!(q.anlagen[0].schienen.get("L1").map(|s| s.name.as_str()), Some("Schiene 1"));
    assert_eq!(elektroplanung::schiene_platzieren::beschriftung("L1", q.anlagen[0].schienen.get("L1")), "L1");
}

/// Vom Original gespeicherte Schiene („Schiene 2“, Kürzel „L2“) wird genauso gelesen.
#[test]
fn original_schiene_name_und_kuerzel() {
    let Ok(dir) = std::env::var("EP_FIXTURES") else { return };
    let path = PathBuf::from(dir).join("x_schiene_kuerzel.epr");
    if !path.exists() {
        return;
    }
    let p = epr::import(&path).unwrap();
    assert_eq!(p.anlagen[0].schienen.get("L2").map(|s| s.name.as_str()), Some("Schiene 2"));
}
