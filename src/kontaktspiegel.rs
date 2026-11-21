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
//!   1 Öffner / 2 Wechsler), darin je eingetragener Anschlussnummer ALT (8 Byte) + STR. W2.epr enthält einen
//!   Spiegel ohne Zeilen (Blatt „Sicherungen -F10, -F1 und -F2“), dessen Spule gelöscht ist.
//! - Übernehmen (0x4995ac): Zeilen ohne Eintrag entfallen; die Referenz einer Zeile ist der Kontakt im Projekt,
//!   dessen Eigentümer damit die Spule wird.
//! - Zeichnen (0x43d4xx): Y beginnt bei der Rahmenhöhe des Blattes (BLT Feld 24), je Zeile 3 mm; ein Wechsler
//!   bekommt davor 2 mm (außer in der ersten Zeile) und danach 0,5 mm mehr. Die Schaltzeichen liegen waagerecht
//!   (Anschlüsse bei ±1,5…3 mm um X), ohne eigene Nummern zeigen sie die Anschlusstexte des referenzierten Symbols.
//!   Daneben steht die Lage des referenzierten Kontakts.
//! Eigene Zeichnung im Nachbau (keine Grafik aus der Exe); Schriftgrößen und Lage der Texte sind geschätzt.
//! Ungeprüft: ALT-Inhalt (hier Feld 0 = Nummer 1–3) und Feld 14 (in W2 30 = Höhe einer Zeile?).

use crate::app::App;
use crate::canvas::{bmk_ctx, times, DrawCtx};
use crate::eprfile::{cstr_bytes, Block};
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

/// Y-Lage der ersten Zeile: Rahmenhöhe des Blattes (BLT Feld 24) wie im Original.
pub fn start_y(s: &Sheet) -> f32 {
    if s.portrait {
        277.0
    } else if s.frame_h > 50.0 {
        s.frame_h
    } else {
        180.0
    }
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

/// Umriss des Spiegels (für Markierung und Treffer).
pub fn bounds(pos: P, rows: &[Zeile]) -> (P, P) {
    let last = zeilen_y(pos.y, rows).last().copied().unwrap_or(pos.y);
    (P::new(pos.x - 7.0, pos.y - 2.4), P::new(pos.x + 15.0, last + 1.2))
}

/// Linien des Schaltzeichens einer Zeile relativ zum Zeilenpunkt (mm, waagerecht liegend).
fn zeichen(art: u8) -> &'static [(f32, f32, f32, f32)] {
    match art {
        // Öffner: Schaltglied liegt am Haken des rechten Anschlusses an
        1 => &[(-3.0, 0.0, -1.5, 0.0), (1.5, 0.0, 3.0, 0.0), (1.5, 0.0, 1.5, -1.3), (-1.5, 0.0, 1.8, -1.1)],
        // Wechsler: gemeinsamer Anschluss links, Öffnerseite unten mit Haken, Schließerseite oben
        2 => &[(-3.0, 0.0, -1.5, 0.0), (1.5, 0.5, 3.0, 0.5), (1.5, 0.5, 1.5, -0.6), (1.5, -1.9, 3.0, -1.9), (-1.5, 0.0, 1.8, -0.8)],
        // Schließer
        _ => &[(-3.0, 0.0, -1.5, 0.0), (1.5, 0.0, 3.0, 0.0), (-1.5, 0.0, 1.5, -1.0)],
    }
}

