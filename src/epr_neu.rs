//! Neu angelegte Blattobjekte im Originalformat (*.epr): Tabelle, Bild (samt Projektbild BLD › JPG),
//! Verweis und Sammelschiene, dazu die Eigenschaften, die `epr_write` für neue Linien, Texte (auch Textboxen,
//! siehe `crate::textbox`), Symbole und
//! Verbindungen noch nicht schrieb (Fixiert, Umbruch, Schriftname, Leitung, Verbindungsbezeichnungen,
//! Alternativtexte, Parameter).
//!
//! Aufbau aus der Speicherroutine des Originals (0x424b8f ff.: ZCN 0x50 Byte, dann je Typ Unterblöcke) und dem
//! Eigenschaftendialog (0x44fbd3 ff.: Bild, 0x44fef1 ff.: Text, 0x450328 ff.: Tabelle):
//! - ZCN Typ 11 (Text/Textbox): STL = Text (TStringList, Zeilen mit CR LF), danach **STR = Schriftname**, falls
//!   eigener Schriftname gesetzt (Feld 9 ist zur Laufzeit der Zeiger darauf). Feld 6 Byte 1 = Umbruch.
//! - ZCN Typ 12 (Bild): keine Unterblöcke. Feld 0–3 Rechteck, Feld 4 = Nummer des Projektbildes (BLD Feld 5),
//!   Feld 5 Byte 0 = Rahmen, Byte 1 = Ausrichtung (0 Zentriert, 1 Eingepasst, 2 Gestreckt), Feld 15 Bit 16 Fixiert.
//! - ZCN Typ 13 (Tabelle): STR = Zellen (je 0-terminiert). Feld 4 Spalten, 5 Zeilen, 6 "Ab", Feld 9 Byte 0
//!   Überschrift, Byte 1 Autobreite, Byte 2 Art (0 Benutzerdaten, 1 Inhaltsverzeichnis).
//! - OBJ (0x40 Byte, Text im TIT): Feld 0/1 eigene Kennung (Delphi-Zeitstempel), Feld 4 Wort 0 = 0xFFFF,
//!   Byte 2 Art (0 Sammelschiene, 1 Verweis, 2 Klemmstelle), Byte 3 = 1 Verweis auf andere Anlage;
//!   Feld 5 Byte 0 (1 Sammelschiene, 2 Verweis). Sammelschiene: Feld 8/9 Kennung der Schiene (SMS Feld 8/9).
//! - VBG: Feld 2/3 und 6/7 Kennungen der verbundenen Objekte, 10/11 eigene Kennung, 13 Leitung (LTG Feld 9).
//! - BLD: Name 0–20, Feld 5 Nummer; JPG mit den JPEG-Daten, über 65535 Byte geteilt (siehe `eprfile`).

use crate::eprfile::{cstr_bytes, encode_cp1252, put_str, Block, EprFile};
use crate::model::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Kennung der Anlage im Blatt (BLT Feld 20/21).
pub const BLT_ANL_AT: usize = 0x50;

fn to01(v: f32) -> i32 {
    (v * 10.0).round() as i32
}
fn mm(v: i32) -> f32 {
    v as f32 / 10.0
}

/// Für den laufenden Speichervorgang vergebene Kennungen und neue Projektlisten-Einträge.
#[derive(Default)]
struct Plan {
    /// Objekt-Id → Kennung (Delphi-Zeitstempel als Bits) für neue OBJ/VBG
    kennung: HashMap<u64, u64>,
    /// Bildpfad (Datei) → Nummer des Projektbildes
    bild_nr: HashMap<String, i32>,
    /// neue Projektbilder: Name, Nummer, JPEG-Daten
    neue_bilder: Vec<(String, i32, Vec<u8>)>,
    /// (Anlage, Schienenname) → Kennung der Schiene
    sms: HashMap<(usize, String), u64>,
    /// neue Schienen: Anlage, Name, Kennung
    neue_sms: Vec<(usize, String, u64)>,
    /// Kennungen neuer Anlagen (ANL Byte 0x88) je Anlagenindex
    anlagen: HashMap<usize, u64>,
    /// Kennungen der Leitungen (LTG Feld 9) der Grundlage, für Leitungen ohne eigene Kennung
    ltg: Vec<i32>,
}

thread_local! {
    static PLAN: RefCell<Plan> = RefCell::new(Plan::default());
}

