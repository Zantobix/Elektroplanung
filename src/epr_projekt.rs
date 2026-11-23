//! Projekt- und Anlagenebene im Originalformat (*.epr): Projekttexte, Stifte, Leitungsdaten, Potentialwerte,
//! Schriftfeld, Anlagen (Zeichnungsnummer, Sammelschienen- und Klemmleistennamen) und Blattdaten (Art, Ort,
//! Strompfade, Pfadbeschreibungen, Begrenzungslinien) lesen und zurückschreiben.
//!
//! Ablage, ermittelt aus der Speicherroutine des Originals (0x4228c4; Kennung je Aufruf über `mov dl, n` in die
//! Kennungstabelle 0x5b9134, Blockinhalt = Speicherstruktur mit der übergebenen Länge), der Laderoutine und den Werten
//! in W2.epr bzw. der Vorlage:
//! - PRJ › TIT = Eigentümer/Rechte (Struktur +0x9c), WRT = Kunde (+0xa0), STR = Standardschrift (+0x98); nur wenn
//!   nicht leer, direkt nach dem PRJ-Kopf. Welche der beiden RichEdit-Felder TIT und WRT sind, folgt nur aus der
//!   Reihenfolge im Dialog (geschätzt).
//! - LTG (44 Byte): Name 0–20, Feld 5 Querschnitt in 1/100 mm² (Vorgabe "1,00" → 100), 6 Aderfarbe (TColor),
//!   7 Anzahl Adern, 8 Byte 0 Kennzeichnung (0 keine, 1 Zahlen, 2 farbig), 9 Kennung (VBG Feld 13), 10 Stift.
//! - POT (40 Byte): Name 0–20, Feld 5 Nennspannung in mV, 6 Nennstrom in mA ("230"/"63" → 230000/63000), 8 Kennung
//!   (SMS Feld 11).
//! - SIT (40 Byte, nur eigene Stifte ab Nr. 4): Name 0–20, Feld 5 Breite in 0,1 mm (Vorgabe "0,2" → 2),
//!   6 Farbe (TColor), 7 Byte 0 Stiftart (geschätzt; in W2.epr nur 0), 8 Nummer = Stiftindex der Objekte.
//! - FUS (16 Byte): Feld 0 Schriftfeldhöhe in 0,1 mm (Vorgabe 150 aus 0x401e40), Feld 1 = 2 (Vorgabe, Bedeutung
//!   offen); GRS Elemente, STR Texte.
//! - ANL: Zeichnungsnummer Byte 116–136 (Feldlängen 55/11/50/20 aus der Anlagenerzeugung 0x4031a4).
//!   SMS (76 Byte): Name 0–20 (Dialog "Name", z. B. "Schiene L1"), TIT = Kürzel ("L1"; im Nachbau der Name der
//!   Schiene), Feld 8/9 Kennung,
//!   11 Potential (POT Feld 8), 18 Stift. KLM (80 Byte): Name 0–20, TIT, Feld 13 Kennung, 15 Etagen.
//! - BLT: Feld 22 Byte 0 Blattart, Byte 51–71 Ort, Feld 25 Strompfade (−1 = von der Anlage), Feld 32 Bit 4
//!   Begrenzungslinien (Pfaddialog 0x499b88: Blatt +0x80 Bit 0x10), Bit 5 „Auto+“ (Blattdialog, 0x461e20),
//!   Feld 32 ab Bit 8 Klemmleiste (Klemmplan);
//!   PFB (4 Byte: Pfadnummer ab 1) › STR = Pfadbeschreibung (Blatt +0x9c, Dialog TPfadDialog).
//!
//! Grundsatz wie in `epr_write`: Ein Feld wird nur geschrieben, wenn sich der Wert im Modell von dem im Block
//! unterscheidet – unveränderte Projekte bleiben byteidentisch.

use crate::eprfile::{cstr_bytes, put_str, Block, EprFile};
use crate::model::*;
use crate::titleblock::TbOwned;

/// Anzahl der fest eingebauten Stifte (Standard, Gestrichelt, Strich/Punkt, Gepunktet); sie stehen nicht in der Datei.
pub const FESTE_STIFTE: usize = 4;

fn txt(d: &[u8], from: usize, to: usize) -> String {
    d.get(from..to.min(d.len())).map(crate::symbols::cstr).unwrap_or_default()
}

/// Text eines Unterblocks (TIT/WRT/STR …), Zeilenumbrüche als "\n".
fn kid_text(b: &Block, tag: &str) -> String {
    b.kid(tag).map(|k| crate::symbols::cstr(&k.data).replace("\r\n", "\n")).unwrap_or_default()
}

