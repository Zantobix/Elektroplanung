//! Symbolbibliothek: liest die Datenbank des Originals (Datenbank\Gruppen.tfi und Symbole.tfi)
//! und hat eingebaute Ersatzsymbole, falls die Datenbank nicht gefunden wird.
//! Format (selbst ermittelt): siehe MECHANICS.md, Abschnitt „Symbolbibliothek“.

use crate::model::{Align, P};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Prim {
    Line(P, P, u8),
    Poly(Vec<P>, u8),
    Bezier(P, P, P, P, u8),
    Ellipse(P, P, u8),
    /// Mittelpunkt, Radius, Startwinkel, Endwinkel (Grad)
    Arc(P, f32, f32, f32),
    /// Mittelpunkt, Text, Schriftgröße (mm)
    Text(P, String, f32),
    /// Rechteck, Text, Ausrichtung (Größe passt sich dem Rechteck an)
    TextBox(P, P, String, Align),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymDef {
    pub key: String,
    pub name: String,
    /// Gruppenpfad, z. B. ["Signaleinrichtung", "Lampen"]
    pub group: Vec<String>,
    /// 200 = Benutzerbauteile, 300 = Standardbauteile
    pub haupt: i64,
    pub fav: bool,
    /// BMK-Vorlage, z. B. "-E1+%P" oder "%SK%R.%C"
    pub bmk: String,
    pub prims: Vec<Prim>,
    pub pins: Vec<P>,
    /// Position der Betriebsmittelkennzeichnung (rechtsbündig), falls angegeben
    pub title: Option<P>,
    /// Sichtbarkeit je Element: (Option, Modus) – Modus 0 nur diese Option, 1 ab dieser, 2 bis zu dieser
    #[serde(default)]
    pub vis: Vec<(u16, u16)>,
    #[serde(default)]
    pub pin_vis: Vec<(u16, u16)>,
    #[serde(default)]
    pub options: Vec<String>,
    /// Zusatzoption je Element (Feld 0x48 im Element): 0 = immer, n = nur wenn Bit n−1 der
    /// Zusatzoptionen der Instanz gesetzt ist (OPT-Blöcke, z. B. "Ports", "SPD" bei Steckdosen)
    #[serde(default)]
    pub flag: Vec<u32>,
    /// Namen der Zusatzoptionen (OPT), Index = Bit
    #[serde(default)]
    pub flag_names: Vec<String>,
    /// Textnummer je Element (nur Textfelder, sonst 0) – für Alternativtexte
    #[serde(default)]
    pub text_idx: Vec<u32>,
    /// Alternativtext-Sätze (ALT): je Satz Liste (Textnummer, Ersatztext)
    #[serde(default)]
    pub alt_sets: Vec<Vec<(u32, String)>>,
    /// Zeichendaten der Original-Bibliothek ("VerX": Kopf, Elemente je 0x50 Byte, Texte, Optionen) –
    /// damit neue Symbole im Originalformat (.epr) geschrieben werden können
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw: Vec<u8>,
    /// Freigaben (Querverweise, Sicherungsübersicht) – Vorgabe für neu gesetzte Symbole
    #[serde(default = "frg_default")]
    pub frg: (bool, bool),
    /// Parameter (GRD › PRA: Feld 0 = Nummer, STR 1 = Name, STR 2 = Vorgabewert), in Texten als %<Name>
    #[serde(default)]
    pub params: Vec<(u32, String, String)>,
    /// Lage des Wertes (Element Typ 10, wie Typ 9 für den Titel)
    #[serde(default)]
    pub value_pos: Option<P>,
    /// Ausrichtung je Textelement (Element Feld 8, parallel zu `prims`): 0 = bleibt waagerecht,
    /// 1 = hochkant, 2 = dreht mit dem Symbol (Original-Ausdruck Seite 1: "M" in "Motorsymbole" gedreht,
    /// Seite 7: "M" im Symbol "Motor" waagerecht)
    #[serde(default)]
    pub text_mode: Vec<u8>,
    /// Gefüllte Flächen (Element Feld 10 ≠ 0 bei Viereck, Dreieck, Ellipse; parallel zu `prims`),
    /// z. B. der Verbindungspunkt im Symbol "Verbindung"
    #[serde(default)]
    pub fill: Vec<bool>,
}

fn frg_default() -> (bool, bool) {
    (true, false)
}

/// Bezeichnung eines Alternativtext-Satzes wie im Original-Kontextmenü: 'a';'b';'c'
pub fn alt_label(set: &[(u32, String)]) -> String {
    set.iter().map(|(_, t)| format!("'{t}'")).collect::<Vec<_>>().join(";")
}

/// Ist ein Element mit Zusatzoption `f` bei den Instanz-Zusatzoptionen `flags` sichtbar?
pub fn flag_visible(flags: u32, f: Option<&u32>) -> bool {
    match f.copied().unwrap_or(0) {
        0 => true,
        n if n <= 32 => flags & (1 << (n - 1)) != 0,
        _ => false,
    }
}

/// Ist ein Element mit (Option, Modus) in der gewählten Option sichtbar?
pub fn visible(sel: u16, v: Option<&(u16, u16)>) -> bool {
    match v {
        None => true,
        Some((opt, mode)) => match mode {
            0 => sel == *opt,
            1 => sel >= *opt,
            2 => sel <= *opt,
            _ => false,
        },
    }
}

/// Symboltext mit Parametern und Auswahltexten:
/// `%<Name>` = Parameterwert der Instanz (VSY › SPA) bzw. Vorgabe (GRD › PRA),
/// `%("Titel";a;b;…)` = Text je Bild/Option des Symbols (0 = a, 1 = b …; z. B. %("NO/NC";11;13)).
pub fn expand_text(s: &str, def: &SymDef, inst: &[(u32, String)], option: u16) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let mut out = s.to_string();
    for (nr, name, dflt) in &def.params {
        let v = inst.iter().find(|(n, _)| n == nr).map(|(_, v)| v.as_str()).unwrap_or(dflt);
        out = out.replace(&format!("%<{name}>"), v);
    }
    while let Some(i) = out.find("%(") {
        let Some(j) = out[i..].find(')') else { break };
        let inner = &out[i + 2..i + j];
        let parts: Vec<&str> = inner.split(';').collect();
        let pick = parts.get(1 + option as usize).or(parts.last()).copied().unwrap_or("").trim_matches('"').to_string();
        out.replace_range(i..i + j + 1, &pick);
    }
    out
}

