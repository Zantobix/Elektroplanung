//! Neu angelegte Blattobjekte im .epr-Format: je Objektart in einem neuen Projekt (Vorlage) und in einer Kopie von
//! W2.epr anlegen, schreiben, wieder lesen und die Daten vergleichen; dazu die Blockstruktur mit einem vom Original
//! geschriebenen Objekt derselben Art aus W2 (gleiche Unterblöcke, gleiche Datenlänge).

use elektroplanung::{epr, epr_write, eprfile::{Block, EprFile}, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn obj(p: &mut Project, kind: Kind, pen: usize, fixed: bool) -> Obj {
    Obj { id: p.alloc_id(), kind, pen, fixed, group: None, raw: None }
}

fn text_kind() -> Kind {
    let (mn, mx) = (P::new(40.0, 50.0), P::new(90.0, 60.0));
    let style = TextStyle { bold: true, underline: true, color: [200, 0, 0], font: "Arial".into(), rect: Some((mn, mx)), ..Default::default() };
    Kind::Text { pos: epr::text_anchor(mn, mx, Align::Center, false), text: "Neuer Text".into(), size: 10.0 * 0.3528, align: Align::Center, vertical: false, style }
}

/// Testbild (Rauschen, damit das JPEG über 64 KB groß wird und geteilt werden muss).
fn jpeg(w: u32, h: u32) -> Vec<u8> {
    let mut s: u32 = 12345;
    let img = image::RgbImage::from_fn(w, h, |_, _| {
        s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
        image::Rgb([(s >> 16) as u8, (s >> 8) as u8, (s >> 24) as u8])
    });
    let mut out = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95).encode_image(&img).unwrap();
    out
}

/// Alle Objektarten (außer Klemmstelle und Symbol, die gesondert geprüft werden).
fn kinds() -> Vec<(&'static str, Kind, usize, bool)> {
    vec![
        ("Linie", Kind::Line { a: P::new(20.0, 30.0), b: P::new(60.0, 30.0) }, 1, false),
        ("Kurve", Kind::Curve { a: P::new(20.0, 40.0), c1: P::new(30.0, 30.0), c2: P::new(40.0, 50.0), b: P::new(50.0, 40.0) }, 0, false),
        ("Rechteck", Kind::Rect { a: P::new(20.0, 60.0), b: P::new(50.0, 80.0) }, 2, true),
        ("Polygon", Kind::Poly { pts: vec![P::new(60.0, 60.0), P::new(80.0, 60.0), P::new(70.0, 75.0)] }, 0, false),
        ("Kreis", Kind::Circle { a: P::new(100.0, 60.0), b: P::new(120.0, 80.0) }, 3, false),
        ("Text", text_kind(), 0, true),
        ("Textbox", elektroplanung::textbox::neu(P::new(130.0, 30.0), P::new(180.0, 50.0), "Zeile 1\nZeile 2"), 0, false),
        (
            "Tabelle",
            Kind::Table { a: P::new(20.0, 100.0), b: P::new(120.0, 130.0), rows: 3, cols: 2, cells: ["Nr", "Name", "1", "Ä-Ö", "", "x"].map(String::from).to_vec(), header: true, fixed_w: true, art: 0, ab: 0 },
            0,
            true,
        ),
        ("Verbindung", Kind::Wire { pts: vec![P::new(150.0, 100.0), P::new(150.0, 140.0), P::new(200.0, 140.0)], leitung: 0, bez: vec![(P::new(150.0, 120.0), "sw".into())], opts: 0 }, 0, false),
        ("Verweis", Kind::Verweis { pos: P::new(250.0, 100.0), text_pos: P::new(257.0, 99.0), text: "=Neu".into(), dir: 0, vertical: false, spitze: false, ziel: None }, 0, false),
        ("Verweis hochkant", Kind::Verweis { pos: P::new(250.0, 120.0), text_pos: P::new(252.0, 110.0), text: "nach oben".into(), dir: 90, vertical: true, spitze: false, ziel: None }, 0, false),
        ("Sammelschiene", Kind::Busbar { a: P::new(30.0, 150.0), x2: 200.0, name: "Neue Schiene".into(), label: P::new(0.0, -4.0), style: 2, sms: 0, xin: String::new(), xout: String::new() }, 0, false),
    ]
}

