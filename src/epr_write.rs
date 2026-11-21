//! Speichern im Originalformat (*.epr).
//!
//! Grundlage ist die beim Öffnen gelesene Originaldatei: Unveränderte Objekte werden mit ihrem
//! Originalblock Byte für Byte zurückgeschrieben, geänderte Objekte werden im Originalblock
//! nachgeführt (Lage, Texte, Drehung …), gelöschte entfallen, neue werden aus Vorlageblöcken erzeugt.
//! Alles, was der Nachbau nicht selbst darstellt (Klemmen, Sammelschienen, Bilder, Bauteile …),
//! bleibt unverändert erhalten.

use crate::eprfile::{cstr_bytes, put_str, Block, EprFile};
use crate::model::*;

/// Vorlage: Struktur eines leeren Projekts im Originalformat (aus einer Originaldatei abgeleitet,
/// Inhalte geleert). Wird für neue Projekte und neue Objekte ohne Originalblock benutzt.
fn template() -> EprFile {
    EprFile::parse(include_bytes!("../resources/template.epr")).expect("Vorlage template.epr ist gültig")
}

fn to01(v: f32) -> i32 {
    (v * 10.0).round() as i32
}

pub fn save(project: &Project, path: &std::path::Path) -> Result<(), String> {
    let bytes = to_bytes(project)?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn to_bytes(project: &Project) -> Result<Vec<u8>, String> {
    Ok(build(project)?.to_bytes())
}

pub fn build(project: &Project) -> Result<EprFile, String> {
    // Umriss (VSY Feld 32–35) für Symbole ohne gültigen Umriss aus der Symboldefinition berechnen
    let mut filled = project.clone();
    for o in filled.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()) {
        if let Kind::Symbol { ext, pos, .. } = &o.kind {
            if ext.bounds.is_none() && crate::symbols::find(&project.symdefs, match &o.kind { Kind::Symbol { key, .. } => key, _ => unreachable!() }).is_some() {
                let p = *pos;
                let (a, b) = crate::canvas::obj_bounds(o, &project.symdefs);
                if let Kind::Symbol { ext, .. } = &mut o.kind {
                    ext.bounds = Some((a.sub(p), b.sub(p)));
                }
            }
        }
    }
    let project = &filled;
    let tpl = template();
    let base = project.raw.as_deref().cloned().unwrap_or_else(|| tpl.clone());
    let mut out = base.clone();
    let prj_src = base.root("PRJ").ok_or("PRJ fehlt in der Vorlage")?;
    let tpl_prj = tpl.root("PRJ").unwrap();
    let mut prj = Block { kids: vec![], ..prj_src.clone() };
    if crate::epr::prj_name(prj_src) != project.name {
        put_str(&mut prj.data, 0, 40, &project.name);
    }
    // Erstellt/Geändert (Delphi-Datum im PRJ-Block): bei neuen Projekten das eigene Datum statt dem der Vorlage,
    // sonst nur, wenn es im Projekt geändert wurde
    let offs = crate::epr::prj_date_offsets(prj_src);
    let read = |o: usize| crate::epr::delphi_date_at(&prj_src.data, o);
    for (i, val) in [&project.erstellt, &project.geaendert].into_iter().enumerate() {
        let parsed = crate::epr::delphi_from_text(val).or_else(|| if project.raw.is_none() { crate::epr::delphi_from_text(&project.erstellt) } else { None });
        if let (Some(&o), Some(v)) = (offs.get(i), parsed) {
            if project.raw.is_none() || read(o).as_deref() != Some(val.as_str()) {
                prj.data[o..o + 8].copy_from_slice(&v.to_le_bytes());
            }
        }
    }

    // Symbole, die noch keinen SYM-Block im Projekt haben (aus der Bibliothek oder neu gezeichnet), anlegen
    let from_template = project.raw.is_none();
    let known: Vec<u64> = if from_template { vec![] } else { prj_src.kids_of("SYM").filter_map(sym_id).collect() };
    let mut new_syms: Vec<Block> = vec![];
    let mut ids: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let tpl_sym = tpl_prj.kid("SYM").cloned();
    let mut stamp = delphi_now();
    for o in project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        if let Kind::Symbol { key, .. } = &o.kind {
            if ids.contains_key(key) {
                continue;
            }
            if let Some(id) = key.strip_prefix("epr:").and_then(|h| u64::from_str_radix(h, 16).ok()).filter(|id| known.contains(id)) {
                ids.insert(key.clone(), id);
                continue;
            }
            let def = project.symdefs.iter().find(|d| &d.key == key);
            if let (Some(def), Some(tpl)) = (def, tpl_sym.as_ref()) {
                stamp += 1.0 / 86_400_000.0; // je Symbol eine Millisekunde später (Kennung = Zeitstempel wie im Original)
                let id = stamp.to_bits();
                new_syms.push(sym_block(def, tpl, id));
                ids.insert(key.clone(), id);
            }
        }
    }
    SYM_IDS.with(|m| *m.borrow_mut() = ids);
    // Kennungen, Projektbilder und Schienen für neue Blattobjekte (epr_neu)
    crate::epr_neu::vorbereiten(project, &base);

    // Unterblöcke des Projekts in Originalreihenfolge; Anlagen an der Stelle der ersten Anlage
    let mut anl_done = false;
    let mut sym_done = false;
    for k in &prj_src.kids {
        if k.is("SYM") && from_template {
            continue;
        }
        // im Symboleditor geänderte Projektsymbole neu schreiben (Kennung, Kopf und Parameter bleiben)
        if k.is("SYM") {
            if let Some(nb) = edited_sym(project, k) {
                prj.kids.push(nb);
                continue;
            }
        }
        if !sym_done && (k.is("ERS") || k.is("FUS") || k.is("ANL") || k.is("BLD")) {
            sym_done = true;
            prj.kids.extend(new_syms.drain(..));
        }
        if k.is("ANL") {
            if !anl_done {
                anl_done = true;
                for a in &project.anlagen {
                    prj.kids.push(anlage_block(project, a, tpl_prj)?);
                }
            }
            continue;
        }
        prj.kids.push(k.clone());
    }
    prj.kids.extend(new_syms.drain(..));
    if !anl_done {
        for a in &project.anlagen {
            prj.kids.push(anlage_block(project, a, tpl_prj)?);
        }
    }
    // Projekt-, Anlagen- und Blattwerte (Leitungen, Potentiale, Stifte, Schriftfeld, Strompfade …)
    crate::epr_projekt::write(project, &mut prj);
    crate::epr_neu::nachbereiten(&mut prj);
    *out.root_mut("PRJ").unwrap() = prj;
    Ok(out)
}

