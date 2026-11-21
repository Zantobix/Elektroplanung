//! Import-Vollständigkeit: alle Blockarten von W2.epr, die das Original darstellt, werden zu Objekten
//! (Sammelschienen, Klemmstellen, Tabellen, Verbindungsbezeichnungen, Bilder, Parameter, BMK-Vorlagen,
//! berechnete Blätter). Sollwerte aus dem Original-Ausdruck "Elektroplanung Wohnung 2.pdf".

use elektroplanung::{canvas, epr, eprfile::EprFile, model::*, symbols};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn sheet<'a>(p: &'a Project, anlage: &str, name: &str) -> (&'a Anlage, &'a Sheet) {
    let a = p.anlagen.iter().find(|a| a.name == anlage).expect("Anlage");
    (a, a.sheets.iter().find(|s| s.name == name).expect("Blatt"))
}

/// Jeder Unterblock eines Blattes (außer reinen Hilfsblöcken) wird zu einem Objekt.
#[test]
fn every_sheet_block_becomes_an_object() {
    let Some(path) = sample() else { return };
    let file = EprFile::read(&path).unwrap();
    let p = epr::import_file(&file).unwrap();
    let blocks: usize = file.root("PRJ").unwrap().kids_of("ANL").flat_map(|a| a.kids_of("BLT")).map(|b| b.kids.len()).sum();
    let objects: usize = p.anlagen.iter().flat_map(|a| &a.sheets).map(|s| s.objects.len()).sum();
    // Ausnahmen ohne Darstellung: zwei Verbindungen mit nur einem Punkt ("Sicherungen -F18, -Q1, -F1 bis -F4").
    // Der frühere Ausreißer „Sammelschiene ohne Länge“ ist ein Kontaktspiegel (OBJ Art 5, siehe unten).
    assert_eq!(objects + 2, blocks, "nicht umgesetzte Blöcke (cargo run --example unmapped)");
}

/// W2 enthält einen Kontaktspiegel ohne Zeilen, dessen Spule gelöscht ist (OBJ Art 5 auf „Sicherungen -F10, -F1
/// und -F2“, X = 205 mm); er wird gelesen und unverändert wieder geschrieben.
#[test]
fn kontaktspiegel_aus_w2() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, s) = sheet(&p, "Unterverteilung 1", "Sicherungen -F10, -F1 und -F2");
    let sp: Vec<(P, u64, usize)> = s.objects.iter().filter_map(|o| if let Kind::Spiegel { pos, owner, rows, .. } = &o.kind { Some((*pos, *owner, rows.len())) } else { None }).collect();
    assert_eq!(sp.len(), 1);
    assert!((sp[0].0.x - 205.0).abs() < 0.01, "{:?}", sp[0].0);
    assert_eq!((sp[0].1, sp[0].2), (0, 0));
}

#[test]
fn busbars_with_cross_references_like_original() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, s) = sheet(&p, "Unterverteilung 1", "Sicherungen -F10, -F1 und -F2");
    let bars: Vec<(&str, f32, &str, &str)> = s
        .objects
        .iter()
        .filter_map(|o| if let Kind::Busbar { name, a, xin, xout, .. } = &o.kind { Some((name.as_str(), a.y, xin.as_str(), xout.as_str())) } else { None })
        .collect();
    // Seite 6 des Ausdrucks: L1, L2 ohne Verweis, N "7/1", -F1.L2 "4/1", -F1.L3 "5/1"
    assert!(bars.contains(&("L1", 22.0, "", "")), "{bars:?}");
    assert!(bars.iter().any(|b| b.0 == "N" && b.3 == "7/1"), "{bars:?}");
    assert!(bars.iter().any(|b| b.0 == "-F1.L2" && b.3 == "4/1"), "{bars:?}");
    assert!(bars.iter().any(|b| b.0 == "-F1.L3" && b.3 == "5/1"), "{bars:?}");
    // Seite 18: -F1.L2 kommt von Blatt 4, Pfad 10
    let (_, s) = sheet(&p, "Unterverteilung 2", "Sicherung -F7");
    assert!(s.objects.iter().any(|o| matches!(&o.kind, Kind::Busbar { name, xin, .. } if name == "-F1.L2" && xin == "4/10")));
}