pub fn find<'a>(lib: &'a [SymDef], key: &str) -> Option<&'a SymDef> {
    lib.iter().find(|s| s.key == key)
}

/// Lokaler Umriss (min, max) eines Symbols in der Grundvariante.
pub fn bounds(def: &SymDef) -> (P, P) {
    bounds_opt(def, 0)
}

/// Lokaler Umriss (min, max) eines Symbols in einer Option.
pub fn bounds_opt(def: &SymDef, option: u16) -> (P, P) {
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    let mut add = |q: P| {
        mn.x = mn.x.min(q.x);
        mn.y = mn.y.min(q.y);
        mx.x = mx.x.max(q.x);
        mx.y = mx.y.max(q.y);
    };
    for (i, pr) in def.prims.iter().enumerate() {
        if !visible(option, def.vis.get(i)) {
            continue;
        }
        match pr {
            Prim::Line(a, b, _) | Prim::Ellipse(a, b, _) => {
                add(*a);
                add(*b)
            }
            Prim::Poly(v, _) => v.iter().for_each(|q| add(*q)),
            Prim::Bezier(a, b, c, d, _) => {
                for q in [a, b, c, d] {
                    add(*q)
                }
            }
            Prim::Arc(c, r, _, _) => {
                add(P::new(c.x - r, c.y - r));
                add(P::new(c.x + r, c.y + r))
            }
            Prim::Text(c, _, s) => {
                add(P::new(c.x - s * 0.5, c.y - s * 0.5));
                add(P::new(c.x + s * 0.5, c.y + s * 0.5))
            }
            Prim::TextBox(..) => {}
        }
    }
    for (i, q) in def.pins.iter().enumerate() {
        if visible(option, def.pin_vis.get(i)) {
            add(*q);
        }
    }
    if mn.x > mx.x {
        return (P::new(-2.0, -2.0), P::new(2.0, 2.0));
    }
    (mn, mx)
}

/// Angaben zum Auflösen einer gespeicherten BMK-Vorlage beim Anzeigen.
#[derive(Clone, Debug, Default)]
pub struct BmkCtx {
    pub sheet: u32,
    /// Ort des Blattes (BLT ab Byte 51, z. B. "W2") für %P / %Ort
    pub ort: String,
    /// Kurzform der Anlage für %A
    pub anlage: String,
}

