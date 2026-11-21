//! Klemmstellen und Klemmleisten im Originalformat (*.epr): Einstellungen des Klemmen-Dialogs (TSKlemme),
//! Bezeichnung mit Gruppenautomatik, Schreiben geänderter und neuer Klemmstellen (OBJ) und Klemmleisten (KLM › KFO).
//!
//! Aufbau einer Klemmstelle (OBJ, 64 Byte; ermittelt aus W2.epr und dem Klemmen-Dialog des Originals, der genau
//! diese Bytes des Objekts vergleicht und setzt):
//! - Byte 0–7 Kennung (Delphi-Zeitstempel), Feld 2/3 Titelposition X/Y (0,1 mm, relativ zur Klemme)
//! - Feld 4 = 0xFFFF, Byte 0x12 Ausrichtung des Titels (0 Links, 1 Mittig, 2 Rechts – Vorgabe 2, daher 0x2FFFF)
//! - Byte 0x14 Objektart (0 = Klemmstelle), Feld 6/7 Lage, 8 Klemmleiste (KLM Feld 13), 9 Nummer, 10 Position
//! - Byte 0x2C Ebene (1 = A …), Byte 0x2D Schalter: Bit 0 Automatisch, Bit 1 Korrektur (Nummer),
//!   Bit 2 Korrektur (Position), Bit 3 Ziel Wechsel; Byte 0x2E Gruppenautomatik
//! - Feld 12/13 im Speicher Zeiger (Feld 13 = Funktion), in der Datei 0; Unterblöcke FKT (Funktion) und TIT (Titel).
//!   Das Original liest FKT unter OBJ als Funktion der Klemme (Lesefunktion, Fall OBJ › 0x28).
//!
//! Klemmleiste (KLM, 80 Byte): Byte 0–43 Name (K_Name), Feld 10/11 Titelposition (0,1 mm), 13 Kennung,
//! 14/16/17 = −1, 15 Ebenen, 18/19 nächste automatische Nummer/Position; TIT = Titel (%O der Klemmstellen);
//! KFO je Klemme (24 Byte): Kennung der Klemmstelle, Kennung ihres Blattes (BLT Byte 72), Nummer, Position.
//! Der Klemmplan des Originals geht diese KFO-Liste durch – neue Klemmstellen brauchen daher einen KFO-Eintrag.

use crate::eprfile::{put_str, Block};
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn yes() -> bool {
    true
}
fn two() -> u8 {
    2
}

/// Einstellungen des Klemmen-Dialogs (TSKlemme), die das Original je Klemmstelle speichert.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KlemmOpt {
    /// Klemmenindex automatisch vergeben
    #[serde(default = "yes")]
    pub auto: bool,
    /// Korrektur der Nummer (mit „Automatisch“: ab hier fortlaufend)
    #[serde(default)]
    pub korr: bool,
    /// Korrektur der Position in der Klemmleiste
    #[serde(default)]
    pub posk: bool,
    /// Flussrichtung im Klemmplan umdrehen (Von/Nach)
    #[serde(default)]
    pub ziel_wechsel: bool,
    /// Titel ohne Klemmleiste, wenn links innerhalb 2 cm eine Klemme derselben Leiste steht
    #[serde(default = "yes")]
    pub gr_auto: bool,
    /// Ausrichtung des Titels: 0 Links, 1 Mittig, 2 Rechts
    #[serde(default = "two")]
    pub align: u8,
    /// Position in der Klemmleiste (Reihenfolge im Klemmplan); 0 = wie die Nummer
    #[serde(default)]
    pub position: u32,
    /// Funktion (Spalte „Funktion“ im Klemmplan)
    #[serde(default)]
    pub funktion: String,
}

impl Default for KlemmOpt {
    fn default() -> Self {
        KlemmOpt { auto: true, korr: false, posk: false, ziel_wechsel: false, gr_auto: true, align: 2, position: 0, funktion: String::new() }
    }
}

