//! Klemmstellen, Klemmleisten und Klemmpläne gegen das Beispielprojekt (W2.epr) und den Original-Ausdruck
//! "Elektroplanung Wohnung 2.pdf" (Seiten 13, 20, 21, 22, 39). Ohne EP_SAMPLE werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, eprfile::EprFile, klemmen, klemmplan, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn anlage<'a>(p: &'a Project, name: &str) -> (usize, &'a Anlage) {
    let i = p.anlagen.iter().position(|a| a.name == name).expect("Anlage");
    (i, &p.anlagen[i])
}

/// Zeilen eines Klemmplans als "Nr+Etage | Von | Nach".
fn rows(p: &Project, anl: &str, leiste: &str) -> Vec<String> {
    let (_, a) = anlage(p, anl);
    let plan = klemmplan::plan(a, leiste, &p.symdefs);
    let f = |z: &Option<klemmplan::Ziel>| z.as_ref().map(|z| format!("{} {} {}", z.kennung, z.kl, z.pfad).split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
    plan.zeilen.iter().map(|z| format!("{}{} | {} | {}", z.nr, if z.level > 0 { ((b'A' + z.level - 1) as char).to_string() } else { String::new() }, f(&z.von), f(&z.nach))).collect()
}

fn check(got: &[String], want: &[&str]) {
    for (w, g) in want.iter().zip(got) {
        assert_eq!(g, w);
    }
    assert_eq!(got.len(), want.len(), "{got:#?}");
}

/// Seite 13 (Unterverteilung 1, -X2): Ziel Wechsel (5, 10, 11, 13) und Eigentümer "-K1" statt "-Q1+W2" (9).
#[test]
fn page13_direction_slots_target_change_and_owner() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    check(
        &rows(&p, "Unterverteilung 1", "-X2"),
        &[
            "1 | -F3 L 4/2 | -X2.1+W2 L 4/4",
            "2 | -X2.8+W2 L 4/4 | ",
            "3 | -F7 L 6/4 | -X7.1+W2 L 6/8",
            "4 | -S7.4+W2 14 6/8 | ",
            "5 | -K1 A1 6/5 | -S7.1+W2 13 6/8",
            "6 | -S7.2+W2 13 6/10 | ",
            "7 | -S7.3+W2 13 6/10 | ",
            "8 | -S7.4+W2 13 6/8 | ",
            "9 | -K1 2 6/6 | -E7.1+W2 1 6/8",
            "10 | -K1 A2 6/5 | -X7.1+W2 N 6/8",
            "11 | -E7.1+W2 2 6/8 | ",
            "12 | -F1.N (Schiene -F1.N) 6/1 | ",
            "13 | -E7.1+W2 PE 6/8 | -X7.1+W2 PE 6/8",
            "14 | -X7.10+W2 PE 6/8 | ",
            "15 | PE (Schiene PE) 6/1 | ",
        ],
    );
}

/// Seite 20 (Unterverteilung 2, -X1, drei Etagen): u. a. 1A mit Ziel Wechsel, 1B über eine kurze Verbindung
/// innerhalb des Klemmenkreises auf die Schiene, 14B/15C mit Eigentümer -K1.
#[test]
fn page20_multilevel_terminal_plan() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let got = rows(&p, "Unterverteilung 2", "-X1");
    for want in [
        "1A | PE (Schiene PE) 2/1 | -E3.1+W2 PE 2/6",
        "1B | -F2.L (Schiene -F2.L) 2/3 | -Q3.1+W2 14 2/4",
        "1C | -F2.N (Schiene -F2.N) 2/2 | -E3.1+W2 2 2/6",
        "2A | PE (Schiene PE) 2/1 | -M3.1+W2 PE 2/6",
        "3A | PE (Schiene PE) 2/1 | -X3.7+W2 PE 2/4",
        "7A | PE (Schiene PE) 4/1 | PE 4/10",
        "9B | -X3 4 5/4 | -S10.1+W2 13 5/6",
        "14A | PE (Schiene PE) 5/1 | -E10.1+W2 PE 5/6",
        "14B | -K1 2 5/4 | -E10.1+W2 1 5/6",
        "15C | -F1.N (Schiene -F1.N) 5/1 | -K1 A2 5/4",
    ] {
        assert!(got.iter().any(|g| g == want), "{want} fehlt: {got:#?}");
    }
    assert_eq!(got.len(), 40);
}

/// Seite 21 (-X2 mit Klemme 0) und Seite 22 (-X3: Ziele auf Klemmen einer anderen Leiste ohne Etage).
#[test]
fn pages21_22_targets() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let x2 = rows(&p, "Unterverteilung 2", "-X2");
    assert_eq!(x2[0], "0B | -F9 L 6/2 | -Q9.1+W2 14 6/4");
    assert_eq!(x2[3], "1C | -F8 N 6/1 | -Q9.1+W2 N 6/3");
    check(
        &rows(&p, "Unterverteilung 2", "-X3"),
        &["1 | -F7 L 5/3 | -X1 8 5/5", "2 | -X1 10 5/5 | -X1 12 5/5", "3 | -K1 A1 5/4 | ", "4 | -X1 9 5/5 | -X1 9 5/5", "5 | -X1 11 5/5 | -X1 11 5/5", "6 | -X1 13 5/5 | -X1 13 5/5"],
    );
}

