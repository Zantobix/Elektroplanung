//! Weitere Projektangaben im Originalformat (*.epr), deren Ablage aus der Speicher- und Laderoutine des Originals
//! ermittelt wurde (Speichern ab 0x4228c4, Laden 0x426310; Dialog-Übernahme im Eigenschaftendialog ab 0x44b700):
//!
//! - **Bauteile** (PRJ › BTL, 232 Byte = die ersten 0xE8 Byte des Speicherobjekts, keine Unterblöcke; angelegt in
//!   0x403740, Dialog TSBauteil 0x4871cc): 0 Kennung (u64, Zeitstempel), 8 Gruppe (u64, Kennung einer Bauteilgruppe
//!   aus Gruppen.tfi, 0 = „nur Projekt“), 0x10 Hersteller (u64, Kennung aus Gruppen.tfi), 0x18 frei, 0x20/0x24/0x28
//!   Breite/Höhe/Tiefe in mm (Vorgabe 500/800/700), 0x2C Name, 0x54 Typ, 0x7C Artikel-Nr., 0xA4 interne Nr.
//!   (je 40 Zeichen), 0xD0 Kennung des Symbols und 0xD8 Bild (Darstellung), 0xE0 Erstellungszeit (Delphi-Datum).
//! - **Bauteilzuordnung**: VSY › IDL bzw. OBJ (Klemmstelle) › IDL, je 8 Byte = Kennung des Bauteils (Liste, VSY+0xBC).
//! - **Platzhaltergruppen** (PRJ › ERS, 12 Byte Name) mit Einträgen TLI (8 Byte Suchtext) › STR (Ersetzungstext),
//!   Dialog TSTextListe (Spalten „Suchtext“, „Ersetzungstext“).
//! - **Verbindungsoptionen** (VBG Feld 14 = Wort bei 0x38: Bit 0x40 Zielführung links/oben, 0x80 rechts/unten,
//!   0x100 Brücke; Bezeichnung BZD Byte 0x0B Bit 0 Hochkant, Feld 7 Bit 0 Markierung, Bit 1 Unterstrichen,
//!   Bit 2 Durch Markierung) – Dialog TSVerbindung 0x452626 ff.
//! - **Sammelschienen-Schalter** (SMS Byte 0x30 Querverweise, 0x31 Klemmen, 0x33 Durchschleifung, 0x34 mit Pfeilen,
//!   0x35 Autoende; „Einspeisung“ zeigt das Original nur an: Kennung bei 0x38 ≠ 0) – Dialog TSSchiene 0x44e9b7 ff.
//! - **Rackposition** des Symbols (VSY Feld 37, −1 = keine) – Feld LO_RekY, 0x44d22e.
//! - **Projektstatus** (PRJ Byte 0x90: 1 = abgeschlossen) und **Alter Klemmenplan** (PRJ Byte 0x70 Bit 0 = 0) –
//!   Dialog TPrjEigen 0x457d56; **Schriftfeld relativ/absolut** (FUS Feld 1 = 2 relativ, 0 absolut) – 0x47b991.
//! - **Projektgrafiken**: Umbenennen, Ersetzen und Löschen vorhandener PRJ › BLD › JPG.
//!
//! Unveränderte Angaben lassen die Blöcke Byte für Byte unverändert.

use crate::eprfile::{cstr_bytes, put_str, Block, EprFile};
use crate::model::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Länge eines Bauteils im .epr (BTL).
pub const BTL_LEN: usize = 0xe8;
const BTL_NAME: usize = 0x2c;
const BTL_TYP: usize = 0x54;
const BTL_ARTIKEL: usize = 0x7c;
const BTL_NUMMER: usize = 0xa4;
const BTL_TEXT: usize = 40;
/// Platzhaltergruppe: Name (ERS, 12 Byte) und Suchtext (TLI, 8 Byte), je mit abschließender Null.
pub const ERS_NAME: usize = 12;
pub const TLI_TEXT: usize = 8;
const PRJ_KLEMMENPLAN: usize = 0x70;
const PRJ_STATUS: usize = 0x90;
/// Rackposition im VSY (Feld 37).
pub const VSY_RACK: usize = 37;

