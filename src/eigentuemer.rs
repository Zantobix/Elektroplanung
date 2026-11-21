//! Eigentümer eines Symbols (VSY › OWR), z. B. die Spule eines Relaiskontakts.
//!
//! Belegt aus dem Original:
//! - VSY › OWR (24 Byte, letzter Unterblock): Byte 0–7 Kennung des Eigentümer-Symbols (VSY Byte 96), 8–15 Kennung
//!   seines Blattes (BLT Byte 72), 16–23 Kennung seiner Anlage (ANL Byte 0x88). W2.epr enthält drei solche
//!   Kontakte (je ein Schließer neben der Spule -K1 in UV1, UV2 und UV3).
//! - Zeichnen (0x438b94 ab 0x43b4xx): hat ein Symbol einen Eigentümer (Laufzeit +0xb4), bildet die
//!   Textroutine 0x46e9f4 (Art 1) die Kennzeichnung aus der des Eigentümers (rekursiv mit dessen Blatt und Anlage)
//!   und hängt die Lage des Eigentümers an. Ausdruck „Elektroplanung Wohnung 2“ Seite 10/18/29 und Aufnahme des
//!   Originals: am Kontakt steht „-K1“ statt der eigenen Vorlage „-Q1+%P“ und darunter kleiner „/5“ (Pfad der Spule
//!   auf demselben Blatt), mittig unter der Kennzeichnung (Grundlinie 2 mm tiefer, Schrift etwa 60 %).
//! - Kontaktspiegel › Übernehmen und Eigentümer › Setzen machen das angeklickte Symbol zum Eigentümer; die eigene
//!   Kennzeichnung des Kontakts bleibt dabei stehen.
//!
//! Die Lage lautet „/Pfad“ auf demselben Blatt, sonst „Blatt/Pfad“ (Aufnahme des Kontaktspiegels: „/6“ und „3/11“).
//! Ob die Freigabe „Querverweise“ den Verweis unter dem Kontakt abschaltet, ist im Dekompilat nicht eindeutig; der
//! Nachbau richtet sich nach der Freigabe des Kontakts.

use crate::canvas::{bmk_ctx, times, DrawCtx};
use crate::eprfile::Block;
use crate::model::*;
use crate::symbols::{self, BmkCtx};
use eframe::egui::{self, Align2, Color32, Pos2, Vec2};

/// Länge des OWR-Blocks
pub const OWR_LEN: usize = 24;
/// Schriftgröße des Querverweises unter der Kennzeichnung (mm) und Abstand der Mitten (mm), gemessen am Ausdruck
pub const QV_SCHRIFT: f32 = 1.5;
pub const QV_ABSTAND: f32 = 2.3;
/// höchstens so viele Stufen (Eigentümer mit eigenem Eigentümer) – schützt vor Ringen
const STUFEN: usize = 4;

fn id_at(d: &[u8], at: usize) -> u64 {
    d.get(at..at + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}

/// Kennung des Eigentümers aus VSY › OWR (Byte 0–7).
pub fn owr_kennung(b: &Block) -> Option<u64> {
    b.kid("OWR").map(|w| id_at(&w.data, 0)).filter(|k| *k != 0)
}

/// Nach dem Einlesen: OWR über die Symbolkennungen (VSY Byte 96) dem Eigentümer-Objekt zuordnen – in der Form und
/// in der Originalform (`RawObj::kind`), damit unveränderte Dateien byteidentisch bleiben. Ein OWR auf ein nicht
/// vorhandenes Symbol bleibt unverknüpft (und im Block erhalten).
pub fn verknuepfen(p: &mut Project) {
    let mut ids: std::collections::HashMap<u64, u64> = Default::default();
    for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        if let Some(r) = o.raw.as_deref().filter(|r| r.block.is("VSY")) {
            let k = id_at(&r.block.data, 96);
            if k != 0 {
                ids.insert(k, o.id);
            }
        }
    }
    for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects) {
        let id = o.id;
        let Some(raw) = o.raw.as_deref_mut().filter(|r| r.block.is("VSY")) else { continue };
        let Some(w) = owr_kennung(&raw.block).and_then(|k| ids.get(&k).copied()).filter(|w| *w != id) else { continue };
        for kind in [&mut o.kind, &mut raw.kind] {
            if let Kind::Symbol { ext, .. } = kind {
                ext.eigentuemer = w;
            }
        }
    }
}

