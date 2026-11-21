//! Import alter Projektdateien des Originals (*.epr, Kennung "EP16").
//! Aufbau (selbst ermittelt): Blöcke 0x99 + 3-Zeichen-Kennung + u16 Länge + i16 (= !Länge) + Daten + u32,
//! Unterblöcke folgen direkt, "0x99 END" schließt einen Block. Koordinaten in 0,1 mm.

use crate::model::*;
use crate::symbols::{self, SymDef};
use crate::titleblock::TbOwned;

use crate::eprfile::{i32_at, Block, EprFile};

fn txt(b: &[u8]) -> String {
    symbols::cstr(b)
}
fn txt_at(d: &[u8], from: usize, to: usize) -> String {
    d.get(from..to.min(d.len())).map(txt).unwrap_or_default()
}
fn mm(v: i32) -> f32 {
    v as f32 / 10.0
}

/// Lage der Datumsangaben (Erstellt, Geändert) im PRJ-Block – wie beim Import gesucht.
pub fn prj_date_offsets(prj: &Block) -> Vec<usize> {
    let mut v = vec![];
    let mut off = 40;
    while off + 8 <= prj.data.len() && v.len() < 2 {
        let f = f64::from_le_bytes(prj.data[off..off + 8].try_into().unwrap());
        if delphi_date(f).is_some() {
            v.push(off);
            off += 8;
        } else {
            off += 4;
        }
    }
    v
}

/// Datum an einer Stelle als TT.MM.JJJJ.
pub fn delphi_date_at(d: &[u8], off: usize) -> Option<String> {
    d.get(off..off + 8).and_then(|b| delphi_date(f64::from_le_bytes(b.try_into().ok()?)))
}

/// TT.MM.JJJJ → Delphi-Datum (Tage seit 30.12.1899).
pub fn delphi_from_text(s: &str) -> Option<f64> {
    let d = chrono::NaiveDate::parse_from_str(s.trim(), "%d.%m.%Y").ok()?;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    Some((d - base).num_days() as f64)
}

/// Delphi-Datum (Tage seit 30.12.1899) als TT.MM.JJJJ.
fn delphi_date(v: f64) -> Option<String> {
    if !(20000.0..80000.0).contains(&v) {
        return None;
    }
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let d = base.checked_add_signed(chrono::Duration::days(v.floor() as i64))?;
    Some(d.format("%d.%m.%Y").to_string())
}

pub fn import(path: &std::path::Path) -> Result<Project, String> {
    let file = EprFile::read(path)?;
    let mut p = import_file(&file)?;
    p.raw = Some(Box::new(file));
    Ok(p)
}