/// Jetzt als Delphi-Datum (Tage seit 30.12.1899) – Kennungen des Originals sind Erstellungszeitpunkte.
fn delphi_now() -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let now = chrono::Local::now().naive_local();
    (now - base).num_milliseconds() as f64 / 86_400_000.0
}

fn id_at(b: &Block, at: usize) -> u64 {
    b.data.get(at..at + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0)
}

fn txt(d: &[u8]) -> String {
    crate::symbols::cstr(d)
}

/// Name einer Schiene (SMS): Kürzel im TIT, sonst Name ab Byte 0.
fn sms_name(s: &Block) -> String {
    s.kid("TIT").map(|t| txt(&t.data)).unwrap_or_else(|| txt(&s.data[..s.data.len().min(20)]))
}

/// JPEG-Daten eines Bildes aus einer Datei (andere Formate werden umgewandelt).
fn jpeg_aus_datei(path: &str) -> Option<Vec<u8>> {
    let d = std::fs::read(path).ok()?;
    if d.starts_with(&[0xff, 0xd8]) {
        return Some(d);
    }
    let img = image::load_from_memory(&d).ok()?.to_rgb8();
    let mut out = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90).encode_image(&img).ok()?;
    Some(out)
}

/// Vor dem Schreiben: Kennungen für neue Verweise, Sammelschienen und Verbindungen vergeben, neue Projektbilder
/// und Schienen planen. `base` = Originaldatei bzw. Vorlage.
pub fn vorbereiten(project: &Project, base: &EprFile) {
    let mut plan = Plan::default();
    let mut stamp = delphi_now();
    let mut next = || {
        stamp += 1.0 / 86_400_000.0;
        stamp.to_bits()
    };
    for (ai, a) in project.anlagen.iter().enumerate() {
        if a.raw.is_none() {
            plan.anlagen.insert(ai, next());
        }
    }
    plan.ltg = base.root("PRJ").map(|p| p.kids_of("LTG").map(|l| l.i32(9)).collect()).unwrap_or_default();
    for o in project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        if o.raw.is_none() && matches!(o.kind, Kind::Verweis { .. } | Kind::Busbar { .. } | Kind::Wire { .. } | Kind::Symbol { .. } | Kind::Spiegel { .. }) {
            plan.kennung.insert(o.id, next());
        }
    }
    // Projektbilder: fehlende aus der Projektliste und Bilddateien neuer Bildobjekte
    let prj = base.root("PRJ");
    let vorhanden: Vec<i32> = if project.raw.is_some() { prj.map(|p| p.kids_of("BLD").map(|b| b.i32(5)).collect()).unwrap_or_default() } else { vec![] };
    let mut max_nr = vorhanden.iter().copied().chain(project.bilder.iter().map(|b| b.nr)).max().unwrap_or(0);
    for b in &project.bilder {
        if !vorhanden.contains(&b.nr) && !b.jpeg.is_empty() {
            plan.neue_bilder.push((b.name.clone(), b.nr, b.jpeg.clone()));
        }
    }
    for o in project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        let Kind::Image { path, .. } = &o.kind else { continue };
        if path.is_empty() || path.starts_with("bld:") || plan.bild_nr.contains_key(path) {
            continue;
        }
        if let Some(jpeg) = jpeg_aus_datei(path) {
            max_nr += 1;
            let name: String = std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default().chars().take(19).collect();
            plan.neue_bilder.push((name, max_nr, jpeg));
            plan.bild_nr.insert(path.clone(), max_nr);
        }
    }
    // Sammelschienen: Kennung der Schiene aus der Anlage (nach Kennung oder Name), sonst neue Schiene
    for (ai, a) in project.anlagen.iter().enumerate() {
        let smss: Vec<(u64, String)> = a.raw.as_deref().map(|(b, _)| b.kids_of("SMS").map(|s| (id_at(s, 32), sms_name(s))).collect()).unwrap_or_default();
        for o in a.sheets.iter().flat_map(|s| &s.objects) {
            let Kind::Busbar { name, sms, .. } = &o.kind else { continue };
            if o.raw.is_some() || plan.sms.contains_key(&(ai, name.clone())) {
                continue;
            }
            let id = smss.iter().find(|(id, _)| *sms != 0 && id == sms).or_else(|| smss.iter().find(|(_, n)| n == name)).map(|(id, _)| *id);
            let id = id.unwrap_or_else(|| {
                let id = next();
                plan.neue_sms.push((ai, name.clone(), id));
                id
            });
            plan.sms.insert((ai, name.clone()), id);
        }
    }
    PLAN.with(|p| *p.borrow_mut() = plan);
}