/// BMK-Vorlage einer Instanz zur Anzeige auflösen (wie im Original-Ausdruck "-X1.%C+%P" → "-X1.1+W2"):
/// %S → "-", %R → Blattnummer, %C → Zähler (VSY Feld 38), %Ort/%P → Blattort, %A → Anlage (Kurzform).
pub fn display_bmk(tpl: &str, counter: i32, ctx: &BmkCtx) -> String {
    if !tpl.contains('%') {
        return tpl.to_string();
    }
    let mut s = tpl.replace("%Ort", &ctx.ort).replace("%P", &ctx.ort).replace("%A", &ctx.anlage);
    s = s.replace("%S", "-").replace("%R", &ctx.sheet.to_string());
    if counter > 0 {
        s = s.replace("%C", &counter.to_string());
    }
    s.trim().to_string()
}

/// BMK aus der Vorlage bilden: %S → "-", %R → Blattnummer, %C → Zähler, Ortsangaben (+%P, +%A, +%Ort) entfallen.
pub fn expand_bmk(tpl: &str, counter: u32, sheet: u32) -> String {
    let mut s = tpl.replace("+%Ort", "").replace("%Ort", "").replace("+%P", "").replace("%P", "").replace("+%A", "").replace("%A", "");
    s = s.replace("%S", "-").replace("%R", &sheet.to_string()).replace("%C", &counter.to_string());
    s.trim().to_string()
}

// ---------------------------------------------------------------- Datenbank lesen

fn u32_at(d: &[u8], p: usize) -> Option<u32> {
    d.get(p..p + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn i32_at(d: &[u8], p: usize) -> i32 {
    d.get(p..p + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
}
fn i64_at(d: &[u8], p: usize) -> i64 {
    d.get(p..p + 8).map(|b| i64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}
fn u16_at(d: &[u8], p: usize) -> u16 {
    d.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0)
}
/// Windows-1252-Text bis zum ersten Nullbyte.
pub fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|c| *c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| cp1252(c)).collect()
}
fn cp1252(c: u8) -> char {
    match c {
        0x80 => '€',
        0x84 => '„',
        0x93 => '“',
        0x94 => '”',
        0x96 => '–',
        0x97 => '—',
        _ => c as char,
    }
}

/// Ordner der Original-Datenbank suchen.
pub fn find_db_dir(extra: Option<&Path>) -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = vec![];
    if let Some(e) = extra {
        cands.push(e.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("Datenbank"));
            if let Some(p) = d.parent() {
                cands.push(p.join("Datenbank"));
            }
        }
    }
    if let Ok(c) = std::env::current_dir() {
        cands.push(c.join("Datenbank"));
    }
    // eigener Ort (z. B. unter Linux: EP_DATENBANK=/pfad/zu/Datenbank) und der Ablageordner des Nachbaus
    if let Ok(d) = std::env::var("EP_DATENBANK") {
        cands.insert(0, PathBuf::from(d));
    }
    cands.push(crate::config_dir().join("Datenbank"));
    cands.into_iter().find(|c| c.join("Symbole.tfi").is_file() && c.join("Gruppen.tfi").is_file())
}

/// Alle vorhandenen Datenbank-Ordner (Reserve, falls einer vom laufenden Original gesperrt ist).
pub fn db_candidates(extra: Option<&Path>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = vec![];
    if let Some(d) = find_db_dir(extra) {
        v.push(d);
    }
    v
}