/// Unmittelbarer Eigentümer eines Symbols (muss ein Symbol im Projekt sein).
fn schritt<'a>(p: &'a Project, o: &Obj) -> Option<(SheetRef, &'a Obj)> {
    let Kind::Symbol { ext, .. } = &o.kind else { return None };
    if ext.eigentuemer == 0 || ext.eigentuemer == o.id {
        return None;
    }
    p.find_obj(ext.eigentuemer).filter(|(_, w)| matches!(w.kind, Kind::Symbol { .. }))
}

/// Letzter Eigentümer in der Kette (None = keiner).
pub fn eigentuemer_von<'a>(p: &'a Project, o: &Obj) -> Option<(SheetRef, &'a Obj)> {
    let mut cur = schritt(p, o)?;
    for _ in 1..STUFEN {
        match schritt(p, cur.1) {
            Some(n) if n.1.id != o.id => cur = n,
            _ => break,
        }
    }
    Some(cur)
}

/// Kennzeichnung, die ein Symbol mit Eigentümer zeigt (die des Eigentümers, aufgelöst auf dessen Blatt);
/// None ohne Eigentümer.
pub fn kennzeichnung(p: &Project, o: &Obj) -> Option<String> {
    let (r, w) = eigentuemer_von(p, o)?;
    let Kind::Symbol { bmk, ext, .. } = &w.kind else { return None };
    let a = p.anlagen.get(r.anlage)?;
    Some(symbols::display_bmk(bmk, ext.counter, &bmk_ctx(a, a.sheets.get(r.sheet)?)))
}

/// Angezeigte Kennzeichnung eines Symbols: die des Eigentümers, sonst die eigene mit `ctx`.
pub fn anzeige_bmk(p: &Project, o: &Obj, ctx: &BmkCtx) -> String {
    kennzeichnung(p, o).unwrap_or_else(|| match &o.kind {
        Kind::Symbol { bmk, ext, .. } => symbols::display_bmk(bmk, ext.counter, ctx),
        _ => String::new(),
    })
}

/// Lage eines Symbols von Blatt `hier` aus gesehen: „/Pfad“ auf demselben Blatt, sonst „Blatt/Pfad“.
pub fn lage_von(p: &Project, id: u64, hier: Option<SheetRef>) -> Option<String> {
    let (r, o) = p.find_obj(id)?;
    let Kind::Symbol { pos, .. } = &o.kind else { return None };
    let s = p.sheet(r)?;
    Some(if hier == Some(r) { format!("/{}", crate::epr::column_at(s, pos.x)) } else { crate::berechnet::blatt_pfad(s, pos.x) })
}

/// Querverweis unter einem Symbol mit Eigentümer: Lage des Eigentümers (nur mit Freigabe „Querverweise“).
pub fn querverweis(p: &Project, o: &Obj, hier: Option<SheetRef>) -> Option<String> {
    let Kind::Symbol { ext, .. } = &o.kind else { return None };
    if !ext.querverweise {
        return None;
    }
    let (_, w) = eigentuemer_von(p, o)?;
    lage_von(p, w.id, hier)
}

/// Blatt, das gerade gezeichnet wird (aus Anlage und Blattnummer des Zeichenkontexts).
pub fn blatt_des_dc(dc: &DrawCtx) -> Option<SheetRef> {
    let a = dc.anlage?;
    let ai = dc.project.anlagen.iter().position(|x| std::ptr::eq(x, a))?;
    let si = a.sheets.iter().position(|s| s.number == dc.bmk.sheet)?;
    Some(SheetRef { anlage: ai, sheet: si })
}

/// Querverweis mittig unter dem Kennzeichnungstext (`mn`/`mx` = Textrechteck in mm; `senkrecht` = Text um −90°
/// gedreht, „unter“ ist dann rechts).
pub fn zeichne_querverweis(dc: &DrawCtx, text: &str, mn: P, mx: P, senkrecht: bool, farbe: Color32) {
    let (p, xf) = (dc.painter, dc.xf);
    let g = p.layout_no_wrap(text.to_string(), times(QV_SCHRIFT * xf.z), farbe);
    let m = P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
    if senkrecht {
        let c = xf.s(P::new(m.x + QV_ABSTAND, m.y));
        let at = Pos2::new(c.x - g.size().y / 2.0, c.y + g.size().x / 2.0);
        p.add(egui::epaint::TextShape::new(at, g, farbe).with_angle(-std::f32::consts::FRAC_PI_2));
    } else {
        let c = xf.s(P::new(m.x, m.y + QV_ABSTAND));
        p.galley(c - Vec2::new(g.size().x / 2.0, g.size().y / 2.0), g, farbe);
    }
}