impl KlemmOpt {
    /// Schalterbyte (OBJ Byte 0x2D) ohne unbekannte Bits.
    pub fn flags(&self) -> u8 {
        self.auto as u8 | (self.korr as u8) << 1 | (self.posk as u8) << 2 | (self.ziel_wechsel as u8) << 3
    }
    /// Position (0 = wie Nummer).
    pub fn pos(&self, nr: u32) -> u32 {
        if self.position == 0 {
            nr
        } else {
            self.position
        }
    }
}

fn txt(b: &Block) -> String {
    crate::symbols::cstr(&b.data)
}

/// Ist der OBJ-Block eine Klemmstelle? Objektart Byte 0x14: 0 Klemmstelle, 1 Sammelschiene, 2 Verweis (W2.epr:
/// 115 × 0, 97 × 1, 20 × 2); Feld 4 Wort 0 = 0xFFFF, Byte 0x12 = Ausrichtung des Titels.
pub fn is_terminal(k: &Block) -> bool {
    k.is("OBJ") && k.data.len() >= 0x30 && k.i32(4) & 0xffff == 0xffff && k.data[0x14] == 0
}

/// Klemmen-Einstellungen aus einem OBJ-Block.
pub fn read_opt(k: &Block) -> KlemmOpt {
    let f = k.data.get(0x2d).copied().unwrap_or(1);
    let nr = k.i32(9).max(0) as u32;
    let pos = k.i32(10).max(0) as u32;
    KlemmOpt {
        auto: f & 1 != 0,
        korr: f & 2 != 0,
        posk: f & 4 != 0,
        ziel_wechsel: f & 8 != 0,
        gr_auto: k.data.get(0x2e).copied().unwrap_or(1) != 0,
        align: k.data.get(0x12).copied().unwrap_or(2).min(2),
        position: if pos == nr { 0 } else { pos },
        funktion: k.kid("FKT").map(txt).unwrap_or_default(),
    }
}

/// Klemmleiste aus einem KLM-Block.
pub fn klemm_info(k: &Block) -> KlemmInfo {
    let intern = crate::symbols::cstr(k.data.get(0..44).unwrap_or(&[]));
    KlemmInfo {
        id: k.i32(13),
        name: k.kid("TIT").map(txt).unwrap_or_else(|| intern.clone()),
        nrs: k.kids_of("KFO").map(|f| f.i32(4).max(0) as u32).collect(),
        intern,
        etagen: k.i32(15).max(1),
        titel_off: (k.i32(10), k.i32(11)),
    }
}

/// Bezeichnung einer Klemmstelle wie im Original (Titelmakros %O Klemmleiste, %C Nummer mit Etage A…D).
/// `gruppe`: Gruppenautomatik greift – der Titel beginnt dann erst bei %C (Ausdruck: "-X2.2A", daneben "2B 3A").
pub fn label(tpl: &str, leiste: &str, nr: u32, level: u8, gruppe: bool) -> String {
    let tpl = if tpl.is_empty() { "%O%B.%C" } else { tpl };
    let tpl = match (gruppe, tpl.find("%C")) {
        (true, Some(i)) => &tpl[i..],
        _ => tpl,
    };
    crate::epr::terminal_label(tpl, leiste, nr, level)
}

/// Greift die Gruppenautomatik für die Klemmstelle `o` auf dem Blatt? (wie das Original: nächste Klemmstelle links
/// auf gleicher Höhe, weniger als 2 cm entfernt, gehört zur selben Klemmleiste)
pub fn grouped(sheet: &Sheet, o: &Obj) -> bool {
    let Kind::Terminal { pos, leiste, opt, .. } = &o.kind else { return false };
    if !opt.gr_auto {
        return false;
    }
    let y = (pos.y * 10.0).round() as i32;
    let x = (pos.x * 10.0).round() as i32;
    let mut best: Option<(i32, bool)> = None;
    for q in &sheet.objects {
        if q.id == o.id {
            continue;
        }
        if let Kind::Terminal { pos: p, leiste: l, .. } = &q.kind {
            let (qx, qy) = ((p.x * 10.0).round() as i32, (p.y * 10.0).round() as i32);
            if qy == y && qx < x && best.map(|b| x - qx < b.0).unwrap_or(true) {
                best = Some((x - qx, l == leiste));
            }
        }
    }
    matches!(best, Some((d, true)) if d < 200)
}