/// Kennung eines Objekts im Originalformat (OBJ Feld 0/1 bzw. VBG Feld 10/11): aus dem Originalblock oder die
/// beim Speichern vergebene. 0 = keine.
pub fn kennung(o: &Obj) -> u64 {
    if let Some(r) = o.raw.as_deref() {
        return if r.block.is("OBJ") {
            id_at(&r.block, 0)
        } else if r.block.is("VBG") {
            id_at(&r.block, 40)
        } else if r.block.is("VSY") {
            id_at(&r.block, 96)
        } else {
            0
        };
    }
    PLAN.with(|p| p.borrow().kennung.get(&o.id).copied().unwrap_or(0))
}

/// Kennung einer Anlage ohne Originalblock (beim Speichern vergeben, in `nachbereiten` nach ANL Byte 0x88 geschrieben).
pub fn anlage_kennung(project: &Project, a: &Anlage) -> u64 {
    let ai = project.anlagen.iter().position(|x| std::ptr::eq(x, a)).or_else(|| project.anlagen.iter().position(|x| x.name == a.name));
    PLAN.with(|p| ai.and_then(|i| p.borrow().anlagen.get(&i).copied()).unwrap_or(0))
}

/// Kennung der Leitung `i` (LTG Feld 9): eigene, sonst die der Grundlage an gleicher Stelle.
fn leitung_id(project: &Project, i: usize) -> i32 {
    match project.leitungen.get(i).map(|l| l.id) {
        Some(id) if id != 0 => id,
        _ => PLAN.with(|p| p.borrow().ltg.get(i).copied().unwrap_or(0)),
    }
}

/// Nummer des Projektbildes zu einem Bildpfad ("bld:<Nr>" oder Datei).
fn bild_nr(path: &str) -> i32 {
    if let Some(n) = path.strip_prefix("bld:").and_then(|n| n.trim().parse().ok()) {
        return n;
    }
    PLAN.with(|p| p.borrow().bild_nr.get(path).copied().unwrap_or(0))
}

/// Leerer ZCN-Block (0x50 Byte) eines Typs; Feld 19 = !Option wie bei allen Elementen.
fn zcn(ty: u8) -> Block {
    let mut b = Block::new("ZCN", vec![0; 0x50]);
    b.data[0x3c] = ty;
    b.set_i32(19, -1);
    b
}

fn set_rect(b: &mut Block, a: P, e: P) {
    let (mn, mx) = crate::canvas::norm_rect(a, e);
    b.set_i32(0, to01(mn.x));
    b.set_i32(1, to01(mn.y));
    b.set_i32(2, to01(mx.x));
    b.set_i32(3, to01(mx.y));
}

fn set_byte(b: &mut Block, at: usize, v: u8) {
    if b.data.len() <= at {
        b.data.resize(at + 1, 0);
    }
    b.data[at] = v;
}

/// Text eines TStringList-Blocks (STL): Zeilen mit CR LF, wie das Original auch nach der letzten Zeile.
fn stl_bytes(text: &str) -> Vec<u8> {
    let mut s = text.replace("\r\n", "\n").replace('\n', "\r\n");
    if !s.ends_with("\r\n") {
        s.push_str("\r\n");
    }
    cstr_bytes(&s)
}

fn set_kid(b: &mut Block, tag: &str, data: Vec<u8>, after: Option<&str>) {
    if let Some(k) = b.kids.iter_mut().find(|k| k.is(tag)) {
        k.data = data;
        return;
    }
    let at = after.and_then(|t| b.kids.iter().position(|k| k.is(t)).map(|i| i + 1)).unwrap_or(b.kids.len());
    b.kids.insert(at, Block::new(tag, data));
}