fn text_block(tag: &str, s: &str) -> Block {
    Block::new(tag, cstr_bytes(&s.replace('\n', "\r\n")))
}

/// Text eines Unterblocks setzen (anlegen, falls er fehlt).
fn set_kid_text(b: &mut Block, tag: &str, s: &str) {
    let data = cstr_bytes(&s.replace('\n', "\r\n"));
    match b.kids.iter_mut().find(|k| k.is(tag)) {
        Some(k) => k.data = data,
        None => b.kids.push(Block::new(tag, data)),
    }
}

/// TColor (0x00BBGGRR) → RGB.
pub fn tcolor(c: i32) -> [u8; 3] {
    if (0..=0xff_ffff).contains(&c) {
        [(c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8]
    } else {
        [0, 0, 0]
    }
}

pub fn to_tcolor(c: [u8; 3]) -> i32 {
    c[0] as i32 | (c[1] as i32) << 8 | (c[2] as i32) << 16
}

/// Vorlage (leeres Projekt im Originalformat), einmal gelesen.
fn vorlage() -> &'static EprFile {
    static T: std::sync::OnceLock<EprFile> = std::sync::OnceLock::new();
    T.get_or_init(|| EprFile::parse(include_bytes!("../resources/template.epr")).expect("Vorlage template.epr ist gültig"))
}

fn vorlage_prj() -> &'static Block {
    vorlage().root("PRJ").expect("PRJ in der Vorlage")
}

/// PRJ-Block, auf dem das Speichern aufbaut (gelesene Datei, sonst Vorlage).
fn basis_prj(project: &Project) -> &Block {
    match project.raw.as_deref().and_then(|f| f.root("PRJ")) {
        Some(b) => b,
        None => vorlage_prj(),
    }
}

// ---------------------------------------------------------------- Werte aus Blöcken

/// Stift aus einem SIT-Block.
pub fn stift_aus(b: &Block) -> Stift {
    Stift { name: txt(&b.data, 0, 20), style: (b.i32(7) & 0xff).clamp(0, 3) as u8, color: tcolor(b.i32(6)), width: b.i32(5) as f32 / 10.0 }
}

fn stift_gleich(a: &Stift, b: &Stift) -> bool {
    a.name == b.name && a.style == b.style && a.color == b.color && (a.width * 10.0).round() == (b.width * 10.0).round()
}

/// Leitungsdaten aus einem LTG-Block: (Querschnitt mm², Adern, Kennzeichnung, Farbe, Stift).
fn leitung_aus(b: &Block) -> (f32, u32, u8, [u8; 3], u8) {
    (b.i32(5) as f32 / 100.0, b.i32(7).max(0) as u32, (b.i32(8) & 0xff) as u8, tcolor(b.i32(6)), b.i32(10).clamp(0, 255) as u8)
}

fn hundertstel(v: f32) -> i32 {
    (v * 100.0).round() as i32
}

/// Zahl aus einem Dialogtext ("230", "0,5", "16 A") in Tausendstel; None bei unlesbarem Text, Some(0) bei leerem.
pub fn tausendstel(s: &str) -> Option<i32> {
    let t: String = s.trim().replace(',', ".").chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
    if s.trim().is_empty() {
        return Some(0);
    }
    t.parse::<f64>().ok().map(|v| (v * 1000.0).round() as i32)
}

