//! Gegenprobe mit Dateien, die das ORIGINAL-Programm gespeichert hat (tests/fixtures, je eine Änderung an
//! W2.epr im Eigenschaftendialog des Originals vorgenommen und dort gespeichert):
//! - x_rahmen.epr: Text "Raum 2" → Rahmen an
//! - x_fix.epr:    Text "Raum 2" → Fixiert an
//! - x_align.epr:  Text "Raum 2" → Textausrichtung Mittig
//! - x_sym1.epr:   Symbol -X2.4 → Querverweise aus, Sicherungsübersicht an, Spiegeln Horizontal
//! Der Nachbau liest diese Einstellungen und schreibt bei derselben Änderung dieselben Feldwerte.

use elektroplanung::{epr, epr_write, eprfile::{Block, EprFile}, model::*};
use std::path::{Path, PathBuf};

fn w2() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}
fn fixture(n: &str) -> Option<PathBuf> {
    // vom Original gespeicherte Vergleichsdateien: Ordner aus EP_FIXTURES, sonst tests/fixtures
    let dir = std::env::var("EP_FIXTURES").map(std::path::PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let p = dir.join(n);
    p.exists().then_some(p)
}

fn find_text<'a>(p: &'a mut Project, t: &str) -> &'a mut Obj {
    p.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).find(|o| matches!(&o.kind, Kind::Text { text, .. } if text == t)).expect("Text fehlt")
}
fn find_sym<'a>(p: &'a mut Project, b: &str) -> &'a mut Obj {
    p.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).find(|o| matches!(&o.kind, Kind::Symbol { bmk, .. } if bmk == b)).expect("Symbol fehlt")
}

/// Block eines Objekts in einer Datei finden (ZCN mit STL-Text bzw. VSY mit TIT).
fn block_of<'a>(f: &'a EprFile, tag: &str, label: &str) -> &'a Block {
    fn walk<'a>(b: &'a Block, tag: &str, label: &str) -> Option<&'a Block> {
        if b.is(tag) {
            let kid = if tag == "ZCN" { "STL" } else { "TIT" };
            if let Some(t) = b.kid(kid) {
                if String::from_utf8_lossy(&t.data).trim_end_matches('\0').trim_end() == label {
                    return Some(b);
                }
            }
        }
        b.kids.iter().find_map(|k| walk(k, tag, label))
    }
    f.roots.iter().find_map(|r| walk(r, tag, label)).expect("Block fehlt")
}

fn same_fields(ours: &Block, orig: &Block, fields: &[usize]) {
    for &i in fields {
        // Umriss (32–35): das Original rundet beim Spiegeln um höchstens 0,1 mm anders
        let tol = if (32..=35).contains(&i) { 1 } else { 0 };
        assert!((ours.i32(i) - orig.i32(i)).abs() <= tol, "Feld {i} weicht vom Original ab: {} statt {}", ours.i32(i), orig.i32(i));
    }
}

fn save_and_read(p: &Project, name: &str) -> EprFile {
    let out = std::env::temp_dir().join(name);
    epr_write::save(p, &out).unwrap();
    EprFile::read(&out).unwrap()
}

#[test]
fn rahmen_like_original() {
    let (Some(src), Some(fx)) = (w2(), fixture("x_rahmen.epr")) else { return };
    let orig = epr::import(&fx).unwrap();
    let mut o2 = orig.clone();
    assert!(matches!(&find_text(&mut o2, "Raum 2").kind, Kind::Text { style, .. } if style.frame), "Rahmen aus Originaldatei nicht erkannt");
    let mut p = epr::import(&src).unwrap();
    if let Kind::Text { style, .. } = &mut find_text(&mut p, "Raum 2").kind {
        style.frame = true;
    }
    let ours = save_and_read(&p, "ep_rahmen.epr");
    let of = EprFile::read(&fx).unwrap();
    same_fields(block_of(&ours, "ZCN", "Raum 2"), block_of(&of, "ZCN", "Raum 2"), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16]);
}

#[test]
fn fixiert_like_original() {
    let (Some(src), Some(fx)) = (w2(), fixture("x_fix.epr")) else { return };
    let mut orig = epr::import(&fx).unwrap();
    assert!(find_text(&mut orig, "Raum 2").fixed, "Fixiert aus Originaldatei nicht erkannt");
    let mut p = epr::import(&src).unwrap();
    find_text(&mut p, "Raum 2").fixed = true;
    let ours = save_and_read(&p, "ep_fix.epr");
    let of = EprFile::read(&fx).unwrap();
    same_fields(block_of(&ours, "ZCN", "Raum 2"), block_of(&of, "ZCN", "Raum 2"), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16]);
}