/// Projekt aus einer gelesenen .epr-Datei aufbauen; jedes Objekt behält seinen Originalblock.
pub fn import_file(file: &EprFile) -> Result<Project, String> {
    let prj = file.root("PRJ").ok_or("Projektblock (PRJ) fehlt")?;
    let mut p = Project::new_default();
    p.anlagen.clear();
    p.name = txt_at(&prj.data, 0, 40);
    // Datumsangaben (Erstellt, Geändert) als Delphi-Datum im Projektblock
    let mut dates = vec![];
    let mut off = 40;
    while off + 8 <= prj.data.len() {
        let v = f64::from_le_bytes(prj.data[off..off + 8].try_into().unwrap());
        if let Some(s) = delphi_date(v) {
            dates.push(s);
        }
        off += 4;
    }
    if let Some(s) = dates.first() {
        p.erstellt = s.clone();
    }
    if let Some(s) = dates.get(1) {
        p.geaendert = s.clone();
    }

    // Leitungen und Potentiale
    let ltg: Vec<Leitung> = prj
        .kids_of("LTG")
        .map(|l| {
            // LTG: Name 0–20, Feld 5 Breite (100 = Standard), 6 Farbe (TColor), 9 Kennung, 10 Strichart
            let c = l.i32(6);
            let color = if (0..=0xff_ffff).contains(&c) { [(c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8] } else { [0, 0, 0] };
            let w = l.i32(5);
            Leitung {
                name: txt_at(&l.data, 0, 20),
                color,
                width_mm: if (10..2000).contains(&w) { w as f32 / 400.0 } else { 0.25 },
                quer: 1.5,
                adern: 3,
                kennz: 0,
                style: l.i32(10).clamp(0, 255) as u8,
                id: l.i32(9),
            }
        })
        .collect();
    if !ltg.is_empty() {
        p.leitungen = ltg;
    }
    let pot: Vec<String> = prj.kids_of("POT").map(|n| txt_at(&n.data, 0, 20)).collect();
    if !pot.is_empty() {
        p.potentiale = pot;
    }

    // Projektsymbole
    p.symdefs.extend(prj.kids_of("SYM").filter_map(symdef_from_block));

    // Schriftfeld des Projekts
    if let Some(fus) = prj.kid("FUS") {
        let texts: Vec<String> = fus.kid("STR").map(|n| n.data.split(|c| *c == 0).map(txt).collect()).unwrap_or_default();
        let mut tb = vec![];
        for g in fus.kids_of("GRS") {
            for e in g.data.chunks_exact(0x50) {
                let v = |i: usize| i32_at(e, i * 4);
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
        if !tb.is_empty() {
            p.titleblock = tb;
        }
    }

    // Projektbilder: BLD (Name 0–20, Feld 5 = Nummer) › JPG (JPEG-Daten, über 64 KB in Teilen)
    for b in prj.kids_of("BLD") {
        let jpeg: Vec<u8> = b.kids_of("JPG").flat_map(|j| j.data.iter().chain(j.parts.iter().flat_map(|(_, d)| d.iter())).copied()).collect();
        p.bilder.push(Bild { name: txt_at(&b.data, 0, 20), nr: b.i32(5), jpeg });
    }
    // Sammelschienen aller Anlagen: Kennung (SMS Feld 8/9) → Name (TIT)
    let mut sms_names: std::collections::HashMap<u64, String> = Default::default();
    for a in prj.kids_of("ANL") {
        for s in a.kids_of("SMS") {
            if let Some(id) = s.data.get(32..40).map(|b| u64::from_le_bytes(b.try_into().unwrap())) {
                sms_names.insert(id, s.kid("TIT").map(|t| txt(&t.data)).unwrap_or_else(|| txt_at(&s.data, 0, 20)));
            }
        }
    }

    let mut next_id: u64 = 1;
    let mut alloc = || {
        next_id += 1;
        next_id
    };

    for a in prj.kids_of("ANL") {
        let name = txt_at(&a.data, 0, 55);
        let von = txt_at(&a.data, 66, 120);
        if p.von.is_empty() && !von.is_empty() {
            p.von = von;
        }
        let columns = match a.i32(44) {
            c @ 1..=40 => c as u32,
            _ => 8,
        };
        let mut anl = Anlage {
            schienen: Default::default(),
            name: if name.is_empty() { format!("Anlage {}", p.anlagen.len() + 1) } else { name },
            sheets: vec![],
            sammelschienen: a.kids_of("SMS").map(|s| s.kid("TIT").map(|t| txt(&t.data)).unwrap_or_else(|| txt_at(&s.data, 0, 20))).collect(),
            // Klemmleisten unter ihrem Titel (%O der Klemmstellen), Einzelheiten in `klemmen`
            klemmleisten: a.kids_of("KLM").map(|k| crate::klemmen::klemm_info(k).name).collect(),
            klemmen: a.kids_of("KLM").map(crate::klemmen::klemm_info).collect(),
            set: AnlSettings {
                kurz: txt_at(&a.data, 55, 66),
                von: txt_at(&a.data, 66, 112),
                znr: txt_at(&a.data, 116, 136),
                format: (a.i32(38), a.i32(39)),
                raender: [a.i32(40), a.i32(41), a.i32(42), a.i32(43)],
                pfade: a.i32(44),
                pfad_hoehe: a.i32(45),
                ab_null: a.i32(49) != 0,
                size_titel: a.i32(46),
                size_wert: a.i32(47),
                size_verbg: a.i32(48),
                size_pfad: a.i32(50),
            },
            raw: None,
        };
        anl.raw = Some(Box::new((a.clone(), anl.name.clone())));
        // Klemmleisten der Anlage: Kennung (KLM Feld 13) → (Name, Etagen = Feld 15)
        let klm: std::collections::HashMap<i32, (String, i32)> = a
            .kids_of("KLM")
            .map(|k| (k.i32(13), (k.kid("TIT").map(|t| txt(&t.data)).unwrap_or_else(|| txt_at(&k.data, 0, 20)), k.i32(15))))
            .collect();
        // Anlagen ohne Klemmleiste bleiben ohne (eine neue Leiste würde sonst beim Speichern als KLM-Block entstehen)
        for b in a.kids_of("BLT") {
            let mut sh = Sheet::new(((b.i32(23) >> 8) & 0xffff).max(1) as u32);
            sh.name = txt_at(&b.data, 0, 40);
            sh.columns = columns;
            sh.col_auto = true;
            // Hochformat: Rahmenhöhe 277 mm statt 190 mm (BLT i32[24]); Neubau EG im Beispiel
            sh.portrait = b.i32(24) > 2400;
            sh.subindex = (b.i32(23) & 0xff) as u8;
            sh.art = (b.i32(22) & 0xff) as u8;
            sh.erstellt = delphi_date_at(&b.data, 112).unwrap_or_default();
            sh.geaendert = delphi_date_at(&b.data, 120).unwrap_or_default();
            // Ort des Blattes (Byte 51 ff., z. B. "W2"), Grundlage für %P in BMK-Vorlagen
            sh.ort = txt_at(&b.data, 51, 72);
            if sh.art == 4 {
                sh.klm = b.i32(32) >> 8;
            } else if matches!(sh.art, 0 | 2 | 6) {
                // Zusatz: Deckblatt mit/ohne Rahmen, Querverweise 1–3 Tabellen, Stückliste (Feld 32 Byte 1)
                sh.zusatz = b.data.get(0x81).copied().unwrap_or(0);
            }
            sh.art_fest = true;
            if !sh.portrait && (500..2100).contains(&b.i32(24)) {
                sh.frame_h = b.i32(24) as f32 / 10.0;
            }
            let mut map = vec![];
            for k in &b.kids {
                let kind = if k.is("ZCN") {
                    drawing(k)
                } else if k.is("VSY") {
                    symbol(k)
                } else if k.is("VBG") {
                    let pts: Vec<P> = k.kids_of("VBP").map(|v| P::new(mm(v.i32(0)), mm(v.i32(1)))).collect();
                    if pts.len() >= 2 {
                        // Feld 13 = Kennung der Leitung (LTG Feld 9)
                        let leitung = p.leitungen.iter().position(|l| l.id == k.i32(13) && l.id != 0).unwrap_or(0);
                        Some((Kind::Wire { pts, leitung, bez: k.kids_of("BZD").filter_map(bzd).collect(), opts: crate::epr_zusatz::wire_opts(k) }, 0))
                    } else {
                        None
                    }
                } else if crate::kontaktspiegel::ist_spiegel(k) {
                    Some((crate::kontaktspiegel::lesen(k), 0))
                } else if k.is("OBJ") {
                    match k.kid("TIT") {
                        _ if crate::klemmen::is_terminal(k) => Some((terminal(k, &klm), 0)),
                        Some(t) => Some((verweis(k, txt(&t.data)), 0)),
                        // Sammelschiene (Feld 4 = 0xFFFF) bzw. Verweis ohne eigenen Text (nur Pfeil, z. B. im Schriftfeld)
                        None if k.i32(4) == 0xffff => busbar(k, &sms_names).map(|b| (b, 0)),
                        None if k.i32(4) & 0xffff == 0xffff => Some((verweis(k, String::new()), 0)),
                        None => None,
                    }
                } else {
                    None
                };
                if let Some((kind, pen)) = kind {
                    let id = alloc();
                    // Fixiert: ZCN Feld 15 Bit 16 (Feld 15 Byte 0 = Typ)
                    let fixed = k.is("ZCN") && (k.i32(15) >> 16) & 1 == 1;
                    let raw = RawObj { block: k.clone(), kind: kind.clone(), pen, fixed };
                    sh.objects.push(Obj { id, kind, pen, fixed, group: None, raw: Some(Box::new(raw)) });
                    map.push(Some(id));
                } else {
                    map.push(None);
                }
            }
            sh.raw = Some(Box::new(RawSheet { block: b.clone(), map, name: sh.name.clone(), number: sh.number, portrait: sh.portrait, subindex: sh.subindex, ort: sh.ort.clone() }));
            anl.sheets.push(sh);
        }
        p.anlagen.push(anl);
    }
    if p.anlagen.is_empty() {
        return Err("Keine Anlagen im Projekt gefunden".into());
    }
    update_busbar_xrefs(&mut p);
    link_verweise(&mut p);
    crate::eigentuemer::verknuepfen(&mut p);
    for a in &mut p.anlagen {
        if a.sheets.is_empty() {
            a.sheets.push(Sheet::new(1));
        }
    }
    p.next_id = next_id + 10;
    crate::epr_projekt::import(prj, &mut p);
    crate::epr_zusatz::import(prj, &mut p);
    // nach der Schriftfeldhöhe (Projektebene): Y-Lage der Kontaktspiegel hängt von ihr ab
    crate::kontaktspiegel::verknuepfen(&mut p);
    Ok(p)
}

/// Zeichenelement eines Blattes (gleicher 0x50-Byte-Aufbau wie in den Symbolen).
fn drawing(k: &Block) -> Option<(Kind, usize)> {
    let e = &k.data;
    if e.len() < 0x50 {
        return None;
    }
    let v = |i: usize| i32_at(e, i * 4);
    let pt = |i: usize| P::new(mm(v(2 * i)), mm(v(2 * i + 1)));
    // Stift: 0–3 fest eingebaut, ab 4 eigene Stifte (PRJ › SIT Feld 8)
    let pen = v(16).clamp(0, 255) as usize;
    let kind = match e[0x3c] {
        1 => Kind::Line { a: pt(0), b: pt(1) },
        2 => Kind::Curve { a: pt(0), c1: pt(1), c2: pt(2), b: pt(3) },
        3 => {
            let q = [pt(0), pt(1), pt(2), pt(3)];
            let axis = (q[0].x - q[1].x).abs() < 0.01 || (q[0].y - q[1].y).abs() < 0.01;
            if axis {
                let (mn, mx) = crate::canvas::pts_bounds(&q);
                Kind::Rect { a: mn, b: mx }
            } else {
                Kind::Poly { pts: q.to_vec() }
            }
        }
        4 => Kind::Poly { pts: vec![pt(0), pt(1), pt(2)] },
        5 => Kind::Circle { a: pt(0), b: pt(1) },
        6 | 11 => {
            let text = k
                .kid("STL")
                .or_else(|| k.kid("STR"))
                .map(|n| txt(&n.data).replace("\r\n", "\n").trim_end().to_string())
                .unwrap_or_default();
            if text.is_empty() {
                return None;
            }
            let pts = (v(4) & 0xffff).clamp(4, 72) as f32;
            let size = if e[0x3c] == 11 { pts * 0.3528 } else { 3.5 };
            let vertical = v(5) == 90 || v(5) == 270;
            let (a, b) = (pt(0), pt(1));
            let (mn, mx) = crate::canvas::norm_rect(a, b);
            // Feld 6: Byte 0 Ausrichtung, Bit 16 Rahmen, Bit 24 Fett; Feld 7 Byte 0 Kursiv; Feld 8 Farbe (TColor = 0x00BBGGRR)
            let f6 = v(6);
            let align = match f6 & 0xff {
                1 => Align::Center,
                2 => Align::Right,
                _ => Align::Left,
            };
            let col = v(8);
            let style = TextStyle {
                frame: f6 & 0x1_0000 != 0,
                bold: (f6 >> 24) & 0xff != 0,
                italic: v(7) & 0xff != 0,
                underline: (v(7) >> 8) & 0xff != 0,
                color: if (0..=0xff_ffff).contains(&col) { [(col & 0xff) as u8, ((col >> 8) & 0xff) as u8, ((col >> 16) & 0xff) as u8] } else { [0, 0, 0] },
                rotation: v(5) as i16,
                wrap: e[0x3c] == 11 && crate::epr_neu::text_extras(k).0,
                rect: Some((mn, mx)),
                font: if e[0x3c] == 11 { crate::epr_neu::text_extras(k).1 } else { String::new() },
            };
            let pos = text_anchor(mn, mx, align, vertical);
            Kind::Text { pos, text, size, align, vertical, style }
        }
        // Tabelle: Feld 0–3 Rechteck, 4 Spalten, 5 Zeilen, Zellen zeilenweise im STR-Unterblock (0-getrennt)
        13 => {
            let (mn, mx) = crate::canvas::norm_rect(pt(0), pt(1));
            let cells: Vec<String> = k.kid("STR").map(|n| table_cells(&n.data)).unwrap_or_default();
            let (header, fixed_w) = crate::epr_neu::tabelle_flags(k);
            // Daten (Feld 9 Byte 2) und „Ab“ (Feld 6)
            Kind::Table { a: mn, b: mx, rows: v(5).clamp(1, 500) as usize, cols: v(4).clamp(1, 50) as usize, cells, header, fixed_w, art: e[0x26], ab: v(6).max(0) as u32 }
        }
        // Bild: Projektbild Feld 4, Rahmen/Ausrichtung Feld 5 (siehe epr_neu)
        12 => crate::epr_neu::bild_lesen(k),
        _ => return None,
    };
    Some((kind, pen))
}

/// Zellen einer Tabelle (STR, 0-getrennt; leere Zellen bleiben erhalten).
pub fn table_cells(d: &[u8]) -> Vec<String> {
    let d = d.strip_suffix(&[0]).unwrap_or(d);
    d.split(|c| *c == 0).map(|c| txt(c).replace("\r\n", "\n")).collect()
}

/// Verbindungsbezeichnung (VBG › BZD): Feld 3/4 Punkt auf der Leitung, Feld 0/1 Textversatz, STR = Text.
/// Lage gegen das Original-PDF (Seite 18, Aderfarben "sw", "br", "gr") geprüft: Text beginnt bei Punkt + Versatz
/// und steht über der Leitung.
fn bzd(b: &Block) -> Option<(P, String)> {
    let t = b.kid("STR").map(|t| txt(&t.data)).unwrap_or_default();
    (!t.is_empty()).then(|| (P::new(mm(b.i32(3) + b.i32(0)), mm(b.i32(4) + b.i32(1))), t))
}

/// Sammelschiene (OBJ, Feld 4 = 0xFFFF): Feld 6/7 Anfang, 10 Ende x, 2/3 Lage des Namens relativ zum Anfang,
/// 8/9 Kennung der Schiene (SMS), 15 Byte 0 = 1 Schutzleiter (Strich/Punkt wie im Original-Ausdruck).
fn busbar(k: &Block, names: &std::collections::HashMap<u64, String>) -> Option<Kind> {
    let a = P::new(mm(k.i32(6)), mm(k.i32(7)));
    let x2 = mm(k.i32(10));
    if x2 <= a.x {
        return None;
    }
    let sms = k.data.get(32..40).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0);
    Some(Kind::Busbar {
        a,
        x2,
        name: names.get(&sms).cloned().unwrap_or_default(),
        label: P::new(mm(k.i32(2)), mm(k.i32(3))),
        style: if k.i32(15) & 0xff == 1 { 2 } else { 0 },
        sms,
        xin: String::new(),
        xout: String::new(),
        lage: k.data.get(0x3c).copied().unwrap_or(0) & 3,
        versatz: k.data.get(0x2c..0x2e).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 10.0).unwrap_or(0.0),
    })
}

/// Klemmstelle (OBJ, Feld 4 = 0x2FFFF), siehe `Kind::Terminal`.
fn terminal(k: &Block, klm: &std::collections::HashMap<i32, (String, i32)>) -> Kind {
    let (name, levels) = klm.get(&k.i32(8)).cloned().unwrap_or_default();
    let lv = (k.i32(11) & 0xff) as u8;
    Kind::Terminal {
        pos: P::new(mm(k.i32(6)), mm(k.i32(7))),
        leiste: name,
        nr: k.i32(9).max(0) as u32,
        level: if levels > 1 { lv } else { 0 },
        text_off: Some(P::new(mm(k.i32(2)), mm(k.i32(3)))),
        tpl: k.kid("TIT").map(|t| txt(&t.data)).unwrap_or_default(),
        opt: crate::klemmen::read_opt(k),
    }
}

/// Bezeichnung einer Klemmstelle wie im Original-Ausdruck: "%O%B.%C" → "-X1.8B" (Leiste, Nummer, Etage).
pub fn terminal_label(tpl: &str, leiste: &str, nr: u32, level: u8) -> String {
    let c = if level > 0 { format!("{nr}{}", (b'A' + level - 1) as char) } else { nr.to_string() };
    let tpl = if tpl.is_empty() { "%O%B.%C" } else { tpl };
    tpl.replace("%O", leiste).replace("%B", "").replace("%C", &c)
}

/// Hinweise nach dem Laden: Unterblöcke von Blättern, die keine Darstellung haben (z. B. Verbindungen mit nur
/// einem Punkt). Sie bleiben beim Speichern unverändert erhalten.
pub fn load_warnings(p: &Project) -> Vec<String> {
    let mut out = vec![];
    for a in &p.anlagen {
        for s in &a.sheets {
            if let Some(r) = &s.raw {
                for (i, m) in r.map.iter().enumerate() {
                    let k = &r.block.kids[i];
                    // bekannte, harmlose Fälle: Verbindung mit nur einem Punkt, Sammelschiene ohne Länge
                    let harmless = (k.is("VBG") && k.kids_of("VBP").count() < 2) || (k.is("OBJ") && k.i32(4) == 0xffff);
                    if m.is_none() && !harmless {
                        out.push(format!("{} / {}: {}-Block Nr. {} ohne Darstellung (bleibt beim Speichern erhalten)", a.name, s.name, k.tag_str(), i + 1));
                    }
                }
            }
        }
    }
    out
}

/// Strompfad (1-basiert) einer x-Lage auf einem Blatt.
pub fn column_at(sh: &Sheet, x: f32) -> u32 {
    let (fx0, _, fx1, _) = sh.frame();
    let n = sh.columns.max(1);
    (((x - fx0) / ((fx1 - fx0) / n as f32)).floor().max(0.0) as u32 + 1).min(n)
}

/// Querverweise der Sammelschienen: je Anlage das vorige/nächste Vorkommen derselben Schiene als "Blatt/Pfad"
/// (wie im Original-Ausdruck "4/10►" am Anfang und "►6/1" am Ende).
pub fn update_busbar_xrefs(p: &mut Project) {
    for a in &mut p.anlagen {
        let mut occ: Vec<(usize, usize, u64, String, String)> = vec![];
        for (si, sh) in a.sheets.iter().enumerate() {
            for (oi, o) in sh.objects.iter().enumerate() {
                if let Kind::Busbar { a: s, x2, sms, .. } = &o.kind {
                    occ.push((si, oi, *sms, format!("{}/{}", sh.number, column_at(sh, s.x + 0.1)), format!("{}/{}", sh.number, column_at(sh, x2 - 0.1))));
                }
            }
        }
        for (i, (si, oi, sms, ..)) in occ.iter().enumerate() {
            let prev = occ[..i].iter().rev().find(|o| o.2 == *sms && o.0 != *si).map(|o| o.4.clone()).unwrap_or_default();
            let next = occ[i + 1..].iter().find(|o| o.2 == *sms && o.0 != *si).map(|o| o.3.clone()).unwrap_or_default();
            let o = &mut a.sheets[*si].objects[*oi];
            let set = |k: &mut Kind| {
                if let Kind::Busbar { xin, xout, sms, .. } = k {
                    if *sms != 0 {
                        *xin = prev.clone();
                        *xout = next.clone();
                    }
                }
            };
            set(&mut o.kind);
            if let Some(r) = o.raw.as_mut() {
                set(&mut r.kind);
            }
        }
    }
}

/// Symbol auf dem Blatt (Verweis auf ein Projektsymbol über dessen Kennung).
fn symbol(k: &Block) -> Option<(Kind, usize)> {
    let e = &k.data;
    if e.len() < 112 {
        return None;
    }
    let id = u64::from_le_bytes(e[104..112].try_into().ok()?);
    let flags = k.i32(18) & 0xff;
    let rot = if flags & 4 != 0 {
        3
    } else if flags & 2 != 0 {
        2
    } else if flags & 1 != 0 {
        1
    } else {
        0
    };
    let scale = f32::from_le_bytes(e[76..80].try_into().ok()?);
    let scale = if scale.is_finite() && (0.05..20.0).contains(&scale) { scale } else { 1.0 };
    let bmk = k.kid("TIT").map(|t| txt(&t.data)).unwrap_or_default();
    let value = k.kid("WRT").map(|t| txt(&t.data)).unwrap_or_default();
    let vert = (k.i32(12) >> 24) & 1 == 1;
    Some((
        Kind::Symbol {
            key: format!("epr:{id:016x}"),
            pos: P::new(mm(k.i32(16)), mm(k.i32(17))),
            rot,
            mirror_h: mirror_from_bits(rot, flags & 16 != 0, flags & 8 != 0).0,
            mirror_v: mirror_from_bits(rot, flags & 16 != 0, flags & 8 != 0).1,
            bmk,
            value,
            scale,
            option: k.i32(22).clamp(0, 255) as u16,
            flags: k.i32(23).max(0) as u32,
            title: Some((P::new(mm(k.i32(10)), mm(k.i32(11))), vert)),
            ext: SymExt {
                title_align: ((k.i32(12) >> 16) & 0xff).min(2) as u8,
                value_off: Some(P::new(mm(k.i32(13)), mm(k.i32(14)))),
                value_align: ((k.i32(15) >> 16) & 0xff).min(2) as u8,
                alt: k.i32(21).clamp(0, 255) as u16,
                querverweise: k.i32(20) & 1 == 0,
                sicherung: (k.i32(20) >> 24) & 1 == 1,
                funktion: k.kid("FKT").map(|t| txt(&t.data)).unwrap_or_default(),
                // Rackposition (VSY Feld 37, Feld LO_RekY des Originals; −1 = keine)
                rack: crate::epr_zusatz::rack(k),
                counter: k.i32(38).max(0),
                texts: k.kids_of("ALT").map(|a| (a.i32(0).max(0) as u32, a.kid("STR").map(|t| txt(&t.data)).unwrap_or_default())).collect(),
                params: k.kids_of("SPA").map(|s| (i32_at(&s.data, 0).max(0) as u32, s.kid("STR").map(|t| txt(&t.data)).unwrap_or_default())).collect(),
                bounds: {
                    let (l, t, r, b) = (k.i32(32), k.i32(33), k.i32(34), k.i32(35));
                    (r > l && b > t).then(|| (P::new(mm(l), mm(t)), P::new(mm(r), mm(b))))
                },
                // Eigentümer (OWR) verknüpft `eigentuemer::verknuepfen` nach dem Einlesen
                eigentuemer: 0,
            },
        },
        0,
    ))
}


/// Verweis (OBJ): Feld 6/7 Lage, 2/3 Textversatz, 14 Richtung in 0,1° (0 rechts, 900 oben), Feld 15 Bit 0 Spitze
/// an der Lage, Feld 4 Byte 3 Text hochkant. Vermessen an allen 13 Verweisen der Installationspläne im
/// Original-Ausdruck von W2 (Seiten 1–4): Das Dreieck (2,4 mm lang, 1,7 mm breit) liegt zwischen der Lage und
/// 2,4 mm in Richtung Feld 14; ohne Bit 0 zeigt die Spitze dorthin ("Raum 12" ►, "=S3+W2" ▼), mit Bit 0 liegt sie
/// an der Lage und zeigt zurück ("Raum 10" auf Blatt 4 ◄ bei 0°, die Pfeile im Schriftfeld ▼ bei 90°, ► bei 180°).
/// Hochkant stehen genau die Texte mit Feld 4 = 0x0101FFFF (Blatt "Altbau EG").
fn verweis(k: &Block, text: String) -> Kind {
    let pos = P::new(mm(k.i32(6)), mm(k.i32(7)));
    let text_pos = P::new(mm(k.i32(6) + k.i32(2)), mm(k.i32(7) + k.i32(3)));
    let ang = k.i32(14).rem_euclid(3600) / 10;
    let spitze = k.i32(15) & 1 == 1;
    let vertical = (k.i32(4) >> 24) & 1 == 1;
    let dir = if spitze { (ang + 180) % 360 } else { ang };
    Kind::Verweis { pos, text_pos, text, dir: dir as u16, vertical, spitze, ziel: None }
}

/// Kennung (8 Byte) eines Blocks ab Byte `at`, 0 wenn zu kurz.
fn id_at(b: &Block, at: usize) -> u64 {
    b.data.get(at..at + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0)
}

/// Kennungen im Originalformat: Anlage (ANL Byte 0x88), Blatt (BLT Byte 0x48), Objekt (OBJ Byte 0).
pub const ANL_ID_AT: usize = 0x88;
pub const BLT_ID_AT: usize = 0x48;

/// Ziele der Verweise aus OBJ Feld 8–13 (Kennungen von Anlage, Blatt und Gegenverweis) in Namen/Nummer/Objekt-Id
/// übersetzen. Im Original vergleicht die Routine bei 0x48056c genau diese Felder (Objekt +0x20/+0x28/+0x30) mit
/// Anlage +0x88, Blatt +0x48 und Objekt +0; Byte 0x3d (Feld 15 Byte 1) = Ziel gesetzt.
fn link_verweise(p: &mut Project) {
    let mut anl: std::collections::HashMap<u64, String> = Default::default();
    let mut blt: std::collections::HashMap<(u64, u64), u32> = Default::default();
    let mut objs: std::collections::HashMap<u64, u64> = Default::default();
    for a in &p.anlagen {
        let Some((ab, _)) = a.raw.as_deref() else { continue };
        let aid = id_at(ab, ANL_ID_AT);
        anl.insert(aid, a.name.clone());
        for s in &a.sheets {
            if let Some(r) = s.raw.as_deref() {
                blt.insert((aid, id_at(&r.block, BLT_ID_AT)), s.number);
            }
            for o in &s.objects {
                if let Some(r) = o.raw.as_deref().filter(|r| r.block.is("OBJ")) {
                    objs.insert(id_at(&r.block, 0), o.id);
                }
            }
        }
    }
    for o in p.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()) {
        let Some(r) = o.raw.as_deref_mut() else { continue };
        if !matches!(r.kind, Kind::Verweis { .. }) || (r.block.i32(15) >> 8) & 0xff == 0 {
            continue;
        }
        let (aid, bid, oid) = (id_at(&r.block, 0x20), id_at(&r.block, 0x28), id_at(&r.block, 0x30));
        let Some(name) = anl.get(&aid) else { continue };
        let z = VerweisZiel { anlage: name.clone(), blatt: blt.get(&(aid, bid)).copied().unwrap_or(0), obj: objs.get(&oid).copied().unwrap_or(0) };
        for k in [&mut r.kind, &mut o.kind] {
            if let Kind::Verweis { ziel, .. } = k {
                *ziel = Some(z.clone());
            }
        }
    }
}

