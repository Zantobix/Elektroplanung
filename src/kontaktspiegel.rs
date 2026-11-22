//! Kontaktspiegel (Kontextmenü „Kontaktspiegel hinzufügen“, Dialog TSSpiegel, Anleitung 7.5): unter einer Spule
//! die zugehörigen Kontakte als kleine Schaltzeichen (Schließer, Öffner, Wechsler) mit Anschlussnummern und der
//! Lage des jeweiligen Kontakts („Blatt/Pfad“). Der Spiegel ist ein eigenes, verknüpftes Blattobjekt
//! (`Kind::Spiegel`): Lage, Nummern und Fundstellen werden bei jedem Zeichnen aus den Kontakten berechnet, und
//! `nachfuehren` nimmt neue Kontakte mit gleicher Kennzeichnung auf bzw. entfernt gelöschte.
//!
//! Belegt aus dem Original:
//! - Ablage (Speicherroutine 0x4228c4 ab 0x424fe2): OBJ mit Objektart 5 (Byte 0x14), Feld 0/1 eigene Kennung,
//!   Feld 4 = 0xFFFF, Feld 6 = X in 0,1 mm, Feld 7 = 0 (Y richtet sich nach dem Blatt), Feld 8/9 = Kennung der Spule
//!   (VSY Byte 96), Byte 0x2c Bit 0 = Zeilen vorhanden. Je Zeile ein Unterblock **STB** (0x1c Byte: Kennung des
//!   Kontakts, seines Blattes (BLT Byte 72) und seiner Anlage (ANL Byte 0x88), Byte 0x18 = Art 0 Schließer /
//!   1 Öffner / 2 Wechsler), darin je eingetragener Anschlussnummer ALT (8 Byte) mit Unterblock STR (ohne 0). W2.epr enthält einen
//!   Spiegel ohne Zeilen (Blatt „Sicherungen -F10, -F1 und -F2“), dessen Spule gelöscht ist.
//! - Übernehmen (0x4995ac): Zeilen ohne Eintrag entfallen; die Referenz einer Zeile ist der Kontakt im Projekt,
//!   dessen Eigentümer (VSY › OWR, `eigentuemer.rs`) damit die Spule wird.
//! - Lage (Zeichenroutine 0x438b94): vor dem Zeichnen setzt das Original BLT Feld 24 = Unterkante der Zeichenfläche
//!   (Blatthöhe − unterer Rand (ANL +0xac bzw. hoch +0xa4) − Schriftfeldhöhe) − größte Höhe (Feld 14) aller
//!   Kontaktspiegel des Blattes, und die X-Lage jedes Spiegels = X der Spule (VSY Feld 16). Der Spiegel steht also
//!   unmittelbar über dem Schriftfeld; W2: Blatt mit dem leeren Spiegel hat Feld 24 = 1870 = 1900 − 30.
//! - Zeichnen (0x43d4xx): Y beginnt bei Feld 24, je Zeile 3 mm; ein Wechsler bekommt davor 2 mm (außer in der
//!   ersten Zeile) und danach 0,5 mm mehr. Jede Zeile ist das Bibliothekssymbol in voller Größe (Maßstab 1, Drehung
//!   3), daher sind Schrift und Schaltzeichen so groß wie an einem Kontakt; ohne eigene Nummern zeigen sie die
//!   Anschlusstexte des referenzierten Symbols. Daneben steht die Lage des referenzierten Kontakts (Textroutine
//!   0x46e9f4 Art 0x18): „/Pfad“ auf demselben Blatt, sonst „Blatt/Pfad“. Gemessen an einer Aufnahme des Originals
//!   (14 Punkte/mm): Ziffern 1,65 mm hoch (Schrift ≈ 2,5 mm), Nummern 3,4 mm links/rechts der Mitte, Lage ab +7 mm.
//! - ALT (8 Byte) vor dem STR einer Anschlussnummer: Feld 0 = Spalte 1–3 (Übernehmen ruft die Listenroutine
//!   0x403084 mit Gitterspalte − 1 auf, Gitterspalten 2–4 = „1“–„3“, und hört bei der ersten leeren Spalte auf),
//!   Feld 1 = 0.
//! - Feld 14 (OBJ 0x38) = Höhe des Spiegels in 0,1 mm: Übernehmen zählt je Zeile 30, vor einem Wechsler (außer in
//!   der ersten Zeile) 25 dazu, mindestens 30 (W2: 30 beim Spiegel ohne Zeilen).
//! - Die X-Lage folgt der Spule beim Zeichnen (siehe oben); gelöscht wird der Spiegel mit ihr nicht (W2 enthält
//!   einen Spiegel, dessen Spule gelöscht ist) – dann bleibt die gespeicherte X-Lage (OBJ Feld 6).
//! - „Alles markieren“ (0x42a710) und Kopieren (0x41a3e4) lassen Kontaktspiegel aus; beim Einfügen eines Spiegels
//!   leert das Original Zeilen und Spule (0x45a3c8).
//! - Die Schaltzeichen nimmt das Original aus drei Bibliothekssymbolen der Programmeinstellungen (Schließer,
//!   Öffner, Wechsler); der Nachbau zeichnet eigene Zeichen (keine Grafik aus der Exe) in den gemessenen Maßen.

use crate::app::App;
use crate::canvas::{bmk_ctx, times, DrawCtx};
use crate::eprfile::{encode_cp1252, Block};
use crate::model::*;
use crate::symbols::{self, Prim, SymDef};
use eframe::egui::{self, Align2, Stroke};
use serde::{Deserialize, Serialize};

pub const ARTEN: [&str; 3] = ["Schließer", "Öffner", "Wechsler"];
/// Zeilenabstand (0x1e) und Zusatzabstände des Wechslers (0x14 davor, 5 danach) in mm
pub const ZEILE: f32 = 3.0;
const WECHSLER_VOR: f32 = 2.0;
const WECHSLER_NACH: f32 = 0.5;
/// Objektart des Kontaktspiegels im OBJ-Block (Byte 0x14)
pub const OBJ_ART: u8 = 5;