pub fn load_db(dir: &Path) -> Result<Vec<SymDef>, String> {
    let g = std::fs::read(dir.join("Gruppen.tfi")).map_err(|e| format!("Gruppen.tfi: {e}"))?;
    let s = std::fs::read(dir.join("Symbole.tfi")).map_err(|e| format!("Symbole.tfi: {e}"))?;

    // Gruppen: [u32 len][01 Name(41)][Art][01 HauptID i64][01 OwnerID i64][01 ID i64]
    let mut groups: HashMap<i64, (String, i64)> = HashMap::new();
    let mut p = u32_at(&g, 0).ok_or("Gruppen.tfi zu kurz")? as usize;
    while let Some(n) = u32_at(&g, p) {
        let n = n as usize;
        if !(8..=4096).contains(&n) || p + n > g.len() {
            break;
        }
        let rec = &g[p + 4..p + n];
        if rec.len() >= 70 {
            let name = cstr(&rec[1..42]);
            let owner = i64_at(rec, 53);
            let id = i64_at(rec, 62);
            groups.insert(id, (name, owner));
        }
        p += n;
    }
    let group_path = |gid: i64| -> Vec<String> {
        let mut path = vec![];
        let mut cur = gid;
        let mut guard = 0;
        while let Some((name, owner)) = groups.get(&cur) {
            path.insert(0, name.clone());
            if *owner == 0 || guard > 16 {
                break;
            }
            cur = *owner;
            guard += 1;
        }
        if path.is_empty() {
            path.push("Sonstiges".into());
        }
        path
    };

    // Symbole: [u32 len][u32 len][01 ID(8)][01 Name(40)][01 HauptID][01 GruppeID][01 Geändert][01 Favorit]
    //          [u32 n][Symbolkopf][u32 n][01 "VerX" Zeichendaten]  (Name 40 Byte)
    let mut out = vec![];
    let mut p = u32_at(&s, 0).ok_or("Symbole.tfi zu kurz")? as usize;
    while let Some(n) = u32_at(&s, p) {
        let n = n as usize;
        if n < 100 || p + n > s.len() {
            break;
        }
        let rec = &s[p + 8..p + n];
        p += n;
        if rec.len() < 90 {
            continue;
        }
        let name = cstr(&rec[10..50]);
        let haupt = i64_at(rec, 51);
        let gid = i64_at(rec, 60);
        let fav = rec.get(78).copied().unwrap_or(0) == 1;
        let Some(hl) = u32_at(rec, 79) else { continue };
        let hl = hl as usize;
        let hdr = rec.get(83..83 + hl).unwrap_or(&[]);
        let bmk = if hdr.len() > 1 { cstr(&hdr[1..]) } else { String::new() };
        let off = 83 + hl;
        let Some(dl) = u32_at(rec, off) else { continue };
        let data = rec.get(off + 4..off + 4 + dl as usize).unwrap_or(&[]);
        if name.is_empty() || !(haupt == 200 || haupt == 300 || haupt == 100) || data.len() < 0x41 {
            continue;
        }
        let Some(el) = parse_drawing(&data[1..]) else { continue };
        let (alt_sets, flag_names) = parse_lib_extras(&data[1..]);
        out.push(SymDef {
            key: format!("db:{haupt}:{gid}:{name}"),
            name,
            group: group_path(gid),
            haupt,
            fav,
            bmk,
            prims: el.prims,
            pins: el.pins,
            title: el.title,
            vis: el.vis,
            pin_vis: el.pin_vis,
            flag: el.flag,
            flag_names,
            text_idx: el.text_idx,
            alt_sets,
            options: el.options,
            raw: data[1..].to_vec(),
            frg: (true, false),
            params: vec![],
            value_pos: el.value_pos,
            text_mode: el.text_mode,
            fill: el.fill,
        });
    }
    if out.is_empty() {
        return Err("Keine Symbole gefunden".into());
    }
    Ok(out)
}

pub struct Elements {
    pub prims: Vec<Prim>,
    pub vis: Vec<(u16, u16)>,
    pub pins: Vec<P>,
    pub pin_vis: Vec<(u16, u16)>,
    pub title: Option<P>,
    pub value_pos: Option<P>,
    pub options: Vec<String>,
    pub flag: Vec<u32>,
    pub text_idx: Vec<u32>,
    pub text_mode: Vec<u8>,
    pub fill: Vec<bool>,
}

/// Zeichendaten "Ver1"/"Ver2": Kopf 0x3c Byte, Elemente zu je 0x50 Byte, danach die Texttabelle und Optionsnamen.
fn parse_drawing(d: &[u8]) -> Option<Elements> {
    if d.len() < 0x3c || &d[0..3] != b"Ver" {
        return None;
    }
    let cnt = i32_at(d, 0x14).max(0) as usize;
    let base = 0x3c;
    if base + cnt * 0x50 > d.len() || cnt > 5000 {
        return None;
    }
    let tp = base + cnt * 0x50;
    let mut texts: Vec<String> = vec![];
    let mut options: Vec<String> = vec![];
    if let Some(n) = u32_at(d, tp) {
        let n = n as usize;
        if tp + 4 + n <= d.len() {
            texts = split_strings(&d[tp + 4..tp + 4 + n]);
            let op = tp + 4 + n;
            if let Some(m) = u32_at(d, op) {
                let m = m as usize;
                if op + 4 + m <= d.len() {
                    options = split_strings(&d[op + 4..op + 4 + m]);
                }
            }
        }
    }
    let mut el = parse_elements(&d[base..tp], &texts);
    el.options = options;
    Some(el)
}

pub fn split_strings(b: &[u8]) -> Vec<String> {
    let mut v: Vec<String> = b.split(|c| *c == 0).map(cstr).collect();
    while v.last().map(|s| s.is_empty()).unwrap_or(false) {
        v.pop();
    }
    v
}