/// Lage der Anschlussnummern 1–3 relativ zum Zeilenpunkt und ihre Ausrichtung.
fn nummern_lage(art: u8) -> [(f32, f32, Align2); 3] {
    match art {
        2 => [(-3.2, 0.0, Align2::RIGHT_CENTER), (3.2, 0.6, Align2::LEFT_CENTER), (3.2, -1.9, Align2::LEFT_CENTER)],
        _ => [(-3.2, 0.0, Align2::RIGHT_CENTER), (3.2, 0.0, Align2::LEFT_CENTER), (3.2, 0.0, Align2::LEFT_CENTER)],
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

/// Angezeigte Kennzeichnung eines Symbols im Projekt.
fn bmk_von(p: &Project, id: u64) -> Option<String> {
    let (r, o) = p.find_obj(id)?;
    let Kind::Symbol { bmk, ext, .. } = &o.kind else { return None };
    let a = &p.anlagen[r.anlage];
    Some(symbols::display_bmk(bmk, ext.counter, &bmk_ctx(a, &a.sheets[r.sheet])))
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

/// Alle Symbole mit gleicher (angezeigter) Kennzeichnung wie die Spule, außer ihr selbst und anderen Spulen mit
/// eigenem Kontaktspiegel: (Objekt-Id, vorgeschlagene Art), in Projektreihenfolge.
pub fn kontakte(p: &Project, lib: &[SymDef], owner: u64) -> Vec<(u64, u8)> {
    let Some(ziel) = bmk_von(p, owner).filter(|b| !b.trim().is_empty()) else { return vec![] };
    let spulen: Vec<u64> = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter_map(|o| if let Kind::Spiegel { owner, .. } = o.kind { Some(owner) } else { None }).collect();
    let mut v = vec![];
    for a in &p.anlagen {
        for s in &a.sheets {
            let ctx = bmk_ctx(a, s);
            for o in &s.objects {
                let Kind::Symbol { bmk, ext, key, .. } = &o.kind else { continue };
                if o.id == owner || spulen.contains(&o.id) {
                    continue;
                }
                if symbols::display_bmk(bmk, ext.counter, &ctx) == ziel {
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

/// Kontaktspiegel nachführen: Y-Lage je Blatt, und bei `auto` neue Kontakte gleicher Kennzeichnung aufnehmen bzw.
/// Zeilen gelöschter oder umbenannter Kontakte entfernen. Rückgabe: ob sich etwas geändert hat.
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
        let y = start_y(s);
        let Some(Kind::Spiegel { owner, rows, auto, .. }) = s.objects.iter().find(|o| o.id == id).map(|o| &o.kind) else { continue };
        let (owner, auto) = (*owner, *auto);
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
                if (pos.y - y).abs() > 0.01 {
                    pos.y = y;
                    changed = true;
                }
                if *rows != neu {
                    *rows = neu;
                    changed = true;
                }
            }
        }
    }
    changed
}

/// Letzte bekannte X-Lage von Spule und Spiegel je Spiegel (für `folgen`).
pub type Lagen = std::collections::HashMap<u64, (u64, f32, f32)>;

/// Der Spiegel folgt waagerecht seiner Spule: wurde seit dem letzten Aufruf nur die Spule verschoben (auf welchem
/// Weg auch immer – Ziehen, Pfeiltasten, Ausrichten, MCP), wandert der Spiegel um denselben Weg mit. Wurden beide
/// bewegt (gemeinsam verschoben, Rückgängig), bleibt er, wo er ist. Rückgabe: ob sich etwas geändert hat.
pub fn folgen(p: &mut Project, lagen: &mut Lagen) -> bool {
    let mut neu: Vec<(u64, u64, f32, f32)> = vec![];
    for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        let Kind::Spiegel { pos, owner, .. } = &o.kind else { continue };
        let Some((_, Obj { kind: Kind::Symbol { pos: sp, .. }, .. })) = p.find_obj(*owner).filter(|_| *owner != 0) else { continue };
        neu.push((o.id, *owner, sp.x, pos.x));
    }
    let mut changed = false;
    let mut weiter = Lagen::default();
    for (id, owner, spule_x, x) in neu {
        let mut x_neu = x;
        if let Some((o0, s0, x0)) = lagen.get(&id) {
            if *o0 == owner && (spule_x - s0).abs() > 1e-4 && (x - x0).abs() <= 1e-4 {
                x_neu = x + (spule_x - s0);
                if let Some(o) = p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects).find(|o| o.id == id) {
                    if let Kind::Spiegel { pos, .. } = &mut o.kind {
                        pos.x = x_neu;
                        changed = true;
                    }
                }
            }
        }
        weiter.insert(id, (owner, spule_x, x_neu));
    }
    *lagen = weiter;
    changed
}

/// Kontaktspiegel zeichnen.
pub fn draw(dc: &DrawCtx, pos: P, rows: &[Zeile], stroke: Stroke) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let tc = stroke.color;
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
                p.text(xf.s(P::new(pos.x + dx, y + dy)), al, t, times(1.8 * z), tc);
            }
        }
        if let Some(l) = lage(dc.project, row.referenz).filter(|_| row.referenz != 0) {
            p.text(xf.s(P::new(pos.x + 7.0, y)), Align2::LEFT_CENTER, l, times(2.0 * z), tc);
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

/// Kontaktspiegel aus einem OBJ-Block lesen; Spule und Kontakte verknüpft `verknuepfen` nach dem Einlesen.
pub fn lesen(k: &Block, sheet: &Sheet) -> Kind {
    let rows = k
        .kids_of("STB")
        .map(|b| {
            let mut z = Zeile { art: b.data.get(0x18).copied().unwrap_or(0).min(2), kennung: [id_at(&b.data, 0), id_at(&b.data, 8), id_at(&b.data, 16)], ..Default::default() };
            // ALT Feld 0 = Nummer des Feldes (1–3), danach STR = Text; ohne ALT der Reihe nach
            let (mut nr, mut weiter) = (None, 0usize);
            for kid in &b.kids {
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
    Kind::Spiegel { pos: P::new(k.i32(6) as f32 / 10.0, start_y(sheet)), owner: 0, rows, auto: false }
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
}

/// OBJ-Block eines Kontaktspiegels (neu oder geändert). Kennungen: `kennung(obj)` für Objekte, `blatt`/`anlage`
/// für Blatt und Anlage eines Objekts (aus `epr_write`/`epr_neu`).
pub fn block(p: &Project, o: &Obj, eigene: u64, kennung: &dyn Fn(&Obj) -> u64, orte: &dyn Fn(SheetRef) -> (u64, u64)) -> Option<Block> {
    let Kind::Spiegel { pos, owner, rows, .. } = &o.kind else { return None };
    let mut b = match o.raw.as_deref() {
        Some(r) => r.block.clone(),
        None => {
            let mut b = Block::new("OBJ", vec![0; 0x40]);
            b.set_i32(14, 30);
            b
        }
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
            stb.kids.push(alt);
            stb.kids.push(Block::new("STR", cstr_bytes(f.trim())));
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

/// Beim Übernehmen wird die Spule Eigentümer der referenzierten Kontakte (Original 0x4995ac): Kontakte mit anderer
/// Kennzeichnung übernehmen Kennzeichnung und Zähler der Spule wie bei „Eigentümer › Setzen“.
pub fn eigentuemer_setzen(p: &mut Project, owner: u64, ids: &[u64]) -> bool {
    let Some(ziel) = bmk_von(p, owner) else { return false };
    let Some(Kind::Symbol { bmk, ext, .. }) = p.find_obj(owner).map(|(_, o)| o.kind.clone()) else { return false };
    let mut changed = false;
    for &id in ids {
        if id == owner || bmk_von(p, id).is_none_or(|b| b == ziel) {
            continue;
        }
        if let Some(o) = p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects).find(|o| o.id == id) {
            if let Kind::Symbol { bmk: b, ext: e, .. } = &mut o.kind {
                *b = bmk.clone();
                e.counter = ext.counter;
                changed = true;
            }
        }
    }
    changed
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
        let Some(d) = self.zusatz.spiegel.as_mut() else { return };
        let Some(zeile) = d.pick else { return };
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
                let y = self.project.sheet(r).map(start_y).unwrap_or(180.0);
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
        assert_eq!((r.len(), r[0].referenz, y), (1, 2, 180.0));
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
    fn spiegel_folgt_spule_und_geloeschten_kontakten() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(sym(1, "-K1", 40.0));
        s.objects.push(sym(2, "-K1", 120.0));
        let rows = vec![Zeile { art: 0, referenz: 2, ..Default::default() }];
        s.objects.push(Obj { id: 9, kind: Kind::Spiegel { pos: P::new(40.0, 180.0), owner: 1, rows, auto: false }, pen: 0, fixed: false, group: None, raw: None });
        let x = |p: &Project| if let Some((_, Obj { kind: Kind::Spiegel { pos, .. }, .. })) = p.find_obj(9) { pos.x } else { panic!() };
        let mut lagen = Lagen::default();
        assert!(!folgen(&mut p, &mut lagen));
        // nur die Spule verschoben: der Spiegel zieht mit
        p.anlagen[0].sheets[0].objects[0].kind.translate(P::new(10.0, 5.0));
        assert!(folgen(&mut p, &mut lagen));
        assert_eq!(x(&p), 50.0);
        // beide verschoben (gemeinsam bzw. Rückgängig): keine doppelte Verschiebung
        p.anlagen[0].sheets[0].objects[0].kind.translate(P::new(-10.0, 0.0));
        p.anlagen[0].sheets[0].objects[2].kind.translate(P::new(-10.0, 0.0));
        assert!(!folgen(&mut p, &mut lagen));
        assert_eq!(x(&p), 40.0);
        // gelöschter Kontakt: Zeile entfällt auch ohne automatische Aufnahme
        p.anlagen[0].sheets[0].objects.remove(1);
        assert!(nachfuehren(&mut p, &[]));
        assert!(matches!(&p.find_obj(9).unwrap().1.kind, Kind::Spiegel { rows, .. } if rows.is_empty()));
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
        assert_eq!((b.i32(6), id_at(&b.data, 0), id_at(&b.data, 0x20), b.data[0x2c] & 1), (400, 77, 1000, 1));
        let stb = b.kid("STB").unwrap();
        assert_eq!((stb.data.len(), id_at(&stb.data, 0), id_at(&stb.data, 8), id_at(&stb.data, 16), stb.data[0x18]), (0x1c, 2000, 5, 6, 2));
        let Kind::Spiegel { rows, pos, .. } = lesen(&b, &p.anlagen[0].sheets[0]) else { panic!() };
        assert_eq!((rows[0].art, rows[0].felder[0].as_str(), rows[0].felder[1].as_str(), rows[0].kennung[0], pos.x), (2, "11", "", 2000, 40.0));
        assert_eq!(rows[0].felder[2], "14");
    }
}