/// Seite 39 (=S3, -X2): Verbindungen zu Klemmen derselben Leiste ohne "Verbindung ist Brücke" sind Ziele.
#[test]
fn page39_same_strip_targets_without_bridge_flag() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let got = rows(&p, "Heizungssteuerung =S3+W2", "-X2");
    check(
        &got[..6],
        &[
            "1A | -X1.N=S3 (Schiene -X1.N=S3) 1/1 | ",
            "1B | -X1.L=S3 (Schiene -X1.L=S3) 1/1 | -X2 2 1/6",
            "2A | -P5.1+W2 N 1/6 | ",
            "2B | -X2 1 1/5 | -P5.1+W2 1 1/6",
            "3A | -X2 2 1/5 | -M1=S3 2 1/5",
            "3B | -P5.1+W2 2 1/6 | -M1=S3 1 1/5",
        ],
    );
    assert_eq!(got[6], "4A | -X2 3 1/7 | -P6.1+W2 N 1/8");
}

/// Klemmen-Einstellungen aus W2: Ziel Wechsel bei -X1 Klemme 4/5 (Unterverteilung 1), Gruppenautomatik überall an.
#[test]
fn terminal_options_are_read() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, a) = anlage(&p, "Unterverteilung 1");
    let opts: Vec<(u32, bool, bool)> = a.sheets.iter().flat_map(|s| &s.objects).filter_map(|o| if let Kind::Terminal { leiste, nr, opt, .. } = &o.kind { (leiste == "-X1").then_some((*nr, opt.ziel_wechsel, opt.gr_auto)) } else { None }).collect();
    assert!(opts.contains(&(4, true, true)) && opts.contains(&(5, true, true)) && opts.contains(&(3, false, true)), "{opts:?}");
    let k = a.klemmen.iter().find(|k| k.name == "-X1").unwrap();
    assert_eq!((k.id, k.etagen, k.titel_off), (0x1c, 1, (0, -50)));
}

/// Gruppenautomatik wie im Ausdruck Seite 38: "-X2.2A", daneben nur "2B", "3A"; "-X2.8A" mit über 2 cm Abstand.
#[test]
fn group_automatic_labels_like_print() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let (_, a) = anlage(&p, "Heizungssteuerung =S3+W2");
    let s = &a.sheets[0];
    let labels: Vec<String> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Terminal { .. })).map(|o| klemmen::label_on_sheet(Some(s), o)).collect();
    for want in ["-X2.2A", "2B", "3A", "4A", "4B", "-X2.8A", "-X2.8B", "-X2.1A"] {
        assert!(labels.iter().any(|l| l == want), "{want} fehlt: {labels:?}");
    }
}

/// Geänderte Klemmen-Einstellungen werden geschrieben und wieder gelesen; andere Klemmstellen bleiben gleich.
#[test]
fn changed_terminal_options_roundtrip() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, _) = anlage(&p, "Unterverteilung 1");
    let mut target = None;
    for s in &mut p.anlagen[ai].sheets {
        for o in &mut s.objects {
            if let Kind::Terminal { leiste, nr, opt, .. } = &mut o.kind {
                if leiste == "-X1" && *nr == 3 && target.is_none() {
                    opt.funktion = "Licht Küche".into();
                    opt.ziel_wechsel = true;
                    opt.align = 0;
                    opt.position = 30;
                    opt.gr_auto = false;
                    target = Some(o.id);
                }
            }
        }
    }
    assert!(target.is_some());
    let bytes = epr_write::to_bytes(&p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    let q = epr::import_file(&f).unwrap();
    let (_, a) = anlage(&q, "Unterverteilung 1");
    let t = a.sheets.iter().flat_map(|s| &s.objects).find_map(|o| if let Kind::Terminal { leiste, nr, opt, .. } = &o.kind { (leiste == "-X1" && *nr == 3).then_some(opt.clone()) } else { None }).unwrap();
    assert_eq!((t.funktion.as_str(), t.ziel_wechsel, t.align, t.position, t.gr_auto, t.auto), ("Licht Küche", true, 0, 30, false, true));
    // KFO-Eintrag führt die neue Position, die Leiste wird nicht verdoppelt
    let anl = f.root("PRJ").unwrap().kids_of("ANL").find(|b| b.data.starts_with(b"Unterverteilung 1")).unwrap();
    let klm: Vec<_> = anl.kids_of("KLM").collect();
    assert_eq!(klm.len(), 2);
    assert!(klm[0].kids_of("KFO").any(|k| k.i32(4) == 3 && k.i32(5) == 30));
    // Klemmplan: Funktion und Reihenfolge nach Position (Klemme 3 jetzt zuletzt)
    let plan = klemmplan::plan(a, "-X1", &q.symdefs);
    let last = plan.zeilen.last().unwrap();
    assert_eq!((last.nr, last.funktion.as_str()), (3, "Licht Küche"));
    // alle anderen Klemmstellen unverändert
    let orig = EprFile::read(&path).unwrap();
    let count = |f: &EprFile| f.root("PRJ").unwrap().kids_of("ANL").flat_map(|a| a.kids_of("BLT")).flat_map(|b| b.kids_of("OBJ")).filter(|k| klemmen::is_terminal(k)).count();
    assert_eq!(count(&orig), count(&f));
}