/// Neuer SYM-Block, falls das Projektsymbol zu `k` gegenüber der Datei geändert wurde.
fn edited_sym(project: &Project, k: &Block) -> Option<Block> {
    let id = sym_id(k)?;
    let key = format!("epr:{id:016x}");
    let def = project.symdefs.iter().find(|d| d.key == key)?;
    let orig = crate::epr::symdef_from_block(k)?;
    let same = |a: &crate::symbols::SymDef, b: &crate::symbols::SymDef| {
        a.name == b.name && a.bmk == b.bmk && a.prims == b.prims && a.pins == b.pins && a.title == b.title && a.vis == b.vis
            && a.pin_vis == b.pin_vis && a.options == b.options && a.flag == b.flag && a.flag_names == b.flag_names
            && a.alt_sets == b.alt_sets && a.text_mode == b.text_mode && a.fill == b.fill && a.value_pos == b.value_pos
            && a.params == b.params
    };
    if same(def, &orig) {
        return None;
    }
    let mut nb = sym_block(def, k, id);
    // Stifte (SIT) erzeugt der Nachbau nicht neu: aus dem Original übernehmen (Parameter PRA schreibt sym_block)
    if let (Some(og), Some(ng)) = (k.kid("GRD"), nb.kids.iter_mut().find(|b| b.is("GRD"))) {
        for c in og.kids.iter().filter(|c| c.is("SIT")) {
            ng.kids.push(c.clone());
        }
    }
    Some(nb)
}

fn anlage_block(project: &Project, a: &Anlage, tpl_prj: &Block) -> Result<Block, String> {
    let tpl_anl = tpl_prj.kid("ANL").ok_or("ANL fehlt in der Vorlage")?;
    let (src, orig_name) = match a.raw.as_deref() {
        Some((b, n)) => (b.clone(), n.clone()),
        None => (Block { kids: tpl_anl.kids.iter().filter(|k| !k.is("BLT")).cloned().collect(), ..tpl_anl.clone() }, String::new()),
    };
    let mut blk = Block { kids: vec![], ..src.clone() };
    if a.raw.is_none() || orig_name != a.name {
        put_str(&mut blk.data, 0, 55, &a.name);
    }
    let old_set = a.raw.as_deref().map(|(b, _)| crate::epr::anl_settings(b));
    // "Erstellt von" steht im Original je Anlage; der Projektwert gilt, wenn die Anlage keinen eigenen hat
    let von = if a.set.von.trim().is_empty() && a.raw.is_none() { project.von.clone() } else { a.set.von.clone() };
    if old_set.as_ref().map(|o| o.von.as_str()) != Some(von.as_str()) {
        put_str(&mut blk.data, 66, 46, &von);
    }
    if old_set.as_ref() != Some(&a.set) {
        let s = &a.set;
        put_str(&mut blk.data, 55, 11, &s.kurz);
        blk.set_i32(38, s.format.0);
        blk.set_i32(39, s.format.1);
        for (i, v) in s.raender.iter().enumerate() {
            blk.set_i32(40 + i, *v);
        }
        blk.set_i32(44, s.pfade);
        blk.set_i32(45, s.pfad_hoehe);
        blk.set_i32(46, s.size_titel);
        blk.set_i32(47, s.size_wert);
        blk.set_i32(48, s.size_verbg);
        blk.set_i32(49, s.ab_null as i32);
        blk.set_i32(50, s.size_pfad);
    }
    let tpl_blt = tpl_anl.kid("BLT").ok_or("BLT fehlt in der Vorlage")?;
    let mut done = false;
    for k in &src.kids {
        if k.is("BLT") {
            if !done {
                done = true;
                for s in &a.sheets {
                    blk.kids.push(sheet_block(project, s, tpl_blt));
                }
            }
            continue;
        }
        blk.kids.push(k.clone());
    }
    if !done {
        for s in &a.sheets {
            blk.kids.push(sheet_block(project, s, tpl_blt));
        }
    }
    Ok(blk)
}