fn id_at(d: &[u8], at: usize) -> u64 {
    d.get(at..at + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}
fn set_id(d: &mut Vec<u8>, at: usize, v: u64) {
    if d.len() < at + 8 {
        d.resize(at + 8, 0);
    }
    d[at..at + 8].copy_from_slice(&v.to_le_bytes());
}
fn txt(d: &[u8], from: usize, len: usize) -> String {
    crate::symbols::cstr(d.get(from..(from + len).min(d.len())).unwrap_or(&[]))
}
/// Zahl wie im Original (nur Ziffern, führendes Minus): "50 mm" → 50.
fn zahl(s: &str) -> i32 {
    let t = s.trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t),
    };
    let v = t.chars().filter(|c| c.is_ascii_digit()).fold(0i64, |v, c| (v * 10 + (c as i64 - 48)).min(i32::MAX as i64)) as i32;
    if neg {
        -v
    } else {
        v
    }
}

/// Jetzt als Delphi-Datum (Tage seit 30.12.1899).
fn delphi_now() -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    (chrono::Local::now().naive_local() - base).num_milliseconds() as f64 / 86_400_000.0
}

// ---------------------------------------------------------------- Gruppen und Hersteller (Gruppen.tfi)

/// Einträge der Datenbank Gruppen.tfi: Kennung → (Art, Name). Art 1 = Bauteilgruppe, 2 = Hersteller.
fn tfi_gruppen() -> &'static HashMap<u64, (u8, String)> {
    static G: std::sync::OnceLock<HashMap<u64, (u8, String)>> = std::sync::OnceLock::new();
    G.get_or_init(|| {
        crate::symbols::find_db_dir(None).and_then(|d| std::fs::read(d.join("Gruppen.tfi")).ok()).map(|g| gruppen_lesen(&g)).unwrap_or_default()
    })
}

/// Gruppen.tfi: [u32 Länge][01 Name(41)][Art][01 HauptID][01 OwnerID][01 ID] – Name ab 1, Art bei 42, ID bei 62.
pub fn gruppen_lesen(g: &[u8]) -> HashMap<u64, (u8, String)> {
    let u32_at = |p: usize| g.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let mut out = HashMap::new();
    let Some(mut p) = u32_at(0) else { return out };
    while let Some(n) = u32_at(p) {
        if !(8..=4096).contains(&n) || p + n > g.len() {
            break;
        }
        let rec = &g[p + 4..p + n];
        if rec.len() >= 70 {
            out.insert(id_at(rec, 62), (rec[42], crate::symbols::cstr(&rec[1..42])));
        }
        p += n;
    }
    out
}

fn gruppe_name(id: u64) -> String {
    if id == 0 {
        return "Projekt".into();
    }
    tfi_gruppen().get(&id).map(|(_, n)| n.clone()).unwrap_or_else(|| format!("Gruppe {id:x}"))
}
fn firma_name(id: u64) -> String {
    if id == 0 {
        return String::new();
    }
    tfi_gruppen().get(&id).map(|(_, n)| n.clone()).unwrap_or_else(|| format!("Hersteller {id:x}"))
}
/// Kennung einer Bauteilgruppe (Art 1) bzw. eines Herstellers (Art 2) nach ihrem Namen.
fn tfi_id(art: u8, name: &str) -> Option<u64> {
    let mut v: Vec<(&u64, &(u8, String))> = tfi_gruppen().iter().filter(|(_, (a, n))| *a == art && n == name).collect();
    v.sort_by_key(|(id, _)| **id);
    v.first().map(|(id, _)| **id)
}
fn gruppe_id(name: &str) -> Option<u64> {
    let n = name.trim();
    if n.is_empty() || n == "Projekt" {
        return Some(0);
    }
    tfi_id(1, n)
}
fn firma_id(name: &str) -> Option<u64> {
    let n = name.trim();
    if n.is_empty() {
        return Some(0);
    }
    tfi_id(2, n)
}

// ---------------------------------------------------------------- Lesen