#[test]
fn ausrichtung_like_original() {
    let (Some(src), Some(fx)) = (w2(), fixture("x_align.epr")) else { return };
    let mut orig = epr::import(&fx).unwrap();
    assert!(matches!(&find_text(&mut orig, "Raum 2").kind, Kind::Text { align: Align::Center, .. }), "Ausrichtung Mittig nicht erkannt");
    let mut p = epr::import(&src).unwrap();
    if let Kind::Text { align, pos, style, vertical, .. } = &mut find_text(&mut p, "Raum 2").kind {
        *align = Align::Center;
        let (a, b) = style.rect.unwrap();
        *pos = epr::text_anchor(a, b, Align::Center, *vertical);
    }
    let ours = save_and_read(&p, "ep_align.epr");
    let of = EprFile::read(&fx).unwrap();
    same_fields(block_of(&ours, "ZCN", "Raum 2"), block_of(&of, "ZCN", "Raum 2"), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16]);
}

#[test]
fn symbol_freigaben_und_spiegeln_like_original() {
    let (Some(src), Some(fx)) = (w2(), fixture("x_sym1.epr")) else { return };
    let mut orig = epr::import(&fx).unwrap();
    match &find_sym(&mut orig, "-X2.4").kind {
        Kind::Symbol { ext, mirror_h, .. } => {
            assert!(!ext.querverweise && ext.sicherung && *mirror_h, "Freigaben/Spiegeln aus Originaldatei nicht erkannt: {ext:?}");
        }
        _ => unreachable!(),
    }
    let mut p = epr::import(&src).unwrap();
    if let Kind::Symbol { ext, mirror_h, .. } = &mut find_sym(&mut p, "-X2.4").kind {
        ext.querverweise = false;
        ext.sicherung = true;
        // wie in der Oberfläche: Umriss beim Spiegeln mitführen
        ext.bounds = ext.bounds.map(|b| retransform_bounds(b, (0, false, false), (0, true, false)));
        *mirror_h = true; // "Spiegeln Horizontal" im Original = Bit 16, bei 0° an der senkrechten Achse
    }
    let ours = save_and_read(&p, "ep_sym1.epr");
    let of = EprFile::read(&fx).unwrap();
    same_fields(block_of(&ours, "VSY", "-X2.4"), block_of(&of, "VSY", "-X2.4"), &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 32, 33, 34, 35]);
}

#[test]
fn text_style_and_alt_text_roundtrip() {
    let Some(src) = w2() else { return };
    let mut p = epr::import(&src).unwrap();
    {
        let o = find_text(&mut p, "Raum 1");
        if let Kind::Text { style, align, .. } = &mut o.kind {
            style.bold = true;
            style.italic = true;
            style.color = [200, 0, 0];
            *align = Align::Right;
        }
    }
    if let Kind::Symbol { ext, flags, .. } = &mut find_sym(&mut p, "-X2.4").kind {
        ext.alt = 2;
        ext.funktion = "Steckdose Küche".into();
        // Parameterwert (VSY › SPA), benutzerdefinierter Text (VSY › ALT) und Zähler %C (Feld 38)
        ext.params = vec![(1, "B16".into())];
        ext.texts = vec![(2, "PE".into())];
        ext.counter = 5;
        *flags = 3;
    }
    let out = std::env::temp_dir().join("ep_style.epr");
    epr_write::save(&p, &out).unwrap();
    let mut back = epr::import(&out).unwrap();
    match &find_text(&mut back, "Raum 1").kind {
        Kind::Text { style, align, .. } => {
            assert!(style.bold && style.italic && style.color == [200, 0, 0] && *align == Align::Right, "{style:?}");
        }
        _ => unreachable!(),
    }
    match &find_sym(&mut back, "-X2.4").kind {
        Kind::Symbol { ext, flags, .. } => {
            assert_eq!((ext.alt, ext.funktion.as_str(), *flags), (2, "Steckdose Küche", 3));
            assert_eq!((ext.params.clone(), ext.texts.clone(), ext.counter), (vec![(1, "B16".to_string())], vec![(2, "PE".to_string())], 5));
        }
        _ => unreachable!(),
    }
}