/// Bezeichnung einer Klemmstelle auf ihrem Blatt (mit Gruppenautomatik).
pub fn label_on_sheet(sheet: Option<&Sheet>, o: &Obj) -> String {
    let Kind::Terminal { leiste, nr, level, tpl, .. } = &o.kind else { return String::new() };
    label(tpl, leiste, *nr, *level, sheet.map(|s| grouped(s, o)).unwrap_or(false))
}

/// Jede Klemmleiste (Namen in `klemmleisten` und auf den Blättern) bekommt eine Kennung und einen Eintrag in
/// `klemmen`, damit Klemmstellen, Klemmplan-Blätter und KLM-Blöcke dieselbe Kennung benutzen. Neue Kennungen
/// zählen projektweit hoch (wie im Original: 0x1C, 0x21, 0x24 …).
pub fn ensure_ids(p: &mut Project) {
    let mut next = p.anlagen.iter().flat_map(|a| &a.klemmen).map(|k| k.id).max().unwrap_or(0).max(0) + 1;
    for a in &mut p.anlagen {
        let mut names: Vec<String> = a.klemmleisten.clone();
        for s in &a.sheets {
            for o in &s.objects {
                if let Kind::Terminal { leiste, .. } = &o.kind {
                    if !names.contains(leiste) {
                        names.push(leiste.clone());
                    }
                }
            }
        }
        for n in names {
            if !a.klemmen.iter().any(|k| k.name == n) {
                a.klemmen.push(KlemmInfo { id: next, name: n.clone(), nrs: vec![], intern: n, etagen: 1, titel_off: (0, -50) });
                next += 1;
            }
        }
        for k in &mut a.klemmen {
            if k.id <= 0 {
                k.id = next;
                next += 1;
            }
        }
    }
}

/// Neue Klemmleiste „Klemme n“ mit Kennung anlegen; Rückgabe: Index in `klemmleisten`.
pub fn new_leiste(p: &mut Project, anlage: usize) -> Option<usize> {
    ensure_ids(p);
    let next = p.anlagen.iter().flat_map(|a| &a.klemmen).map(|k| k.id).max().unwrap_or(0) + 1;
    let a = p.anlagen.get_mut(anlage)?;
    let mut n = a.klemmleisten.len() + 1;
    while a.klemmleisten.contains(&format!("Klemme {n}")) {
        n += 1;
    }
    let name = format!("Klemme {n}");
    a.klemmleisten.push(name.clone());
    a.klemmen.push(KlemmInfo { id: next, name: name.clone(), nrs: vec![], intern: name, etagen: 1, titel_off: (0, -50) });
    Some(a.klemmleisten.len() - 1)
}

thread_local! {
    /// Kennungen neuer Klemmstellen (Objekt-Id → OBJ-Kennung), damit OBJ und KFO dieselbe benutzen
    static UIDS: std::cell::RefCell<HashMap<u64, u64>> = Default::default();
}

/// Jetzt als Delphi-Datum (Tage seit 30.12.1899).
fn delphi_now() -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    (chrono::Local::now().naive_local() - base).num_milliseconds() as f64 / 86_400_000.0
}