/// Elementliste (je 0x50 Byte) in Zeichenelemente umsetzen. Texte: 1-basiert, Text 1 = BMK-Vorlage.
pub fn parse_elements(data: &[u8], texts: &[String]) -> Elements {
    let mut el = Elements { prims: vec![], vis: vec![], pins: vec![], pin_vis: vec![], title: None, value_pos: None, options: vec![], flag: vec![], text_idx: vec![], text_mode: vec![], fill: vec![] };
    for e in data.chunks_exact(0x50) {
        let v = |i: usize| i32_at(e, i * 4) as f32 / 10.0;
        let pt = |i: usize| P::new(v(2 * i), v(2 * i + 1));
        let t = e[0x3c];
        let style = (i32_at(e, 0x40).clamp(0, 3)) as u8;
        let opt = u16_at(e, 0x44);
        let mode = u16_at(e, 0x46);
        let page = i32_at(e, 0x48);
        if page < 0 || page > 32 || (page != 0 && matches!(t, 7 | 9)) {
            continue;
        }
        let vv = (opt, mode);
        let tidx = if t == 6 { i32_at(e, 7 * 4).max(0) as u32 } else { 0 };
        let tmode = if t == 6 { i32_at(e, 8 * 4).clamp(0, 2) as u8 } else { 0 };
        let filled = matches!(t, 3 | 4 | 5) && i32_at(e, 10 * 4) & 0xff != 0;
        let mut push = |p: Prim| {
            el.prims.push(p);
            el.vis.push(vv);
            el.flag.push(page as u32);
            el.text_idx.push(tidx);
            el.text_mode.push(tmode);
            el.fill.push(filled);
        };
        match t {
            1 => push(Prim::Line(pt(0), pt(1), style)),
            2 => push(Prim::Bezier(pt(0), pt(1), pt(2), pt(3), style)),
            3 => push(Prim::Poly(vec![pt(0), pt(1), pt(2), pt(3)], style)),
            4 => push(Prim::Poly(vec![pt(0), pt(1), pt(2)], style)),
            5 => push(Prim::Ellipse(pt(0), pt(1), style)),
            6 => {
                let idx = i32_at(e, 7 * 4);
                let al = match i32_at(e, 16) {
                    1 => Align::Center,
                    2 => Align::Right,
                    _ => Align::Left,
                };
                // Text 1 ist bei manchen Symbolen die BMK-Vorlage (z. B. "-M%C+%P" in "Motorsymbole"; die BMK zeichnet
                // die Instanz selbst), bei anderen ein echter Symboltext ("N" in "Verbindung", "M ~" in "Motor")
                let is_bmk = idx == 1 && texts.first().map(|t| t.contains('%')).unwrap_or(true);
                if idx >= 1 && !is_bmk {
                    if let Some(txt) = texts.get(idx as usize - 1) {
                        if !txt.contains("%{") {
                            push(Prim::TextBox(pt(0), pt(1), txt.clone(), al));
                        }
                    }
                }
            }
            7 => {
                let (a, b) = (pt(0), pt(1));
                if a.dist(b) > 0.01 {
                    push(Prim::Line(a, b, 0));
                }
                el.pins.push(a);
                el.pin_vis.push(vv);
            }
            9 => el.title = Some(pt(0)),
            10 => el.value_pos = Some(pt(0)),
            _ => {}
        }
    }
    el
}

// ---------------------------------------------------------------- Ersatzsymbole

const fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

fn def(key: &str, name: &str, group: &[&str], bmk: &str, prims: Vec<Prim>, pins: Vec<P>) -> SymDef {
    SymDef {
        key: format!("builtin:{key}"),
        name: name.into(),
        group: group.iter().map(|s| s.to_string()).collect(),
        haupt: 300,
        fav: true,
        bmk: bmk.into(),
        prims,
        pins,
        title: None,
        vis: vec![],
        pin_vis: vec![],
        flag: vec![],
        flag_names: vec![],
        text_idx: vec![],
        alt_sets: vec![],
        raw: vec![],
        frg: (true, false),
        params: vec![],
        value_pos: None,
        text_mode: vec![],
        fill: vec![],
        options: vec![],
    }
}