fn reload(p: &Project) -> (Project, EprFile) {
    let bytes = epr_write::to_bytes(p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    (epr::import_file(&f).unwrap(), f)
}

/// Gleichheit zweier Formen mit Toleranz für Zahlen (0,1-mm-Raster, Schriftgrad in pt).
fn close(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;
    match (a, b) {
        (Number(x), Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 0.06,
        (Array(x), Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b)),
        (Object(x), Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w))),
        _ => a == b,
    }
}

/// Erwartete Form nach dem Lesen: Kennung der Schiene und Bildpfad vergibt erst das Speichern.
fn normalize(k: &Kind) -> Kind {
    let mut k = k.clone();
    if let Kind::Busbar { sms, .. } = &mut k {
        *sms = 0;
    }
    k
}

fn assert_same(name: &str, want: &Kind, got: &Kind) {
    let (a, b) = (serde_json::to_value(normalize(want)).unwrap(), serde_json::to_value(normalize(got)).unwrap());
    assert!(close(&a, &b), "{name}: geschrieben {a}\n gelesen {b}");
}

fn tag_set(b: &Block) -> std::collections::BTreeSet<String> {
    b.kids.iter().map(|k| k.tag_str().to_string()).collect()
}

/// Vom Original geschriebener Block derselben Art in W2.epr.
fn w2_vorbild(w2: &EprFile, b: &Block) -> Option<Block> {
    let mut found = None;
    fn walk(x: &Block, want: &Block, found: &mut Option<Block>) {
        if found.is_some() {
            return;
        }
        let same = x.tag == want.tag
            && match x.tag_str() {
                "ZCN" => x.data.get(0x3c) == want.data.get(0x3c),
                "OBJ" => x.i32(4) == want.i32(4) && x.kid("TIT").is_some() == want.kid("TIT").is_some(),
                "VBG" => x.kid("BZD").is_some() == want.kid("BZD").is_some(),
                "VSY" => tag_set(x) == tag_set(want),
                _ => true,
            };
        if same && x.data.len() >= 0x3c && x.tag_str() != "BLT" && x.tag_str() != "ANL" && x.tag_str() != "PRJ" {
            *found = Some(x.clone());
            return;
        }
        for k in &x.kids {
            walk(k, want, found);
        }
    }
    for r in &w2.roots {
        walk(r, b, &mut found);
    }
    found
}

/// Neues Projekt aus der Vorlage: jede Objektart einzeln anlegen, schreiben, lesen.
#[test]
fn new_objects_roundtrip_in_new_project() {
    for (name, kind, pen, fixed) in kinds() {
        let mut p = Project::new_default();
        let o = obj(&mut p, kind.clone(), pen, fixed);
        p.anlagen[0].sheets[0].objects.push(o);
        assert!(epr_write::unsupported(&p).is_empty(), "{name}: {:?}", epr_write::unsupported(&p));
        let (back, _) = reload(&p);
        let objs = &back.anlagen[0].sheets[0].objects;
        assert_eq!(objs.len(), 1, "{name}: {objs:?}");
        assert_same(name, &kind, &objs[0].kind);
        if !matches!(kind, Kind::Wire { .. } | Kind::Verweis { .. } | Kind::Busbar { .. }) {
            assert_eq!((objs[0].pen, objs[0].fixed), (pen, fixed), "{name}: Stift/Fixiert");
        }
        if let Kind::Busbar { name, .. } = &kind {
            assert!(back.anlagen[0].sammelschienen.contains(name), "neue Schiene fehlt: {:?}", back.anlagen[0].sammelschienen);
        }
    }
}