fn sheet_block(project: &Project, s: &Sheet, tpl_blt: &Block) -> Block {
    let (src, map) = match s.raw.as_deref() {
        Some(r) => (r.block.clone(), r.map.clone()),
        None => (Block { kids: vec![], ..tpl_blt.clone() }, vec![]),
    };
    let mut blk = Block { kids: vec![], ..src.clone() };
    let changed_head = match s.raw.as_deref() {
        Some(r) => r.name != s.name || r.number != s.number || r.portrait != s.portrait || r.subindex != s.subindex,
        None => true,
    };
    if changed_head {
        put_str(&mut blk.data, 0, 40, &s.name);
        let old = blk.i32(23);
        blk.set_i32(23, (old & !0x00ff_ffff) | s.subindex as i32 | ((s.number as i32) << 8));
        // Format: Feld 24 = Rahmenhöhe in 0,1 mm (hoch 2770, quer 1800…1900), Feld 32 = 7 hoch / 6 quer (aus W2.epr)
        let portrait_now = blk.i32(24) > 2400;
        if s.raw.is_none() || portrait_now != s.portrait {
            let h = if s.portrait { 2770 } else if s.frame_h > 50.0 { (s.frame_h * 10.0).round() as i32 } else { 1800 };
            blk.set_i32(24, h);
            blk.set_i32(32, if s.portrait { 7 } else { 6 });
        }
    }
    // Blattdaten: neues Blatt = heute erstellt; geänderter Inhalt = heute geändert (wie das Original)
    let today = crate::epr::delphi_from_text(&crate::model::today()).unwrap_or(0.0);
    let changed = match s.raw.as_deref() {
        None => true,
        Some(r) => r.map.iter().filter(|m| m.is_some()).count() != s.objects.len() || s.objects.iter().any(|o| o.raw.as_ref().map(|x| x.kind != o.kind || x.pen != o.pen || x.fixed != o.fixed).unwrap_or(true)),
    };
    if blk.data.len() >= 128 {
        if s.raw.is_none() {
            let created = crate::epr::delphi_from_text(&s.erstellt).unwrap_or(today);
            blk.data[112..120].copy_from_slice(&created.to_le_bytes());
            // Kennung des Blattes (Zeitstempel) neu vergeben
            let stamp = new_sheet_id(s);
            blk.data[72..80].copy_from_slice(&stamp.to_le_bytes());
        }
        if changed {
            blk.data[120..128].copy_from_slice(&today.to_le_bytes());
        }
    }
    let by_id: std::collections::HashMap<u64, &Obj> = s.objects.iter().map(|o| (o.id, o)).collect();
    let mut written = std::collections::HashSet::new();
    for (i, k) in src.kids.iter().enumerate() {
        match map.get(i).copied().flatten() {
            Some(id) => {
                if let Some(o) = by_id.get(&id) {
                    if let Some(b) = object_block(project, o) {
                        blk.kids.push(b);
                    }
                    written.insert(id);
                }
            }
            None => blk.kids.push(k.clone()),
        }
    }
    for o in &s.objects {
        if !written.contains(&o.id) {
            if let Some(b) = object_block(project, o) {
                blk.kids.push(b);
            }
        }
    }
    blk
}

/// Block für ein Objekt: Original unverändert, verschoben oder aus den Daten neu erzeugt.
pub fn object_block(project: &Project, o: &Obj) -> Option<Block> {
    let mut b = object_block_raw(project, o)?;
    crate::epr_projekt::leitung_setzen(project, o, &mut b);
    Some(b)
}

fn object_block_raw(project: &Project, o: &Obj) -> Option<Block> {
    if let Some(r) = o.raw.as_deref() {
        if r.kind == o.kind && r.pen == o.pen && r.fixed == o.fixed {
            return Some(r.block.clone());
        }
        let mut b = r.block.clone();
        patch(&mut b, &r.kind, &o.kind, o.pen);
        if let (Kind::Verweis { ziel: alt, .. }, Kind::Verweis { ziel, .. }) = (&r.kind, &o.kind) {
            if alt != ziel {
                write_ziel(project, &mut b, ziel.as_ref());
            }
        }
        crate::epr_neu::patch_more(project, o, &r.kind, &mut b);
        if b.is("ZCN") {
            let v = b.i32(15);
            b.set_i32(15, if o.fixed { v | 0x1_0000 } else { v & !0x1_0000 });
        }
        return Some(b);
    }
    let mut b = new_block(project, o)?;
    crate::epr_neu::nachfuehren(project, o, &mut b);
    Some(b)
}

/// Verschiebung zwischen zwei Formen gleicher Art, falls sich nur die Lage geändert hat.
fn pure_shift(old: &Kind, new: &Kind) -> Option<P> {
    let d = first_point(new)?.sub(first_point(old)?);
    let mut moved = old.clone();
    moved.translate(d);
    (moved == *new).then_some(d)
}

fn first_point(k: &Kind) -> Option<P> {
    Some(match k {
        Kind::Line { a, .. } | Kind::Curve { a, .. } | Kind::Rect { a, .. } | Kind::Triangle { a, .. } | Kind::Circle { a, .. } => *a,
        Kind::TextBox { a, .. } | Kind::Table { a, .. } | Kind::Image { a, .. } => *a,
        Kind::Text { pos, .. } | Kind::Symbol { pos, .. } | Kind::Terminal { pos, .. } => *pos,
        Kind::Wire { pts, .. } | Kind::Poly { pts } => *pts.first()?,
        Kind::Verweis { pos, .. } => *pos,
        Kind::Busbar { a, .. } => *a,
    })
}

fn set_child_text(b: &mut Block, tag: &str, text: &str) {
    let data = cstr_bytes(&text.replace('\n', "\r\n"));
    match b.kids.iter_mut().find(|k| k.is(tag)) {
        Some(k) => k.data = data,
        None => b.kids.push(Block::new(tag, data)),
    }
}