/// Eine Zeile des Kontaktspiegels (Dialog: Nr, Art, 1, 2, 3, Referenz).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Zeile {
    /// 0 Schließer, 1 Öffner, 2 Wechsler
    pub art: u8,
    /// Anschlussnummern 1–3 (leer = aus dem referenzierten Symbol)
    #[serde(default)]
    pub felder: [String; 3],
    /// Objekt-Id des Kontakts (0 = keiner)
    #[serde(default)]
    pub referenz: u64,
    /// Kennungen aus dem .epr (Kontakt, Blatt, Anlage), solange der Kontakt nicht im Projekt gefunden wurde
    #[serde(default)]
    pub kennung: [u64; 3],
}

/// Schriftgröße der Anschlussnummern und der Lage (mm), gemessen an der Aufnahme des Originals
pub const SCHRIFT: f32 = 2.5;
/// Abstand der Anschlussnummern von der Mitte und Beginn der Lage rechts davon (mm)
const NUMMER_X: f32 = 3.4;
const LAGE_X: f32 = 7.0;

/// Unterkante der Zeichenfläche: Blatthöhe − unterer Rand der Anlage − Schriftfeldhöhe (W2: 210 − 5 − 15 = 190 mm,
/// hoch 297 − 5 − 15 = 277 mm – BLT Feld 24 ohne Kontaktspiegel).
pub fn zeichen_unterkante(p: &Project, s: &Sheet) -> f32 {
    s.frame().3 - p.title_h()
}

/// Y-Lage der ersten Zeile wie im Original (BLT Feld 24): Unterkante der Zeichenfläche minus größte Höhe der
/// Kontaktspiegel des Blattes – der Spiegel steht unmittelbar über dem Schriftfeld.
pub fn start_y(p: &Project, s: &Sheet) -> f32 {
    let h = s.objects.iter().filter_map(|o| if let Kind::Spiegel { rows, .. } = &o.kind { Some(hoehe(rows)) } else { None }).max().unwrap_or(0);
    zeichen_unterkante(p, s) - h as f32 / 10.0
}

/// Y-Lagen der Zeilen ab `y0`.
pub fn zeilen_y(y0: f32, rows: &[Zeile]) -> Vec<f32> {
    let mut y = y0;
    let mut v = vec![];
    for (i, r) in rows.iter().enumerate() {
        if r.art == 2 && i > 0 {
            y += WECHSLER_VOR;
        }
        v.push(y);
        if r.art == 2 {
            y += WECHSLER_NACH;
        }
        y += ZEILE;
    }
    v
}

/// Höhe des Spiegels in 0,1 mm wie beim Übernehmen im Original (OBJ Feld 14): je Zeile 30, vor einem Wechsler
/// außer in der ersten Zeile 25 mehr, mindestens 30.
pub fn hoehe(rows: &[Zeile]) -> i32 {
    let h: i32 = rows.iter().enumerate().map(|(i, r)| if r.art == 2 && i > 0 { 55 } else { 30 }).sum();
    h.max(30)
}

/// Umriss des Spiegels (für Markierung und Treffer).
pub fn bounds(pos: P, rows: &[Zeile]) -> (P, P) {
    let last = zeilen_y(pos.y, rows).last().copied().unwrap_or(pos.y);
    (P::new(pos.x - 7.5, pos.y - 2.6), P::new(pos.x + 15.0, last + 2.0))
}

/// Linien des Schaltzeichens einer Zeile relativ zum Zeilenpunkt (mm, waagerecht liegend).
fn zeichen(art: u8) -> &'static [(f32, f32, f32, f32)] {
    match art {
        // Öffner: Schaltglied liegt am Haken des rechten Anschlusses an
        1 => &[(-3.0, 0.0, -1.5, 0.0), (1.5, 0.0, 3.0, 0.0), (1.5, 0.0, 1.5, -1.3), (-1.5, 0.0, 1.8, -1.1)],
        // Wechsler wie im Original: gemeinsamer Anschluss links, Öffnerseite (11) oben mit Haken, Schließerseite (14) unten
        2 => &[(-3.0, 0.0, -1.5, 0.0), (1.5, -1.9, 3.0, -1.9), (1.5, -1.9, 1.5, -0.8), (1.5, 0.5, 3.0, 0.5), (-1.5, 0.0, 1.8, -0.8)],
        // Schließer
        _ => &[(-3.0, 0.0, -1.5, 0.0), (1.5, 0.0, 3.0, 0.0), (-1.5, 0.0, 1.5, -1.0)],
    }
}

/// Lage der Anschlussnummern 1–3 relativ zum Zeilenpunkt und ihre Ausrichtung.
fn nummern_lage(art: u8) -> [(f32, f32, Align2); 3] {
    match art {
        2 => [(-NUMMER_X, 0.0, Align2::RIGHT_CENTER), (NUMMER_X, -1.9, Align2::LEFT_CENTER), (NUMMER_X, 0.5, Align2::LEFT_CENTER)],
        _ => [(-NUMMER_X, 0.0, Align2::RIGHT_CENTER), (NUMMER_X, 0.0, Align2::LEFT_CENTER), (NUMMER_X, 0.0, Align2::LEFT_CENTER)],
    }
}

/// Anschlusstexte eines Symbols (Texte ohne Vorlage, z. B. „13“, „14“) – die automatische Beschriftung.
pub fn anschluesse(def: &SymDef) -> Vec<String> {
    def.prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text(_, t, _) | Prim::TextBox(_, _, t, _) if !t.contains('%') && !t.trim().is_empty() => Some(t.trim().to_string()),
            _ => None,
        })
        .collect()
}