#[test]
fn terminals_and_wire_labels_like_original() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, s) = sheet(&p, "Unterverteilung 2", "Sicherung -F7");
    let labels: Vec<String> = s
        .objects
        .iter()
        .filter_map(|o| if let Kind::Terminal { tpl, leiste, nr, level, .. } = &o.kind { Some(epr::terminal_label(tpl, leiste, *nr, *level)) } else { None })
        .collect();
    for want in ["-X1.8B", "-X1.9C", "-X1.8A", "-X3.1", "-X3.4", "-X1.15C"] {
        assert!(labels.iter().any(|l| l == want), "{want} fehlt: {labels:?}");
    }
    let bez: Vec<&str> = s.objects.iter().filter_map(|o| if let Kind::Wire { bez, .. } = &o.kind { Some(bez) } else { None }).flatten().map(|(_, t)| t.as_str()).collect();
    for want in ["sw", "br", "gr"] {
        assert!(bez.contains(&want), "Aderfarbe {want} fehlt: {bez:?}");
    }
    assert!(!s.objects.iter().any(|o| matches!(&o.kind, Kind::Verweis { text, .. } if text.contains('%'))), "Klemmstellen dürfen keine Verweise sein");
}

#[test]
fn tables_images_and_parameters() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, s) = sheet(&p, "Netzwerkschrank =S1+W2", "-PP1");
    let t = s.objects.iter().find_map(|o| if let Kind::Table { rows, cols, cells, .. } = &o.kind { Some((*rows, *cols, cells.clone())) } else { None }).expect("Tabelle");
    assert_eq!((t.0, t.1), (13, 2));
    assert_eq!(&t.2[..4], &["Port", "Angeschlossenes Betriebsmittel", "1", "-PP1.3=S2+W2"]);
    assert_eq!(p.bilder.len(), 4);
    assert!(p.bilder.iter().all(|b| b.jpeg.starts_with(&[0xff, 0xd8])), "JPEG-Daten");
    assert!(p.bilder[0].jpeg.len() > 65535, "über 64 KB geteilte Bilder werden zusammengesetzt");
    // Parameter: LS-Schalter zeigt den Instanzwert "B10" statt %<Typ>
    let def = p.symdefs.iter().find(|d| d.params.iter().any(|(_, n, _)| n == "Typ")).expect("Symbol mit Parameter");
    assert_eq!(symbols::expand_text("%<Typ>", def, &[(1, "B10".into())], 0), "B10");
    assert_eq!(symbols::expand_text("%(\"NO/NC\";11;13)", def, &[], 1), "13");
}

#[test]
fn bmk_templates_are_resolved_for_display() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (a, s) = sheet(&p, "Unterverteilung 1", "Sicherungen -F10, -F1 und -F2");
    let ctx = canvas::bmk_ctx(a, s);
    let shown: Vec<String> = s.objects.iter().filter_map(|o| if let Kind::Symbol { bmk, ext, .. } = &o.kind { Some(symbols::display_bmk(bmk, ext.counter, &ctx)) } else { None }).collect();
    for want in ["-F10", "-F1", "-F2", "-X1.1+W2", "-X1.11+W2", "-F1-X1.4"] {
        assert!(shown.iter().any(|b| b == want), "{want} fehlt: {shown:?}");
    }
}

#[test]
fn generated_sheets_fuse_overview_and_terminal_plan() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (a, s) = sheet(&p, "Unterverteilung 1", "Sicherungsbezeichnung");
    assert_eq!(s.art, 5);
    let rows = canvas::fuse_rows(a);
    let first: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    // Seite 5 des Ausdrucks: -F1 … -F10, -K1
    assert_eq!(first, ["-F1", "-F2", "-F3", "-F4", "-F5", "-F6", "-F7", "-F8", "-F9", "-F10", "-K1"]);
    assert_eq!(rows[0][2], "RCD für -F2 bis -F7");
    let (a, s) = sheet(&p, "Unterverteilung 1", "Klemmplan -X1");
    assert_eq!(s.art, 4);
    let k = a.klemmen.iter().find(|k| k.id == s.klm).expect("Klemmleiste zum Klemmplan");
    assert_eq!(k.name, "-X1");
    let mut nrs = k.nrs.clone();
    nrs.sort();
    assert_eq!((nrs.first(), nrs.last()), (Some(&3), Some(&19)), "Klemmen 3…19 wie im Ausdruck Seite 12");
}