/// Bauteil aus einem BTL-Block.
pub fn bauteil_aus(b: &Block) -> Bauteil {
    let d = &b.data;
    Bauteil {
        gruppe: gruppe_name(id_at(d, 8)),
        name: txt(d, BTL_NAME, BTL_TEXT),
        typ: txt(d, BTL_TYP, BTL_TEXT),
        firma: firma_name(id_at(d, 0x10)),
        artikel: txt(d, BTL_ARTIKEL, BTL_TEXT),
        breite: b.i32(8).to_string(),
        hoehe: b.i32(9).to_string(),
        tiefe: b.i32(10).to_string(),
        nummer: txt(d, BTL_NUMMER, BTL_TEXT),
    }
}

/// Platzhaltergruppe aus einem ERS-Block.
pub fn platzhalter_aus(b: &Block) -> Platzhalter {
    Platzhalter {
        name: txt(&b.data, 0, ERS_NAME),
        werte: b.kids_of("TLI").map(|t| (txt(&t.data, 0, TLI_TEXT), t.kid("STR").map(|s| crate::symbols::cstr(&s.data)).unwrap_or_default())).collect(),
    }
}

/// Verbindungsoptionen (`WIRE_*`) aus einem VBG-Block.
pub fn wire_opts(k: &Block) -> u8 {
    let f = k.i32(14);
    let mut o = 0;
    for (bit, w) in [(0x40, WIRE_ZIEL_A), (0x80, WIRE_ZIEL_B), (0x100, WIRE_BRUECKE)] {
        if f & bit != 0 {
            o |= w;
        }
    }
    if let Some(z) = k.kid("BZD") {
        if z.data.get(0x0b).copied().unwrap_or(0) & 1 != 0 {
            o |= WIRE_HOCHKANT;
        }
        let v = z.i32(7);
        for (bit, w) in [(1, WIRE_MARKIERUNG), (2, WIRE_UNTERSTRICH), (4, WIRE_DURCH)] {
            if v & bit != 0 {
                o |= w;
            }
        }
    }
    o
}

/// Rackposition eines Symbols (VSY Feld 37, −1 = keine).
pub fn rack(k: &Block) -> i32 {
    if k.data.len() >= (VSY_RACK + 1) * 4 {
        k.i32(VSY_RACK)
    } else {
        -1
    }
}

/// Schalter einer Sammelschiene (SMS) in die Einstellungen übernehmen.
pub fn schiene_lesen(s: &Block, set: &mut SchieneSet) {
    let byte = |at: usize| s.data.get(at).copied().unwrap_or(0) != 0;
    set.quer = byte(0x30);
    set.klemmen = byte(0x31);
    set.schleife = byte(0x33);
    set.pfeile = byte(0x34);
    set.autoende = byte(0x35);
    set.einspeisung = id_at(&s.data, 0x38) != 0;
}

/// Projektstatus, „Alter Klemmenplan“, Schriftfeld absolut, Bauteile samt Zuordnung und Platzhaltergruppen.
pub fn import(prj: &Block, p: &mut Project) {
    p.abgeschlossen = prj.data.get(PRJ_STATUS).copied().unwrap_or(0) == 1;
    p.alter_klemmenplan = prj.data.get(PRJ_KLEMMENPLAN).map(|v| v & 1 == 0).unwrap_or(false);
    if let Some(f) = prj.kid("FUS").filter(|f| f.data.len() >= 8) {
        p.tb_absolut = f.i32(1) & 2 == 0;
    }
    let btl: Vec<&Block> = prj.kids_of("BTL").collect();
    if !btl.is_empty() {
        p.bauteile = btl.iter().map(|b| bauteil_aus(b)).collect();
        let ids: Vec<u64> = btl.iter().map(|b| id_at(&b.data, 0)).collect();
        p.part_of.clear();
        for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
            let Some(r) = o.raw.as_deref() else { continue };
            if let Some(i) = r.block.kid("IDL").and_then(|l| ids.iter().position(|id| *id == id_at(&l.data, 0))) {
                p.part_of.insert(o.id, i);
            }
        }
    }
    let ers: Vec<Platzhalter> = prj.kids_of("ERS").map(platzhalter_aus).collect();
    if !ers.is_empty() {
        p.platzhalter = ers;
    }
}

// ---------------------------------------------------------------- Schreiben

/// Für den laufenden Speichervorgang: Kennung je Bauteil und der zugehörige BTL-Block der Grundlage.
#[derive(Default)]
struct Plan {
    btl_ids: Vec<u64>,
    btl_src: Vec<Option<Block>>,
    ers_src: Vec<Option<Block>>,
    /// Bauteilkennungen der Grundlage (Zuordnungen auf unbekannte Bauteile bleiben unangetastet)
    alle_ids: Vec<u64>,
}