/// Angezeigte Kennzeichnung eines Symbols im Projekt (mit Eigentümer: dessen Kennzeichnung).
fn bmk_von(p: &Project, id: u64) -> Option<String> {
    let (r, o) = p.find_obj(id)?;
    if !matches!(o.kind, Kind::Symbol { .. }) {
        return None;
    }
    let a = &p.anlagen[r.anlage];
    Some(crate::eigentuemer::anzeige_bmk(p, o, &bmk_ctx(a, &a.sheets[r.sheet])))
}

/// Art eines Kontakts aus dem Symbolnamen (Öffner/Wechsler, sonst Schließer).
fn art_aus(def: Option<&SymDef>) -> u8 {
    let n = def.map(|d| d.name.to_lowercase()).unwrap_or_default();
    if n.contains("öffner") || n.contains("oeffner") {
        1
    } else if n.contains("wechsler") {
        2
    } else {
        0
    }
}

/// Alle Symbole mit gleicher (angezeigter) Kennzeichnung wie die Spule oder mit ihr als Eigentümer, außer ihr
/// selbst und anderen Spulen mit eigenem Kontaktspiegel: (Objekt-Id, vorgeschlagene Art), in Projektreihenfolge.
pub fn kontakte(p: &Project, lib: &[SymDef], owner: u64) -> Vec<(u64, u8)> {
    let Some(ziel) = bmk_von(p, owner).filter(|b| !b.trim().is_empty()) else { return vec![] };
    let spulen: Vec<u64> = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter_map(|o| if let Kind::Spiegel { owner, .. } = o.kind { Some(owner) } else { None }).collect();
    let mut v = vec![];
    for a in &p.anlagen {
        for s in &a.sheets {
            let ctx = bmk_ctx(a, s);
            for o in &s.objects {
                let Kind::Symbol { ext, key, .. } = &o.kind else { continue };
                if o.id == owner || spulen.contains(&o.id) {
                    continue;
                }
                if ext.eigentuemer == owner || crate::eigentuemer::anzeige_bmk(p, o, &ctx) == ziel {
                    v.push((o.id, art_aus(symbols::find(lib, key))));
                }
            }
        }
    }
    v
}

/// Lage eines Objekts als „Blatt/Pfad“.
pub fn lage(p: &Project, id: u64) -> Option<String> {
    let (r, o) = p.find_obj(id)?;
    let Kind::Symbol { pos, .. } = &o.kind else { return None };
    Some(crate::berechnet::blatt_pfad(&p.anlagen[r.anlage].sheets[r.sheet], pos.x))
}

/// Kontaktspiegel nachführen: Y-Lage je Blatt, X-Lage der Spule (auf demselben Blatt), und bei `auto` neue
/// Kontakte gleicher Kennzeichnung aufnehmen bzw. Zeilen gelöschter oder umbenannter Kontakte entfernen.
/// Rückgabe: ob sich etwas geändert hat.
pub fn nachfuehren(p: &mut Project, lib: &[SymDef]) -> bool {
    let mut todo: Vec<(SheetRef, u64)> = vec![];
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                if matches!(o.kind, Kind::Spiegel { .. }) {
                    todo.push((SheetRef { anlage: ai, sheet: si }, o.id));
                }
            }
        }
    }
    let mut changed = false;
    for (r, id) in todo {
        let Some(s) = p.sheet(r) else { continue };
        let Some(Kind::Spiegel { owner, rows, auto, .. }) = s.objects.iter().find(|o| o.id == id).map(|o| &o.kind) else { continue };
        let (owner, auto) = (*owner, *auto);
        let spule_x = s.objects.iter().find(|o| o.id == owner && owner != 0).and_then(|o| if let Kind::Symbol { pos, .. } = &o.kind { Some(pos.x) } else { None });
        let mut neu = rows.clone();
        // Zeilen gelöschter Kontakte entfallen (Zeilen ohne Kontakt im Projekt, z. B. aus dem .epr, bleiben)
        neu.retain(|z| z.referenz == 0 || p.find_obj(z.referenz).is_some());
        if auto && p.find_obj(owner).is_some() {
            let k = kontakte(p, lib, owner);
            neu.retain(|z| z.referenz == 0 || k.iter().any(|(i, _)| *i == z.referenz));
            for (i, art) in k {
                if !neu.iter().any(|z| z.referenz == i) {
                    neu.push(Zeile { art, referenz: i, ..Default::default() });
                }
            }
        }
        if let Some(o) = p.sheet_mut(r).and_then(|s| s.objects.iter_mut().find(|o| o.id == id)) {
            if let Kind::Spiegel { pos, rows, .. } = &mut o.kind {
                if let Some(x) = spule_x.filter(|x| (pos.x - x).abs() > 0.01) {
                    pos.x = x;
                    changed = true;
                }
                if *rows != neu {
                    *rows = neu;
                    changed = true;
                }
            }
        }
    }
    // Y nach den neuen Zeilen: die größte Höhe aller Spiegel des Blattes bestimmt die Lage
    changed |= y_setzen(p);
    changed
}

/// Y-Lage aller Kontaktspiegel nach `start_y` setzen; Rückgabe: ob sich etwas geändert hat.
fn y_setzen(p: &mut Project) -> bool {
    let mut neu: Vec<(SheetRef, f32)> = vec![];
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            if s.objects.iter().any(|o| matches!(o.kind, Kind::Spiegel { .. })) {
                neu.push((SheetRef { anlage: ai, sheet: si }, start_y(p, s)));
            }
        }
    }
    let mut changed = false;
    for (r, y) in neu {
        for o in p.sheet_mut(r).into_iter().flat_map(|s| s.objects.iter_mut()) {
            if let Kind::Spiegel { pos, .. } = &mut o.kind {
                if (pos.y - y).abs() > 0.01 {
                    pos.y = y;
                    changed = true;
                }
            }
        }
    }
    changed
}