/// OBJ-Kennung einer Klemmstelle: aus dem Originalblock, sonst einmalig neu vergeben (Zeitstempel).
pub fn uid(o: &Obj) -> u64 {
    if let Some(r) = o.raw.as_deref() {
        if let Some(b) = r.block.data.get(0..8) {
            return u64::from_le_bytes(b.try_into().unwrap());
        }
    }
    UIDS.with(|m| {
        let mut m = m.borrow_mut();
        let n = m.len() as f64;
        *m.entry(o.id).or_insert_with(|| (delphi_now() + (n + 1.0) / 86_400_000.0).to_bits())
    })
}

/// Kennung eines Blattes (BLT Byte 72) – aus dem Original, sonst wie `epr_write` sie für neue Blätter vergibt.
fn sheet_uid(s: &Sheet) -> u64 {
    match s.raw.as_deref().and_then(|r| r.block.data.get(72..80)) {
        Some(b) => u64::from_le_bytes(b.try_into().unwrap()),
        None => crate::epr_write::new_sheet_id(s).to_bits(),
    }
}

fn leiste_id(a: &Anlage, name: &str) -> i32 {
    a.klemmen.iter().find(|k| k.name == name).map(|k| k.id).unwrap_or(0)
}

fn etagen(a: &Anlage, name: &str) -> i32 {
    a.klemmen.iter().find(|k| k.name == name).map(|k| k.etagen).unwrap_or(1).max(1)
}

fn set_text(b: &mut Block, tag: &str, text: &str, before: Option<&str>) {
    let data = crate::eprfile::encode_cp1252(text);
    if let Some(k) = b.kids.iter_mut().find(|k| k.is(tag)) {
        k.data = data;
        return;
    }
    let at = before.and_then(|t| b.kids.iter().position(|k| k.is(t))).unwrap_or(b.kids.len());
    b.kids.insert(at, Block::new(tag, data));
}

/// Felder einer Klemmstelle in einen OBJ-Block schreiben (unbekannte Bits bleiben).
fn write_fields(b: &mut Block, a: &Anlage, k: &Kind) {
    let Kind::Terminal { pos, leiste, nr, level, text_off, tpl, opt } = k else { return };
    if b.data.len() < 64 {
        b.data.resize(64, 0);
    }
    let to01 = |v: f32| (v * 10.0).round() as i32;
    let off = text_off.unwrap_or(P::new(-1.0, -2.0));
    b.set_i32(2, to01(off.x));
    b.set_i32(3, to01(off.y));
    b.set_i32(4, 0xffff | (opt.align.min(2) as i32) << 16);
    b.data[0x14] = 0;
    b.set_i32(6, to01(pos.x));
    b.set_i32(7, to01(pos.y));
    b.set_i32(8, leiste_id(a, leiste));
    b.set_i32(9, *nr as i32);
    b.set_i32(10, opt.pos(*nr) as i32);
    b.data[0x2c] = if etagen(a, leiste) > 1 { (*level).max(1) } else { 1 };
    b.data[0x2d] = (b.data[0x2d] & 0xf0) | opt.flags();
    b.data[0x2e] = opt.gr_auto as u8;
    let tpl = if tpl.is_empty() { "%O%B.%C" } else { tpl.as_str() };
    if b.kid("TIT").map(txt).as_deref() != Some(tpl) {
        set_text(b, "TIT", tpl, None);
    }
    let fkt = b.kid("FKT").map(txt).unwrap_or_default();
    if fkt != opt.funktion {
        if opt.funktion.is_empty() {
            b.kids.retain(|k| !k.is("FKT"));
        } else {
            set_text(b, "FKT", &opt.funktion, Some("TIT"));
        }
    }
}

/// OBJ-Block einer Klemmstelle: Original unverändert, nachgeführt oder neu (siehe Modulbeschreibung).
pub fn terminal_block(project: &Project, o: &Obj) -> Option<Block> {
    let (r, _) = project.find_obj(o.id)?;
    let a = project.anlagen.get(r.anlage)?;
    if let Some(raw) = o.raw.as_deref() {
        if raw.kind == o.kind && raw.pen == o.pen && raw.fixed == o.fixed && raw.block.i32(8) == leiste_id(a, terminal_leiste(&o.kind)) {
            return Some(raw.block.clone());
        }
        let mut b = raw.block.clone();
        write_fields(&mut b, a, &o.kind);
        return Some(b);
    }
    let mut b = Block::new("OBJ", vec![0; 64]);
    b.data[0..8].copy_from_slice(&uid(o).to_le_bytes());
    b.data[0x2d] = 0;
    write_fields(&mut b, a, &o.kind);
    Some(b)
}