/// Zellen einer Tabelle als STR-Inhalt wie in W2: durch 0 getrennt, **ohne** abschließende 0, leere Zellen am Ende
/// entfallen (höchstens Zeilen × Spalten). Mit abschließender 0 zeigte das Original die Tabelle nicht an
/// (Gegenprobe 21.11.2026: eine Zelle zu viel).
fn tabellen_str(rows: usize, cols: usize, cells: &[String]) -> Vec<u8> {
    let mut used: Vec<&String> = cells.iter().take(rows.max(1) * cols.max(1)).collect();
    while used.last().is_some_and(|c| c.is_empty()) {
        used.pop();
    }
    let mut d = vec![];
    for (i, c) in used.iter().enumerate() {
        if i > 0 {
            d.push(0);
        }
        d.extend(encode_cp1252(&c.replace("\r\n", "\n").replace('\n', "\r\n")));
    }
    d
}

/// Art im Bilddialog des Originals (0 Zentriert, 1 Eingepasst, 2 Gestreckt) ↔ `Kind::Image::mode`.
fn bild_art(mode: u8) -> u8 {
    match mode {
        1 => 0,
        2 => 1,
        _ => 2,
    }
}

/// Werte eines Bildes oder einer Tabelle in einen ZCN-Block schreiben (neu oder geändert).
fn schreibe_zcn(b: &mut Block, k: &Kind) {
    match k {
        Kind::Image { a, b: e, path, mode, frame } => {
            set_rect(b, *a, *e);
            b.set_i32(4, bild_nr(path));
            set_byte(b, 0x14, *frame as u8);
            set_byte(b, 0x15, bild_art(*mode));
        }
        Kind::Table { a, b: e, rows, cols, cells, header, fixed_w, art, ab } => {
            set_rect(b, *a, *e);
            b.set_i32(4, (*cols).max(1) as i32);
            b.set_i32(5, (*rows).max(1) as i32);
            b.set_i32(6, *ab as i32);
            set_byte(b, 0x24, *header as u8);
            set_byte(b, 0x25, !*fixed_w as u8);
            set_byte(b, 0x26, *art);
            // Das Original schreibt den STR nur bei nicht leerem Inhalt (Speicherroutine 0x424db5); ein leerer STR
            // ließ es die Tabelle als fehlerhaft melden (Gegenprobe 21.11.2026, Tabellen ohne Zellen)
            let d = tabellen_str(*rows, *cols, cells);
            if d.is_empty() {
                b.kids.retain(|k| !k.is("STR"));
            } else {
                set_kid(b, "STR", d, None);
            }
        }
        _ => {}
    }
}

fn set_fixed(b: &mut Block, fixed: bool) {
    let v = b.i32(15);
    b.set_i32(15, if fixed { v | 0x1_0000 } else { v & !0x1_0000 });
}

/// Neuer Block für Tabelle, Bild, Verweis und Sammelschiene (`None` für andere Arten).
pub fn new_block(project: &Project, o: &Obj) -> Option<Block> {
    match &o.kind {
        Kind::Image { .. } | Kind::Table { .. } => {
            let ty = match o.kind {
                Kind::Image { .. } => 12,
                _ => 13,
            };
            let mut b = zcn(ty);
            schreibe_zcn(&mut b, &o.kind);
            b.set_i32(16, o.pen as i32);
            set_fixed(&mut b, o.fixed);
            Some(b)
        }
        Kind::Verweis { pos, text_pos, text, dir, vertical, spitze, ziel } => {
            let mut b = Block::new("OBJ", vec![0; 0x40]);
            b.data[0..8].copy_from_slice(&kennung(o).to_le_bytes());
            b.set_i32(2, to01(text_pos.x) - to01(pos.x));
            b.set_i32(3, to01(text_pos.y) - to01(pos.y));
            // Byte 3 = Text hochkant (siehe `crate::epr::verweis`)
            b.set_i32(4, crate::epr_write::verweis_feld4(0x1_ffff, *vertical));
            b.set_i32(5, 2);
            b.set_i32(6, to01(pos.x));
            b.set_i32(7, to01(pos.y));
            b.set_i32(14, crate::epr_write::verweis_winkel(*dir, *spitze));
            b.set_i32(15, *spitze as i32);
            crate::epr_write::write_ziel(project, &mut b, ziel.as_ref());
            if !text.is_empty() {
                b.kids.push(Block::new("TIT", cstr_bytes(text)));
            }
            Some(b)
        }
        Kind::Busbar { a, x2, name, label, lage, versatz, .. } => {
            let ai = project.find_obj(o.id).map(|(r, _)| r.anlage).unwrap_or(0);
            let sms = PLAN.with(|p| p.borrow().sms.get(&(ai, name.clone())).copied().unwrap_or(0));
            let mut b = Block::new("OBJ", vec![0; 0x40]);
            b.data[0..8].copy_from_slice(&kennung(o).to_le_bytes());
            b.set_i32(2, to01(label.x));
            b.set_i32(3, to01(label.y));
            b.set_i32(4, 0xffff);
            b.set_i32(5, 1);
            b.set_i32(6, to01(a.x));
            b.set_i32(7, to01(a.y));
            b.data[32..40].copy_from_slice(&sms.to_le_bytes());
            b.set_i32(10, to01(*x2));
            // Feld 15 Byte 0 = Lage (Bit 0 unten, Bit 1 fest); die Strichart steht beim Stift der Schiene (SMS)
            b.set_i32(15, *lage as i32);
            b.data[0x2c..0x2e].copy_from_slice(&((versatz * 10.0).round() as i16).to_le_bytes());
            Some(b)
        }
        Kind::Spiegel { .. } => spiegel_block(project, o),
        _ => None,
    }
}