/// Kontaktspiegel zeichnen.
pub fn draw(dc: &DrawCtx, pos: P, rows: &[Zeile], stroke: Stroke) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let tc = stroke.color;
    let hier = crate::eigentuemer::blatt_des_dc(dc);
    for (row, y) in rows.iter().zip(zeilen_y(pos.y, rows)) {
        for (x0, y0, x1, y1) in zeichen(row.art) {
            p.line_segment([xf.s(P::new(pos.x + x0, y + y0)), xf.s(P::new(pos.x + x1, y + y1))], stroke);
        }
        // Anschlussnummern: eigene Einträge, sonst die Anschlusstexte des Kontakts
        let auto: Vec<String> = dc
            .project
            .find_obj(row.referenz)
            .filter(|_| row.referenz != 0)
            .and_then(|(_, o)| if let Kind::Symbol { key, .. } = &o.kind { symbols::find(dc.lib, key) } else { None })
            .map(anschluesse)
            .unwrap_or_default();
        let n = if row.art == 2 { 3 } else { 2 };
        for (i, (dx, dy, al)) in nummern_lage(row.art).into_iter().enumerate().take(n) {
            let t = if row.felder[i].trim().is_empty() { auto.get(i).cloned().unwrap_or_default() } else { row.felder[i].clone() };
            if !t.is_empty() {
                p.text(xf.s(P::new(pos.x + dx, y + dy)), al, t, times(SCHRIFT * z), tc);
            }
        }
        // Lage des Kontakts: „/Pfad“ auf dem Blatt des Spiegels, sonst „Blatt/Pfad“
        if let Some(l) = crate::eigentuemer::lage_von(dc.project, row.referenz, hier).filter(|_| row.referenz != 0) {
            p.text(xf.s(P::new(pos.x + LAGE_X, y)), Align2::LEFT_CENTER, l, times(SCHRIFT * z), tc);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Originalformat

/// Ist der OBJ-Block ein Kontaktspiegel?
pub fn ist_spiegel(k: &Block) -> bool {
    k.is("OBJ") && k.data.len() >= 0x30 && k.i32(4) & 0xffff == 0xffff && k.data[0x14] == OBJ_ART
}

fn id_at(d: &[u8], at: usize) -> u64 {
    d.get(at..at + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}

/// Kontaktspiegel aus einem OBJ-Block lesen; Spule, Kontakte und Y-Lage setzt `verknuepfen` nach dem Einlesen.
pub fn lesen(k: &Block) -> Kind {
    let rows = k
        .kids_of("STB")
        .map(|b| {
            let mut z = Zeile { art: b.data.get(0x18).copied().unwrap_or(0).min(2), kennung: [id_at(&b.data, 0), id_at(&b.data, 8), id_at(&b.data, 16)], ..Default::default() };
            // ALT Feld 0 = Nummer des Feldes (1–3), darin STR = Text; ohne ALT der Reihe nach (STR neben dem ALT
            // schrieben ältere Fassungen des Nachbaus)
            let (mut nr, mut weiter) = (None, 0usize);
            let kinder = b.kids.iter().flat_map(|k| std::iter::once(k).chain(k.kids.iter().filter(|_| k.is("ALT"))));
            for kid in kinder {
                if kid.is("ALT") {
                    nr = usize::try_from(kid.i32(0) - 1).ok();
                } else if kid.is("STR") {
                    let i = nr.take().unwrap_or(weiter);
                    if i < 3 {
                        z.felder[i] = symbols::cstr(&kid.data);
                    }
                    weiter = i + 1;
                }
            }
            z
        })
        .collect();
    Kind::Spiegel { pos: P::new(k.i32(6) as f32 / 10.0, 0.0), owner: 0, rows, auto: false }
}

/// Kennung eines Symbols im Original (VSY Byte 96).
fn vsy_kennung(o: &Obj) -> u64 {
    o.raw.as_deref().filter(|r| r.block.is("VSY")).map(|r| id_at(&r.block.data, 96)).unwrap_or(0)
}

/// Nach dem Einlesen: Spule (OBJ Feld 8/9) und Kontakte (STB) über die Symbolkennungen den Objekten zuordnen.
/// Die Originalform (`RawObj::kind`) wird mitgeführt, damit unveränderte Dateien byteidentisch bleiben.
pub fn verknuepfen(p: &mut Project) {
    let mut ids: std::collections::HashMap<u64, u64> = Default::default();
    for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        let k = vsy_kennung(o);
        if k != 0 {
            ids.insert(k, o.id);
        }
    }
    for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects) {
        let Some(raw) = o.raw.as_deref_mut() else { continue };
        if !ist_spiegel(&raw.block) {
            continue;
        }
        let spule = ids.get(&id_at(&raw.block.data, 0x20)).copied().unwrap_or(0);
        for kind in [&mut o.kind, &mut raw.kind] {
            if let Kind::Spiegel { owner, rows, .. } = kind {
                *owner = spule;
                for z in rows.iter_mut() {
                    if let Some(i) = ids.get(&z.kennung[0]) {
                        z.referenz = *i;
                    }
                }
            }
        }
    }
    // Y-Lage wie beim Zeichnen im Original (BLT Feld 24 wird dabei neu berechnet), in beiden Formen
    y_setzen(p);
    for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects) {
        if let (Kind::Spiegel { pos, .. }, Some(raw)) = (&o.kind, o.raw.as_deref_mut()) {
            if let Kind::Spiegel { pos: rp, .. } = &mut raw.kind {
                rp.y = pos.y;
            }
        }
    }
}