/// Geänderte Werte in einen Originalblock übernehmen.
fn patch(b: &mut Block, old: &Kind, new: &Kind, pen: usize) {
    let shift = pure_shift(old, new);
    if b.is("ZCN") {
        if let Some(d) = shift {
            // Texte, Bilder und Tabellen haben nur zwei Punkte (Feld 4–7 sind Schrift, Spalten …)
            let n = if matches!(b.data.get(0x3c), Some(6 | 11 | 12 | 13)) { 4 } else { 8 };
            for i in 0..n {
                let v = b.i32(i);
                b.set_i32(i, v + if i % 2 == 0 { to01(d.x) } else { to01(d.y) });
            }
        } else {
            write_zcn_geometry(b, new);
        }
        if let Kind::Text { text, size, .. } = new {
            let tag = if b.kid("STL").is_some() { "STL" } else { "STR" };
            let old_text = if let Kind::Text { text, .. } = old { text.as_str() } else { "" };
            if old_text != text {
                set_child_text(b, tag, text);
            }
            if b.data[0x3c] == 11 {
                let old_size = if let Kind::Text { size, .. } = old { *size } else { 0.0 };
                if (old_size - size).abs() > 0.01 {
                    let pts = (size / 0.3528).round() as i32;
                    let v = b.i32(4);
                    b.set_i32(4, (v & !0xffff) | (pts & 0xffff));
                }
                let old_style = if let Kind::Text { style, align, .. } = old { Some((style.clone(), *align)) } else { None };
                if let Kind::Text { style, align, .. } = new {
                    if old_style.as_ref() != Some(&(style.clone(), *align)) {
                        write_text_style(b, style, *align);
                    }
                }
            }
        }
        b.set_i32(16, pen as i32);
        return;
    }
    if b.is("VSY") {
        if let Kind::Symbol { key, pos, rot, mirror_h, mirror_v, bmk, value, scale, option, title, flags, ext } = new {
            // auf eine andere Symboldefinition umgestellt ("Nur dieses" im Symboleditor): Kennung des SYM-Blocks
            if matches!(old, Kind::Symbol { key: ok, .. } if ok != key) && b.data.len() >= 112 {
                if let Some(id) = SYM_IDS.with(|m| m.borrow().get(key).copied()) {
                    b.data[104..112].copy_from_slice(&id.to_le_bytes());
                }
            }
            b.set_i32(23, *flags as i32);
            let old_ext = if let Kind::Symbol { ext, .. } = old { Some(ext.clone()) } else { None };
            if old_ext.as_ref() != Some(ext) {
                let v12 = b.i32(12) & !0x00ff_0000;
                b.set_i32(12, v12 | (ext.title_align as i32) << 16);
                let v15 = b.i32(15) & !0x00ff_0000;
                b.set_i32(15, v15 | (ext.value_align as i32) << 16);
                if let Some(vo) = ext.value_off {
                    b.set_i32(13, to01(vo.x));
                    b.set_i32(14, to01(vo.y));
                }
                b.set_i32(21, ext.alt as i32);
                if let Some((a, c)) = ext.bounds {
                    b.set_i32(32, to01(a.x));
                    b.set_i32(33, to01(a.y));
                    b.set_i32(34, to01(c.x));
                    b.set_i32(35, to01(c.y));
                }
                let v20 = b.i32(20) & !(1 | 1 << 24);
                b.set_i32(20, v20 | (!ext.querverweise) as i32 | (ext.sicherung as i32) << 24);
                let old_fkt = old_ext.as_ref().map(|e| e.funktion.as_str()).unwrap_or("");
                if old_fkt != ext.funktion {
                    if ext.funktion.is_empty() {
                        b.kids.retain(|k| !k.is("FKT"));
                    } else {
                        set_child_text(b, "FKT", &ext.funktion);
                    }
                }
                // Parameterwerte (SPA) und benutzerdefinierte Texte (ALT) der Instanz
                if old_ext.as_ref().map(|e| (&e.params, &e.texts)) != Some((&ext.params, &ext.texts)) {
                    crate::parameter::sync_vsy_kids(b, &ext.params, &ext.texts);
                }
                if old_ext.as_ref().map(|e| e.counter) != Some(ext.counter) {
                    b.set_i32(38, if ext.counter > 0 { ext.counter } else { -1 });
                }
            }
            b.set_i32(16, to01(pos.x));
            b.set_i32(17, to01(pos.y));
            let mut f = b.i32(18) & !0x1f;
            f |= match rot % 4 {
                1 => 1,
                2 => 2,
                3 => 4,
                _ => 0,
            };
            // Original: Bit 16 = Spiegeln an der senkrechten, Bit 8 an der waagerechten Achse (nach der Drehung)
            let (b16, b8) = mirror_to_bits(*rot, *mirror_h, *mirror_v);
            if b16 {
                f |= 16;
            }
            if b8 {
                f |= 8;
            }
            b.set_i32(18, f);
            if b.data.len() >= 80 {
                b.data[76..80].copy_from_slice(&scale.to_le_bytes());
            }
            b.set_i32(22, *option as i32);
            if let Some((t, vert)) = title {
                b.set_i32(10, to01(t.x));
                b.set_i32(11, to01(t.y));
                let v = b.i32(12) & !(1 << 24);
                b.set_i32(12, v | ((*vert as i32) << 24));
            }
            if let Kind::Symbol { bmk: ob, value: ov, .. } = old {
                if ob != bmk {
                    set_child_text(b, "TIT", bmk);
                }
                if ov != value {
                    set_child_text(b, "WRT", value);
                }
            }
        }
        return;
    }
    if b.is("VBG") {
        if let Kind::Wire { pts, .. } = new {
            let tpl = b.kids.iter().find(|k| k.is("VBP")).cloned().unwrap_or_else(|| Block::new("VBP", vec![0; 8]));
            let mut it = pts.iter();
            let mut kids = vec![];
            for k in &b.kids {
                if k.is("VBP") {
                    if let Some(p) = it.next() {
                        let mut k = k.clone();
                        k.set_i32(0, to01(p.x));
                        k.set_i32(1, to01(p.y));
                        kids.push(k);
                    }
                } else {
                    kids.push(k.clone());
                }
            }
            for p in it {
                let mut k = tpl.clone();
                k.set_i32(0, to01(p.x));
                k.set_i32(1, to01(p.y));
                kids.push(k);
            }
            b.kids = kids;
        }
        return;
    }
    if b.is("OBJ") {
        // Klemmstelle: Lage (Feld 6/7) und Nummer (Feld 9/10)
        if let Kind::Terminal { pos, nr, .. } = new {
            b.set_i32(6, to01(pos.x));
            b.set_i32(7, to01(pos.y));
            b.set_i32(9, *nr as i32);
            b.set_i32(10, *nr as i32);
            return;
        }
        // Sammelschiene: Anfang (6/7), Ende x (10), Namensversatz (2/3)
        if let Kind::Busbar { a, x2, label, .. } = new {
            b.set_i32(6, to01(a.x));
            b.set_i32(7, to01(a.y));
            b.set_i32(10, to01(*x2));
            b.set_i32(2, to01(label.x));
            b.set_i32(3, to01(label.y));
            return;
        }
        if let Kind::Verweis { pos, text_pos, text, dir, vertical, .. } = new {
            b.set_i32(6, to01(pos.x));
            b.set_i32(7, to01(pos.y));
            b.set_i32(2, to01(text_pos.x) - to01(pos.x));
            b.set_i32(3, to01(text_pos.y) - to01(pos.y));
            if let Kind::Verweis { text: ot, dir: od, vertical: ov, .. } = old {
                if ot != text {
                    set_child_text(b, "TIT", text);
                }
                if od != dir || ov != vertical || ot.is_empty() != text.is_empty() {
                    let (a, bit) = verweis_winkel(b.i32(4) == 0x0101_ffff, text.is_empty(), *dir, *vertical);
                    b.set_i32(14, a);
                    let f = b.i32(15);
                    b.set_i32(15, if bit { f | 1 } else { f & !1 });
                }
            }
        }
    }
}