/// Eigene Ersatzsymbole im IEC-Stil (falls die Original-Datenbank fehlt).
pub fn builtin() -> Vec<SymDef> {
    use Prim::*;
    vec![
        def(
            "leistungskontakt",
            "Leistungskontakt",
            &["Kontakte/Schalter"],
            "%SQ%C",
            vec![Line(p(0.0, -7.0), p(0.0, -3.0), 0), Arc(p(0.0, -3.0), 0.9, 180.0, 360.0), Line(p(0.0, 7.0), p(0.0, 2.0), 0), Line(p(0.0, 2.0), p(-1.5, -3.5), 0)],
            vec![p(0.0, -7.0), p(0.0, 7.0)],
        ),
        def(
            "schliesser",
            "Schließer",
            &["Kontakte/Schalter"],
            "%SS%C",
            vec![Line(p(0.0, -7.0), p(0.0, -3.0), 0), Line(p(0.0, 7.0), p(0.0, 2.0), 0), Line(p(0.0, 2.0), p(-1.5, -3.5), 0)],
            vec![p(0.0, -7.0), p(0.0, 7.0)],
        ),
        def(
            "oeffner",
            "Öffner",
            &["Kontakte/Schalter"],
            "%SS%C",
            vec![Line(p(0.0, -7.0), p(0.0, -3.0), 0), Line(p(0.0, -3.0), p(2.0, -3.0), 0), Line(p(0.0, 7.0), p(0.0, 2.0), 0), Line(p(0.0, 2.0), p(2.0, -3.5), 0)],
            vec![p(0.0, -7.0), p(0.0, 7.0)],
        ),
        def(
            "motor",
            "Motor",
            &["Motore"],
            "%SM%C",
            vec![Line(p(0.0, -10.0), p(0.0, -6.0), 0), Ellipse(p(-6.0, -6.0), p(6.0, 6.0), 0), Text(p(0.0, -0.8), "M".into(), 5.0), Text(p(0.0, 3.2), "3~".into(), 2.8)],
            vec![p(0.0, -10.0)],
        ),
        def(
            "motorschutz",
            "Motorschutz",
            &["Motore"],
            "%SF%C",
            vec![
                Line(p(0.0, -7.0), p(0.0, -1.5), 0),
                Poly(vec![p(-3.0, -2.5), p(3.0, -2.5), p(3.0, 2.5), p(-3.0, 2.5)], 0),
                Line(p(0.0, -1.5), p(1.5, -1.5), 0),
                Line(p(1.5, -1.5), p(1.5, 1.5), 0),
                Line(p(1.5, 1.5), p(0.0, 1.5), 0),
                Line(p(0.0, 1.5), p(0.0, 7.0), 0),
            ],
            vec![p(0.0, -7.0), p(0.0, 7.0)],
        ),
        def(
            "relais",
            "Relais",
            &["Relais"],
            "%SK%C",
            vec![Line(p(0.0, -6.0), p(0.0, -3.0), 0), Poly(vec![p(-6.0, -3.0), p(6.0, -3.0), p(6.0, 4.0), p(-6.0, 4.0)], 0), Line(p(0.0, 4.0), p(0.0, 7.0), 0)],
            vec![p(0.0, -6.0), p(0.0, 7.0)],
        ),
        def(
            "sicherung",
            "Sicherung",
            &["Schutzeinrichtung"],
            "%SF%C",
            vec![Poly(vec![p(-1.5, -3.5), p(1.5, -3.5), p(1.5, 3.5), p(-1.5, 3.5)], 0), Line(p(0.0, -6.0), p(0.0, 6.0), 0)],
            vec![p(0.0, -6.0), p(0.0, 6.0)],
        ),
        def(
            "io_baugruppe",
            "IO Baugruppe",
            &["Siemens"],
            "%SA%C",
            vec![Line(p(0.0, -6.0), p(0.0, -3.0), 0), Poly(vec![p(-8.0, -3.0), p(8.0, -3.0), p(8.0, 3.0), p(-8.0, 3.0)], 0), Text(p(0.0, 0.0), "IO".into(), 3.5), Line(p(0.0, 3.0), p(0.0, 6.0), 0)],
            vec![p(0.0, -6.0), p(0.0, 6.0)],
        ),
        def(
            "lampe",
            "Lampe",
            &["Signaleinrichtung", "Lampen"],
            "-E%C",
            vec![
                Line(p(0.0, -5.6), p(0.0, -3.4), 0),
                Ellipse(p(-3.5, -3.5), p(3.5, 3.5), 0),
                Line(p(-2.4, -2.5), p(2.4, 2.3), 0),
                Line(p(-2.4, 2.3), p(2.4, -2.5), 0),
                Line(p(0.0, 5.6), p(0.0, 3.4), 0),
            ],
            vec![p(0.0, -5.6), p(0.0, 5.6)],
        ),
    ]
}