thread_local! {
    static PLAN: RefCell<Plan> = RefCell::new(Plan::default());
}

/// Zuordnung Modell ↔ Blöcke: zuerst gleiche Einträge, dann die übrigen der Reihe nach.
fn zuordnen<T: PartialEq>(soll: &[T], ist: &[T]) -> Vec<Option<usize>> {
    let mut used = vec![false; ist.len()];
    let mut out = vec![None; soll.len()];
    for (i, s) in soll.iter().enumerate() {
        if let Some(j) = (0..ist.len()).find(|&j| !used[j] && ist[j] == *s) {
            used[j] = true;
            out[i] = Some(j);
        }
    }
    let mut rest = (0..ist.len()).filter(|j| !used[*j]).collect::<Vec<_>>().into_iter();
    for o in out.iter_mut().filter(|o| o.is_none()) {
        *o = rest.next();
    }
    out
}

/// Vor dem Schreiben: Bauteile und Platzhaltergruppen den Blöcken der Grundlage zuordnen, Kennungen vergeben.
pub fn vorbereiten(project: &Project, base: &EprFile) {
    let prj = base.root("PRJ").filter(|_| project.raw.is_some());
    let btl: Vec<Block> = prj.map(|p| p.kids_of("BTL").cloned().collect()).unwrap_or_default();
    let decoded: Vec<Bauteil> = btl.iter().map(bauteil_aus).collect();
    let m = zuordnen(&project.bauteile, &decoded);
    // um eine halbe Millisekunde versetzt: epr_neu vergibt im selben Speichervorgang ganze Millisekunden
    let mut stamp = delphi_now() + 0.5 / 86_400_000.0;
    let mut plan = Plan { alle_ids: btl.iter().map(|b| id_at(&b.data, 0)).collect(), ..Default::default() };
    for j in &m {
        match j {
            Some(j) => {
                plan.btl_ids.push(id_at(&btl[*j].data, 0));
                plan.btl_src.push(Some(btl[*j].clone()));
            }
            None => {
                stamp += 1.0 / 86_400_000.0;
                plan.btl_ids.push(stamp.to_bits());
                plan.btl_src.push(None);
            }
        }
    }
    let ers: Vec<Block> = prj.map(|p| p.kids_of("ERS").cloned().collect()).unwrap_or_default();
    let names: Vec<String> = ers.iter().map(|e| txt(&e.data, 0, ERS_NAME)).collect();
    let soll: Vec<String> = project.platzhalter.iter().map(|g| g.name.clone()).collect();
    plan.ers_src = zuordnen(&soll, &names).into_iter().map(|j| j.map(|j| ers[j].clone())).collect();
    PLAN.with(|p| *p.borrow_mut() = plan);
}

/// BTL-Block eines Bauteils: Grundlage (falls vorhanden) mit den geänderten Werten.
fn btl_block(b: &Bauteil, src: Option<&Block>, id: u64) -> Block {
    let mut blk = match src {
        Some(s) => s.clone(),
        None => {
            let mut n = Block::new("BTL", vec![0; BTL_LEN]);
            set_id(&mut n.data, 0, id);
            // Vorgaben wie beim Anlegen im Original (0x403740)
            n.set_i32(8, 500);
            n.set_i32(9, 800);
            n.set_i32(10, 700);
            n.data[0xe0..0xe8].copy_from_slice(&delphi_now().to_le_bytes());
            n
        }
    };
    if blk.data.len() < BTL_LEN {
        blk.data.resize(BTL_LEN, 0);
    }
    let alt = bauteil_aus(&blk);
    for (soll, ist, at) in [(&b.name, &alt.name, BTL_NAME), (&b.typ, &alt.typ, BTL_TYP), (&b.artikel, &alt.artikel, BTL_ARTIKEL), (&b.nummer, &alt.nummer, BTL_NUMMER)] {
        if soll != ist {
            put_str(&mut blk.data, at, BTL_TEXT, soll);
        }
    }
    for (soll, ist, f) in [(&b.breite, &alt.breite, 8), (&b.hoehe, &alt.hoehe, 9), (&b.tiefe, &alt.tiefe, 10)] {
        if soll.trim() != ist && zahl(soll) != blk.i32(f) {
            blk.set_i32(f, zahl(soll));
        }
    }
    if b.gruppe.trim() != alt.gruppe && !(b.gruppe.trim().is_empty() && alt.gruppe == "Projekt") {
        if let Some(g) = gruppe_id(&b.gruppe) {
            set_id(&mut blk.data, 8, g);
        }
    }
    if b.firma.trim() != alt.firma {
        if let Some(f) = firma_id(&b.firma) {
            set_id(&mut blk.data, 0x10, f);
        }
    }
    blk
}