/// Klemmplan -X1 (Ausdruck Seite 12): Von Ziel / Nach Ziel je Klemme und Brücken.
#[test]
fn terminal_plan_targets_like_original_print() {
    use elektroplanung::klemmplan;
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (a, _) = sheet(&p, "Unterverteilung 1", "Klemmplan -X1");
    let plan = klemmplan::plan(a, "-X1", &p.symdefs);
    let f = |z: &Option<klemmplan::Ziel>| z.as_ref().map(|z| format!("{} {} {}", z.kennung, z.kl, z.pfad)).unwrap_or_default();
    let rows: Vec<(u32, String, String)> = plan.zeilen.iter().map(|z| (z.nr, f(&z.von), f(&z.nach))).collect();
    let want: &[(u32, &str, &str)] = &[
        (3, "-F2.L (Schiene -F2.L)  3/1", "-Q1.1.1+W2 11 3/11"),
        (4, "-Q1.2.1+W2 11 3/11", ""),
        (5, "-Q1.3+W2 11 3/11", ""),
        (6, "-Q1.1.1+W2 14 3/11", "-Q1.1.2+W2 14 3/11"),
        (7, "-Q1.1.1+W2 12 3/11", "-Q1.1.2+W2 12 3/11"),
        (8, "-Q1.2.1+W2 14 3/11", "-Q1.2.2+W2 14 3/11"),
        (9, "-Q1.2.1+W2 12 3/11", "-Q1.2.2+W2 12 3/11"),
        (10, "-Q1.1.2+W2 11 3/11", "-E1.1+W2 1 3/11"),
        (11, "-Q1.2.2+W2 11 3/11", "-E1.2+W2 1 3/11"),
        (12, "-Q1.1.1+W2 N 3/9", "-Q1.1.2+W2 N 3/10"),
        (13, "-M1.1+W2 U1 3/15", "-Q1.2.2+W2 N 3/9"),
        (14, "-E1.1+W2 2 3/11", "-M1.1+W2 U1 3/15"),
        (15, "-E1.2+W2 2 3/11", "-F2.N (Schiene -F2.N)  3/1"),
        (16, "-Q1.1.1+W2 PE 3/9", "-Q1.1.2+W2 PE 3/10"),
        (17, "-M1.1+W2 PE 3/15", "-Q1.2.2+W2 PE 3/9"),
        (18, "-E1.1+W2 PE 3/11", "-M1.1+W2 PE 3/15"),
        (19, "-E1.3+W2 PE 3/14", "PE (Schiene PE)  3/1"),
    ];
    for (w, r) in want.iter().zip(&rows) {
        assert_eq!((w.0, w.1, w.2), (r.0, r.1.as_str(), r.2.as_str()), "Klemme {}", w.0);
    }
    assert_eq!(rows.len(), want.len());
    // Brücken 3–5, 12/14, 13/15 (zweite Spur), 16/18, 17/19 (zweite Spur)
    let br: Vec<(Vec<u32>, usize)> = plan.bruecken.iter().map(|b| (b.rows.iter().map(|r| plan.zeilen[*r].nr).collect(), b.lane)).collect();
    assert_eq!(br, vec![(vec![3, 4, 5], 0), (vec![12, 14], 0), (vec![13, 15], 1), (vec![16, 18], 0), (vec![17, 19], 1)]);
}

#[test]
fn sheet_dates_like_original_print() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // Ausdruck Seite 6: "Erstellt: 01.03.2025", "Geändert: 22.03.2025"
    let (_, s) = sheet(&p, "Unterverteilung 1", "Sicherungen -F10, -F1 und -F2");
    assert_eq!((s.erstellt.as_str(), s.geaendert.as_str()), ("01.03.2025", "22.03.2025"));
}