/// Eigenschaften, die die Vorlageblöcke aus `epr_write` für neue Objekte noch nicht tragen, nachtragen.
pub fn nachfuehren(project: &Project, o: &Obj, b: &mut Block) {
    if b.is("ZCN") {
        set_fixed(b, o.fixed);
        if let Kind::Text { text, style, .. } = &o.kind {
            if b.data[0x3c] == 11 {
                set_kid(b, "STL", stl_bytes(text), None);
                set_byte(b, 0x19, style.wrap as u8);
                schriftname(b, &style.font);
            }
        }
    }
    if b.is("VBG") {
        if let Kind::Wire { leitung, bez, .. } = &o.kind {
            // keine Verbindung zu Objekten der Vorlage: Kennungen der Enden leeren, eigene Kennung setzen
            for i in [2, 3, 6, 7] {
                b.set_i32(i, 0);
            }
            b.data[40..48].copy_from_slice(&kennung(o).to_le_bytes());
            b.set_i32(13, leitung_id(project, *leitung));
            bezeichnungen(b, bez);
            verweis_enden(project, o, b);
        }
    }
    if b.is("VSY") {
        if let Kind::Symbol { ext, .. } = &o.kind {
            // eigene Kennung (VSY Byte 96; Spule eines Kontaktspiegels, Eigentümer), statt der der Vorlage
            let k = kennung(o);
            if k != 0 && b.data.len() >= 104 {
                b.data[96..104].copy_from_slice(&k.to_le_bytes());
            }
            b.set_i32(38, ext.counter);
            // ALT/SPA neu aufbauen (ersetzt vorhandene, z. B. schon vom Patch geschriebene – nie doppelt)
            crate::parameter::sync_vsy_kids(b, &ext.params, &ext.texts);
            // Eigentümer (OWR) neu, auch ein OWR der Vorlage entfernen
            crate::eigentuemer::owr_schreiben(project, b, ext.eigentuemer);
        }
    }
}

/// VBG-Feld der Kennung am Anfang (Feld 2/3, Anschlussnummer Feld 4) bzw. Ende (Feld 6/7, Anschluss Feld 8).
pub const VBG_ENDEN: [usize; 2] = [2, 6];

/// Leitungsenden an der Pfeilspitze eines Verweises: Kennung des Verweises (OBJ Feld 0/1) in VBG Feld 2/3 bzw.
/// 6/7, Anschluss 0 in Feld 4 bzw. 8 – so verweisen die Verbindungen in W2 auf Klemmstellen und Sammelschienen
/// (ebenfalls OBJ-Blöcke). Ein Ende, das einen Verweis dieses Blattes verlassen hat, verliert die Kennung.
/// Ebenso Enden an Klemmstellen: die Leitung endet dort (wie alle 111 Klemmstellen in W2) am Kreis, 0,8 mm vom
/// Mittelpunkt (klemme_leitung.rs).
fn verweis_enden(project: &Project, o: &Obj, b: &mut Block) {
    let Kind::Wire { pts, .. } = &o.kind else { return };
    let Some(sheet) = project.find_obj(o.id).and_then(|(r, _)| project.sheet(r)) else { return };
    let verweise: Vec<(P, u64, f32)> = sheet
        .objects
        .iter()
        .filter_map(|v| match &v.kind {
            Kind::Verweis { pos, .. } => Some((*pos, kennung(v), 0.05)),
            Kind::Terminal { pos, .. } => Some((*pos, crate::klemmen::uid(v), crate::klemme_leitung::KLEMME_R + 0.05)),
            _ => None,
        })
        .collect();
    for (ende, feld) in [pts.first(), pts.last()].into_iter().zip(VBG_ENDEN) {
        let Some(p) = ende else { continue };
        let at = feld * 4;
        if b.data.len() < at + 12 {
            continue;
        }
        match verweise.iter().find(|(q, k, r)| *k != 0 && q.dist(*p) <= *r) {
            Some((_, k, _)) => {
                b.data[at..at + 8].copy_from_slice(&k.to_le_bytes());
                b.set_i32(feld + 2, 0);
            }
            None if verweise.iter().any(|(_, k, _)| *k != 0 && *k == id_at(b, at)) => {
                b.data[at..at + 8].fill(0);
                b.set_i32(feld + 2, 0);
            }
            None => {}
        }
    }
}

