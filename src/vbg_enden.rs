//! Leitungsenden (VBG) beim Speichern aus der Geometrie bestimmen – wie das Original sie schreibt (Maus-Befund
//! M21, 22.11.2026: im Nachbau gezeichnete Leitungen trugen Art 1 ohne Ziel bzw. Art 0, der Klemmplan des Originals
//! blieb leer).
//!
//! Jedes Ende einer Verbindung trägt Anschlussart (Byte 0 bzw. 0x14), Ziel (Feld 2/3 bzw. 6/7) und Anschluss
//! (Feld 4 bzw. 8). Abgeleitet aus allen 1754 Enden in W2 (tests/vbg_enden_w2.rs):
//! - Art 1 Symbol: Ziel = VSY-Kennung (Byte 96), Anschluss = **Feld 4 des Anschlusselements** (Typ 7) in den
//!   Zeichenelementen des Projektsymbols (SYM › GRD › GRS) – nicht der Index; z. B. „SPD“ 1, 2, 5, 6, 7. Die Lage
//!   des Anschlusses verschiebt sich je Option um Feld 13 (0,1 mm) waagerecht (mehrpolige Leistungskontakte und
//!   Sicherungen: Option 2 → zweimal −3,5 mm). Bei Anschlüssen mit Kreis (Verteilerbox) endet die Leitung wie an
//!   Klemmstellen 0,8 mm vor dem Anschluss.
//! - Art 2 Sammelschiene: Ziel = OBJ-Kennung der Schiene (Feld 0/1), Anschluss 0; 1 nur am Leitungsende, wenn es am
//!   Ende der Schiene liegt (ohne Knotenpunkt, siehe knoten.rs), siehe [`schienen_anschluss`].
//! - Art 3 andere Leitung (Ende im Innern): Ziel = deren VBG-Kennung (Feld 10/11), Anschluss −1.
//! - Art 5 Klemmstelle (Leitung endet 0,8 mm vor dem Mittelpunkt): Ziel = OBJ-Kennung, Anschluss 0.
//!
//! - Art 6 Verweis (Pfeilspitze): Ziel = OBJ-Kennung, Anschluss 0 (kommt in W2 nicht vor; aus dem Dekompilat:
//!   MouseDown/MouseUp beim Zeichnen setzen je nach OBJ-Byte 0x14 Art 2 (1 Sammelschiene), 6 (2 Verweis), sonst 5).
//!
//! Freie Enden: Art 0, Ziel 0, Anschluss 0.

use crate::eprfile::{i32_at, Block};
use crate::model::*;
use crate::symbols::{self, SymDef};
use std::cell::RefCell;
use std::collections::HashMap;

pub const ART_SYMBOL: u8 = 1;
pub const ART_SCHIENE: u8 = 2;
pub const ART_LEITUNG: u8 = 3;
pub const ART_KLEMME: u8 = 5;
pub const ART_VERWEIS: u8 = 6;
const T: f32 = 0.05;
/// Leitung endet am Kreis einer Klemmstelle bzw. eines Anschlusses, 0,8 mm vor dem Mittelpunkt
const NAH: f32 = crate::klemme_leitung::KLEMME_R + T;

/// Byte der Anschlussart, Feld der Zielkennung, Feld der Anschlussnummer – für Anfang (0) und Ende (1).
pub const ENDEN: [(usize, usize, usize); 2] = [(0, 2, 4), (0x14, 6, 8)];

/// Angaben eines Leitungsendes im VBG.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Ende {
    pub art: u8,
    pub ziel: u64,
    pub nr: i32,
}

impl Ende {
    /// Gespeichertes Ende `i` (0 Anfang, 1 Ende) eines VBG-Blocks.
    pub fn lesen(b: &Block, i: usize) -> Ende {
        let (art_at, id_feld, nr_feld) = ENDEN[i];
        let ziel = b.data.get(id_feld * 4..id_feld * 4 + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0);
        Ende { art: b.data.get(art_at).copied().unwrap_or(0), ziel, nr: b.i32(nr_feld) }
    }

    /// In den VBG-Block schreiben (die übrigen Bytes der Felder 0/1 bzw. 5 bleiben, wie sie sind).
    pub fn schreiben(&self, b: &mut Block, i: usize) {
        let (art_at, id_feld, nr_feld) = ENDEN[i];
        if b.data.len() < 0x24 {
            b.data.resize(0x24, 0);
        }
        b.data[art_at] = self.art;
        b.data[id_feld * 4..id_feld * 4 + 8].copy_from_slice(&self.ziel.to_le_bytes());
        b.set_i32(nr_feld, self.nr);
    }
}

/// Zeichenelemente (je 0x50 Byte) eines Symbols, wie sie im Projekt gespeichert werden: der SYM-Block der
/// Grundlage, solange das Symbol nicht geändert wurde, sonst wie `epr_write` ihn neu schreibt.
fn elemente(project: &Project, def: &SymDef) -> Vec<u8> {
    let id = def.key.strip_prefix("epr:").and_then(|h| u64::from_str_radix(h, 16).ok());
    let sym = id.and_then(|id| project.raw.as_deref()?.root("PRJ")?.kids_of("SYM").find(|s| s.data.get(80..88) == Some(&id.to_le_bytes()[..])));
    if let Some(k) = sym.filter(|k| !crate::epr_write::sym_geaendert(project, k)) {
        if let Some(grd) = k.kid("GRD") {
            return grd.kids_of("GRS").flat_map(|g| g.data.iter().copied()).collect();
        }
    }
    crate::epr_write::sym_elements(def).0
}