/// Richtung eines Verweises für OBJ Feld 14 (0,1°) und Feld 15 Bit 0 – Umkehrung von `crate::epr::verweis`.
pub fn verweis_winkel(cross: bool, ohne_text: bool, dir: u16, vertical: bool) -> (i32, bool) {
    let d = dir as i32 % 360;
    if ohne_text {
        return ((d + 180) % 360 * 10, vertical);
    }
    if cross {
        let a = (d + 90) % 360;
        return (a * 10, vertical ^ (a == 180));
    }
    let a = if vertical { (d + 270) % 360 } else { d };
    (a * 10, vertical)
}

/// Ziel eines Verweises in OBJ Feld 8–13 (Kennungen von Anlage, Blatt, Gegenverweis) und Feld 15 Byte 1
/// (Ziel gesetzt) schreiben. Kennungen gibt es nur für Teile, die aus der .epr stammen (bzw. für neue Blätter
/// die beim Speichern vergebene); was sich nicht auflösen lässt, bleibt 0.
pub(crate) fn write_ziel(project: &Project, b: &mut Block, ziel: Option<&crate::model::VerweisZiel>) {
    let id = |blk: &Block, at: usize| blk.data.get(at..at + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0);
    let mut ids = [0u64; 3];
    if let Some(z) = ziel {
        if let Some(a) = project.anlagen.iter().find(|a| a.name == z.anlage) {
            ids[0] = a.raw.as_deref().map(|(blk, _)| id(blk, crate::epr::ANL_ID_AT)).unwrap_or_else(|| crate::epr_neu::anlage_kennung(project, a));
            if let Some(s) = a.sheets.iter().find(|s| s.number == z.blatt) {
                ids[1] = match s.raw.as_deref() {
                    Some(r) => id(&r.block, crate::epr::BLT_ID_AT),
                    None => new_sheet_id(s).to_bits(),
                };
            }
        }
        if let Some((_, o)) = project.find_obj(z.obj).filter(|_| z.obj != 0) {
            ids[2] = crate::epr_neu::kennung(o);
        }
    }
    if b.data.len() < 64 {
        b.data.resize(64, 0);
    }
    for (i, v) in ids.iter().enumerate() {
        b.data[0x20 + i * 8..0x28 + i * 8].copy_from_slice(&v.to_le_bytes());
    }
    let f = b.i32(15) & !0xff00;
    b.set_i32(15, f | if ids[0] != 0 { 0x100 } else { 0 });
}

/// Kennung eines neuen Blattes (Zeitstempel wie im Original, je Blattnummer eine Sekunde später).
fn new_sheet_id(s: &Sheet) -> f64 {
    crate::epr::delphi_from_text(&crate::model::today()).unwrap_or(0.0) + (s.number as f64) / 86_400.0
}

/// Texteinstellungen in ein Textelement (ZCN Typ 11) schreiben; unbekannte Bits bleiben erhalten.
fn write_text_style(b: &mut Block, st: &TextStyle, align: Align) {
    let a = match align {
        Align::Left => 0,
        Align::Center => 1,
        Align::Right => 2,
    };
    let f6 = b.i32(6) & 0x00fe_ff00;
    b.set_i32(6, f6 | a | if st.frame { 0x1_0000 } else { 0 } | if st.bold { 1 << 24 } else { 0 });
    let f7 = b.i32(7) & !0xffff;
    b.set_i32(7, f7 | st.italic as i32 | (st.underline as i32) << 8);
    b.set_i32(8, st.color[0] as i32 | (st.color[1] as i32) << 8 | (st.color[2] as i32) << 16);
    b.set_i32(5, st.rotation as i32);
}