/// OBJ-Block eines Kontaktspiegels (neu oder geändert). Kennungen: `kennung(obj)` für Objekte, `blatt`/`anlage`
/// für Blatt und Anlage eines Objekts (aus `epr_write`/`epr_neu`).
pub fn block(p: &Project, o: &Obj, eigene: u64, kennung: &dyn Fn(&Obj) -> u64, orte: &dyn Fn(SheetRef) -> (u64, u64)) -> Option<Block> {
    let Kind::Spiegel { pos, owner, rows, .. } = &o.kind else { return None };
    let mut b = match o.raw.as_deref() {
        Some(r) => r.block.clone(),
        None => Block::new("OBJ", vec![0; 0x40]),
    };
    if b.data.len() < 0x40 {
        b.data.resize(0x40, 0);
    }
    b.data[0..8].copy_from_slice(&eigene.to_le_bytes());
    b.set_i32(4, (b.i32(4) & !0xffff) | 0xffff);
    b.data[0x14] = OBJ_ART;
    b.set_i32(6, (pos.x * 10.0).round() as i32);
    let spule = p.find_obj(*owner).filter(|_| *owner != 0).map(|(_, x)| kennung(x));
    if let Some(k) = spule {
        b.data[0x20..0x28].copy_from_slice(&k.to_le_bytes());
    }
    b.data[0x2c] = (b.data[0x2c] & !1) | (!rows.is_empty()) as u8;
    b.set_i32(14, hoehe(rows));
    b.kids.retain(|k| !k.is("STB"));
    for z in rows {
        let mut stb = Block::new("STB", vec![0; 0x1c]);
        let ids = match p.find_obj(z.referenz).filter(|_| z.referenz != 0) {
            Some((r, x)) => {
                let (blatt, anlage) = orte(r);
                [kennung(x), blatt, anlage]
            }
            None => z.kennung,
        };
        for (i, v) in ids.iter().enumerate() {
            stb.data[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
        }
        stb.data[0x18] = z.art.min(2);
        for (i, f) in z.felder.iter().enumerate().filter(|(_, f)| !f.trim().is_empty()) {
            let mut alt = Block::new("ALT", vec![0; 8]);
            alt.set_i32(0, i as i32 + 1);
            // STR als Unterblock des ALT, ohne abschließende 0 (Speicherroutine 0x424fe2: STB › ALT › STR, Länge =
            // Textlänge); als Geschwister des ALT meldete das Original „Fehler beim erzeugen des Objekts: STR“
            alt.kids.push(Block::new("STR", encode_cp1252(f.trim())));
            stb.kids.push(alt);
        }
        b.kids.push(stb);
    }
    Some(b)
}

/// „Set Referenz“ abgeschlossen: Zeile `zeile` bekommt das Symbol `id` als Referenz (die leere letzte Zeile wird
/// dabei neu angelegt, mit der Art aus dem Symbolnamen). Die Spule selbst ist keine Referenz.
pub fn referenz_setzen(d: &mut SpiegelDlg, zeile: usize, id: u64, art: u8) -> bool {
    if id == 0 || id == d.owner {
        return false;
    }
    match d.rows.get_mut(zeile) {
        Some(z) => z.referenz = id,
        None => d.rows.push(Zeile { art, referenz: id, ..Default::default() }),
    }
    d.sel = Some(zeile.min(d.rows.len() - 1));
    d.pick = None;
    true
}

/// Beim Übernehmen wird die Spule Eigentümer der referenzierten Kontakte (Original 0x4995ac): VSY › OWR, die
/// Kontakte zeigen danach die Kennzeichnung der Spule und darunter ihre Lage (`eigentuemer.rs`).
pub fn eigentuemer_setzen(p: &mut Project, owner: u64, ids: &[u64]) -> bool {
    crate::eigentuemer::setzen(p, owner, ids)
}

// ---------------------------------------------------------------------------------------------------------
// Dialog (TSSpiegel)

/// Zustand des Dialogs: Spule, vorhandener Spiegel (oder neu), Zeilen, gewählte Zeile.
pub struct SpiegelDlg {
    pub owner: u64,
    pub obj: Option<u64>,
    pub rows: Vec<Zeile>,
    pub sel: Option<usize>,
    pub auto: bool,
    /// „Set Referenz“: der Dialog ist ausgeblendet und wartet auf einen Klick auf das Symbol für diese Zeile
    pub pick: Option<usize>,
}

impl App {
    /// Kontextmenü „Kontaktspiegel hinzufügen“: vorhandenen Spiegel der Spule bearbeiten, sonst einen neuen mit
    /// allen Kontakten gleicher Kennzeichnung vorschlagen.
    pub fn spiegel_oeffnen(&mut self) {
        let Some(o) = self.selected_objs().into_iter().find(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Spiegel { .. })) else { return };
        if let Kind::Spiegel { .. } = o.kind {
            self.spiegel_props(o.id);
            return;
        }
        let vorhanden = self.sheet().and_then(|s| s.objects.iter().find(|x| matches!(x.kind, Kind::Spiegel { owner, .. } if owner == o.id))).map(|x| x.id);
        if let Some(id) = vorhanden {
            self.spiegel_props(id);
            return;
        }
        let rows = kontakte(&self.project, &self.lib, o.id).into_iter().map(|(referenz, art)| Zeile { art, referenz, ..Default::default() }).collect();
        self.zusatz.spiegel = Some(SpiegelDlg { owner: o.id, obj: None, rows, sel: None, auto: true, pick: None });
    }

    /// Eigenschaften eines Kontaktspiegels (Doppelklick/Eigenschaften); false, wenn `id` keiner ist.
    pub fn spiegel_props(&mut self, id: u64) -> bool {
        let Some(Kind::Spiegel { owner, rows, auto, .. }) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| o.kind.clone()) else { return false };
        self.zusatz.spiegel = Some(SpiegelDlg { owner, obj: Some(id), rows, sel: None, auto, pick: None });
        true
    }

    /// Wartet „Set Referenz“ auf einen Klick auf das Blatt?
    pub fn spiegel_pick_aktiv(&self) -> bool {
        self.zusatz.spiegel.as_ref().is_some_and(|d| d.pick.is_some())
    }

    /// Klick bei „Set Referenz“ (SetzeReferenz1Click 0x4b5d20: Dialog ausblenden, Hauptfenster wartet auf das Symbol):
    /// `Some(id)` = angeklicktes Symbol, `None` = Abbruch. Danach erscheint der Dialog wieder.
    pub fn spiegel_pick_ende(&mut self, id: Option<u64>) {
        let art = id.and_then(|i| self.project.find_obj(i)).and_then(|(_, o)| if let Kind::Symbol { key, .. } = &o.kind { Some(art_aus(symbols::find(&self.lib, key))) } else { None }).unwrap_or(0);
        let eig = id.and_then(|i| self.project.find_obj(i)).and_then(|(_, o)| if let Kind::Symbol { ext, .. } = &o.kind { Some(ext.eigentuemer) } else { None });
        let Some(d) = self.zusatz.spiegel.as_mut() else { return };
        let Some(zeile) = d.pick else { return };
        // Original (0x4b6674): hat das Referenz-Symbol keinen bzw. einen anderen Eigentümer, erst bestätigen
        // (OK = der Eigentümer des Kontaktspiegels wird übernommen, Abbrechen = keine Referenz)
        if let (Some(i), Some(e)) = (id, eig) {
            if i != d.owner && e != d.owner {
                let text = if e == 0 { crate::meldungen::KS_OHNE_EIGENTUEMER } else { crate::meldungen::KS_ANDERER_EIGENTUEMER };
                if !crate::meldungen::frage_ok(crate::meldungen::T_ACHTUNG_X, text) {
                    d.pick = None;
                    self.info = "Referenz setzen abgebrochen".into();
                    return;
                }
            }
        }
        match id {
            Some(i) if referenz_setzen(d, zeile, i, art) => self.info = format!("Referenz für Zeile {} gesetzt: {}", zeile + 1, lage(&self.project, i).unwrap_or_default()),
            Some(_) => self.info = "Referenz setzen: bitte einen Kontakt anklicken, nicht die Spule (Esc = Abbruch)".into(),
            None => {
                d.pick = None;
                self.info = "Referenz setzen abgebrochen".into();
            }
        }
    }

    /// Dialog „Kontaktspiegel“ zeichnen und übernehmen.
    pub fn spiegel_ui(&mut self, ctx: &egui::Context) {
        // „Set Referenz“ läuft: Dialog ausgeblendet, bis das Symbol angeklickt ist
        if self.zusatz.spiegel.as_ref().is_some_and(|d| d.pick.is_some()) {
            return;
        }
        let Some(mut d) = self.zusatz.spiegel.take() else { return };
        let mut done: Option<bool> = None;
        let mut setze_referenz: Option<usize> = None;
        // Auswahl für die Referenz: alle Symbole gleicher Kennzeichnung mit ihrer Lage, dazu per „Set Referenz“
        // gewählte andere Symbole
        let mut ids: Vec<u64> = kontakte(&self.project, &self.lib, d.owner).into_iter().map(|(i, _)| i).collect();
        for z in &d.rows {
            if z.referenz != 0 && !ids.contains(&z.referenz) && self.project.find_obj(z.referenz).is_some() {
                ids.push(z.referenz);
            }
        }
        let wahl: Vec<(u64, String)> = std::iter::once((0, String::new()))
            .chain(ids.into_iter().map(|i| {
                let name = self.project.find_obj(i).and_then(|(_, o)| if let Kind::Symbol { key, .. } = &o.kind { symbols::find(&self.lib, key).map(|d| d.name.clone()) } else { None }).unwrap_or_default();
                (i, format!("{} {name}", lage(&self.project, i).unwrap_or_default()))
            }))
            .collect();
        egui::Window::new("Kontaktspiegel").collapsible(false).resizable(false).default_width(380.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                egui::Grid::new("spiegel_rows").striped(true).num_columns(6).show(ui, |ui| {
                    for h in ["Nr", "Art", "1", "2", "3", "Referenz"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    // letzte, leere Zeile = neue Zeile wie im Original
                    let n = d.rows.len();
                    let mut neu = Zeile::default();
                    let mut anhaengen = false;
                    for i in 0..=n {
                        let z = if i < n { &mut d.rows[i] } else { &mut neu };
                        let vorher = z.clone();
                        let nr = ui.selectable_label(d.sel == Some(i), (i + 1).to_string());
                        if nr.clicked() || nr.secondary_clicked() {
                            d.sel = Some(i);
                        }
                        // Kontextmenü der Liste wie im Original (PopupMenu1 › „Setze Referenz“)
                        nr.context_menu(|ui| {
                            if ui.button("Setze Referenz").clicked() {
                                setze_referenz = Some(i);
                                ui.close();
                            }
                        });
                        egui::ComboBox::from_id_salt(("sp_art", i)).width(80.0).selected_text(ARTEN[z.art.min(2) as usize]).show_ui(ui, |ui| {
                            for (k, a) in ARTEN.iter().enumerate() {
                                ui.selectable_value(&mut z.art, k as u8, *a);
                            }
                        });
                        for f in z.felder.iter_mut() {
                            ui.add(egui::TextEdit::singleline(f).desired_width(28.0));
                        }
                        let text = wahl.iter().find(|(id, _)| *id == z.referenz).map(|(_, t)| t.clone()).unwrap_or_else(|| if z.referenz == 0 { String::new() } else { "?".into() });
                        egui::ComboBox::from_id_salt(("sp_ref", i)).width(130.0).selected_text(text).show_ui(ui, |ui| {
                            for (id, t) in &wahl {
                                ui.selectable_value(&mut z.referenz, *id, if t.is_empty() { "(keine)" } else { t.as_str() });
                            }
                        });
                        ui.end_row();
                        if i == n && *z != vorher {
                            anhaengen = true;
                        }
                    }
                    if anhaengen {
                        d.rows.push(neu);
                    }
                });
            });
            ui.checkbox(&mut d.auto, "Kontakte gleicher Kennzeichnung automatisch aufnehmen");
            ui.horizontal(|ui| {
                if ui.button("Zeile löschen").clicked() {
                    if let Some(i) = d.sel.take().filter(|i| *i < d.rows.len()) {
                        d.rows.remove(i);
                        // gelöschte Kontakte nicht gleich wieder aufnehmen
                        d.auto = false;
                    }
                }
                // Original: Knopf rechts neben „Zeile löschen“ (Button4, gleicher Handler wie das Kontextmenü)
                if ui.add_enabled(d.sel.is_some(), egui::Button::new("Set Referenz")).on_hover_text("Referenz der markierten Zeile: danach das Symbol auf dem Blatt anklicken").clicked() {
                    setze_referenz = d.sel;
                }
                ui.add_space(40.0);
                if ui.button("Abbruch").clicked() {
                    done = Some(false);
                }
                if ui.button("OK").clicked() {
                    done = Some(true);
                }
            });
        });
        if let Some(i) = setze_referenz.filter(|_| done.is_none()) {
            d.sel = Some(i);
            d.pick = Some(i);
            self.placing = None;
            self.info = "Referenz setzen: Symbol auf dem Blatt anklicken (Esc = Abbruch)".into();
        }
        match done {
            None => self.zusatz.spiegel = Some(d),
            Some(false) => {}
            Some(true) => self.spiegel_uebernehmen(d),
        }
    }

    fn spiegel_uebernehmen(&mut self, d: SpiegelDlg) {
        // leere Zeilen entfallen wie im Original
        let rows: Vec<Zeile> = d.rows.into_iter().filter(|z| z.referenz != 0 || z.felder.iter().any(|f| !f.trim().is_empty())).collect();
        let Some((r, owner)) = self.project.find_obj(d.owner).map(|(r, o)| (r, o.clone())) else { return };
        self.checkpoint();
        let refs: Vec<u64> = rows.iter().map(|z| z.referenz).filter(|i| *i != 0).collect();
        eigentuemer_setzen(&mut self.project, d.owner, &refs);
        match d.obj {
            Some(id) => {
                if let Some(o) = self.project.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects).find(|o| o.id == id) {
                    if let Kind::Spiegel { rows: r0, auto, .. } = &mut o.kind {
                        *r0 = rows;
                        *auto = d.auto;
                    }
                }
            }
            None => {
                let x = if let Kind::Symbol { pos, .. } = owner.kind { pos.x } else { 0.0 };
                let y = self.project.sheet(r).map(|s| start_y(&self.project, s)).unwrap_or(180.0);
                let id = self.project.alloc_id();
                if let Some(s) = self.project.sheet_mut(r) {
                    s.objects.push(Obj { id, kind: Kind::Spiegel { pos: P::new(x, y), owner: d.owner, rows, auto: d.auto }, pen: 0, fixed: false, group: None, raw: None });
                }
                if self.cur() == Some(r) {
                    self.selection = vec![id];
                }
            }
        }
        nachfuehren(&mut self.project, &self.lib);
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, bmk: &str, x: f32) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "k".into(), pos: P::new(x, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn zeilen_und_wechsler() {
        let r = |art| Zeile { art, ..Default::default() };
        assert_eq!(zeilen_y(180.0, &[r(0), r(2), r(1)]), vec![180.0, 185.0, 188.5]);
        assert_eq!(zeilen_y(10.0, &[r(2)]), vec![10.0]);
    }

    #[test]
    fn spiegel_fuehrt_kontakte_nach() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(sym(1, "-K1", 40.0));
        s.objects.push(sym(2, "-K1", 120.0));
        s.objects.push(sym(3, "-K2", 120.0));
        s.objects.push(Obj { id: 9, kind: Kind::Spiegel { pos: P::new(40.0, 0.0), owner: 1, rows: vec![], auto: true }, pen: 0, fixed: false, group: None, raw: None });
        assert!(nachfuehren(&mut p, &[]));
        let rows = |p: &Project| if let Some((_, Obj { kind: Kind::Spiegel { rows, pos, .. }, .. })) = p.find_obj(9) { (rows.clone(), pos.y) } else { panic!() };
        let (r, y) = rows(&p);
        // über dem Schriftfeld: 210 − 5 − 15 − 3 mm (eine Zeile)
        assert_eq!((r.len(), r[0].referenz, y), (1, 2, 187.0));
        assert_eq!(lage(&p, 2).as_deref(), Some("1/4"));
        // neuer Kontakt gleicher Kennzeichnung wird aufgenommen, umbenannter entfernt
        p.anlagen[0].sheets[0].objects.push(sym(4, "-K1", 200.0));
        if let Kind::Symbol { bmk, .. } = &mut p.anlagen[0].sheets[0].objects[1].kind {
            *bmk = "-K3".into();
        }
        assert!(nachfuehren(&mut p, &[]));
        assert_eq!(rows(&p).0.iter().map(|z| z.referenz).collect::<Vec<_>>(), vec![4]);
        assert!(!nachfuehren(&mut p, &[]));
    }

    #[test]
    fn geloeschter_kontakt_entfaellt() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(sym(1, "-K1", 40.0));
        s.objects.push(sym(2, "-K1", 120.0));
        let rows = vec![Zeile { art: 0, referenz: 2, ..Default::default() }];
        s.objects.push(Obj { id: 9, kind: Kind::Spiegel { pos: P::new(40.0, 180.0), owner: 1, rows, auto: false }, pen: 0, fixed: false, group: None, raw: None });
        // gelöschter Kontakt: Zeile entfällt auch ohne automatische Aufnahme
        p.anlagen[0].sheets[0].objects.remove(1);
        assert!(nachfuehren(&mut p, &[]));
        assert!(matches!(&p.find_obj(9).unwrap().1.kind, Kind::Spiegel { rows, .. } if rows.is_empty()));
        // gelöschte Spule: der Spiegel bleibt wie im Original (W2.epr enthält einen solchen Spiegel)
        p.anlagen[0].sheets[0].objects.remove(0);
        nachfuehren(&mut p, &[]);
        assert!(p.find_obj(9).is_some());
    }

    /// Lage wie in der Aufnahme des Originals (Gegenprobe Blatt 6 „Sicherung -F7“): Schließer + Wechsler, Höhe 85,
    /// erste Zeile 181,5 mm (gemessen 181,4), Wechsler 186,5 mm, Unterkante 190 mm = Oberkante Schriftfeld.
    #[test]
    fn lage_ueber_dem_schriftfeld() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(sym(1, "-K1", 64.0));
        s.objects.push(sym(2, "-K1", 120.0));
        let rows = vec![Zeile { art: 0, referenz: 2, ..Default::default() }, Zeile { art: 2, ..Default::default() }];
        s.objects.push(Obj { id: 9, kind: Kind::Spiegel { pos: P::new(10.0, 0.0), owner: 1, rows: rows.clone(), auto: false }, pen: 0, fixed: false, group: None, raw: None });
        assert_eq!(hoehe(&rows), 85);
        assert_eq!(start_y(&p, &p.anlagen[0].sheets[0]), 181.5);
        assert_eq!(zeilen_y(181.5, &rows), vec![181.5, 186.5]);
        // ein zweiter, kleinerer Spiegel: die größte Höhe zählt (Original: Maximum von Feld 14)
        p.anlagen[0].sheets[0].objects.push(Obj { id: 10, kind: Kind::Spiegel { pos: P::new(200.0, 0.0), owner: 0, rows: vec![Zeile::default()], auto: false }, pen: 0, fixed: false, group: None, raw: None });
        assert!(nachfuehren(&mut p, &[]));
        let lage = |p: &Project, id| if let Kind::Spiegel { pos, .. } = p.find_obj(id).unwrap().1.kind { pos } else { panic!() };
        // X folgt der Spule, ohne Spule bleibt sie stehen
        assert_eq!((lage(&p, 9), lage(&p, 10)), (P::new(64.0, 181.5), P::new(200.0, 181.5)));
        if let Kind::Symbol { pos, .. } = &mut p.anlagen[0].sheets[0].objects[0].kind {
            pos.x = 80.0;
        }
        assert!(nachfuehren(&mut p, &[]));
        assert_eq!(lage(&p, 9).x, 80.0);
        assert!(!nachfuehren(&mut p, &[]));
        // Schriftfeldhöhe des Projekts und Hochformat
        p.tb_hoehe = 25.0;
        assert_eq!(start_y(&p, &p.anlagen[0].sheets[0]), 171.5);
        p.tb_hoehe = 0.0;
        p.anlagen[0].sheets[0].portrait = true;
        assert_eq!(start_y(&p, &p.anlagen[0].sheets[0]), 268.5);
    }

    #[test]
    fn hoehe_wie_im_original() {
        let r = |art| Zeile { art, ..Default::default() };
        assert_eq!(hoehe(&[]), 30);
        assert_eq!(hoehe(&[r(2)]), 30);
        assert_eq!(hoehe(&[r(0), r(2), r(1)]), 115);
    }

    #[test]
    fn set_referenz() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1", 40.0));
        p.anlagen[0].sheets[0].objects.push(sym(2, "-K1", 120.0));
        p.anlagen[0].sheets[0].objects.push(sym(3, "-S5", 200.0));
        let mut d = SpiegelDlg { owner: 1, obj: None, rows: vec![Zeile { art: 0, referenz: 2, ..Default::default() }], sel: Some(0), auto: true, pick: Some(0) };
        // die Spule selbst ist keine Referenz: weiter warten
        assert!(!referenz_setzen(&mut d, 0, 1, 0));
        assert_eq!((d.rows[0].referenz, d.pick), (2, Some(0)));
        // vorhandene Zeile umsetzen
        assert!(referenz_setzen(&mut d, 0, 3, 1));
        assert_eq!((d.rows[0].referenz, d.rows[0].art, d.pick), (3, 0, None));
        // leere letzte Zeile: neue Zeile mit Art aus dem Symbol
        assert!(referenz_setzen(&mut d, 1, 2, 2));
        assert_eq!((d.rows.len(), d.rows[1].referenz, d.rows[1].art, d.sel), (2, 2, 2, Some(1)));
        // Übernehmen: -S5 bekommt die Kennzeichnung der Spule und bleibt so auch bei automatischer Aufnahme
        assert!(eigentuemer_setzen(&mut p, 1, &[3, 2]));
        assert_eq!(bmk_von(&p, 3).as_deref(), Some("-K1"));
        assert!(!eigentuemer_setzen(&mut p, 1, &[3, 2]));
        assert_eq!(kontakte(&p, &[], 1).iter().map(|k| k.0).collect::<Vec<_>>(), vec![2, 3]);
    }

    #[test]
    fn block_rundreise() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1", 40.0));
        p.anlagen[0].sheets[0].objects.push(sym(2, "-K1", 120.0));
        let rows = vec![Zeile { art: 2, felder: ["11".into(), String::new(), "14".into()], referenz: 2, kennung: [0; 3] }];
        let o = Obj { id: 9, kind: Kind::Spiegel { pos: P::new(40.0, 180.0), owner: 1, rows, auto: false }, pen: 0, fixed: false, group: None, raw: None };
        let b = block(&p, &o, 77, &|x| x.id * 1000, &|_| (5, 6)).unwrap();
        assert!(ist_spiegel(&b));
        assert_eq!((b.i32(6), id_at(&b.data, 0), id_at(&b.data, 0x20), b.data[0x2c] & 1, b.i32(14)), (400, 77, 1000, 1, 30));
        let stb = b.kid("STB").unwrap();
        assert_eq!((stb.data.len(), id_at(&stb.data, 0), id_at(&stb.data, 8), id_at(&stb.data, 16), stb.data[0x18]), (0x1c, 2000, 5, 6, 2));
        let Kind::Spiegel { rows, pos, .. } = lesen(&b) else { panic!() };
        assert_eq!((rows[0].art, rows[0].felder[0].as_str(), rows[0].felder[1].as_str(), rows[0].kennung[0], pos.x), (2, "11", "", 2000, 40.0));
        assert_eq!(rows[0].felder[2], "14");
    }
}