/// Tausendstel als Dialogtext ("230", "0,5").
pub fn als_text(v: i32) -> String {
    if v == 0 {
        return String::new();
    }
    let s = format!("{:.3}", v as f64 / 1000.0);
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// Schriftfeld aus einem FUS-Block (wie beim Import).
pub fn schriftfeld_aus(fus: &Block) -> Vec<TbOwned> {
    let texts: Vec<String> = fus.kid("STR").map(|n| n.data.split(|c| *c == 0).map(crate::symbols::cstr).collect()).unwrap_or_default();
    let mut tb = vec![];
    for g in fus.kids_of("GRS") {
        for e in g.data.chunks_exact(0x50) {
            let v = |i: usize| crate::eprfile::i32_at(e, i * 4);
            match e[0x3c] {
                1 => tb.push(TbOwned::Line(v(0), v(1), v(2), v(3))),
                6 => {
                    let idx = v(7);
                    let t = if idx >= 1 { texts.get(idx as usize - 1).cloned().unwrap_or_default() } else { String::new() };
                    let al = match v(4) & 0xff {
                        1 => Align::Center,
                        2 => Align::Right,
                        _ => Align::Left,
                    };
                    tb.push(TbOwned::Text(v(0), v(1), v(2), v(3), t.replace("\r\n", "\n"), al, v(9).clamp(1, 8) as u8));
                }
                12 => tb.push(TbOwned::Logo(v(0), v(1), v(2), v(3))),
                _ => {}
            }
        }
    }
    tb
}

/// Pfadbeschreibungen eines Blattes (PFB, Nummer ab 1) als Liste je Strompfad.
fn pfade_aus(b: &Block) -> Vec<String> {
    let mut v: Vec<String> = vec![];
    for k in b.kids_of("PFB") {
        let nr = k.i32(0);
        if (1..=200).contains(&nr) {
            if v.len() < nr as usize {
                v.resize(nr as usize, String::new());
            }
            v[nr as usize - 1] = kid_text(k, "STR");
        }
    }
    v
}

fn ohne_leere_enden(v: &[String]) -> &[String] {
    let n = v.iter().rposition(|s| !s.is_empty()).map(|i| i + 1).unwrap_or(0);
    &v[..n]
}

/// Strompfade eines Blattes aus BLT Feld 25 (−1 = von der Anlage): (von der Anlage, Anzahl).
fn strompfade_aus(b: &Block) -> (bool, i32) {
    let v = b.i32(25);
    (v < 1, v)
}

fn sms_name(s: &Block) -> String {
    s.kid("TIT").map(|t| crate::symbols::cstr(&t.data)).unwrap_or_else(|| txt(&s.data, 0, 20))
}

// ---------------------------------------------------------------- Import

/// Ergänzt ein aus .epr gelesenes Projekt um die Werte der Projekt-, Anlagen- und Blattebene, die `epr::import_file`
/// nicht selbst liest.
pub fn import(prj: &Block, p: &mut Project) {
    p.owner = kid_text(prj, "TIT");
    p.kunde = kid_text(prj, "WRT");
    p.schrift = kid_text(prj, "STR");
    // eigene Stifte: Nummer (Feld 8) = Index in der Stiftliste
    for b in prj.kids_of("SIT") {
        let id = b.i32(8);
        if (FESTE_STIFTE as i32..256).contains(&id) {
            let id = id as usize;
            while p.stifte.len() <= id {
                let n = p.stifte.len() + 1;
                p.stifte.push(Stift { name: format!("Stift {n}"), style: 0, color: [0, 0, 0], width: 0.2 });
            }
            p.stifte[id] = stift_aus(b);
        }
    }
    // Leitungen wurden in Blockreihenfolge angelegt
    for (l, b) in p.leitungen.iter_mut().zip(prj.kids_of("LTG")) {
        let (q, a, k, ..) = leitung_aus(b);
        l.quer = q;
        l.adern = a;
        l.kennz = k;
    }
    for b in prj.kids_of("POT") {
        let (u, i) = (b.i32(5), b.i32(6));
        if u != 0 || i != 0 {
            p.pot_werte.insert(txt(&b.data, 0, 20), (als_text(u), als_text(i)));
        }
    }
    if let Some(f) = prj.kid("FUS") {
        let h = f.i32(0);
        if (30..=600).contains(&h) && h != 150 {
            p.tb_hoehe = h as f32 / 10.0;
        }
    }
    let pots: std::collections::HashMap<i32, String> = prj.kids_of("POT").map(|b| (b.i32(8), txt(&b.data, 0, 20))).collect();
    for (a, ab) in p.anlagen.iter_mut().zip(prj.kids_of("ANL")) {
        for s in ab.kids_of("SMS") {
            // SMS Name 0–20 ("Schiene L1") bleibt unberührt; der Nachbau führt die Schiene unter ihrem Kürzel (TIT)
            let mut set = SchieneSet { potential: pots.get(&s.i32(11)).cloned().unwrap_or_default(), stift: s.i32(18).clamp(0, 255) as usize, ..Default::default() };
            crate::epr_zusatz::schiene_lesen(s, &mut set);
            a.schienen.entry(sms_name(s)).or_insert(set);
        }
        for (s, b) in a.sheets.iter_mut().zip(ab.kids_of("BLT")) {
            let (auto, n) = strompfade_aus(b);
            if !auto {
                s.col_auto = false;
                s.columns = n.clamp(1, 100) as u32;
            }
            s.pfad_linien = b.i32(32) & 0x10 != 0;
            s.auto_plus = b.i32(32) & 0x20 != 0;
            let labels = pfade_aus(b);
            if !labels.is_empty() {
                s.col_labels = labels;
            }
        }
    }
}

// ---------------------------------------------------------------- Zuordnung Modell ↔ Blöcke

/// Zuordnung der Modelleinträge zu vorhandenen Blöcken: zuerst über den Schlüssel (`passt`), danach die übrigen
/// Einträge (für die `frei` gilt) der Reihe nach auf die übrigen Blöcke (Umbenennung).
fn zuordnen(n: usize, blocks: &[&Block], passt: impl Fn(usize, &Block) -> bool, frei: impl Fn(usize) -> bool) -> Vec<Option<usize>> {
    let mut used = vec![false; blocks.len()];
    let mut out = vec![None; n];
    for (i, o) in out.iter_mut().enumerate() {
        if let Some(j) = (0..blocks.len()).find(|&j| !used[j] && passt(i, blocks[j])) {
            used[j] = true;
            *o = Some(j);
        }
    }
    let mut rest = (0..blocks.len()).filter(|j| !used[*j]).collect::<Vec<_>>().into_iter();
    for (i, o) in out.iter_mut().enumerate() {
        if o.is_none() && frei(i) {
            *o = rest.next();
        }
    }
    out
}

/// Kennungen (LTG Feld 9) der Leitungen in Modellreihenfolge, wie sie beim Speichern vergeben werden:
/// vorhandene bleiben, neue Leitungen erhalten die nächste freie Nummer.
pub fn leitung_ids(project: &Project) -> Vec<i32> {
    leitung_zuordnung(project).1
}

fn leitung_zuordnung(project: &Project) -> (Vec<Option<usize>>, Vec<i32>) {
    let src = basis_prj(project);
    let blocks: Vec<&Block> = src.kids_of("LTG").collect();
    let l = &project.leitungen;
    let m = zuordnen(l.len(), &blocks, |i, b| l[i].id != 0 && b.i32(9) == l[i].id, |i| l[i].id == 0);
    let mut next = blocks.iter().map(|b| b.i32(9)).chain(l.iter().map(|x| x.id)).max().unwrap_or(0).max(0) + 1;
    let ids = m
        .iter()
        .enumerate()
        .map(|(i, mm)| match mm {
            Some(j) => blocks[*j].i32(9),
            None if l[i].id != 0 => l[i].id,
            None => {
                next += 1;
                next - 1
            }
        })
        .collect();
    (m, ids)
}

/// Kennungen (POT Feld 8) der Potentiale in Modellreihenfolge.
pub fn pot_ids(project: &Project) -> Vec<i32> {
    pot_zuordnung(project).1
}

fn pot_zuordnung(project: &Project) -> (Vec<Option<usize>>, Vec<i32>) {
    let src = basis_prj(project);
    let blocks: Vec<&Block> = src.kids_of("POT").collect();
    let p = &project.potentiale;
    let m = zuordnen(p.len(), &blocks, |i, b| txt(&b.data, 0, 20) == p[i], |_| true);
    let mut next = blocks.iter().map(|b| b.i32(8)).max().unwrap_or(2).max(2) + 1;
    let ids = m
        .iter()
        .map(|mm| match mm {
            Some(j) => blocks[*j].i32(8),
            None => {
                next += 1;
                next - 1
            }
        })
        .collect();
    (m, ids)
}

/// Verbindung (VBG): Kennung der Leitung in Feld 13 nachführen, wenn die Verbindung neu ist oder eine andere
/// Leitung bekommen hat.
pub fn leitung_setzen(project: &Project, o: &Obj, b: &mut Block) {
    let Kind::Wire { leitung, .. } = &o.kind else { return };
    if !b.is("VBG") {
        return;
    }
    let alt = o.raw.as_deref().and_then(|r| if let Kind::Wire { leitung, .. } = &r.kind { Some(*leitung) } else { None });
    if alt == Some(*leitung) {
        return;
    }
    if let Some(id) = leitung_ids(project).get(*leitung) {
        b.set_i32(13, *id);
    }
}

/// Alle Unterblöcke einer Art durch `list` ersetzen (an der Stelle des ersten bisherigen, sonst nach dem letzten
/// Block einer der Arten in `nach`). Gleiche Liste = keine Änderung.
fn set_group(parent: &mut Block, tag: &str, list: Vec<Block>, nach: &[&str]) {
    let same = {
        let old: Vec<&Block> = parent.kids_of(tag).collect();
        old.len() == list.len() && old.iter().zip(&list).all(|(a, b)| **a == *b)
    };
    if same {
        return;
    }
    let first = parent.kids.iter().position(|k| k.is(tag));
    parent.kids.retain(|k| !k.is(tag));
    let at = first.unwrap_or_else(|| parent.kids.iter().rposition(|k| nach.iter().any(|t| k.is(t))).map(|i| i + 1).unwrap_or(0));
    let at = at.min(parent.kids.len());
    parent.kids.splice(at..at, list);
}

// ---------------------------------------------------------------- Schreiben

/// Projekt-, Anlagen- und Blattwerte in den fertig aufgebauten PRJ-Block übernehmen (aufgerufen von
/// `epr_write::build`, nachdem Anlagen und Blätter geschrieben sind).
pub fn write(project: &Project, prj: &mut Block) {
    let tpl = vorlage_prj();
    // Projekttexte (TIT, WRT, STR direkt nach dem Kopf)
    let soll = [("TIT", &project.owner), ("WRT", &project.kunde), ("STR", &project.schrift)];
    if soll.iter().any(|(t, s)| kid_text(prj, t) != **s) {
        prj.kids.retain(|k| !(k.is("TIT") || k.is("WRT") || k.is("STR")));
        let neu: Vec<Block> = soll.iter().filter(|(_, s)| !s.is_empty()).map(|(t, s)| text_block(t, s)).collect();
        prj.kids.splice(0..0, neu);
    }
    write_leitungen(project, prj, tpl);
    let pot_ids = write_potentiale(project, prj, tpl);
    write_stifte(project, prj, tpl);
    write_schriftfeld(project, prj, tpl);
    let pots: std::collections::HashMap<&str, i32> = project.potentiale.iter().map(|s| s.as_str()).zip(pot_ids.iter().copied()).collect();
    let anl: Vec<usize> = prj.kids.iter().enumerate().filter(|(_, k)| k.is("ANL")).map(|(i, _)| i).collect();
    for (a, &ki) in project.anlagen.iter().zip(&anl) {
        write_anlage(project, a, &mut prj.kids[ki], &pots);
    }
}

fn write_leitungen(project: &Project, prj: &mut Block, tpl: &Block) {
    let (m, ids) = leitung_zuordnung(project);
    let blocks: Vec<Block> = prj.kids_of("LTG").cloned().collect();
    // Zuordnung bezieht sich auf die Basisdatei; der aufgebaute Block hat dieselben LTG-Blöcke
    if blocks.len() != basis_prj(project).kids_of("LTG").count() {
        return;
    }
    let mut tpl_ltg = tpl.kid("LTG").cloned().unwrap_or_else(|| Block::new("LTG", vec![0; 44]));
    tpl_ltg.kids.clear();
    let list = project
        .leitungen
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut b = m[i].map(|j| blocks[j].clone()).unwrap_or_else(|| tpl_ltg.clone());
            if b.data.len() < 44 {
                b.data.resize(44, 0);
            }
            if txt(&b.data, 0, 20) != l.name {
                put_str(&mut b.data, 0, 20, &l.name);
            }
            let alt = leitung_aus(&b);
            if (hundertstel(alt.0), alt.1, alt.2, alt.3, alt.4) != (hundertstel(l.quer), l.adern, l.kennz, l.color, l.style) {
                if hundertstel(alt.0) != hundertstel(l.quer) {
                    b.set_i32(5, hundertstel(l.quer));
                }
                if alt.3 != l.color {
                    b.set_i32(6, to_tcolor(l.color));
                }
                if alt.1 != l.adern {
                    b.set_i32(7, l.adern as i32);
                }
                if alt.2 != l.kennz {
                    let v = b.i32(8) & !0xff;
                    b.set_i32(8, v | l.kennz as i32);
                }
                if alt.4 != l.style {
                    b.set_i32(10, l.style as i32);
                }
            }
            if b.i32(9) != ids[i] {
                b.set_i32(9, ids[i]);
            }
            b
        })
        .collect();
    set_group(prj, "LTG", list, &["TIT", "WRT", "STR"]);
}