/// ERS-Block einer Platzhaltergruppe (Name, Einträge TLI › STR).
fn ers_block(g: &Platzhalter, src: Option<&Block>) -> Block {
    let mut blk = src.cloned().unwrap_or_else(|| Block::new("ERS", vec![0; ERS_NAME]));
    if blk.data.len() < ERS_NAME {
        blk.data.resize(ERS_NAME, 0);
    }
    let alt = platzhalter_aus(&blk);
    if alt.name != g.name {
        put_str(&mut blk.data, 0, ERS_NAME, &g.name);
    }
    if alt.werte != g.werte {
        blk.kids.retain(|k| !k.is("TLI"));
        for (k, v) in &g.werte {
            let mut t = Block::new("TLI", vec![0; TLI_TEXT]);
            put_str(&mut t.data, 0, TLI_TEXT, k);
            if !v.is_empty() {
                t.kids.push(Block::new("STR", cstr_bytes(v)));
            }
            blk.kids.push(t);
        }
    }
    blk
}

/// Gruppe von Blöcken einer Kennung im Projektblock ersetzen (nur wenn sich etwas geändert hat); neue Blöcke vor
/// dem ersten Block aus `vor`.
fn ersetzen(prj: &mut Block, tag: &str, list: Vec<Block>, vor: &[&str]) {
    let alt: Vec<&Block> = prj.kids_of(tag).collect();
    if alt.len() == list.len() && alt.iter().zip(&list).all(|(a, b)| *a == b) {
        return;
    }
    let at = prj.kids.iter().position(|k| k.is(tag)).or_else(|| prj.kids.iter().position(|k| vor.iter().any(|t| k.is(t)))).unwrap_or(prj.kids.len());
    let before = prj.kids[..at].iter().filter(|k| k.is(tag)).count();
    prj.kids.retain(|k| !k.is(tag));
    let at = at - before;
    prj.kids.splice(at..at, list);
}

/// Nach dem Aufbau des Projektblocks: Bauteile, Platzhalter, Projektstatus, Schriftfeld und Projektgrafiken.
pub fn nachbereiten(project: &Project, prj: &mut Block) {
    let plan = PLAN.with(|p| std::mem::take(&mut *p.borrow_mut()));
    let btl: Vec<Block> = project.bauteile.iter().enumerate().map(|(i, b)| btl_block(b, plan.btl_src.get(i).and_then(|s| s.as_ref()), plan.btl_ids.get(i).copied().unwrap_or(0))).collect();
    ersetzen(prj, "BTL", btl, &["ERS", "FUS", "ANL", "BLD"]);
    let ers: Vec<Block> = project.platzhalter.iter().enumerate().map(|(i, g)| ers_block(g, plan.ers_src.get(i).and_then(|s| s.as_ref()))).collect();
    ersetzen(prj, "ERS", ers, &["FUS", "ANL", "BLD"]);
    // Projektstatus und Klemmenplan
    if prj.data.len() > PRJ_STATUS {
        let ist = prj.data[PRJ_STATUS] == 1;
        if ist != project.abgeschlossen {
            prj.data[PRJ_STATUS] = project.abgeschlossen as u8;
        }
        let alt_ist = prj.data[PRJ_KLEMMENPLAN] & 1 == 0;
        if alt_ist != project.alter_klemmenplan {
            prj.data[PRJ_KLEMMENPLAN] ^= 1;
        }
    }
    // Schriftfeld: Maßangaben relativ (Feld 1 = 2) oder absolut (0) – das Original schreibt das ganze Feld
    if let Some(f) = prj.kids.iter_mut().find(|k| k.is("FUS") && k.data.len() >= 8) {
        if (f.i32(1) & 2 == 0) != project.tb_absolut {
            f.set_i32(1, if project.tb_absolut { 0 } else { 2 });
        }
    }
    projektgrafiken(project, prj);
}

