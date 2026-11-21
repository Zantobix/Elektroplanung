//! Kontaktspiegel, berechnete Tabellen (Inhaltsverzeichnis/Querverweise) und der Zusatz der Listenblätter im
//! .epr: anlegen bzw. ändern, schreiben, wieder lesen und die Feldwerte im geschriebenen Block prüfen – in einer
//! Kopie von W2.epr und in einem neuen Projekt aus der Vorlage.

use elektroplanung::{epr, epr_write, eprfile::{Block, EprFile}, kontaktspiegel, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn reload(p: &Project) -> (Project, EprFile) {
    let bytes = epr_write::to_bytes(p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    (epr::import_file(&f).unwrap(), f)
}

fn id_at(d: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(d[at..at + 8].try_into().unwrap())
}

/// Alle Blöcke einer Art (rekursiv).
fn blocks<'a>(f: &'a EprFile, pred: &dyn Fn(&Block) -> bool) -> Vec<&'a Block> {
    fn walk<'a>(b: &'a Block, pred: &dyn Fn(&Block) -> bool, out: &mut Vec<&'a Block>) {
        if pred(b) {
            out.push(b);
        }
        for k in &b.kids {
            walk(k, pred, out);
        }
    }
    let mut out = vec![];
    for r in &f.roots {
        walk(r, pred, &mut out);
    }
    out
}

fn spiegel_von(p: &Project) -> Vec<(SheetRef, Obj)> {
    let mut v = vec![];
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                if matches!(o.kind, Kind::Spiegel { .. }) {
                    v.push((SheetRef { anlage: ai, sheet: si }, o.clone()));
                }
            }
        }
    }
    v
}

fn bmk_pos(p: &Project, id: u64) -> (String, P) {
    match p.find_obj(id).map(|(_, o)| o.kind.clone()) {
        Some(Kind::Symbol { bmk, pos, .. }) => (bmk, pos),
        k => panic!("kein Symbol: {k:?}"),
    }
}