fn write_potentiale(project: &Project, prj: &mut Block, tpl: &Block) -> Vec<i32> {
    let (m, ids) = pot_zuordnung(project);
    let blocks: Vec<Block> = prj.kids_of("POT").cloned().collect();
    if blocks.len() != basis_prj(project).kids_of("POT").count() {
        return ids;
    }
    let mut tpl_pot = tpl.kid("POT").cloned().unwrap_or_else(|| Block::new("POT", vec![0; 40]));
    tpl_pot.kids.clear();
    let list = project
        .potentiale
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let mut b = m[i].map(|j| blocks[j].clone()).unwrap_or_else(|| {
                // neues Potential: ohne Werte der Vorlage
                let mut b = tpl_pot.clone();
                b.set_i32(5, 0);
                b.set_i32(6, 0);
                b
            });
            if txt(&b.data, 0, 20) != *name {
                put_str(&mut b.data, 0, 20, name);
            }
            let (u, a) = project.pot_werte.get(name).cloned().unwrap_or_default();
            for (f, s) in [(5, u), (6, a)] {
                if let Some(v) = tausendstel(&s) {
                    if b.i32(f) != v {
                        b.set_i32(f, v);
                    }
                }
            }
            if b.i32(8) != ids[i] {
                b.set_i32(8, ids[i]);
            }
            b
        })
        .collect();
    set_group(prj, "POT", list, &["LTG", "TIT", "WRT", "STR"]);
    ids
}