/// Ankerpunkt eines Textes in seiner Textbox je nach Ausrichtung (wie das Zeichnen ihn erwartet).
pub fn text_anchor(mn: P, mx: P, align: Align, vertical: bool) -> P {
    if vertical {
        let y = match align {
            Align::Left => mx.y,
            Align::Center => (mn.y + mx.y) / 2.0,
            Align::Right => mn.y,
        };
        P::new((mn.x + mx.x) / 2.0, y)
    } else {
        let x = match align {
            Align::Left => mn.x,
            Align::Center => (mn.x + mx.x) / 2.0,
            Align::Right => mx.x,
        };
        P::new(x, (mn.y + mx.y) / 2.0)
    }
}

/// Anlageneinstellungen aus einem ANL-Block (Vergleich beim Zurückschreiben).
pub fn anl_settings(a: &Block) -> AnlSettings {
    AnlSettings {
        kurz: txt_at(&a.data, 55, 66),
        von: txt_at(&a.data, 66, 112),
        znr: txt_at(&a.data, 116, 136),
        format: (a.i32(38), a.i32(39)),
        raender: [a.i32(40), a.i32(41), a.i32(42), a.i32(43)],
        pfade: a.i32(44),
        pfad_hoehe: a.i32(45),
        ab_null: a.i32(49) != 0,
        size_titel: a.i32(46),
        size_wert: a.i32(47),
        size_verbg: a.i32(48),
        size_pfad: a.i32(50),
    }
}