/// W2-Kopie: unter einer Spule einen Kontaktspiegel mit allen Kontakten gleicher Kennzeichnung anlegen; nach dem
/// Speichern verweisen OBJ Feld 8/9 und die STB-Zeilen auf die Symbolkennungen (VSY Byte 96) von Spule und
/// Kontakten, und beim Lesen werden Spule und Kontakte wieder verknüpft.
#[test]
fn neuer_kontaktspiegel_in_w2() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    // erste Spule mit mindestens zwei Kontakten gleicher Kennzeichnung
    let (r, owner, k) = p
        .anlagen
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| a.sheets.iter().enumerate().map(move |(si, s)| (ai, si, s)))
        .flat_map(|(ai, si, s)| s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(move |o| (SheetRef { anlage: ai, sheet: si }, o.id)))
        .find_map(|(r, id)| {
            let k = kontaktspiegel::kontakte(&p, &[], id);
            (k.len() >= 2).then_some((r, id, k))
        })
        .expect("W2 hat Symbole mit gleicher Kennzeichnung");
    let rows: Vec<kontaktspiegel::Zeile> = k.iter().enumerate().map(|(i, (id, _))| kontaktspiegel::Zeile { art: (i % 3) as u8, referenz: *id, felder: if i == 0 { ["13".into(), "14".into(), String::new()] } else { Default::default() }, ..Default::default() }).collect();
    let x = bmk_pos(&p, owner).1.x;
    let y = kontaktspiegel::start_y(p.sheet(r).unwrap());
    let id = p.alloc_id();
    p.sheet_mut(r).unwrap().objects.push(Obj { id, kind: Kind::Spiegel { pos: P::new(x, y), owner, rows: rows.clone(), auto: false }, pen: 0, fixed: false, group: None, raw: None });
    let (back, f) = reload(&p);

    // Block: Art 5, X, Spule, Zeilen mit Kontakt-, Blatt- und Anlagenkennung
    let vsy: Vec<&Block> = blocks(&f, &|b| b.is("VSY") && b.data.len() >= 104);
    let neu: Vec<&Block> = blocks(&f, &|b| kontaktspiegel::ist_spiegel(b) && b.kids.iter().any(|k| k.is("STB")));
    assert_eq!(neu.len(), 1);
    let b = neu[0];
    assert_eq!((b.data[0x14], b.i32(4) & 0xffff, b.i32(6), b.data[0x2c] & 1), (5, 0xffff, (x * 10.0).round() as i32, 1));
    let spule = id_at(&b.data, 0x20);
    assert!(spule != 0 && vsy.iter().any(|v| id_at(&v.data, 96) == spule), "Spule {spule:x} ohne Symbol");
    let stb: Vec<&Block> = b.kids_of("STB").collect();
    assert_eq!(stb.len(), rows.len());
    let blatt_ids: Vec<u64> = blocks(&f, &|b| b.is("BLT")).iter().map(|b| id_at(&b.data, 72)).collect();
    let anl_ids: Vec<u64> = blocks(&f, &|b| b.is("ANL")).iter().map(|b| id_at(&b.data, 0x88)).collect();
    for (z, s) in rows.iter().zip(&stb) {
        assert_eq!((s.data.len(), s.data[0x18]), (0x1c, z.art));
        assert!(vsy.iter().any(|v| id_at(&v.data, 96) == id_at(&s.data, 0)), "Kontakt ohne Symbol");
        assert!(blatt_ids.contains(&id_at(&s.data, 8)), "Blattkennung");
        assert!(anl_ids.contains(&id_at(&s.data, 16)), "Anlagenkennung");
    }
    // erste Zeile: Anschlussnummern 1 und 2 als ALT mit STR-Unterblock ohne 0 (wie die Speicherroutine des Originals)
    assert_eq!(stb[0].kids.iter().map(|k| k.tag_str().to_string()).collect::<Vec<_>>(), ["ALT", "ALT"]);
    assert_eq!((stb[0].kids[0].i32(0), stb[0].kids[1].i32(0)), (1, 2));
    assert_eq!(stb[0].kids[0].kids.iter().map(|k| (k.tag_str().to_string(), k.data.clone())).collect::<Vec<_>>(), [("STR".to_string(), b"13".to_vec())]);

    // wieder gelesen: gleiche Spule, gleiche Kontakte (über Kennzeichnung und Lage)
    let sp = spiegel_von(&back);
    let (_, o) = sp.iter().find(|(_, o)| matches!(&o.kind, Kind::Spiegel { rows, .. } if !rows.is_empty())).expect("Spiegel gelesen");
    let Kind::Spiegel { pos, owner: o2, rows: r2, .. } = &o.kind else { unreachable!() };
    assert!((pos.x - x).abs() < 0.06 && (pos.y - y).abs() < 0.01, "{pos:?}");
    assert_eq!(bmk_pos(&back, *o2), bmk_pos(&p, owner));
    assert_eq!(r2.len(), rows.len());
    for (a, b) in rows.iter().zip(r2) {
        assert_eq!((a.art, &a.felder), (b.art, &b.felder));
        assert_eq!(bmk_pos(&back, b.referenz), bmk_pos(&p, a.referenz));
    }
    // zweites Speichern ohne Änderung: gleiche Bytes
    let mut back = back;
    back.raw = Some(Box::new(f.clone()));
    let (b1, b2) = (epr_write::to_bytes(&back).unwrap(), f.to_bytes());
    let erste = b1.iter().zip(&b2).position(|(x, y)| x != y);
    assert_eq!((b1.len(), erste), (b2.len(), None), "erste Abweichung {:?}", erste.map(|i| (&b1[i.saturating_sub(40)..(i + 40).min(b1.len())], &b2[i.saturating_sub(40)..(i + 40).min(b2.len())])));
}