fn terminal_leiste(k: &Kind) -> &str {
    match k {
        Kind::Terminal { leiste, .. } => leiste,
        _ => "",
    }
}

/// KFO-Eintrag (24 Byte).
fn kfo(term: u64, sheet: u64, nr: u32, pos: u32) -> Block {
    let mut d = vec![0u8; 24];
    d[0..8].copy_from_slice(&term.to_le_bytes());
    d[8..16].copy_from_slice(&sheet.to_le_bytes());
    d[16..20].copy_from_slice(&(nr as i32).to_le_bytes());
    d[20..24].copy_from_slice(&(pos as i32).to_le_bytes());
    Block::new("KFO", d)
}

/// KLM-Blöcke einer Anlage nachführen: unveränderte Klemmleisten bleiben byteidentisch, geänderte werden im
/// Originalblock nachgeführt, neue aus den Daten erzeugt. KFO: vorhandene Einträge in ihrer Reihenfolge
/// (Nummer/Position nachgeführt, gelöschte Klemmstellen entfernt), neue Klemmstellen hinten angehängt.
pub fn sync_klm(a: &Anlage, blk: &mut Block) {
    let raw_anl = a.raw.as_deref().map(|(b, _)| b);
    let raw_klm: Vec<&Block> = raw_anl.map(|b| b.kids_of("KLM").collect()).unwrap_or_default();
    // Klemmstellen im Original (alle Leisten) – deren KFO-Einträge entfallen, wenn die Klemmstelle fehlt
    let orig_terms: std::collections::HashSet<u64> = raw_anl
        .map(|b| b.kids_of("BLT").flat_map(|s| s.kids_of("OBJ")).filter(|k| is_terminal(k)).filter_map(|k| k.data.get(0..8).map(|d| u64::from_le_bytes(d.try_into().unwrap()))).collect())
        .unwrap_or_default();
    let mut out = vec![];
    for info in &a.klemmen {
        // gewünschte Einträge: Klemmstellen der Leiste in Blatt-/Objektreihenfolge
        let mut want: Vec<(u64, u64, u32, u32)> = vec![];
        for s in &a.sheets {
            let su = sheet_uid(s);
            for o in &s.objects {
                if let Kind::Terminal { leiste, nr, opt, .. } = &o.kind {
                    if *leiste == info.name {
                        want.push((uid(o), su, *nr, opt.pos(*nr)));
                    }
                }
            }
        }
        let raw = raw_klm.iter().find(|k| k.i32(13) == info.id).copied().filter(|_| a.raw.is_some());
        let mut b = match raw {
            Some(r) => r.clone(),
            None => {
                let mut b = Block::new("KLM", vec![0; 80]);
                for f in [14, 16, 17] {
                    b.set_i32(f, -1);
                }
                b.set_i32(13, info.id);
                b
            }
        };
        let mut kids: Vec<Block> = vec![];
        let mut used = std::collections::HashSet::new();
        for k in b.kids.iter().filter(|k| k.is("KFO")) {
            let t = k.data.get(0..8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0);
            match want.iter().find(|w| w.0 == t) {
                Some(w) => {
                    used.insert(t);
                    let mut k = k.clone();
                    if k.i32(4) != w.2 as i32 || k.i32(5) != w.3 as i32 {
                        k.set_i32(4, w.2 as i32);
                        k.set_i32(5, w.3 as i32);
                    }
                    kids.push(k);
                }
                // Klemme ohne Klemmstelle (aus der Indexübersicht) bleibt, gelöschte Klemmstelle entfällt
                None if !orig_terms.contains(&t) => kids.push(k.clone()),
                None => {}
            }
        }
        for w in want.iter().filter(|w| !used.contains(&w.0)) {
            kids.push(kfo(w.0, w.1, w.2, w.3));
        }
        let changed_kfo = raw.map(|r| r.kids_of("KFO").cloned().collect::<Vec<_>>() != kids).unwrap_or(true);
        let old = raw.map(klemm_info);
        let same_head = old.as_ref().map(|o| o.name == info.name && o.intern == info.intern && o.etagen == info.etagen.max(1) && o.titel_off == info.titel_off).unwrap_or(false);
        if !same_head {
            put_str(&mut b.data, 0, 44, if info.intern.is_empty() { &info.name } else { &info.intern });
            b.set_i32(10, info.titel_off.0);
            b.set_i32(11, info.titel_off.1);
            b.set_i32(15, info.etagen.max(1));
            set_text(&mut b, "TIT", &info.name, Some("KFO"));
        }
        if changed_kfo {
            let max = kids.iter().map(|k| k.i32(4)).chain(kids.iter().map(|k| k.i32(5))).max().unwrap_or(0);
            b.set_i32(18, b.i32(18).max(max + 1));
            b.set_i32(19, b.i32(19).max(max + 1));
        }
        let tit: Vec<Block> = b.kids.iter().filter(|k| !k.is("KFO")).cloned().collect();
        b.kids = tit;
        b.kids.extend(kids);
        if raw.is_none() && !b.kids.iter().any(|k| k.is("TIT")) {
            b.kids.insert(0, Block::new("TIT", crate::eprfile::encode_cp1252(&info.name)));
        }
        out.push(b);
    }
    // an der Stelle der ersten Klemmleiste (sonst vor dem ersten Blatt) einsetzen
    let at = if a.raw.is_some() { blk.kids.iter().position(|k| k.is("KLM")) } else { None }.or_else(|| blk.kids.iter().position(|k| k.is("BLT"))).unwrap_or(blk.kids.len());
    let before = blk.kids[..at].iter().filter(|k| k.is("KLM")).count();
    blk.kids.retain(|k| !k.is("KLM"));
    blk.kids.splice(at - before..at - before, out);
}