fn write_stifte(project: &Project, prj: &mut Block, tpl: &Block) {
    let blocks: Vec<Block> = prj.kids_of("SIT").cloned().collect();
    let mut tpl_sit = tpl.kid("SIT").cloned().unwrap_or_else(|| Block::new("SIT", vec![0; 40]));
    tpl_sit.kids.clear();
    let list = project
        .stifte
        .iter()
        .enumerate()
        .skip(FESTE_STIFTE)
        .map(|(id, s)| {
            let mut b = blocks.iter().find(|b| b.i32(8) == id as i32).cloned().unwrap_or_else(|| {
                let mut b = tpl_sit.clone();
                b.set_i32(8, id as i32);
                b
            });
            let alt = stift_aus(&b);
            if !stift_gleich(&alt, s) {
                if alt.name != s.name {
                    put_str(&mut b.data, 0, 20, &s.name);
                }
                if (alt.width * 10.0).round() != (s.width * 10.0).round() {
                    b.set_i32(5, (s.width * 10.0).round() as i32);
                }
                if alt.color != s.color {
                    b.set_i32(6, to_tcolor(s.color));
                }
                if alt.style != s.style {
                    let v = b.i32(7) & !0xff;
                    b.set_i32(7, v | s.style as i32);
                }
            }
            b
        })
        .collect();
    set_group(prj, "SIT", list, &["POT", "LTG", "TIT", "WRT", "STR"]);
}