/// Vorhandene Projektbilder: entfernte löschen, Namen und Bilddaten nachführen.
fn projektgrafiken(project: &Project, prj: &mut Block) {
    // gelöscht = in der Grundlage vorhanden, aber nicht mehr im Projekt (neue Bilder aus Bilddateien legt
    // epr_neu an, sie stehen nicht in `bilder`)
    let vorher: Vec<i32> = project.raw.as_deref().and_then(|r| r.root("PRJ")).map(|p| p.kids_of("BLD").map(|b| b.i32(5)).collect()).unwrap_or_default();
    prj.kids.retain(|k| !k.is("BLD") || !vorher.contains(&k.i32(5)) || project.bilder.iter().any(|b| b.nr == k.i32(5)));
    for k in prj.kids.iter_mut().filter(|k| k.is("BLD")) {
        let Some(b) = project.bilder.iter().find(|b| b.nr == k.i32(5)) else { continue };
        if txt(&k.data, 0, 20) != b.name {
            put_str(&mut k.data, 0, 20, &b.name);
        }
        let jpeg: Vec<u8> = k.kids_of("JPG").flat_map(|j| j.data.iter().chain(j.parts.iter().flat_map(|(_, d)| d.iter())).copied()).collect();
        if jpeg != b.jpeg && !b.jpeg.is_empty() {
            let neu = crate::epr_neu::bld_block(&b.name, b.nr, &b.jpeg);
            k.kids.retain(|j| !j.is("JPG"));
            k.kids.extend(neu.kids);
        }
    }
}

/// Für jedes geschriebene Blattobjekt: Bauteilzuordnung (IDL), Verbindungsoptionen und Rackposition.
pub fn objekt(project: &Project, o: &Obj, b: &mut Block) {
    match &o.kind {
        Kind::Symbol { ext, .. } if b.is("VSY") => {
            if b.data.len() >= (VSY_RACK + 1) * 4 && b.i32(VSY_RACK) != ext.rack {
                b.set_i32(VSY_RACK, ext.rack);
            }
            zuordnung(project, o, b);
        }
        Kind::Terminal { .. } if b.is("OBJ") => zuordnung(project, o, b),
        Kind::Wire { opts, .. } if b.is("VBG") => verbindung(*opts, b),
        _ => {}
    }
}

fn zuordnung(project: &Project, o: &Obj, b: &mut Block) {
    let (soll, bekannt) = PLAN.with(|p| {
        let p = p.borrow();
        let soll = project.part_of.get(&o.id).and_then(|&i| p.btl_ids.get(i).copied());
        let ist = b.kid("IDL").map(|l| id_at(&l.data, 0));
        (soll, ist.map(|id| p.alle_ids.contains(&id) || p.btl_ids.contains(&id)).unwrap_or(true))
    });
    let ist = b.kid("IDL").map(|l| id_at(&l.data, 0));
    // Zuordnung auf ein Bauteil, das der Nachbau nicht kennt: unverändert lassen
    if soll == ist || (soll.is_none() && !bekannt) {
        return;
    }
    b.kids.retain(|k| !k.is("IDL"));
    if let Some(id) = soll {
        let idl = Block::new("IDL", id.to_le_bytes().to_vec());
        // Reihenfolge wie im Original: VSY … FKT, IDL; OBJ … FKT, IDL, TIT
        let at = if b.is("OBJ") { b.kids.iter().position(|k| k.is("TIT")).unwrap_or(b.kids.len()) } else { b.kids.len() };
        b.kids.insert(at, idl);
    }
}