/// Blattart (BLT Feld 22 Byte 0) und Klemmleiste des Klemmplans (Feld 32 ab Bit 8) schreiben.
pub fn patch_blt(blk: &mut Block, s: &Sheet) {
    if (s.art != 0 || s.raw.is_some()) && (blk.i32(22) & 0xff) as u8 != s.art {
        let v = blk.i32(22);
        blk.set_i32(22, (v & !0xff) | s.art as i32);
    }
    // Feld 32: Byte 0 Format, ab Bit 8 die Leiste – nur bei Klemmplänen (bzw. nach einem Wechsel weg vom Klemmplan) ändern
    let f = blk.i32(32);
    let was_plan = s.raw.as_ref().map(|r| (r.block.i32(22) & 0xff) == 4).unwrap_or(false);
    let want = if s.art == 4 { (s.klm << 8) | (f & 0xff) } else if was_plan { f & 0xff } else { f };
    if want != f {
        blk.set_i32(32, want);
    }
}

/// Eingaben des Klemmen-Dialogs zum Klemmenindex.
pub struct Eingabe {
    pub leiste: String,
    pub nummer: String,
    pub auto: bool,
    pub korr: bool,
    pub position: String,
    pub posk: bool,
    /// Ebene (0 = einstöckig, 1 = A …)
    pub ebene: u8,
}