#[test]
fn library_and_drawn_symbols_are_written_to_epr() {
    let Some(src) = w2() else { return };
    let mut p = epr::import(&src).unwrap();
    // ein eingebautes Ersatzsymbol (ohne Original-Elementdaten → Encoder) und ein Symbol mit Originaldaten
    let lib = elektroplanung::symbols::builtin();
    let def = lib.iter().find(|d| d.name == "Lampe").or_else(|| lib.first()).unwrap().clone();
    p.symdefs.push(def.clone());
    let id = p.alloc_id();
    let pos = P::new(150.0, 100.0);
    p.anlagen[0].sheets[0].objects.push(Obj {
        id,
        kind: Kind::Symbol { key: def.key.clone(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: "-E99".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
        pen: 0,
        fixed: false,
        group: None,
        raw: None,
    });
    assert!(epr_write::unsupported(&p).is_empty(), "{:?}", epr_write::unsupported(&p));
    let out = std::env::temp_dir().join("ep_libsym.epr");
    epr_write::save(&p, &out).unwrap();
    let f = EprFile::read(&out).unwrap();
    assert_eq!(f.bad_checksums, 0);
    let back = epr::import(&out).unwrap();
    let n_before = epr::import(&src).unwrap().symdefs.len();
    assert_eq!(back.symdefs.len(), n_before + 1, "neuer SYM-Block fehlt");
    let new_def = back.symdefs.iter().find(|d| d.name == def.name && d.bmk == def.bmk).expect("Symboldefinition fehlt");
    assert!(!new_def.prims.is_empty());
    let placed = back.anlagen[0].sheets[0].objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { bmk, .. } if bmk == "-E99")).expect("Symbol fehlt");
    if let Kind::Symbol { key, pos: q, .. } = &placed.kind {
        assert_eq!(key, &new_def.key);
        assert!(q.dist(pos) < 0.11);
    }
}

#[test]
fn library_options_survive_epr_save() {
    let (Some(src), Some(db)) = (w2(), w2().map(|p| p.parent().unwrap().join("Datenbank")).filter(|d| d.exists())) else { return };
    let lib = match elektroplanung::symbols::load_db(&db) {
        Ok(l) => l,
        Err(e) => {
            // Das Original sperrt seine Datenbank, solange es läuft
            eprintln!("Bibliothek nicht lesbar ({e}) – Test übersprungen");
            return;
        }
    };
    let def = lib.iter().find(|d| d.name == "Steckdose" && !d.flag_names.is_empty()).expect("Bibliotheks-Steckdose mit Optionen").clone();
    let mut p = epr::import(&src).unwrap();
    p.symdefs.push(def.clone());
    let id = p.alloc_id();
    p.anlagen[0].sheets[0].objects.push(Obj {
        id,
        kind: Kind::Symbol { key: def.key.clone(), pos: P::new(120.0, 120.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-X99".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 2, ext: SymExt { alt: 1, ..Default::default() } },
        pen: 0,
        fixed: false,
        group: None,
        raw: None,
    });
    let out = std::env::temp_dir().join("ep_libopts.epr");
    epr_write::save(&p, &out).unwrap();
    let back = epr::import(&out).unwrap();
    let o = back.anlagen[0].sheets[0].objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { bmk, .. } if bmk == "-X99")).unwrap();
    let Kind::Symbol { key, flags, ext, .. } = &o.kind else { unreachable!() };
    let d = back.symdefs.iter().find(|d| &d.key == key).unwrap();
    assert_eq!(d.flag_names, def.flag_names);
    assert_eq!(d.alt_sets, def.alt_sets);
    assert_eq!((*flags, ext.alt), (2, 1));
}

#[test]
fn edited_symbol_keeps_options_and_alt_texts() {
    // Symbol wie nach dem Symboleditor: ohne Original-Elementdaten, Elemente werden neu kodiert
    let Some(src) = w2() else { return };
    let mut p = epr::import(&src).unwrap();
    let mut def = p.symdefs.iter().find(|d| d.name == "Steckdose" && !d.flag_names.is_empty()).unwrap().clone();
    def.key = "proj:editiert".into();
    def.name = "Steckdose editiert".into();
    def.raw.clear();
    p.symdefs.push(def.clone());
    let id = p.alloc_id();
    p.anlagen[0].sheets[0].objects.push(Obj {
        id,
        kind: Kind::Symbol { key: def.key.clone(), pos: P::new(60.0, 60.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-X77".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 3, ext: SymExt { alt: 2, ..Default::default() } },
        pen: 0,
        fixed: false,
        group: None,
        raw: None,
    });
    let out = std::env::temp_dir().join("ep_edited_sym.epr");
    epr_write::save(&p, &out).unwrap();
    let back = epr::import(&out).unwrap();
    let d = back.symdefs.iter().find(|d| d.name == "Steckdose editiert").expect("SYM fehlt");
    assert_eq!(d.flag_names, def.flag_names);
    assert_eq!(d.alt_sets, def.alt_sets);
    // Textelement mit Nummer 4 (Anzahl) und Zusatzoption 1 ist erhalten
    let has = d.prims.iter().enumerate().any(|(i, pr)| matches!(pr, elektroplanung::symbols::Prim::TextBox(..)) && d.text_idx.get(i) == Some(&4) && d.flag.get(i) == Some(&1));
    assert!(has, "Text 4 mit Option 1 fehlt: {:?} {:?}", d.text_idx, d.flag);
}