/// Schriftfeld (FUS): Höhe in Feld 0, Elemente (GRS) und Texte (STR). Unbekannte Elementarten des Originals
/// bleiben angehängt erhalten, die bisherigen Texte behalten ihre Nummern.
fn write_schriftfeld(project: &Project, prj: &mut Block, tpl: &Block) {
    let soll = if project.titleblock.is_empty() { crate::titleblock::default_owned() } else { project.titleblock.clone() };
    let h = (project.title_h() * 10.0).round() as i32;
    // Maßangaben „Absolut“: Koordinaten in 0,1 mm (das Modell führt sie immer relativ, epr_zusatz)
    let soll = if project.tb_absolut { crate::titleblock::nach_absolut(&soll, h) } else { soll };
    let Some(fi) = prj.kids.iter().position(|k| k.is("FUS")).or_else(|| {
        let f = tpl.kid("FUS")?.clone();
        let at = prj.kids.iter().position(|k| k.is("ANL") || k.is("BLD")).unwrap_or(prj.kids.len());
        prj.kids.insert(at, f);
        Some(at)
    }) else {
        return;
    };
    let fus = &mut prj.kids[fi];
    if fus.i32(0) != h {
        fus.set_i32(0, h);
    }
    if schriftfeld_aus(fus) == soll {
        return;
    }
    let mut texts: Vec<String> = fus.kid("STR").map(|n| crate::symbols::split_strings(&n.data)).unwrap_or_default().into_iter().map(|t| t.replace("\r\n", "\n")).collect();
    let orig: Vec<Vec<u8>> = fus.kids_of("GRS").flat_map(|g| g.data.chunks_exact(0x50).map(|c| c.to_vec()).collect::<Vec<_>>()).collect();
    let (known, unknown): (Vec<Vec<u8>>, Vec<Vec<u8>>) = orig.into_iter().partition(|e| matches!(e[0x3c], 1 | 6 | 12));
    let mut out = vec![];
    for (i, e) in soll.iter().enumerate() {
        let ty: u8 = match e {
            TbOwned::Line(..) => 1,
            TbOwned::Text(..) => 6,
            TbOwned::Logo(..) => 12,
        };
        let mut el = known.get(i).filter(|k| k[0x3c] == ty).cloned().unwrap_or_else(|| vec![0; 0x50]);
        let f4 = crate::eprfile::i32_at(&el, 16) & !0xff;
        let mut set = |idx: usize, v: i32| el[idx * 4..idx * 4 + 4].copy_from_slice(&v.to_le_bytes());
        let (a, b, c, d) = match e {
            TbOwned::Line(a, b, c, d) | TbOwned::Logo(a, b, c, d) | TbOwned::Text(a, b, c, d, ..) => (*a, *b, *c, *d),
        };
        set(0, a);
        set(1, b);
        set(2, c);
        set(3, d);
        if let TbOwned::Text(_, _, _, _, t, al, lines) = e {
            let idx = match texts.iter().position(|x| x == t) {
                Some(k) => k + 1,
                None => {
                    texts.push(t.clone());
                    texts.len()
                }
            };
            let al = match al {
                Align::Left => 0,
                Align::Center => 1,
                Align::Right => 2,
            };
            set(4, f4 | al);
            set(7, idx as i32);
            set(9, *lines as i32);
        }
        el[0x3c] = ty;
        out.extend_from_slice(&el);
    }
    for e in &unknown {
        out.extend_from_slice(e);
    }
    let mut strs = vec![];
    for t in &texts {
        strs.extend(cstr_bytes(&t.replace('\n', "\r\n")));
    }
    let mut kids = vec![Block::new("GRS", out), Block::new("STR", strs)];
    kids.extend(fus.kids.iter().filter(|k| !k.is("GRS") && !k.is("STR")).cloned());
    fus.kids = kids;
}