thread_local! {
    /// Anschlüsse je Symbol für den laufenden Speichervorgang (Grundlage, Schlüssel, Anzahl Anschlüsse)
    static ANSCHLUESSE: RefCell<HashMap<(usize, String, usize), Vec<(i32, f32)>>> = RefCell::new(HashMap::new());
}

/// Zu Beginn des Speicherns: zwischengespeicherte Anschlüsse verwerfen (Symbole können geändert worden sein).
pub fn beginnen() {
    ANSCHLUESSE.with(|a| a.borrow_mut().clear());
}

/// Anschlüsse eines Symbols in der Reihenfolge von `def.pins`: Nummer (Element Typ 7, Feld 4) und waagerechter
/// Versatz je Option in mm (Feld 13). Ausgelassen wie beim Lesen (`symbols::parse_elements`): Seite < 0 oder > 32.
pub fn anschluesse(project: &Project, def: &SymDef) -> Vec<(i32, f32)> {
    let schluessel = (project.raw.as_deref().map(|r| r as *const _ as usize).unwrap_or(0), def.key.clone(), def.pins.len());
    if let Some(v) = ANSCHLUESSE.with(|a| a.borrow().get(&schluessel).cloned()) {
        return v;
    }
    let v = anschluesse_neu(project, def);
    ANSCHLUESSE.with(|a| a.borrow_mut().insert(schluessel, v.clone()));
    v
}

fn anschluesse_neu(project: &Project, def: &SymDef) -> Vec<(i32, f32)> {
    let els = elemente(project, def);
    let v: Vec<(i32, f32)> = els
        .chunks_exact(0x50)
        .filter(|e| e[0x3c] == 7 && (0..=32).contains(&i32_at(e, 0x48)))
        .map(|e| (i32_at(e, 16), i32_at(e, 0x34) as f32 / 10.0))
        .collect();
    if v.len() == def.pins.len() {
        v
    } else {
        // passt nicht zur Definition (sollte nicht vorkommen): fortlaufend ab 1 wie `epr_write::encode_elements`
        (1..=def.pins.len() as i32).map(|n| (n, 0.0)).collect()
    }
}

/// Sichtbare Anschlüsse eines Symbols auf dem Blatt: (Lage, Nummer).
pub fn symbol_anschluesse(project: &Project, o: &Obj) -> Vec<(P, i32)> {
    let Kind::Symbol { key, pos, rot, mirror_h, mirror_v, scale, option, flags, .. } = &o.kind else { return vec![] };
    let Some(def) = symbols::find(&project.symdefs, key) else { return vec![] };
    let nrn = anschluesse(project, def);
    def.pins
        .iter()
        .enumerate()
        .filter(|(i, _)| symbols::visible(*option, def.pin_vis.get(*i)) && symbols::flag_visible(*flags, def.pin_info.get(*i).map(|x| &x.1)))
        .map(|(i, q)| {
            let (nr, versatz) = nrn[i];
            (sym_xf(P::new(q.x + versatz * *option as f32, q.y), *pos, *rot, *mirror_h, *mirror_v, *scale), nr)
        })
        .collect()
}

/// Liegt `p` auf der Sammelschiene?
fn auf_schiene(k: &Kind, p: P) -> bool {
    matches!(k, Kind::Busbar { a, x2, .. } if (p.y - a.y).abs() < T && p.x >= a.x.min(*x2) - T && p.x <= a.x.max(*x2) + T)
}

/// Anschluss am Ende (`ende` = true) einer Leitung an einer Sammelschiene: 1, wenn sie an einem Ende der Schiene
/// liegt (dort zeichnet das Original keinen Knotenpunkt), sonst 0. Am Anfang schreibt das Original immer 0
/// (MouseDown 0x41d0xx: Feld 4 = 0; MouseUp 0x40a9xx: Feld 8 = „Anschluss getroffen“).
pub fn schienen_anschluss(k: &Kind, p: P, ende: bool) -> i32 {
    match k {
        Kind::Busbar { a, x2, .. } if ende && ((p.x - a.x).abs() < T || (p.x - x2).abs() < T) => 1,
        _ => 0,
    }
}