/// Eigener Schriftname eines Textes: STR nach dem STL (leer = Projektschrift, kein STR).
fn schriftname(b: &mut Block, font: &str) {
    if font.trim().is_empty() {
        b.kids.retain(|k| !k.is("STR"));
    } else {
        set_kid(b, "STR", cstr_bytes(font.trim()), Some("STL"));
    }
}

/// Verbindungsbezeichnungen (BZD, 44 Byte): Feld 3/4 und 5/6 Punkt, 0/1 Textversatz (hier 0), 9/10 wie im Original
/// meist (80, 2), STR = Text.
fn bezeichnungen(b: &mut Block, bez: &[(P, String)]) {
    b.kids.retain(|k| !k.is("BZD"));
    for (p, t) in bez {
        let mut z = Block::new("BZD", vec![0; 44]);
        for i in [3, 5] {
            z.set_i32(i, to01(p.x));
            z.set_i32(i + 1, to01(p.y));
        }
        z.set_i32(9, 80);
        z.set_i32(10, 2);
        z.kids.push(Block::new("STR", cstr_bytes(t)));
        b.kids.push(z);
    }
}

/// Kontaktspiegel (OBJ Art 5 mit STB-Zeilen, siehe `kontaktspiegel`): Kennungen von Spiegel, Spule und Kontakten,
/// Blatt (BLT Byte 72) und Anlage (ANL Byte 0x88) der Kontakte.
pub fn spiegel_block(project: &Project, o: &Obj) -> Option<Block> {
    crate::kontaktspiegel::block(project, o, kennung(o), &kennung, &|r| orte(project, r))
}

/// Kennungen von Blatt (BLT Byte 72) und Anlage (ANL Byte 0x88) eines Blattes – für STB und OWR.
pub fn orte(project: &Project, r: SheetRef) -> (u64, u64) {
    let Some(a) = project.anlagen.get(r.anlage) else { return (0, 0) };
    let anlage = a.raw.as_deref().map(|(b, _)| id_at(b, crate::epr::ANL_ID_AT)).unwrap_or_else(|| anlage_kennung(project, a));
    let blatt = a.sheets.get(r.sheet).map(|s| s.raw.as_deref().map(|x| id_at(&x.block, crate::epr::BLT_ID_AT)).unwrap_or_else(|| crate::epr_write::new_sheet_id(s).to_bits())).unwrap_or(0);
    (blatt, anlage)
}

/// Zusatz der Blattarten Deckblatt, Querverweise und Stückliste (BLT Feld 32 Byte 1; beim Klemmplan steht dort
/// die Klemmleiste, die `klemmen::patch_blt` schreibt). Nur bei Änderung, damit unveränderte Blätter gleich bleiben.
pub fn patch_zusatz(b: &mut Block, s: &Sheet) {
    // neues Blatt mit ausdrücklich gewählter Art 0 (Deckblatt): `klemmen::patch_blt` lässt hier die Art der
    // Vorlage stehen (Feld 22 Byte 0)
    if s.raw.is_none() && s.art_fest && b.data.len() > 88 && b.data[88] != s.art {
        b.data[88] = s.art;
    }
    if matches!(s.blattart(), 0 | 2 | 6) && b.data.len() > 0x81 && b.data[0x81] != s.zusatz {
        b.data[0x81] = s.zusatz;
    }
}