fn write_anlage(project: &Project, a: &Anlage, blk: &mut Block, pots: &std::collections::HashMap<&str, i32>) {
    // Zeichnungsnummer (bei neuen Anlagen ohne eigene: die des Projekts)
    let znr = if a.set.znr.trim().is_empty() && a.raw.is_none() { project.znr.clone() } else { a.set.znr.clone() };
    if txt(&blk.data, 116, 136) != znr {
        put_str(&mut blk.data, 116, 20, &znr);
    }
    write_schienen(a, blk, pots);
    // Klemmleisten (KLM › KFO) schreibt klemmen::sync_klm beim Aufbau der Anlage
    let blt: Vec<usize> = blk.kids.iter().enumerate().filter(|(_, k)| k.is("BLT")).map(|(i, _)| i).collect();
    for (s, &ki) in a.sheets.iter().zip(&blt) {
        write_blatt(a, s, &mut blk.kids[ki]);
    }
}

fn write_schienen(a: &Anlage, blk: &mut Block, pots: &std::collections::HashMap<&str, i32>) {
    let blocks: Vec<Block> = blk.kids_of("SMS").cloned().collect();
    let refs: Vec<&Block> = blocks.iter().collect();
    let names = &a.sammelschienen;
    let m = zuordnen(names.len(), &refs, |i, b| sms_name(b) == names[i], |_| true);
    // Potentialnamen der Datei (für den Vergleich)
    let list = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let mut b = match m[i] {
                Some(j) => blocks[j].clone(),
                None => {
                    let mut b = Block::new("SMS", vec![0; 76]);
                    b.set_i32(7, 0xffff);
                    b.data[32..40].copy_from_slice(&crate::kennungen::neu().to_le_bytes());
                    b.set_i32(12, 1);
                    b
                }
            };
            if sms_name(&b) != *name || b.kid("TIT").is_none() {
                set_kid_text(&mut b, "TIT", name);
            }
            let set = a.schienen.get(name).cloned().unwrap_or_default();
            // freier Name (SMS Byte 0–20); neue Schiene ohne Namen wie im Original „Schiene <Kürzel>“. Nur bei
            // Änderung schreiben (Rundreise unveränderter Dateien byteidentisch)
            let frei = if set.name.trim().is_empty() { format!("Schiene {name}") } else { set.name.clone() };
            if m[i].is_none() || crate::symbols::cstr(&b.data[..b.data.len().min(20)]) != frei {
                put_str(&mut b.data, 0, 20, &frei);
            }
            // Potential: nur schreiben, wenn es sich gegenüber der Kennung im Block geändert hat
            let pid = pots.get(set.potential.as_str()).copied().unwrap_or(0);
            let alt_name = pots.iter().find(|(_, v)| **v == b.i32(11)).map(|(k, _)| *k).unwrap_or("");
            if alt_name != set.potential {
                b.set_i32(11, pid);
            }
            if b.i32(18) != set.stift as i32 {
                b.set_i32(18, set.stift as i32);
            }
            crate::epr_zusatz::schiene_schreiben(&set, &mut b);
            b
        })
        .collect();
    set_group(blk, "SMS", list, &[]);
}