fn verbindung(opts: u8, b: &mut Block) {
    let ist = wire_opts(b);
    if ist == opts {
        return;
    }
    let f = b.i32(14);
    let mut v = f & !0x1c0;
    for (bit, w) in [(0x40, WIRE_ZIEL_A), (0x80, WIRE_ZIEL_B), (0x100, WIRE_BRUECKE)] {
        if opts & w != 0 {
            v |= bit;
        }
    }
    if v != f {
        b.set_i32(14, v);
    }
    let lab = WIRE_HOCHKANT | WIRE_MARKIERUNG | WIRE_UNTERSTRICH | WIRE_DURCH;
    if (ist ^ opts) & lab == 0 {
        return;
    }
    if b.kid("BZD").is_none() {
        if opts & lab == 0 {
            return;
        }
        b.kids.push(Block::new("BZD", vec![0; 0x2c]));
    }
    let z = b.kids.iter_mut().find(|k| k.is("BZD")).unwrap();
    if z.data.len() < 0x2c {
        z.data.resize(0x2c, 0);
    }
    z.data[0x0b] = (z.data[0x0b] & !1) | (opts & WIRE_HOCHKANT != 0) as u8;
    let mut w = z.i32(7) & !7;
    for (bit, o) in [(1, WIRE_MARKIERUNG), (2, WIRE_UNTERSTRICH), (4, WIRE_DURCH)] {
        if opts & o != 0 {
            w |= bit;
        }
    }
    z.set_i32(7, w);
}

/// Schalter einer Sammelschiene in den SMS-Block schreiben (nur geänderte).
pub fn schiene_schreiben(set: &SchieneSet, s: &mut Block) {
    if s.data.len() < 0x40 {
        s.data.resize(0x4c.max(s.data.len()), 0);
    }
    let mut ist = SchieneSet::default();
    schiene_lesen(s, &mut ist);
    for (soll, alt, at) in [(set.quer, ist.quer, 0x30), (set.klemmen, ist.klemmen, 0x31), (set.schleife, ist.schleife, 0x33), (set.pfeile, ist.pfeile, 0x34), (set.autoende, ist.autoende, 0x35)] {
        if soll != alt {
            s.data[at] = soll as u8;
        }
    }
    // Einspeisung: nur abschalten möglich (die Kennung der Einspeisung vergibt das Original)
    if ist.einspeisung && !set.einspeisung {
        set_id(&mut s.data, 0x38, 0);
    }
}

/// Name einer Schiene (SMS): Kürzel im TIT, sonst Name ab Byte 0.
fn sms_name(s: &Block) -> String {
    s.kid("TIT").map(|t| crate::symbols::cstr(&t.data)).unwrap_or_else(|| txt(&s.data, 0, 20))
}