/// Neues Symbol aus der eingebauten Bibliothek mit BMK, Wert, Zähler, Alternativtext und Parameter.
#[test]
fn new_symbol_with_all_properties() {
    let mut p = Project::new_default();
    let def = elektroplanung::symbols::builtin().into_iter().next().unwrap();
    let key = def.key.clone();
    p.symdefs.push(def);
    let ext = SymExt { funktion: "Licht".into(), counter: 3, texts: vec![(1, "PE".into())], params: vec![(1, "B10".into())], querverweise: false, sicherung: true, ..Default::default() };
    let kind = Kind::Symbol { key, pos: P::new(100.0, 100.0), rot: 1, mirror_h: true, mirror_v: false, bmk: "-F%C".into(), value: "16A".into(), scale: 1.5, option: 0, title: Some((P::new(5.0, -3.0), false)), flags: 2, ext };
    let o = obj(&mut p, kind.clone(), 0, false);
    p.anlagen[0].sheets[0].objects.push(o);
    let (back, _) = reload(&p);
    let got = &back.anlagen[0].sheets[0].objects[0].kind;
    let (Kind::Symbol { pos, rot, mirror_h, bmk, value, scale, flags, ext, title, .. }, Kind::Symbol { ext: want, .. }) = (got, &kind) else { panic!("{got:?}") };
    assert_eq!((*pos, *rot, *mirror_h, bmk.as_str(), value.as_str(), *scale, *flags), (P::new(100.0, 100.0), 1, true, "-F%C", "16A", 1.5, 2));
    assert_eq!(title.map(|t| t.0), Some(P::new(5.0, -3.0)));
    assert_eq!((ext.funktion.as_str(), ext.counter, ext.querverweise, ext.sicherung), ("Licht", 3, false, true));
    assert_eq!((&ext.texts, &ext.params), (&want.texts, &want.params));
}

/// Bilder: Projektbild (bld:) und Bilddatei – die Datei wird als JPEG ins Projekt übernommen, über 64 KB geteilt.
#[test]
fn new_images_with_project_pictures() {
    let mut p = Project::new_default();
    let klein = jpeg(40, 30);
    p.bilder.push(Bild { name: "Logo".into(), nr: 1, jpeg: klein.clone() });
    let gross = jpeg(400, 300);
    assert!(gross.len() > 0x1_0000 * 2, "Testbild zu klein: {}", gross.len());
    let datei = std::env::temp_dir().join("ep_neu_test_bild.jpg");
    std::fs::write(&datei, &gross).unwrap();
    let a = Kind::Image { a: P::new(20.0, 20.0), b: P::new(60.0, 50.0), path: "bld:1".into(), mode: 2, frame: true };
    let b = Kind::Image { a: P::new(70.0, 20.0), b: P::new(150.0, 80.0), path: datei.to_string_lossy().to_string(), mode: 1, frame: false };
    for k in [a.clone(), b.clone()] {
        let o = obj(&mut p, k, 0, false);
        p.anlagen[0].sheets[0].objects.push(o);
    }
    let (back, f) = reload(&p);
    let objs = &back.anlagen[0].sheets[0].objects;
    assert_same("Bild bld", &a, &objs[0].kind);
    let Kind::Image { path, mode, frame, .. } = &objs[1].kind else { panic!() };
    assert_eq!((*mode, *frame), (1, false));
    let nr: i32 = path.strip_prefix("bld:").unwrap().parse().unwrap();
    assert_eq!(back.bilder.iter().find(|b| b.nr == nr).map(|b| b.jpeg.clone()), Some(gross.clone()), "Bilddatei als Projektbild");
    assert_eq!(back.bilder.iter().find(|b| b.nr == 1).map(|b| b.jpeg.clone()), Some(klein));
    let jpg = f.root("PRJ").unwrap().kids_of("BLD").find(|b| b.i32(5) == nr).unwrap().kid("JPG").unwrap().clone();
    assert_eq!(jpg.data.len(), 0xffff);
    assert_eq!(jpg.chunk.map(|c| c[1] as usize), Some(jpg.parts.len() + 1));
    let _ = std::fs::remove_file(datei);
}

/// Verweis und Gegenverweis auf zwei neuen Blättern zeigen nach dem Lesen aufeinander.
#[test]
fn new_cross_references_link_each_other() {
    let mut p = Project::new_default();
    p.anlagen[0].sheets.push(Sheet::new(2));
    let (ida, idb) = (p.alloc_id(), p.alloc_id());
    let anl = p.anlagen[0].name.clone();
    let v = |pos: P, text: &str, blatt: u32, obj: u64| Kind::Verweis { pos, text_pos: P::new(pos.x + 7.0, pos.y - 1.0), text: text.into(), dir: 0, vertical: false, spitze: false, ziel: Some(VerweisZiel { anlage: anl.clone(), blatt, obj }) };
    p.anlagen[0].sheets[0].objects.push(Obj { id: ida, kind: v(P::new(250.0, 50.0), "nach 2", 2, idb), pen: 0, fixed: false, group: None, raw: None });
    p.anlagen[0].sheets[1].objects.push(Obj { id: idb, kind: v(P::new(30.0, 50.0), "von 1", 1, ida), pen: 0, fixed: false, group: None, raw: None });
    let (back, _) = reload(&p);
    let a = &back.anlagen[0].sheets[0].objects[0];
    let b = &back.anlagen[0].sheets[1].objects[0];
    let ziel = |o: &Obj| if let Kind::Verweis { ziel, .. } = &o.kind { ziel.clone() } else { None };
    assert_eq!(ziel(a), Some(VerweisZiel { anlage: anl.clone(), blatt: 2, obj: b.id }));
    assert_eq!(ziel(b), Some(VerweisZiel { anlage: anl, blatt: 1, obj: a.id }));
}