/// Geometrie eines Zeichenelements (Punkte in 0,1 mm) und Typbyte setzen.
fn write_zcn_geometry(b: &mut Block, k: &Kind) {
    if b.data.len() < 0x50 {
        b.data.resize(0x50, 0);
    }
    let mut set = |i: usize, p: P| {
        b.set_i32(2 * i, to01(p.x));
        b.set_i32(2 * i + 1, to01(p.y));
    };
    let ty: u8 = match k {
        Kind::Line { a, b: e } => {
            set(0, *a);
            set(1, *e);
            1
        }
        Kind::Curve { a, c1, c2, b: e } => {
            set(0, *a);
            set(1, *c1);
            set(2, *c2);
            set(3, *e);
            2
        }
        Kind::Rect { a, b: e } => {
            set(0, *a);
            set(1, P::new(e.x, a.y));
            set(2, *e);
            set(3, P::new(a.x, e.y));
            3
        }
        Kind::Poly { pts } if pts.len() == 4 => {
            for (i, p) in pts.iter().enumerate() {
                set(i, *p);
            }
            3
        }
        Kind::Poly { pts } | Kind::Wire { pts, .. } => {
            for (i, p) in pts.iter().take(3).enumerate() {
                set(i, *p);
            }
            4
        }
        Kind::Triangle { a, b: e } => {
            let t = crate::canvas::triangle_pts(*a, *e);
            for (i, p) in t.iter().enumerate() {
                set(i, *p);
            }
            4
        }
        Kind::Circle { a, b: e } => {
            set(0, *a);
            set(1, *e);
            5
        }
        Kind::Text { style: TextStyle { rect: Some((a, e)), .. }, .. } => {
            set(0, *a);
            set(1, *e);
            11
        }
        Kind::Text { pos, text, size, vertical, .. } => {
            // Textrahmen um die Einfügeposition (Import nimmt links/mittig bzw. bei hochkant mittig/unten)
            let lines = text.lines().count().max(1) as f32;
            // großzügig wie das Original: zu knappe Rahmen schneiden dort die Unterlängen ab (Gegenprobe "Testtext")
            // plus Rand: bei einzelnen Zeichen ("+", "S") zeigte das Original sonst nur einen Punkt
            let w = size * 0.7 * text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32 + size * 0.8;
            let h = size * 1.6 * lines;
            if *vertical {
                set(0, P::new(pos.x - h / 2.0, pos.y - w));
                set(1, P::new(pos.x + h / 2.0, pos.y));
            } else {
                set(0, P::new(pos.x, pos.y - h / 2.0));
                set(1, P::new(pos.x + w, pos.y + h / 2.0));
            }
            11
        }
        _ => b.data[0x3c],
    };
    b.data[0x3c] = ty;
    if let Kind::Text { size, vertical, style, align, .. } = k {
        let pts = (size / 0.3528).round() as i32;
        let v = b.i32(4);
        b.set_i32(4, (v & !0xffff) | (pts & 0xffff));
        let mut st = style.clone();
        if st.rotation == 0 && *vertical {
            st.rotation = 90;
        }
        write_text_style(b, &st, *align);
    }
}

/// Neuer Block für ein Objekt ohne Original (aus den Vorlageblöcken).
fn new_block(project: &Project, o: &Obj) -> Option<Block> {
    let tpl = template();
    let blt = tpl.root("PRJ")?.kid("ANL")?.kid("BLT")?;
    match &o.kind {
        Kind::Symbol { key, .. } => {
            let _ = project;
            let id = SYM_IDS.with(|m| m.borrow().get(key).copied())?;
            let mut b = blt.kid("VSY")?.clone();
            b.kids.retain(|k| !k.is("TIT") && !k.is("WRT"));
            b.data[104..112].copy_from_slice(&id.to_le_bytes());
            let old = Kind::Symbol { key: key.clone(), pos: P::default(), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() };
            patch(&mut b, &old, &o.kind, o.pen);
            Some(b)
        }
        Kind::Wire { .. } => {
            let mut b = blt.kid("VBG")?.clone();
            b.kids.retain(|k| !k.is("VBP"));
            patch(&mut b, &Kind::Wire { pts: vec![], leitung: 0, bez: vec![], opts: 0 }, &o.kind, o.pen);
            Some(b)
        }
        Kind::Text { .. } => {
            let mut b = blt.kids_of("ZCN").find(|z| z.data.get(0x3c) == Some(&11))?.clone();
            b.kids.retain(|k| !k.is("STL") && !k.is("STR"));
            write_zcn_geometry(&mut b, &o.kind);
            if let Kind::Text { text, .. } = &o.kind {
                set_child_text(&mut b, "STL", text);
            }
            b.set_i32(16, o.pen as i32);
            Some(b)
        }
        Kind::Line { .. } | Kind::Curve { .. } | Kind::Rect { .. } | Kind::Poly { .. } | Kind::Triangle { .. } | Kind::Circle { .. } => {
            let mut b = blt.kids_of("ZCN").find(|z| z.data.get(0x3c) != Some(&11))?.clone();
            b.kids.clear();
            write_zcn_geometry(&mut b, &o.kind);
            b.set_i32(16, o.pen as i32);
            Some(b)
        }
        // Textbox, Tabelle, Bild, Verweis, Sammelschiene (Klemmstelle: noch nicht)
        _ => crate::epr_neu::new_block(project, o),
    }
}