/// Neues Projekt: Kontaktspiegel samt neu angelegter Spule und Kontakte (Kennungen vergibt das Speichern).
#[test]
fn neuer_kontaktspiegel_in_neuem_projekt() {
    let mut p = Project::new_default();
    let def = elektroplanung::symbols::builtin().into_iter().next().unwrap();
    let key = def.key.clone();
    p.symdefs.push(def);
    let sym = |p: &mut Project, x: f32| Obj { id: p.alloc_id(), kind: Kind::Symbol { key: key.clone(), pos: P::new(x, 60.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-K7".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
    let (a, b) = (sym(&mut p, 40.0), sym(&mut p, 150.0));
    let (ida, idb) = (a.id, b.id);
    p.anlagen[0].sheets[0].objects.extend([a, b]);
    let id = p.alloc_id();
    let rows = vec![kontaktspiegel::Zeile { art: 1, referenz: idb, ..Default::default() }];
    p.anlagen[0].sheets[0].objects.push(Obj { id, kind: Kind::Spiegel { pos: P::new(40.0, 180.0), owner: ida, rows, auto: false }, pen: 0, fixed: false, group: None, raw: None });
    let (back, _) = reload(&p);
    let sp = spiegel_von(&back);
    assert_eq!(sp.len(), 1);
    let Kind::Spiegel { owner, rows, .. } = &sp[0].1.kind else { unreachable!() };
    assert_eq!(bmk_pos(&back, *owner).1, P::new(40.0, 60.0));
    assert_eq!((rows.len(), rows[0].art), (1, 1));
    assert_eq!(bmk_pos(&back, rows[0].referenz).1, P::new(150.0, 60.0));
}

/// Tabelle mit Daten „Querverweise“ bzw. „Inhaltsverzeichnis“: Art in ZCN Feld 9 Byte 2, „Ab“ in Feld 6.
#[test]
fn tabellenart_wird_gespeichert() {
    let mut p = Project::new_default();
    for (art, ab, y) in [(2u8, 3u32, 30.0), (1, 1, 100.0)] {
        let id = p.alloc_id();
        let k = Kind::Table { a: P::new(20.0, y), b: P::new(120.0, y + 40.0), rows: 4, cols: 2, cells: vec![String::new(); 8], header: true, fixed_w: false, art, ab };
        p.anlagen[0].sheets[0].objects.push(Obj { id, kind: k, pen: 0, fixed: false, group: None, raw: None });
    }
    let (back, f) = reload(&p);
    let zcn: Vec<&Block> = blocks(&f, &|b| b.is("ZCN") && b.data.len() > 0x3c && b.data[0x3c] == 13);
    assert_eq!(zcn.iter().map(|b| (b.data[0x26], b.i32(6))).collect::<Vec<_>>(), [(2, 3), (1, 1)]);
    // leere Zellen: kein STR (das Original schreibt keinen leeren STR und meldet ihn als Fehler)
    assert!(zcn.iter().all(|b| !b.kids.iter().any(|k| k.is("STR"))));
    let got: Vec<(u8, u32)> = back.anlagen[0].sheets[0].objects.iter().filter_map(|o| if let Kind::Table { art, ab, .. } = o.kind { Some((art, ab)) } else { None }).collect();
    assert_eq!(got, [(2, 3), (1, 1)]);
}

/// Zusatz der Blattarten (BLT Feld 32 Byte 1) und Blattart neuer Blätter (Feld 22 Byte 0).
#[test]
fn zusatz_der_blattarten_neues_projekt() {
    let mut p = Project::new_default();
    for (nr, art, zusatz) in [(2u32, 2u8, 3u8), (3, 6, 1), (4, 6, 2), (5, 0, 1)] {
        let mut s = Sheet::new(nr);
        s.art = art;
        s.art_fest = true;
        s.zusatz = zusatz;
        p.anlagen[0].sheets.push(s);
    }
    let (back, f) = reload(&p);
    let blt: Vec<(u8, u8)> = blocks(&f, &|b| b.is("BLT")).iter().map(|b| (b.data[88], b.data[0x81])).collect();
    assert_eq!(&blt[1..], [(2, 3), (6, 1), (6, 2), (0, 1)]);
    let got: Vec<(u8, u8)> = back.anlagen[0].sheets[1..].iter().map(|s| (s.blattart(), s.zusatz)).collect();
    assert_eq!(got, [(2, 3), (6, 1), (6, 2), (0, 1)]);
}

/// W2-Kopie: Zusatz eines vorhandenen Blattes ändern; nur Byte 0x81 dieses Blattes ändert sich.
#[test]
fn zusatz_der_blattarten_in_w2() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let orig = EprFile::read(&path).unwrap();
    let si = p.anlagen[0].sheets.iter().position(|s| s.blattart() == 3).expect("Schaltplan");
    {
        let s = &mut p.anlagen[0].sheets[si];
        s.art = 6;
        s.zusatz = 2;
    }
    let (back, f) = reload(&p);
    assert_eq!((back.anlagen[0].sheets[si].blattart(), back.anlagen[0].sheets[si].zusatz), (6, 2));
    let alt: Vec<&Block> = blocks(&orig, &|b| b.is("BLT"));
    let neu: Vec<&Block> = blocks(&f, &|b| b.is("BLT"));
    assert_eq!(alt.len(), neu.len());
    let i = alt.iter().position(|b| id_at(&b.data, 72) == id_at(&neu[si].data, 72)).unwrap_or(si);
    let diff: Vec<usize> = (0..alt[i].data.len()).filter(|k| alt[i].data[*k] != neu[si].data[*k] && !(120..128).contains(k)).collect();
    assert_eq!(diff, [88, 0x81]);
}