/// Projektname aus dem PRJ-Block (wie beim Import gelesen).
pub fn prj_name(prj: &Block) -> String {
    txt_at(&prj.data, 0, 40)
}

/// Name (20 Zeichen) eines LTG/POT-Blocks.
pub fn name20(b: &Block) -> String {
    txt_at(&b.data, 0, 20)
}

/// Projektsymbol (SYM-Block) in eine Symboldefinition umsetzen (auch zum Erkennen geänderter Symbole beim Schreiben).
pub fn symdef_from_block(s: &Block) -> Option<SymDef> {
    let name = txt_at(&s.data, 0, 40);
    let bmk = txt_at(&s.data, 40, 60);
    let id = s.data.get(80..88).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0);
    let grd = s.kid("GRD")?;
    let texts: Vec<String> = grd.kid("STR").map(|n| symbols::split_strings(&n.data)).unwrap_or_default();
    let options: Vec<String> = grd.kid("SEI").map(|n| symbols::split_strings(&n.data)).unwrap_or_default();
    let elems: Vec<u8> = grd.kids_of("GRS").flat_map(|g| g.data.iter().copied()).collect();
    // Zusatzoptionen (OPT: Feld 1 = Nummer, Name im STR-Unterblock)
    // Alternativtexte: ALT (Feld 0 = Textnummer, Feld 1 = Satz), Ersatztext im STR-Unterblock (fehlt = leer)
    let mut alt_sets: Vec<Vec<(u32, String)>> = vec![];
    for a in grd.kids_of("ALT") {
        let set = a.i32(1).clamp(1, 64) as usize;
        if alt_sets.len() < set {
            alt_sets.resize(set, vec![]);
        }
        alt_sets[set - 1].push((a.i32(0).max(0) as u32, a.kid("STR").map(|t| txt(&t.data)).unwrap_or_default()));
    }
    let mut flag_names: Vec<String> = vec![];
    for o in grd.kids_of("OPT") {
        let n = o.i32(1).clamp(1, 32) as usize;
        if flag_names.len() < n {
            flag_names.resize(n, String::new());
        }
        flag_names[n - 1] = o.kid("STR").map(|t| txt(&t.data)).unwrap_or_default();
    }
    // Parameter: PRA (Feld 0 = Nummer), erster STR = Name, zweiter STR = Vorgabewert
    let params: Vec<(u32, String, String)> = grd
        .kids_of("PRA")
        .map(|a| {
            let mut st = a.kids_of("STR").map(|t| txt(&t.data));
            (a.i32(0).max(0) as u32, st.next().unwrap_or_default(), st.next().unwrap_or_default())
        })
        .collect();
    let el = symbols::parse_elements(&elems, &texts);
    Some(SymDef {
        key: format!("epr:{id:016x}"),
        name,
        group: vec!["Projekt".into()],
        haupt: 0,
        fav: false,
        bmk,
        prims: el.prims,
        pins: el.pins,
        title: el.title,
        vis: el.vis,
        pin_vis: el.pin_vis,
        pin_info: el.pin_info,
        flag: el.flag,
        flag_names,
        text_idx: el.text_idx,
        alt_sets,
        raw: vec![],
        frg: crate::symbol_freigaben::aus_sym(&s.data).unwrap_or(crate::symbol_freigaben::VORGABE),
        params,
        value_pos: el.value_pos,
        text_mode: el.text_mode,
        fill: el.fill,
        pens: crate::symbol_stifte::aus_grd(grd),
        options,
    })
}