/// Angaben, die sich auch mit dieser Ablage nicht ins .epr schreiben lassen.
pub fn nicht_gespeichert(project: &Project) -> Vec<String> {
    let mut v = vec![];
    for b in &project.bauteile {
        let alt = (b.gruppe.trim(), b.firma.trim());
        if gruppe_id(alt.0).is_none() && !alt.0.starts_with("Gruppe ") {
            v.push(format!("Bauteil \u{201e}{}\u{201c}: Gruppe \u{201e}{}\u{201c} ist keine Bauteilgruppe der Bibliothek", b.name, alt.0));
        }
        if firma_id(alt.1).is_none() && !alt.1.starts_with("Hersteller ") {
            v.push(format!("Bauteil \u{201e}{}\u{201c}: Hersteller \u{201e}{}\u{201c} steht nicht in der Bibliothek", b.name, alt.1));
        }
        for t in [&b.name, &b.typ, &b.artikel, &b.nummer] {
            if crate::eprfile::encode_cp1252(t).len() >= BTL_TEXT {
                v.push(format!("Bauteil \u{201e}{}\u{201c}: \u{201e}{t}\u{201c} wird auf 39 Zeichen gekürzt", b.name));
            }
        }
    }
    for g in &project.platzhalter {
        if crate::eprfile::encode_cp1252(&g.name).len() >= ERS_NAME {
            v.push(format!("Platzhaltergruppe \u{201e}{}\u{201c}: Name wird auf 11 Zeichen gekürzt", g.name));
        }
        for (k, _) in &g.werte {
            if crate::eprfile::encode_cp1252(k).len() >= TLI_TEXT {
                v.push(format!("Platzhaltergruppe \u{201e}{}\u{201c}: Suchtext \u{201e}{k}\u{201c} wird auf 7 Zeichen gekürzt", g.name));
            }
        }
    }
    for b in &project.bilder {
        if crate::eprfile::encode_cp1252(&b.name).len() >= 20 {
            v.push(format!("Projektgrafik \u{201e}{}\u{201c}: Name wird auf 19 Zeichen gekürzt", b.name));
        }
    }
    // Einspeisung einer Schiene lässt sich nur im Original setzen (Kennung des einspeisenden Objekts)
    let base = project.raw.as_deref().and_then(|r| r.root("PRJ"));
    for (ai, a) in project.anlagen.iter().enumerate() {
        let anl = base.and_then(|p| p.kids_of("ANL").nth(ai));
        for (name, s) in &a.schienen {
            let ist = anl.and_then(|b| b.kids_of("SMS").find(|k| sms_name(k) == *name)).map(|k| id_at(&k.data, 0x38) != 0).unwrap_or(false);
            if s.einspeisung && !ist {
                v.push(format!("{}: Einspeisung der Sammelschiene {name} (setzt das Original selbst)", a.name));
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zahlen_wie_im_original() {
        assert_eq!(zahl("500"), 500);
        assert_eq!(zahl(" 45 mm"), 45);
        assert_eq!(zahl("-3"), -3);
        assert_eq!(zahl(""), 0);
    }

    #[test]
    fn bauteil_rundreise() {
        let b = Bauteil { gruppe: "Projekt".into(), name: "Schütz".into(), typ: "DILM12".into(), firma: String::new(), artikel: "123".into(), breite: "45".into(), hoehe: "80".into(), tiefe: "70".into(), nummer: "K1".into() };
        let blk = btl_block(&b, None, 42);
        assert_eq!(blk.data.len(), BTL_LEN);
        assert_eq!(id_at(&blk.data, 0), 42);
        assert_eq!(bauteil_aus(&blk), b);
        // unverändert: Block bleibt gleich
        assert_eq!(btl_block(&b, Some(&blk), 42), blk);
    }

    #[test]
    fn platzhalter_rundreise() {
        let g = Platzhalter { name: "SPS".into(), werte: vec![("E1".into(), "Eingang 1".into()), ("A1".into(), String::new())] };
        let blk = ers_block(&g, None);
        assert_eq!(platzhalter_aus(&blk), g);
        assert_eq!(blk.kids.len(), 2);
        assert_eq!(ers_block(&g, Some(&blk)), blk);
    }

    #[test]
    fn verbindungsoptionen_rundreise() {
        for opts in [0, WIRE_ZIEL_A | WIRE_BRUECKE, WIRE_ZIEL_B | WIRE_HOCHKANT | WIRE_MARKIERUNG | WIRE_DURCH, WIRE_UNTERSTRICH] {
            let mut b = Block::new("VBG", vec![0; 0x3c]);
            b.set_i32(12, 1);
            verbindung(opts, &mut b);
            assert_eq!(wire_opts(&b), opts);
            assert_eq!(b.i32(12), 1);
        }
    }

    #[test]
    fn schienen_schalter_rundreise() {
        let mut s = Block::new("SMS", vec![0; 76]);
        let set = SchieneSet { quer: false, pfeile: true, klemmen: true, schleife: true, autoende: true, ..Default::default() };
        schiene_schreiben(&set, &mut s);
        let mut back = SchieneSet::default();
        schiene_lesen(&s, &mut back);
        assert_eq!((back.quer, back.pfeile, back.klemmen, back.schleife, back.autoende, back.einspeisung), (false, true, true, true, true, false));
        assert_eq!(&s.data[0x30..0x36], &[0, 1, 0, 1, 1, 1]);
    }

    #[test]
    fn gruppen_tfi() {
        // Datensatz: Länge 74, 01 Name(41) Art ... ID bei 62
        let mut rec = vec![0u8; 70];
        rec[0] = 1;
        rec[1..8].copy_from_slice(b"Siemens");
        rec[42] = 2;
        rec[62..70].copy_from_slice(&0x2au64.to_le_bytes());
        let mut g = 4u32.to_le_bytes().to_vec();
        g.extend_from_slice(&74u32.to_le_bytes());
        g.extend_from_slice(&rec);
        let m = gruppen_lesen(&g);
        assert_eq!(m[&0x2a], (2, "Siemens".to_string()));
    }
}