fn write_blatt(a: &Anlage, s: &Sheet, b: &mut Block) {
    let _ = a;
    if b.data.len() < 140 {
        return;
    }
    // Blattart (Feld 22) und Klemmleiste (Feld 32 ab Bit 8) schreibt klemmen::patch_blt
    if txt(&b.data, 51, 71) != s.ort {
        put_str(&mut b.data, 51, 20, &s.ort);
    }
    let (auto, n) = strompfade_aus(b);
    let soll = if s.col_auto { -1 } else { s.columns as i32 };
    if auto != s.col_auto || (!s.col_auto && n != soll) {
        b.set_i32(25, soll);
    }
    let f32_ = b.i32(32);
    let mut v = f32_;
    if (v & 0x10 != 0) != s.pfad_linien {
        v ^= 0x10;
    }
    if (v & 0x20 != 0) != s.auto_plus {
        v ^= 0x20;
    }
    if v != f32_ {
        b.set_i32(32, v);
    }
    if ohne_leere_enden(&pfade_aus(b)) != ohne_leere_enden(&s.col_labels) {
        b.kids.retain(|k| !k.is("PFB"));
        for (i, t) in s.col_labels.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
            let mut p = Block::new("PFB", ((i + 1) as i32).to_le_bytes().to_vec());
            p.kids.push(text_block("STR", t));
            b.kids.push(p);
        }
    }
}

// ---------------------------------------------------------------- nicht speicherbare Angaben

/// Angaben des Projekts, die das Originalformat (.epr) nach heutigem Stand nicht aufnehmen kann (Ablage unbekannt);
/// sie bleiben nur im eigenen Format (.epj) erhalten. Leere Liste = nichts geht verloren.
pub fn nicht_gespeichert(project: &Project) -> Vec<String> {
    // Bauteile, Platzhalter, Projektstatus, Alter Klemmenplan, Schriftfeld absolut, Schienen-Schalter und
    // Verbindungsoptionen schreibt epr_zusatz; von dort kommen nur Sonderfälle (zu lange Texte, unbekannte Gruppen)
    let mut v = crate::epr_zusatz::nicht_gespeichert(project);
    if !project.ort.trim().is_empty() {
        v.push("Ort des Projekts (das Original kennt den Ort nur je Blatt)".into());
    }
    let std = Project::new_default().stifte;
    for (i, s) in project.stifte.iter().enumerate().take(FESTE_STIFTE) {
        if std.get(i).map(|d| !stift_gleich(d, s)).unwrap_or(false) {
            v.push(format!("Änderungen am fest eingebauten Stift \u{201e}{}\u{201c}", std[i].name));
        }
    }
    for (name, (u, i)) in &project.pot_werte {
        if tausendstel(u).is_none() || tausendstel(i).is_none() {
            v.push(format!("Potential {name}: Nennspannung/-strom ist keine Zahl"));
        }
    }
    for a in &project.anlagen {
        for (name, s) in &a.schienen {
            // Der Nachbau führt die Schiene unter ihrem Kürzel (SMS › TIT); ein abweichendes zweites Kürzel (alte
            // .epj) hat im Original kein Gegenstück (dort ist SMS Byte 0–20 der freie Name, das Kürzel ist der TIT)
            if !s.kurz.trim().is_empty() && s.kurz != *name {
                v.push(format!("{}: Kürzel \u{201e}{}\u{201c} der Sammelschiene {name}", a.name, s.kurz));
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zahlen_in_tausendsteln() {
        assert_eq!(tausendstel("230"), Some(230_000));
        assert_eq!(tausendstel("0,5 A"), Some(500));
        assert_eq!(tausendstel(""), Some(0));
        assert_eq!(tausendstel("viel"), None);
        assert_eq!(als_text(230_000), "230");
        assert_eq!(als_text(500), "0,5");
        assert_eq!(als_text(0), "");
    }

    #[test]
    fn zuordnung_erst_schluessel_dann_reihenfolge() {
        let a = Block::new("POT", b"L1\0".to_vec());
        let b = Block::new("POT", b"L2\0".to_vec());
        let blocks = vec![&a, &b];
        let names = ["L2", "X"];
        let m = zuordnen(2, &blocks, |i, bl| crate::symbols::cstr(&bl.data) == names[i], |_| true);
        assert_eq!(m, vec![Some(1), Some(0)]);
    }
}