/// Alternativtexte und Optionsnamen aus den Zeichendaten der Bibliothek ("Ver2", nach Texten und Bildnamen):
/// SIT-Blöcke zu je 40 Byte, dann ALT-Einträge (Textnummer, Satz, mit Text 0/1, zwei Zeiger [+ u32 Länge + Text],
/// getrennt durch u32 0), dann OPT-Einträge (… Bitnummer, Zeiger, u32 Länge + Name).
/// Ermittelt am Bibliothekssymbol "Steckdose" und gegengeprüft mit dessen SYM-Block in W2.epr.
pub fn parse_lib_extras(raw: &[u8]) -> (Vec<Vec<(u32, String)>>, Vec<String>) {
    let mut alt: Vec<Vec<(u32, String)>> = vec![];
    let mut opts: Vec<String> = vec![];
    if raw.len() < 0x3c || &raw[0..3] != b"Ver" {
        return (alt, opts);
    }
    let u = |p: usize| raw.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let cnt = u(0x14).unwrap_or(0) as usize;
    let mut p = 0x3c + cnt * 0x50;
    let Some(tl) = u(p) else { return (alt, opts) };
    p += 4 + tl as usize;
    let Some(sl) = u(p) else { return (alt, opts) };
    p += 4 + sl as usize;
    // ALT-Kette ab einem SIT-Vielfachen suchen
    let parse_alt = |mut q: usize| -> (Vec<(u32, u32, String)>, usize) {
        let mut v = vec![];
        loop {
            if u(q) == Some(0) && v.is_empty() {
                q += 4;
            }
            let (Some(idx), Some(set), Some(has)) = (u(q), u(q + 4), u(q + 8)) else { break };
            if !(1..=999).contains(&idx) || !(1..=64).contains(&set) || has > 1 {
                break;
            }
            q += 20;
            let mut text = String::new();
            if has == 1 {
                let Some(n) = u(q) else { break };
                if n > 200 {
                    break;
                }
                text = raw.get(q + 4..q + 4 + n as usize).map(cstr).unwrap_or_default();
                q += 4 + n as usize;
            } else if u(q) == Some(0) {
                q += 4;
            }
            v.push((idx, set, text));
        }
        (v, q)
    };
    let mut best = (vec![], p);
    for k in 0..6 {
        let (v, q) = parse_alt(p + k * 40);
        if v.len() > best.0.len() {
            best = (v, q);
        }
    }
    for (idx, set, text) in best.0 {
        let s = set as usize;
        if alt.len() < s {
            alt.resize(s, vec![]);
        }
        alt[s - 1].push((idx, text));
    }
    // OPT-Einträge: Bitnummer (1…32), Zeiger, Länge (1…60), Name
    let mut q = best.1;
    while q + 12 <= raw.len() {
        let (bit, len) = (u(q).unwrap_or(0), u(q + 8).unwrap_or(0));
        let name = raw.get(q + 12..q + 12 + len as usize);
        if (1..=32).contains(&bit) && (1..=60).contains(&len) && name.map(|n| n.iter().all(|c| *c >= 32)).unwrap_or(false) {
            let b = bit as usize;
            if opts.len() < b {
                opts.resize(b, String::new());
            }
            opts[b - 1] = cstr(name.unwrap());
            q += 12 + len as usize;
        } else {
            q += 1;
        }
    }
    (alt, opts)
}

// ---------------------------------------------------------------- Bearbeiten (Symboleditor, MCP)

impl SymDef {
    /// Parallele Listen (Sichtbarkeit, Zusatzoption, Textnummer, Textausrichtung, Füllung) auf die Zahl der
    /// Grundelemente bringen.
    pub fn normalize(&mut self) {
        let n = self.prims.len();
        self.vis.resize(n, (0, 1));
        self.flag.resize(n, 0);
        self.text_idx.resize(n, 0);
        self.text_mode.resize(n, 0);
        self.fill.resize(n, false);
        self.pin_vis.resize(self.pins.len(), (0, 1));
    }

    /// Nächste freie Textnummer (Text 1 ist die BMK-Vorlage).
    pub fn next_text_nr(&self) -> u32 {
        self.text_idx.iter().copied().max().unwrap_or(1).max(1) + 1
    }

    /// Grundelement anhängen; Rückgabe: Index.
    pub fn push_prim(&mut self, p: Prim, vis: (u16, u16), flag: u32, text_mode: u8, fill: bool) -> usize {
        self.normalize();
        let t = if matches!(p, Prim::TextBox(..) | Prim::Text(..)) { self.next_text_nr() } else { 0 };
        self.prims.push(p);
        self.vis.push(vis);
        self.flag.push(flag);
        self.text_idx.push(t);
        self.text_mode.push(text_mode);
        self.fill.push(fill);
        self.raw.clear();
        self.prims.len() - 1
    }

    /// Grundelement entfernen.
    pub fn remove_prim(&mut self, i: usize) -> Result<(), String> {
        self.normalize();
        if i >= self.prims.len() {
            return Err(format!("Element {i} gibt es nicht ({} Elemente)", self.prims.len()));
        }
        self.prims.remove(i);
        self.vis.remove(i);
        self.flag.remove(i);
        self.text_idx.remove(i);
        self.text_mode.remove(i);
        self.fill.remove(i);
        self.raw.clear();
        Ok(())
    }