/// Geänderte Tabelle/Bild bzw. Schriftname, Umbruch und Verbindungsdaten in den Originalblock übernehmen.
pub fn patch_more(project: &Project, o: &Obj, old: &Kind, b: &mut Block) {
    if b.is("ZCN") {
        match (&o.kind, b.data.get(0x3c).copied()) {
            (Kind::Image { .. }, Some(12)) | (Kind::Table { .. }, Some(13)) => schreibe_zcn(b, &o.kind),
            (Kind::Text { style, .. }, Some(11)) => {
                let alt = if let Kind::Text { style, .. } = old { Some(style) } else { None };
                if alt.map(|s| s.font != style.font) != Some(false) {
                    schriftname(b, &style.font);
                }
                if alt.map(|s| s.wrap != style.wrap) != Some(false) {
                    set_byte(b, 0x19, style.wrap as u8);
                }
            }
            _ => {}
        }
    }
    if b.is("VBG") {
        if let (Kind::Wire { leitung, bez, pts, .. }, Kind::Wire { leitung: ol, bez: ob, pts: op, .. }) = (&o.kind, old) {
            if leitung != ol {
                b.set_i32(13, leitung_id(project, *leitung));
            }
            if bez != ob {
                bezeichnungen(b, bez);
            }
            if (pts.first(), pts.last()) != (op.first(), op.last()) {
                verweis_enden(project, o, b);
            }
        }
    }
    // Eigentümer (OWR) nur bei Änderung, damit ein OWR auf ein nicht gefundenes Symbol erhalten bleibt
    if let (Kind::Symbol { ext, .. }, Kind::Symbol { ext: alt, .. }) = (&o.kind, old) {
        if b.is("VSY") && ext.eigentuemer != alt.eigentuemer {
            crate::eigentuemer::owr_schreiben(project, b, ext.eigentuemer);
        }
    }
}

/// Nach dem Aufbau des Projektblocks: neue Projektbilder (BLD › JPG) und Schienen (SMS) einfügen.
pub fn nachbereiten(prj: &mut Block) {
    let plan = PLAN.with(|p| std::mem::take(&mut *p.borrow_mut()));
    // Projektbilder nach dem letzten vorhandenen, sonst vor der ersten Anlage
    let mut at = prj.kids.iter().rposition(|k| k.is("BLD")).map(|i| i + 1).or_else(|| prj.kids.iter().position(|k| k.is("ANL"))).unwrap_or(prj.kids.len());
    for (name, nr, jpeg) in &plan.neue_bilder {
        prj.kids.insert(at, bld_block(name, *nr, jpeg));
        at += 1;
    }
    // Schienen je Anlage (Anlagenblöcke stehen in der Reihenfolge des Projekts)
    let anl: Vec<usize> = prj.kids.iter().enumerate().filter(|(_, k)| k.is("ANL")).map(|(i, _)| i).collect();
    // Kennungen neuer Anlagen (sonst trügen alle die der Vorlage)
    for (ai, id) in &plan.anlagen {
        if let Some(a) = anl.get(*ai).map(|&i| &mut prj.kids[i]) {
            let at = crate::epr::ANL_ID_AT;
            if a.data.len() >= at + 8 {
                a.data[at..at + 8].copy_from_slice(&id.to_le_bytes());
            }
        }
    }
    // Jedes Blatt trägt die Kennung seiner Anlage (BLT Byte 0x50, in W2 ausnahmslos gleich ANL Byte 0x88).
    // Neue Blätter und Blätter neuer Anlagen trugen sonst die Anlagenkennung der Vorlage; in solchen Projekten
    // zeichnete das Original keine Tabellen (Gegenprobe 21.11.2026).
    for &i in &anl {
        let a = &mut prj.kids[i];
        let at = crate::epr::ANL_ID_AT;
        let Some(id) = a.data.get(at..at + 8).map(|d| d.to_vec()) else { continue };
        for s in a.kids.iter_mut().filter(|k| k.is("BLT") && k.data.len() >= BLT_ANL_AT + 8) {
            if s.data[BLT_ANL_AT..BLT_ANL_AT + 8] != id[..] {
                s.data[BLT_ANL_AT..BLT_ANL_AT + 8].copy_from_slice(&id);
            }
        }
    }
    for (ai, name, id) in &plan.neue_sms {
        let Some(&i) = anl.get(*ai) else { continue };
        let a = &mut prj.kids[i];
        // hat inzwischen ein anderer Teil des Speicherns die Schiene angelegt, deren Kennung übernehmen
        let found = a.kids_of("SMS").find(|s| sms_name(s) == *name).map(|s| id_at(s, 32));
        if let Some(real) = found {
            for o in a.kids.iter_mut().filter(|k| k.is("BLT")).flat_map(|s| s.kids.iter_mut()) {
                if o.is("OBJ") && o.i32(4) == 0xffff && id_at(o, 32) == *id {
                    o.data[32..40].copy_from_slice(&real.to_le_bytes());
                }
            }
            continue;
        }
        let pos = a.kids.iter().rposition(|k| k.is("SMS")).map(|j| j + 1).or_else(|| a.kids.iter().position(|k| k.is("BLT"))).unwrap_or(a.kids.len());
        a.kids.insert(pos, sms_block(name, *id));
    }
}