thread_local! {
    /// Kennungen (SYM) je Symbolschlüssel für den laufenden Speichervorgang
    static SYM_IDS: std::cell::RefCell<std::collections::HashMap<String, u64>> = Default::default();
}

fn sym_id(s: &Block) -> Option<u64> {
    s.data.get(80..88).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

/// Jetzt als Delphi-Datum (Tage seit 30.12.1899) – das Original kennzeichnet Symbole mit ihrem Erstellungszeitpunkt.
fn delphi_now() -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let now = chrono::Local::now().naive_local();
    (now - base).num_milliseconds() as f64 / 86_400_000.0
}

/// Zeichenelemente eines Symbols im Originalformat: aus den Bibliotheksdaten oder aus den Grundelementen erzeugt.
/// Rückgabe: Elemente (je 0x50 Byte), Texte (Nr. 1 = BMK-Vorlage), Bildnamen.
pub fn sym_elements(def: &crate::symbols::SymDef) -> (Vec<u8>, Vec<String>, Vec<String>) {
    let d = &def.raw;
    if d.len() >= 0x3c && &d[0..3] == b"Ver" {
        let cnt = crate::eprfile::i32_at(d, 0x14).max(0) as usize;
        let tp = 0x3c + cnt * 0x50;
        if tp <= d.len() {
            let mut texts = vec![];
            let mut opts = vec![];
            if let Some(n) = d.get(tp..tp + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize) {
                if let Some(t) = d.get(tp + 4..tp + 4 + n) {
                    texts = crate::symbols::split_strings(t);
                    let op = tp + 4 + n;
                    if let Some(m) = d.get(op..op + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize) {
                        if let Some(o) = d.get(op + 4..op + 4 + m) {
                            opts = crate::symbols::split_strings(o);
                        }
                    }
                }
            }
            return (d[0x3c..tp].to_vec(), texts, opts);
        }
    }
    encode_elements(def)
}