    /// Neue Seite (Bildvariante) anhängen; Rückgabe: Nummer. Ohne Seiten entsteht zuerst "Standard".
    pub fn add_page(&mut self, name: &str) -> u16 {
        if self.options.is_empty() {
            self.options.push("Standard".into());
        }
        self.options.push(name.into());
        self.raw.clear();
        (self.options.len() - 1) as u16
    }

    /// Seite löschen: Elemente nur dieser Seite entfallen, spätere Seiten rücken vor. Seite 0 bleibt.
    pub fn delete_page(&mut self, page: u16) -> Result<(), String> {
        if page == 0 || page as usize >= self.options.len() {
            return Err("Seite 0 (Grundbild) bzw. eine nicht vorhandene Seite kann nicht gelöscht werden".into());
        }
        self.normalize();
        self.options.remove(page as usize);
        let mut i = 0;
        while i < self.prims.len() {
            let v = self.vis[i];
            if v.0 == page && v.1 == 0 {
                self.remove_prim(i)?;
                continue;
            }
            if v.0 > page {
                self.vis[i].0 -= 1;
            }
            i += 1;
        }
        let mut k = 0;
        while k < self.pins.len() {
            let v = self.pin_vis[k];
            if v.0 == page && v.1 == 0 {
                self.pins.remove(k);
                self.pin_vis.remove(k);
                continue;
            }
            if v.0 > page {
                self.pin_vis[k].0 -= 1;
            }
            k += 1;
        }
        self.raw.clear();
        Ok(())
    }

    /// Zusatzoption (OPT) anhängen; Rückgabe: Nummer n (Bit n−1 der Instanz-Zusatzoptionen).
    pub fn add_flag(&mut self, name: &str) -> Result<u32, String> {
        if self.flag_names.len() >= 32 {
            return Err("höchstens 32 Zusatzoptionen".into());
        }
        self.flag_names.push(name.into());
        self.raw.clear();
        Ok(self.flag_names.len() as u32)
    }

    /// Zusatzoption n löschen: ihre Elemente entfallen, höhere Nummern rücken vor.
    pub fn delete_flag(&mut self, n: u32) -> Result<(), String> {
        if n == 0 || n as usize > self.flag_names.len() {
            return Err(format!("Zusatzoption {n} gibt es nicht"));
        }
        self.normalize();
        self.flag_names.remove(n as usize - 1);
        let mut i = 0;
        while i < self.prims.len() {
            if self.flag[i] == n {
                self.remove_prim(i)?;
                continue;
            }
            if self.flag[i] > n {
                self.flag[i] -= 1;
            }
            i += 1;
        }
        self.raw.clear();
        Ok(())
    }

    /// Alternativtext setzen (Satz 1-basiert, Textnummer wie im Element); leerer Text = Text ausblenden.
    pub fn set_alt_text(&mut self, set: usize, text_nr: u32, text: &str) {
        if set == 0 {
            return;
        }
        if self.alt_sets.len() < set {
            self.alt_sets.resize(set, vec![]);
        }
        let s = &mut self.alt_sets[set - 1];
        match s.iter_mut().find(|(n, _)| *n == text_nr) {
            Some(e) => e.1 = text.into(),
            None => s.push((text_nr, text.into())),
        }
        self.raw.clear();
    }

    /// Alternativtext-Satz (1-basiert) löschen.
    pub fn delete_alt_set(&mut self, set: usize) -> Result<(), String> {
        if set == 0 || set > self.alt_sets.len() {
            return Err(format!("Alternativtext-Satz {set} gibt es nicht"));
        }
        self.alt_sets.remove(set - 1);
        self.raw.clear();
        Ok(())
    }

    /// Neues, leeres Symbol mit Grundbild "Standard".
    pub fn empty(key: &str, name: &str, bmk: &str) -> SymDef {
        SymDef {
            key: key.into(),
            name: name.into(),
            group: vec!["Projekt".into()],
            haupt: 0,
            fav: false,
            bmk: bmk.into(),
            prims: vec![],
            pins: vec![],
            title: None,
            vis: vec![],
            pin_vis: vec![],
            options: vec!["Standard".into()],
            flag: vec![],
            flag_names: vec![],
            text_idx: vec![],
            alt_sets: vec![],
            raw: vec![],
            frg: (true, false),
            params: vec![],
            value_pos: None,
            text_mode: vec![],
            fill: vec![],
        }
    }
}