/// Schiene (SMS, 76 Byte): Name ab Byte 0, Feld 7 = 0xFFFF, 8/9 Kennung, 12 = 1 (wie alle Schienen in W2), TIT = Kürzel.
fn sms_block(name: &str, id: u64) -> Block {
    let mut s = Block::new("SMS", vec![0; 76]);
    put_str(&mut s.data, 0, 28, name);
    s.set_i32(7, 0xffff);
    s.data[32..40].copy_from_slice(&id.to_le_bytes());
    s.set_i32(12, 1);
    s.kids.push(Block::new("TIT", cstr_bytes(name)));
    s
}

/// Projektbild (BLD, 28 Byte: Name 0–20, Feld 5 Nummer) mit JPG; über 65534 Byte in Teile zu 65535 Byte geteilt,
/// Teilkopf = Nr., Anzahl, u16 Länge des letzten Teils.
pub fn bld_block(name: &str, nr: i32, jpeg: &[u8]) -> Block {
    let mut b = Block::new("BLD", vec![0; 28]);
    put_str(&mut b.data, 0, 20, name);
    b.set_i32(5, nr);
    let mut j = Block::new("JPG", vec![]);
    if jpeg.len() < 0xffff {
        j.data = jpeg.to_vec();
    } else {
        let teile: Vec<&[u8]> = jpeg.chunks(0xffff).collect();
        let n = teile.len().min(255) as u8;
        let last = (teile.last().map(|t| t.len()).unwrap_or(0) as u16).to_le_bytes();
        for (i, t) in teile.iter().enumerate().take(255) {
            let head = [i as u8 + 1, n, last[0], last[1]];
            if i == 0 {
                j.data = t.to_vec();
                j.chunk = Some(head);
            } else {
                j.parts.push((head, t.to_vec()));
            }
        }
    }
    b.kids.push(j);
    b
}

// ---------------------------------------------------------------------------------------------------------------
// Lesen (aus `epr::drawing`)

/// Bild auf dem Blatt (ZCN Typ 12) → `Kind::Image` mit Pfad "bld:<Nr>".
pub fn bild_lesen(k: &Block) -> Kind {
    let (a, b) = crate::canvas::norm_rect(P::new(mm(k.i32(0)), mm(k.i32(1))), P::new(mm(k.i32(2)), mm(k.i32(3))));
    let mode = match k.data.get(0x15).copied().unwrap_or(2) {
        0 => 1,
        1 => 2,
        _ => 0,
    };
    Kind::Image { a, b, path: format!("bld:{}", k.i32(4)), mode, frame: k.data.get(0x14).copied().unwrap_or(0) != 0 }
}

/// Überschrift und gleiche Spaltenbreiten einer Tabelle (Feld 9 Byte 0/1).
pub fn tabelle_flags(k: &Block) -> (bool, bool) {
    (k.data.get(0x24).copied().unwrap_or(0) != 0, k.data.get(0x25).copied().unwrap_or(1) == 0)
}

/// Umbruch (Feld 6 Byte 1) und eigener Schriftname (STR nach STL) eines Textes (ZCN Typ 11).
pub fn text_extras(k: &Block) -> (bool, String) {
    let font = if k.kid("STL").is_some() { k.kid("STR").map(|s| txt(&s.data)).unwrap_or_default() } else { String::new() };
    (k.data.get(0x19).copied().unwrap_or(0) != 0, font)
}