/// Grundelemente in Elemente zu je 0x50 Byte umsetzen (Typ bei 0x3c, Stiftart 0x40, Bild/Modus 0x44/0x46, Zusatzoption 0x48).
fn encode_elements(def: &crate::symbols::SymDef) -> (Vec<u8>, Vec<String>, Vec<String>) {
    use crate::symbols::Prim;
    let mut out = vec![];
    let mut texts = vec![def.bmk.clone()];
    let mut el = |ty: u8, pts: &[P], style: u8, vis: (u16, u16), flag: u32, extra: &[(usize, i32)]| {
        let mut e = vec![0u8; 0x50];
        for (i, p) in pts.iter().take(4).enumerate() {
            e[i * 8..i * 8 + 4].copy_from_slice(&to01(p.x).to_le_bytes());
            e[i * 8 + 4..i * 8 + 8].copy_from_slice(&to01(p.y).to_le_bytes());
        }
        for (idx, v) in extra {
            e[idx * 4..idx * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        e[0x3c] = ty;
        e[0x40..0x44].copy_from_slice(&(style as i32).to_le_bytes());
        e[0x44..0x46].copy_from_slice(&vis.0.to_le_bytes());
        e[0x46..0x48].copy_from_slice(&vis.1.to_le_bytes());
        e[0x48..0x4c].copy_from_slice(&(flag as i32).to_le_bytes());
        e[0x4c..0x50].copy_from_slice(&(!(flag as i32)).to_le_bytes());
        out.extend_from_slice(&e);
    };
    if let Some(t) = def.title {
        el(9, &[t], 0, (0, 1), 0, &[(7, 1), (9, 1)]);
    }
    if let Some(t) = def.value_pos {
        el(10, &[t], 0, (0, 1), 0, &[]);
    }
    let fill = |i: usize| def.fill.get(i).copied().unwrap_or(false) as i32;
    for (i, pr) in def.prims.iter().enumerate() {
        let vis = def.vis.get(i).copied().unwrap_or((0, 1));
        let flag = def.flag.get(i).copied().unwrap_or(0);
        match pr {
            Prim::Line(a, b, st) => el(1, &[*a, *b], *st, vis, flag, &[]),
            Prim::Bezier(a, b, c, d, st) => el(2, &[*a, *b, *c, *d], *st, vis, flag, &[]),
            Prim::Poly(v, st) if v.len() == 4 => el(3, v, *st, vis, flag, &[(10, fill(i))]),
            Prim::Poly(v, st) => {
                // Linienzug in Dreiecke/Linien zerlegen (das Format kennt nur 3- und 4-Ecke)
                if v.len() == 3 {
                    el(4, v, *st, vis, flag, &[(10, fill(i))]);
                } else {
                    for w in v.windows(2) {
                        el(1, w, *st, vis, flag, &[]);
                    }
                }
            }
            Prim::Ellipse(a, b, st) => el(5, &[*a, *b], *st, vis, flag, &[(10, fill(i))]),
            Prim::Arc(c, r, a0, a1) => {
                let n = 8;
                let pts: Vec<P> = (0..=n).map(|k| {
                    let a = (a0 + (a1 - a0) * k as f32 / n as f32).to_radians();
                    P::new(c.x + r * a.cos(), c.y + r * a.sin())
                }).collect();
                for w in pts.windows(2) {
                    el(1, w, 0, vis, flag, &[]);
                }
            }
            Prim::TextBox(a, b, s, al) => {
                // Textnummer beibehalten (Alternativtexte beziehen sich darauf)
                let want = def.text_idx.get(i).copied().unwrap_or(0) as usize;
                let n = if want >= 1 {
                    if texts.len() < want {
                        texts.resize(want, String::new());
                    }
                    texts[want - 1] = s.clone();
                    want
                } else {
                    texts.push(s.clone());
                    texts.len()
                };
                let al = match al {
                    Align::Left => 0,
                    Align::Center => 1,
                    Align::Right => 2,
                };
                let mode = def.text_mode.get(i).copied().unwrap_or(0) as i32;
                el(6, &[*a, *b], 0, vis, flag, &[(4, al), (7, n as i32), (8, mode), (9, s.lines().count().max(1) as i32)]);
            }
            Prim::Text(c, s, size) => {
                texts.push(s.clone());
                let (a, b) = (P::new(c.x - size * 0.3 * s.chars().count() as f32, c.y - size / 2.0), P::new(c.x + size * 0.3 * s.chars().count() as f32, c.y + size / 2.0));
                el(6, &[a, b], 0, vis, flag, &[(4, 1), (7, texts.len() as i32), (9, 1)]);
            }
        }
    }
    for (i, p) in def.pins.iter().enumerate() {
        let vis = def.pin_vis.get(i).copied().unwrap_or((0, 1));
        // Feld 5: Textnummer der Anschlussbezeichnung; Zusatzoption wie bei den Grundelementen
        let (t, flag) = def.pin_info.get(i).copied().unwrap_or((0, 0));
        el(7, &[*p, *p], 0, vis, flag, &[(5, t as i32)]);
    }
    (out, texts, def.options.clone())
}

/// SYM-Block (Projektsymbol) aus einer Symboldefinition nach dem Muster der Vorlage.
fn sym_block(def: &crate::symbols::SymDef, tpl: &Block, id: u64) -> Block {
    let mut b = Block { kids: vec![], ..tpl.clone() };
    put_str(&mut b.data, 0, 40, &def.name);
    put_str(&mut b.data, 40, 20, &def.bmk);
    if b.data.len() < 88 {
        b.data.resize(88, 0);
    }
    b.data[80..88].copy_from_slice(&id.to_le_bytes());
    let (els, texts, opts) = sym_elements(def);
    let mut grd = tpl.kid("GRD").map(|g| Block { kids: vec![], ..g.clone() }).unwrap_or_else(|| Block::new("GRD", vec![100, 0, 0, 0, 25, 0, 0, 0]));
    grd.kids.push(Block::new("GRS", els));
    let mut strs = vec![];
    for t in &texts {
        strs.extend(crate::eprfile::cstr_bytes(t));
    }
    grd.kids.push(Block::new("STR", strs));
    if !opts.is_empty() {
        let mut o = vec![];
        for t in &opts {
            o.extend(crate::eprfile::cstr_bytes(t));
        }
        grd.kids.push(Block::new("SEI", o));
    }
    // Alternativtexte (ALT: Textnummer, Satz; Ersatztext als STR, leer = ohne STR) und Optionen (OPT: 0, Bit; Name als STR)
    for (si, set) in def.alt_sets.iter().enumerate() {
        for (idx, text) in set {
            let mut d = (*idx as i32).to_le_bytes().to_vec();
            d.extend_from_slice(&(si as i32 + 1).to_le_bytes());
            let mut alt = Block::new("ALT", d);
            if !text.is_empty() {
                alt.kids.push(Block::new("STR", crate::eprfile::encode_cp1252(text)));
            }
            grd.kids.push(alt);
        }
    }
    for (bi, name) in def.flag_names.iter().enumerate() {
        if name.is_empty() {
            continue;
        }
        let mut d = 0i32.to_le_bytes().to_vec();
        d.extend_from_slice(&(bi as i32 + 1).to_le_bytes());
        let mut opt = Block::new("OPT", d);
        opt.kids.push(Block::new("STR", crate::eprfile::encode_cp1252(name)));
        grd.kids.push(opt);
    }
    // Parameter (PRA: Nummer; STR Name, STR Vorgabe)
    grd.kids.extend(crate::parameter::pra_blocks(&def.params));
    b.kids.push(grd);
    b
}

/// Objekte, die beim Speichern im .epr-Format nicht geschrieben werden können.
pub fn unsupported(project: &Project) -> Vec<String> {
    let _ = build(project);
    let mut v = vec![];
    for a in &project.anlagen {
        for s in &a.sheets {
            for o in &s.objects {
                if o.raw.is_none() && new_block(project, o).is_none() {
                    v.push(format!("{} / {}: {}", a.name, s.name, kind_name(&o.kind)));
                }
            }
        }
    }
    v
}

pub fn kind_name(k: &Kind) -> &'static str {
    match k {
        Kind::Line { .. } => "Linie",
        Kind::Curve { .. } => "Kurve",
        Kind::Rect { .. } => "Rechteck",
        Kind::Triangle { .. } => "Dreieck",
        Kind::Circle { .. } => "Kreis",
        Kind::Text { .. } => "Text",
        Kind::TextBox { .. } => "Textbox",
        Kind::Table { .. } => "Tabelle",
        Kind::Image { .. } => "Bild",
        Kind::Symbol { .. } => "Symbol",
        Kind::Poly { .. } => "Polygon",
        Kind::Wire { .. } => "Verbindung",
        Kind::Terminal { .. } => "Klemmstelle",
        Kind::Verweis { .. } => "Verweis",
        Kind::Busbar { .. } => "Sammelschiene",
    }
}