/// Ziel des Leitungsendes `p` der Leitung `eigen` auf dem Blatt nach der Geometrie (`None` = frei).
pub fn bestimmen(project: &Project, sheet: &Sheet, eigen: u64, p: P, ende: bool) -> Option<Ende> {
    let kennung = crate::epr_neu::kennung;
    let pins: Vec<(&Obj, Vec<(P, i32)>)> = sheet.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| (o, symbol_anschluesse(project, o))).collect();
    // Symbolanschluss genau getroffen
    for (o, ps) in &pins {
        if let Some((_, nr)) = ps.iter().find(|(q, _)| q.dist(p) < T) {
            return Some(Ende { art: ART_SYMBOL, ziel: kennung(o), nr: *nr });
        }
    }
    // Sammelschiene vor Klemmstelle (W2 Blatt 2: kurze Leitung von einer Klemme zur Schiene, deren Ende auch 0,8 mm
    // vor der nächsten Klemme unter der Schiene liegt)
    if let Some(o) = sheet.objects.iter().find(|o| auf_schiene(&o.kind, p)) {
        return Some(Ende { art: ART_SCHIENE, ziel: kennung(o), nr: schienen_anschluss(&o.kind, p, ende) });
    }
    // Klemmstelle: die Leitung endet am Kreis
    if let Some(o) = sheet.objects.iter().find(|o| matches!(&o.kind, Kind::Terminal { pos, .. } if pos.dist(p) <= NAH)) {
        return Some(Ende { art: ART_KLEMME, ziel: crate::klemmen::uid(o), nr: 0 });
    }
    if let Some(o) = sheet.objects.iter().find(|o| o.id != eigen && matches!(&o.kind, Kind::Wire { pts, .. } if crate::knoten::im_innern(pts, p))) {
        return Some(Ende { art: ART_LEITUNG, ziel: kennung(o), nr: -1 });
    }
    // Anschluss mit Kreis (Verteilerbox): nächster Anschluss bis 0,85 mm
    let nah = pins.iter().flat_map(|(o, ps)| ps.iter().map(move |(q, nr)| (q.dist(p), *o, *nr))).filter(|(d, _, _)| *d <= NAH).min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, o, nr)) = nah {
        return Some(Ende { art: ART_SYMBOL, ziel: kennung(o), nr });
    }
    sheet.objects.iter().find(|v| matches!(&v.kind, Kind::Verweis { pos, .. } if pos.dist(p) < T)).map(|v| Ende { art: ART_VERWEIS, ziel: kennung(v), nr: 0 })
}

/// Ziel des gespeicherten Endes, falls es ein unverändertes Objekt dieses Blattes der passenden Art ist.
fn ziel_unveraendert(sheet: &Sheet, e: &Ende) -> bool {
    e.ziel != 0
        && sheet.objects.iter().any(|x| {
            let passt = matches!((&x.kind, e.art), (Kind::Symbol { .. }, ART_SYMBOL) | (Kind::Busbar { .. }, ART_SCHIENE) | (Kind::Wire { .. }, ART_LEITUNG) | (Kind::Terminal { .. }, ART_KLEMME) | (Kind::Verweis { .. }, ART_VERWEIS));
            passt && x.raw.as_deref().is_some_and(|r| r.kind == x.kind) && crate::epr_neu::kennung(x) == e.ziel
        })
}

/// Beide Enden der Leitung `o` im VBG-Block `b` aus der Geometrie schreiben. `alt` = Punkte beim Laden (None bei
/// neuen Leitungen). Ein unbewegtes Ende, dessen gespeichertes Ziel ein unverändertes Objekt der passenden Art ist,
/// bleibt, wie das Original es schrieb (W2 Blatt 2: eine Leitung endet 9 mm hinter dem Ende der später gekürzten
/// Schiene L1 und trägt sie weiter als Ziel). Alle übrigen Enden – neue, bewegte, solche mit verschobenem,
/// gelöschtem oder fehlendem Ziel (Nachbau-Dateien vor dem 22.11.2026: Art 1 ohne Ziel) – folgen der Geometrie.
pub fn schreiben(project: &Project, o: &Obj, b: &mut Block, alt: Option<&[P]>) {
    let Kind::Wire { pts, .. } = &o.kind else { return };
    if pts.len() < 2 || b.data.len() < 0x24 {
        return;
    }
    let Some(sheet) = project.find_obj(o.id).and_then(|(r, _)| project.sheet(r)) else { return };
    for i in 0..2 {
        let p = if i == 0 { pts[0] } else { pts[pts.len() - 1] };
        let gespeichert = Ende::lesen(b, i);
        let unbewegt = alt.and_then(|a| if i == 0 { a.first() } else { a.last() }).is_some_and(|a| a.dist(p) <= 1e-4);
        if unbewegt && ziel_unveraendert(sheet, &gespeichert) {
            continue;
        }
        let ende = bestimmen(project, sheet, o.id, p, i == 1).unwrap_or_default();
        if ende != gespeichert {
            ende.schreiben(b, i);
        }
    }
}

/// Unveränderte Leitung beim Speichern: Block mit nachgeführten Enden, falls ein gespeichertes Ende nicht mehr zur
/// Geometrie passt (Symbol, Klemmstelle, Schiene oder Zielleitung verschoben/gelöscht, Datei ohne gültige Enden),
/// sonst `None` (Originalblock bleibt byteidentisch).
pub fn veraltet(project: &Project, o: &Obj, b: &Block) -> Option<Block> {
    let Kind::Wire { pts, .. } = &o.kind else { return None };
    let mut neu = b.clone();
    schreiben(project, o, &mut neu, Some(pts));
    (neu.data != b.data).then_some(neu)
}