/// W2-Kopie: jede Objektart auf ein vorhandenes Blatt setzen, schreiben, lesen; Blockstruktur wie vom Original.
#[test]
fn new_objects_in_w2_copy_match_original_blocks() {
    let Some(path) = sample() else { return };
    let w2 = EprFile::read(&path).unwrap();
    let base = epr::import(&path).unwrap();
    let mut all = kinds();
    // Verweis mit Ziel auf ein vorhandenes Blatt der eigenen Anlage
    let anl = base.anlagen[0].name.clone();
    let blatt = base.anlagen[0].sheets[1].number;
    all.push(("Verweis mit Ziel", Kind::Verweis { pos: P::new(260.0, 140.0), text_pos: P::new(267.0, 139.0), text: "Ziel".into(), dir: 0, vertical: false, spitze: false, ziel: Some(VerweisZiel { anlage: anl, blatt, obj: 0 }) }, 0, false));
    // Symbol eines vorhandenen Projektsymbols (Wert, Funktion)
    let key = base.symdefs.first().unwrap().key.clone();
    let ext = SymExt { funktion: "Neu".into(), ..Default::default() };
    all.push(("Symbol", Kind::Symbol { key, pos: P::new(200.0, 60.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-Q9".into(), value: "".into(), scale: 1.0, option: 0, title: Some((P::new(4.0, -2.0), false)), flags: 0, ext }, 0, false));
    for (name, kind, pen, fixed) in all {
        let mut p = base.clone();
        let o = obj(&mut p, kind.clone(), pen, fixed);
        p.anlagen[0].sheets[0].objects.push(o);
        let (back, f) = reload(&p);
        let got = back.anlagen[0].sheets[0].objects.last().unwrap();
        if let (Kind::Symbol { bmk, ext, pos, .. }, Kind::Symbol { key, .. }) = (&got.kind, &kind) {
            assert_eq!((bmk.as_str(), ext.funktion.as_str(), *pos), ("-Q9", "Neu", P::new(200.0, 60.0)));
            assert!(matches!(&got.kind, Kind::Symbol { key: k, .. } if k == key), "Symbol verweist auf das vorhandene Projektsymbol");
        } else {
            assert_same(name, &kind, &got.kind);
        }
        // vorhandene Objekte bleiben unverändert
        assert_eq!(back.anlagen[0].sheets[0].objects.len(), base.anlagen[0].sheets[0].objects.len() + 1, "{name}");
        // Blockstruktur gegen das Vorbild aus W2
        let blk = f.root("PRJ").unwrap().kids_of("ANL").next().unwrap().kids_of("BLT").next().unwrap().kids.last().unwrap().clone();
        if let Some(v) = w2_vorbild(&w2, &blk) {
            let mut tags = tag_set(&blk);
            if name == "Text" {
                // eigener Schriftname: STR direkt nach dem STL (Speicherroutine des Originals); W2 hat keinen solchen Text
                let order: Vec<&str> = blk.kids.iter().map(|k| k.tag_str()).collect();
                assert_eq!(order, ["STL", "STR"]);
                tags.remove("STR");
            }
            assert_eq!((blk.data.len(), tags), (v.data.len(), tag_set(&v)), "{name}: Blockstruktur wie im Original");
        } else {
            assert!(matches!(blk.data.get(0x3c), Some(4 | 12)), "{name}: kein Vorbild in W2");
        }
        // alle anderen Blätter/Anlagen byteidentisch
        let orig_anl: Vec<_> = w2.root("PRJ").unwrap().kids_of("ANL").skip(1).cloned().collect();
        let new_anl: Vec<_> = f.root("PRJ").unwrap().kids_of("ANL").skip(1).cloned().collect();
        assert!(orig_anl == new_anl, "{name}: andere Anlagen verändert");
    }
}

/// Ein vorhandenes Bild/Textbox/Tabelle ändern: der Originalblock wird nachgeführt; Verschieben ändert nur die Lage.
#[test]
fn changed_new_objects_are_patched() {
    let mut p = Project::new_default();
    for (_, k, pen, fixed) in kinds().into_iter().filter(|(n, ..)| matches!(*n, "Tabelle" | "Textbox" | "Text")) {
        let o = obj(&mut p, k, pen, fixed);
        p.anlagen[0].sheets[0].objects.push(o);
    }
    let (mut back, _) = reload(&p);
    for o in &mut back.anlagen[0].sheets[0].objects {
        o.kind.translate(P::new(10.0, 5.0));
        match &mut o.kind {
            Kind::Table { cells, .. } => cells[1] = "Geändert".into(),
            Kind::Text { style, text, .. } if style.font.is_empty() => {
                *text = "Neu".into();
                style.wrap = true;
            }
            Kind::Text { style, .. } => style.font = "Courier New".into(),
            _ => {}
        }
    }
    let want: Vec<Kind> = back.anlagen[0].sheets[0].objects.iter().map(|o| o.kind.clone()).collect();
    let (again, _) = reload(&back);
    for (w, g) in want.iter().zip(&again.anlagen[0].sheets[0].objects) {
        assert_same("geändert", w, &g.kind);
    }
}

/// Zellen wie das Original: durch 0 getrennt, ohne abschließende 0, leere Zellen am Ende entfallen
/// (eine abschließende 0 = eine Zelle zu viel, das Original zeigte die Tabelle dann nicht).
#[test]
fn table_cells_are_separated_like_original() {
    let mut p = Project::new_default();
    let cells = ["Nr", "", "1", "a", "", ""].map(String::from).to_vec();
    let k = Kind::Table { a: P::new(20.0, 20.0), b: P::new(80.0, 50.0), rows: 3, cols: 2, cells, header: true, fixed_w: false, art: 0, ab: 0 };
    let o = obj(&mut p, k, 0, false);
    p.anlagen[0].sheets[0].objects.push(o);
    let (back, f) = reload(&p);
    let blt = f.root("PRJ").unwrap().kids_of("ANL").next().unwrap().kids_of("BLT").next().unwrap();
    let s = &blt.kids.last().unwrap().kid("STR").unwrap().data;
    assert_eq!(s.as_slice(), b"Nr\0\x001\0a");
    let Kind::Table { cells, .. } = &back.anlagen[0].sheets[0].objects[0].kind else { panic!() };
    assert_eq!(cells, &["Nr", "", "1", "a"]);
}

fn blatt_anlage_ok(f: &EprFile) -> bool {
    f.root("PRJ").unwrap().kids_of("ANL").all(|a| a.kids_of("BLT").all(|b| b.data[0x50..0x58] == a.data[0x88..0x90]))
}

/// Jedes Blatt trägt die Kennung seiner Anlage (BLT 0x50 = ANL 0x88) – wie in W2; auch neue Blätter und Anlagen.
#[test]
fn sheets_carry_their_anlage_id() {
    let mut p = Project::new_default();
    p.anlagen[0].sheets.push(Sheet::new(2));
    let mut a2 = p.anlagen[0].clone();
    a2.name = "Anlage 2".into();
    p.anlagen.push(a2);
    let (_, f) = reload(&p);
    assert!(blatt_anlage_ok(&f));
    let ids: Vec<_> = f.root("PRJ").unwrap().kids_of("ANL").map(|a| a.data[0x88..0x90].to_vec()).collect();
    assert_ne!(ids[0], ids[1], "neue Anlagen brauchen eigene Kennungen");
    let Some(path) = sample() else { return };
    let w2 = EprFile::read(&path).unwrap();
    assert!(blatt_anlage_ok(&w2), "Annahme aus W2");
    let mut p = epr::import(&path).unwrap();
    let n = p.anlagen[3].sheets.len() as u32 + 1;
    p.anlagen[3].sheets.push(Sheet::new(n));
    let (_, f) = reload(&p);
    assert!(blatt_anlage_ok(&f));
}