/// Neue Klemmleiste, neue Klemmstellen und ein neues Klemmplan-Blatt werden ins .epr geschrieben: KLM mit Titel,
/// Kennung und KFO-Einträgen (Kennung der Klemmstelle und ihres Blattes), OBJ mit der Leistenkennung, BLT mit
/// Blattart 4 und Leiste – und nach dem Laden ergibt sich derselbe Klemmplan.
#[test]
fn new_strip_terminals_and_plan_sheet_are_written() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, _) = anlage(&p, "Unterverteilung 1");
    let li = klemmen::new_leiste(&mut p, ai).unwrap();
    let name = p.anlagen[ai].klemmleisten[li].clone();
    let id = p.anlagen[ai].klemmen.iter().find(|k| k.name == name).unwrap().id;
    assert!(id > 0xbb, "neue Kennung nach allen vorhandenen: {id}");
    // zwei Klemmstellen auf dem ersten Schaltplan-Blatt
    let si = p.anlagen[ai].sheets.iter().position(|s| s.art == 3).unwrap();
    for (i, x) in [200.0f32, 210.0].iter().enumerate() {
        let oid = p.alloc_id();
        let opt = klemmen::KlemmOpt { funktion: if i == 0 { "Reserve".into() } else { String::new() }, ..Default::default() };
        p.anlagen[ai].sheets[si].objects.push(Obj { id: oid, kind: Kind::Terminal { pos: P::new(*x, 60.0), leiste: name.clone(), nr: i as u32 + 1, level: 0, text_off: None, tpl: String::new(), opt }, pen: 0, fixed: false, group: None, raw: None });
    }
    let mut sh = Sheet::new(99);
    sh.name = "Klemmplan neu".into();
    sh.art = 4;
    sh.klm = id;
    p.anlagen[ai].sheets.push(sh);
    let bytes = epr_write::to_bytes(&p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    let anl = f.root("PRJ").unwrap().kids_of("ANL").find(|b| b.data.starts_with(b"Unterverteilung 1")).unwrap();
    let klm = anl.kids_of("KLM").find(|k| k.i32(13) == id).expect("neuer KLM-Block");
    assert_eq!(klm.kid("TIT").map(|t| t.data.clone()), Some(name.as_bytes().to_vec()));
    assert_eq!((klm.i32(14), klm.i32(15), klm.i32(18)), (-1, 1, 3));
    let kfo: Vec<_> = klm.kids_of("KFO").collect();
    assert_eq!(kfo.len(), 2);
    let blt = anl.kids_of("BLT").nth(si).unwrap();
    let objs: Vec<_> = blt.kids_of("OBJ").filter(|k| k.i32(8) == id).collect();
    assert_eq!(objs.len(), 2);
    assert_eq!(objs[0].data[0..8], kfo[0].data[0..8], "KFO verweist auf die Klemmstelle");
    assert_eq!(blt.data[72..80], kfo[0].data[8..16], "KFO verweist auf das Blatt");
    assert_eq!((objs[0].i32(4), objs[0].data[0x2d], objs[0].data[0x2e]), (0x2ffff, 1, 1), "Vorgaben wie eine neue Klemme im Original");
    assert!(objs[0].kid("FKT").is_some() && objs[1].kid("FKT").is_none());
    let plan_blt = anl.kids_of("BLT").last().unwrap();
    assert_eq!((plan_blt.i32(22) & 0xff, plan_blt.i32(32) >> 8), (4, id));
    // nach dem Laden: Leiste, Klemmen und Klemmplan
    let mut q = epr::import_file(&f).unwrap();
    q.raw = Some(Box::new(f.clone()));
    let (_, a) = anlage(&q, "Unterverteilung 1");
    assert!(a.klemmleisten.contains(&name));
    let s = a.sheets.last().unwrap();
    assert_eq!((s.art, s.klm), (4, id));
    let plan = klemmplan::plan(a, &name, &q.symdefs);
    assert_eq!(plan.zeilen.iter().map(|z| (z.nr, z.funktion.as_str())).collect::<Vec<_>>(), vec![(1, "Reserve"), (2, "")]);
    // erneutes Speichern ist stabil
    let again = epr_write::to_bytes(&q).unwrap();
    let diff = bytes.iter().zip(&again).position(|(a, b)| a != b);
    assert_eq!((again.len(), diff), (bytes.len(), None), "{:?}", diff.map(|d| (&bytes[d.saturating_sub(40)..d + 40], &again[d.saturating_sub(40)..d + 40])));
}
