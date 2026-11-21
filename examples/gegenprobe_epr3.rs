//! Testdatei für das Original (Gegenprobe epr_zusatz): eine Kopie von W2.epr mit Bauteil samt Zuordnung,
//! Rackposition, Platzhaltergruppen mit Einträgen, geänderten Verbindungsoptionen, geänderten Schienen-Schaltern,
//! umbenannter, ersetzter und gelöschter Projektgrafik, Projektstatus „abgeschlossen“, „Alter Klemmenplan“ und
//! Schriftfeld „Absolut“.
//!
//! `cargo run --release --example gegenprobe_epr3 [-- <W2.epr> <ziel.epr>]`; Vorgabe: EP_SAMPLE →
//! %TEMP%\ep_orig_test\gegenprobe_epr3.epr. Die Quelle wird nur gelesen: zuerst nach %TEMP%\ep_orig_test kopiert,
//! gearbeitet wird mit der Kopie.
use elektroplanung::{epr, epr_projekt, epr_write, model::*};
use std::path::PathBuf;

/// Erkennbares Ersatzbild: rot-weiße Streifen mit blauem Rahmen, 160 × 120 Pixel.
fn ersatzbild() -> Vec<u8> {
    let img = image::RgbImage::from_fn(160, 120, |x, y| {
        if x < 8 || y < 8 || x >= 152 || y >= 112 {
            image::Rgb([0, 0, 255])
        } else if (x / 20) % 2 == 0 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    });
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90).encode_image(&img).expect("JPEG");
    jpeg
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let quelle = a.get(1).cloned().or_else(|| std::env::var("EP_SAMPLE").ok()).expect("W2.epr angeben oder EP_SAMPLE setzen");
    let ordner = std::env::temp_dir().join("ep_orig_test");
    std::fs::create_dir_all(&ordner).expect("Ordner anlegen");
    let ziel = a.get(2).map(PathBuf::from).unwrap_or_else(|| ordner.join("gegenprobe_epr3.epr"));
    if let Some(d) = ziel.parent() {
        std::fs::create_dir_all(d).expect("Zielordner anlegen");
    }
    // nur mit einer Kopie arbeiten
    let kopie = ordner.join("gegenprobe_epr3_quelle.epr");
    std::fs::copy(&quelle, &kopie).expect("Quelle kopieren");
    let mut p = epr::import(&kopie).expect("Lesen");
    let mut wo: Vec<String> = vec![];

    // 1. Bauteil (Bauteilgruppe und Hersteller aus Gruppen.tfi) und Zuordnung zum ersten Symbol, dort auch
    //    Rackposition 3. „Projekt“ ist keine Bauteilgruppe des Originals (dort „Keine Zuordnung“).
    p.bauteile.push(Bauteil {
        gruppe: "Schütze".into(),
        name: "Gegenprobe Schütz".into(),
        typ: "DILM12-10".into(),
        firma: "Siemens".into(),
        artikel: "276830".into(),
        breite: "45".into(),
        hoehe: "68".into(),
        tiefe: "75".into(),
        nummer: "4711".into(),
    });
    'sym: for a in p.anlagen.iter_mut() {
        for s in a.sheets.iter_mut() {
            for o in s.objects.iter_mut() {
                if let Kind::Symbol { bmk, ext, .. } = &mut o.kind {
                    ext.rack = 3;
                    wo.push(format!("Symbol {bmk} ({} / Blatt {} \u{201e}{}\u{201c}), Eigenschaften: Rackposition 3, Bauteil \u{201e}Gegenprobe Schütz\u{201c}", a.name, s.number, s.name));
                    p.part_of.insert(o.id, 0);
                    break 'sym;
                }
            }
        }
    }
    wo.push("Projekt › Bauteile: \u{201e}Gegenprobe Schütz\u{201c}, Gruppe Schütze, Typ DILM12-10, Hersteller Siemens, Artikel-Nr. 276830, B/H/T 45/68/75 mm, interne Nr. 4711".into());

    // 2. Platzhalter: Einträge in der vorhandenen Gruppe SPS und eine neue Gruppe
    if let Some(g) = p.platzhalter.first_mut() {
        g.werte = vec![("E0.0".into(), "Taster Flur".into()), ("A0.0".into(), "Licht Flur".into())];
        wo.push(format!("Platzhalter: Gruppe {} mit E0.0 = \u{201e}Taster Flur\u{201c}, A0.0 = \u{201e}Licht Flur\u{201c}", g.name));
    }
    p.platzhalter.push(Platzhalter { name: "Gegenprobe".into(), werte: vec![("RAUM".into(), "Küche".into())] });
    wo.push("Platzhalter: neue Gruppe \u{201e}Gegenprobe\u{201c} mit RAUM = \u{201e}Küche\u{201c}".into());

    // 3. Verbindungsoptionen: zwei Verbindungen mit Bezeichnung, eine Brücke entfernen
    let mut n_bez = 0;
    let mut bruecke_weg = false;
    for a in p.anlagen.iter_mut() {
        for s in a.sheets.iter_mut() {
            for o in s.objects.iter_mut() {
                let Kind::Wire { opts, bez, pts, .. } = &mut o.kind else { continue };
                let lage = format!("{} / Blatt {}, ab ({:.1}/{:.1}) mm", a.name, s.number, pts[0].x, pts[0].y);
                if !bez.is_empty() && *opts == 0 && n_bez < 2 {
                    if n_bez == 0 {
                        *opts = WIRE_ZIEL_A | WIRE_HOCHKANT | WIRE_MARKIERUNG;
                        wo.push(format!("Verbindung \u{201e}{}\u{201c} ({lage}): Zielführung links/oben, Hochkant, Markierung; sonst aus", bez[0].1));
                    } else {
                        *opts = WIRE_ZIEL_B | WIRE_BRUECKE | WIRE_UNTERSTRICH | WIRE_DURCH;
                        wo.push(format!("Verbindung \u{201e}{}\u{201c} ({lage}): Zielführung rechts/unten, Brücke, Unterstrichen, Durch Markierung; sonst aus", bez[0].1));
                    }
                    n_bez += 1;
                } else if !bruecke_weg && *opts == WIRE_BRUECKE {
                    *opts = 0;
                    bruecke_weg = true;
                    wo.push(format!("Verbindung ({lage}), in W2 eine Brücke: Brücke aus, alle Optionen aus"));
                }
            }
        }
    }

    // 4. Schienen-Schalter: Schiene PE der ersten Anlage, die eine hat
    if let Some(a) = p.anlagen.iter_mut().find(|a| a.schienen.contains_key("PE")) {
        let s = a.schienen.get_mut("PE").unwrap();
        s.quer = false;
        s.klemmen = true;
        s.schleife = true;
        s.autoende = true;
        s.pfeile = !s.pfeile;
        wo.push(format!(
            "Sammelschiene PE in {}: Querverweise aus, Klemmen an, Durchschleifung an, Autoende an, mit Pfeilen {} (in W2 umgekehrt), Einspeisung unverändert",
            a.name,
            if s.pfeile { "an" } else { "aus" }
        ));
    }

    // 5. Projektgrafiken: erste umbenennen, zweite ersetzen, letzte (falls unbenutzt) löschen
    let benutzt: Vec<String> = p
        .anlagen
        .iter()
        .flat_map(|a| &a.sheets)
        .flat_map(|s| &s.objects)
        .filter_map(|o| if let Kind::Image { path, .. } = &o.kind { Some(path.clone()) } else { None })
        .collect();
    if let Some(b) = p.bilder.first_mut() {
        wo.push(format!("Projektgrafik Nr. {} \u{201e}{}\u{201c} heißt jetzt \u{201e}Gegenprobe Bild\u{201c} (Bild unverändert)", b.nr, b.name));
        b.name = "Gegenprobe Bild".into();
    }
    if let Some(b) = p.bilder.get_mut(1) {
        b.jpeg = ersatzbild();
        wo.push(format!("Projektgrafik Nr. {} \u{201e}{}\u{201c}: Bild ersetzt durch rot-weiße senkrechte Streifen mit blauem Rand (160 × 120)", b.nr, b.name));
    }
    if p.bilder.len() > 2 {
        let i = p.bilder.len() - 1;
        if !benutzt.contains(&format!("bld:{}", p.bilder[i].nr)) {
            let b = p.bilder.remove(i);
            wo.push(format!("Projektgrafik Nr. {} \u{201e}{}\u{201c} gelöscht (fehlt in der Liste)", b.nr, b.name));
        }
    }

    // 6. Projekteigenschaften und Schriftfeld
    p.abgeschlossen = true;
    p.alter_klemmenplan = true;
    p.tb_absolut = true;
    wo.push("Projekteigenschaften: Status \u{201e}Projekt ist abgeschlossen\u{201c}, \u{201e}Alter Klemmenplan\u{201c} angehakt".into());
    wo.push("Schriftfeld: Maßangaben absolut (FUS Feld 1 = 0, Koordinaten in 0,1 mm umgerechnet wie im Original); Blätter und Schriftfeld-Editor zeigen das Schriftfeld wie in W2".into());

    let fehlt = epr_projekt::nicht_gespeichert(&p);
    assert!(fehlt.is_empty(), "nicht speicherbar (Gruppen.tfi lesbar? sonst EP_DATENBANK auf eine Kopie setzen): {fehlt:?}");
    epr_write::save(&p, &ziel).expect("Speichern");
    // Gegenprobe im Nachbau
    let back = epr::import(&ziel).expect("Lesen");
    assert_eq!(back.bauteile, p.bauteile);
    assert_eq!(back.part_of.len(), 1);
    assert_eq!(back.platzhalter, p.platzhalter);
    assert!(back.abgeschlossen && back.alter_klemmenplan && back.tb_absolut);
    assert_eq!(back.bilder.len(), p.bilder.len());
    assert!(back.bilder.iter().zip(&p.bilder).all(|(x, y)| x.name == y.name && x.nr == y.nr && x.jpeg == y.jpeg));
    println!("{} geschrieben (Quelle kopiert nach {})", ziel.display(), kopie.display());
    println!("nicht speicherbar: {fehlt:?}");
    for w in wo {
        println!("- {w}");
    }
}