/// Querverweis unter einer rechtsbündig an `anker` stehenden Kennzeichnung (Symbole ohne Titellage).
pub fn zeichne_querverweis_rechts(dc: &DrawCtx, text: &str, bmk: &str, anker: P, groesse: f32, farbe: Color32) {
    let z = dc.xf.z;
    let w = dc.painter.layout_no_wrap(bmk.to_string(), times(groesse * z), farbe).size().x / z;
    dc.painter.text(dc.xf.s(P::new(anker.x - w / 2.0, anker.y + QV_ABSTAND)), Align2::CENTER_CENTER, text, times(QV_SCHRIFT * z), farbe);
}

/// OWR eines VSY-Blocks setzen (Eigentümer `owner` = Objekt-Id) bzw. entfernen (0 oder nicht gefunden).
pub fn owr_schreiben(project: &Project, b: &mut Block, owner: u64) {
    b.kids.retain(|k| !k.is("OWR"));
    let Some((r, w)) = project.find_obj(owner).filter(|_| owner != 0) else { return };
    let k = crate::epr_neu::kennung(w);
    if k == 0 {
        return;
    }
    let (blatt, anlage) = crate::epr_neu::orte(project, r);
    let mut d = vec![0u8; OWR_LEN];
    for (i, v) in [k, blatt, anlage].iter().enumerate() {
        d[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
    }
    // letzter Unterblock wie in W2 (TIT, ALT …, OWR)
    b.kids.push(Block::new("OWR", d));
}

/// Eigentümer setzen (Kontaktspiegel › Übernehmen, Eigentümer › Setzen): die Symbole `ids` bekommen `owner` als
/// Eigentümer; Rückgabe: ob sich etwas geändert hat.
pub fn setzen(p: &mut Project, owner: u64, ids: &[u64]) -> bool {
    if p.find_obj(owner).is_none() {
        return false;
    }
    let mut changed = false;
    for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects) {
        if o.id == owner || !ids.contains(&o.id) {
            continue;
        }
        if let Kind::Symbol { ext, .. } = &mut o.kind {
            if ext.eigentuemer != owner {
                ext.eigentuemer = owner;
                changed = true;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, bmk: &str, x: f32) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "k".into(), pos: P::new(x, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn kennzeichnung_und_lage_des_eigentuemers() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1", 40.0));
        p.anlagen[0].sheets[0].objects.push(sym(2, "-Q1", 120.0));
        let mut s2 = Sheet::new(2);
        s2.objects.push(sym(3, "-Q2", 60.0));
        p.anlagen[0].sheets.push(s2);
        let hier = SheetRef { anlage: 0, sheet: 0 };
        let o2 = p.find_obj(2).unwrap().1.clone();
        assert_eq!((kennzeichnung(&p, &o2), querverweis(&p, &o2, Some(hier))), (None, None));
        assert!(setzen(&mut p, 1, &[2, 3]));
        assert!(!setzen(&mut p, 1, &[2, 3]));
        let o2 = p.find_obj(2).unwrap().1.clone();
        let o3 = p.find_obj(3).unwrap().1.clone();
        // eigene Vorlage bleibt, gezeigt wird die der Spule
        assert!(matches!(&o2.kind, Kind::Symbol { bmk, .. } if bmk == "-Q1"));
        assert_eq!(kennzeichnung(&p, &o2).as_deref(), Some("-K1"));
        // Spule auf demselben Blatt: „/Pfad“, vom anderen Blatt aus „Blatt/Pfad“
        assert_eq!(querverweis(&p, &o2, Some(hier)).as_deref(), Some("/1"));
        assert_eq!(querverweis(&p, &o3, Some(SheetRef { anlage: 0, sheet: 1 })).as_deref(), Some("1/1"));
        // ohne Freigabe „Querverweise“ kein Verweis
        if let Some(Obj { kind: Kind::Symbol { ext, .. }, .. }) = p.anlagen[0].sheets[1].objects.first_mut() {
            ext.querverweise = false;
        }
        let o3 = p.find_obj(3).unwrap().1.clone();
        assert_eq!(querverweis(&p, &o3, None), None);
        // Ring: Spule gehört dem Kontakt – keine Endlosschleife
        setzen(&mut p, 2, &[1]);
        let o1 = p.find_obj(1).unwrap().1.clone();
        assert!(kennzeichnung(&p, &o1).is_some());
    }
}