/// Klemmenindex wie im Original übernehmen (Anleitung 7.2, blattübergreifend innerhalb der Anlage):
/// - Nummer gilt, wenn „Automatisch“ aus ist oder „Korrektur“ an; mit „Automatisch“ + „Korrektur“ werden die
///   folgenden automatisch nummerierten Klemmen derselben Leiste ab diesem Wert fortlaufend nummeriert.
/// - „Automatisch“ ohne Korrektur: Wechsel der Leiste vergibt die nächste freie Nummer, sonst bleibt sie.
/// - Position mit „Korrektur“: diese und die folgenden Klemmen bekommen fortlaufende Positionen.
/// Reihenfolge der „folgenden“ Klemmen = Blatt- und Objektreihenfolge (Erstellreihenfolge im Original).
pub fn apply_index(p: &mut Project, id: u64, e: &Eingabe) {
    let Some((r, o)) = p.find_obj(id) else { return };
    let Kind::Terminal { leiste: old_leiste, nr: old_nr, opt: old_opt, .. } = &o.kind else { return };
    let (old_leiste, old_nr, old_pos) = (old_leiste.clone(), *old_nr, old_opt.pos(*old_nr));
    let a = &mut p.anlagen[r.anlage];
    // Klemmstellen der Ziel-Leiste in Erstellreihenfolge: (Blatt, Objekt)
    let order: Vec<(usize, usize)> = a
        .sheets
        .iter()
        .enumerate()
        .flat_map(|(si, s)| s.objects.iter().enumerate().filter(|(_, o)| matches!(&o.kind, Kind::Terminal { leiste, .. } if *leiste == e.leiste)).map(move |(oi, _)| (si, oi)))
        .collect();
    let me = a.sheets.iter().enumerate().find_map(|(si, s)| s.objects.iter().position(|o| o.id == id).map(|oi| (si, oi)));
    let Some(me) = me else { return };
    let max = order.iter().filter(|x| **x != me).filter_map(|(si, oi)| match &a.sheets[*si].objects[*oi].kind {
        Kind::Terminal { nr, .. } => Some(*nr),
        _ => None,
    }).max().unwrap_or(0);
    let typed: Option<u32> = e.nummer.trim().parse().ok();
    let nr = if !e.auto || e.korr { typed.unwrap_or(old_nr) } else if old_leiste != e.leiste { max + 1 } else { old_nr };
    let typed_pos: Option<u32> = e.position.trim().parse().ok();
    let pos = if e.posk { typed_pos.unwrap_or(old_pos) } else if old_leiste != e.leiste { nr } else { old_pos };
    if let Kind::Terminal { leiste, nr: n, level, opt, .. } = &mut a.sheets[me.0].objects[me.1].kind {
        *leiste = e.leiste.clone();
        *n = nr;
        *level = e.ebene;
        opt.auto = e.auto;
        opt.korr = e.korr;
        opt.posk = e.posk;
        opt.position = if pos == nr { 0 } else { pos };
    }
    // folgende Klemmen fortlaufend (Klemmen einer Etagengruppe – gleiche alte Nummer – behalten dieselbe)
    let start = order.iter().position(|x| *x == me).map(|i| i + 1).unwrap_or(order.len());
    if e.auto && e.korr {
        let mut map: Vec<(u32, u32)> = vec![(old_nr, nr)];
        let mut next = nr + 1;
        for (si, oi) in order[start..].iter().copied() {
            if let Kind::Terminal { nr: n, opt, .. } = &mut a.sheets[si].objects[oi].kind {
                // eine weitere Korrektur beginnt ihre eigene Folge; fest nummerierte Klemmen bleiben
                if opt.korr {
                    break;
                }
                if !opt.auto {
                    continue;
                }
                let new = match map.iter().find(|m| m.0 == *n) {
                    Some(m) => m.1,
                    None => {
                        map.push((*n, next));
                        next += 1;
                        next - 1
                    }
                };
                let p = opt.pos(*n);
                *n = new;
                opt.position = if p == new { 0 } else { p };
            }
        }
    }
    if e.posk {
        let mut map: Vec<(u32, u32)> = vec![(old_pos, pos)];
        let mut next = pos + 1;
        for (si, oi) in order[start..].iter().copied() {
            if let Kind::Terminal { nr: n, opt, .. } = &mut a.sheets[si].objects[oi].kind {
                let cur = opt.pos(*n);
                let new = match map.iter().find(|m| m.0 == cur) {
                    Some(m) => m.1,
                    None => {
                        map.push((cur, next));
                        next += 1;
                        next - 1
                    }
                };
                opt.position = if new == *n { 0 } else { new };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with(n: u32) -> (Project, Vec<u64>) {
        let mut p = Project::new_default();
        let mut ids = vec![];
        for i in 0..n {
            let id = p.alloc_id();
            ids.push(id);
            p.anlagen[0].sheets[0].objects.push(Obj { id, kind: Kind::Terminal { pos: P::new(20.0 + 10.0 * i as f32, 50.0), leiste: "Klemme 1".into(), nr: i + 1, level: 0, text_off: None, tpl: String::new(), opt: KlemmOpt::default() }, pen: 0, fixed: false, group: None, raw: None });
        }
        (p, ids)
    }

    fn nrs(p: &Project) -> Vec<(u32, u32)> {
        p.anlagen[0].sheets[0].objects.iter().filter_map(|o| if let Kind::Terminal { nr, opt, .. } = &o.kind { Some((*nr, opt.pos(*nr))) } else { None }).collect()
    }

    #[test]
    fn correction_renumbers_following_terminals() {
        let (mut p, ids) = project_with(4);
        let e = Eingabe { leiste: "Klemme 1".into(), nummer: "21".into(), auto: true, korr: true, position: String::new(), posk: false, ebene: 0 };
        apply_index(&mut p, ids[1], &e);
        assert_eq!(nrs(&p), vec![(1, 1), (21, 2), (22, 3), (23, 4)]);
        // Position mit Korrektur: diese und die folgenden Klemmen
        let e = Eingabe { leiste: "Klemme 1".into(), nummer: "1".into(), auto: true, korr: false, position: "10".into(), posk: true, ebene: 0 };
        apply_index(&mut p, ids[0], &e);
        assert_eq!(nrs(&p), vec![(1, 10), (21, 11), (22, 12), (23, 13)]);
        // ohne Automatik gilt die eingetragene Nummer, die anderen bleiben
        let e = Eingabe { leiste: "Klemme 1".into(), nummer: "7".into(), auto: false, korr: false, position: "13".into(), posk: false, ebene: 0 };
        apply_index(&mut p, ids[3], &e);
        assert_eq!(nrs(&p), vec![(1, 10), (21, 11), (22, 12), (7, 13)]);
    }

    #[test]
    fn group_label_starts_at_number() {
        assert_eq!(label("%O%B.%C", "-X2", 2, 2, false), "-X2.2B");
        assert_eq!(label("%O%B.%C", "-X2", 2, 2, true), "2B");
    }

    #[test]
    fn flags_roundtrip() {
        let o = KlemmOpt { auto: true, korr: false, posk: true, ziel_wechsel: true, ..Default::default() };
        assert_eq!(o.flags(), 0b1101);
        let mut b = Block::new("OBJ", vec![0; 64]);
        b.set_i32(4, 0x2ffff);
        b.set_i32(8, 5);
        b.set_i32(9, 3);
        b.set_i32(10, 7);
        b.data[0x2d] = o.flags();
        b.data[0x2e] = 1;
        b.kids.push(Block::new("FKT", b"Licht".to_vec()));
        b.kids.push(Block::new("TIT", b"%O%B.%C".to_vec()));
        assert!(is_terminal(&b));
        let r = read_opt(&b);
        assert_eq!((r.auto, r.korr, r.posk, r.ziel_wechsel, r.gr_auto, r.align, r.position, r.funktion.as_str()), (true, false, true, true, true, 2, 7, "Licht"));
    }
}
