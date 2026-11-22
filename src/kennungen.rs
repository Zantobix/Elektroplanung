//! Kennungen neuer Objekte (OBJ, VBG, VSY, SMS, SYM, ANL) beim Speichern im Originalformat.
//!
//! Das Original kennzeichnet Objekte mit ihrem Erstellungszeitpunkt (Delphi-Datum als 8-Byte-Double). Früher
//! vergaben mehrere Teile des Speicherns (epr_neu, epr_write, klemmen, klemme_leitung, epr_projekt) je eigene
//! Folgen „jetzt + n ms“ – so trugen z. B. die neue Klemmstelle und die erste neue Leitung dieselbe Kennung
//! (Maus-Befund M21, x_nachbau_klemmplan_k2.epr: VBG Feld 10/11 = OBJ Feld 0/1 der Klemmstelle). Jetzt gibt es
//! eine einzige, streng steigende Folge, die außerdem alle schon in der Datei bzw. im Projekt stehenden Kennungen
//! überspringt.

use crate::eprfile::{Block, EprFile};
use crate::model::{Obj, Project};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

const MS: f64 = 1.0 / 86_400_000.0;

thread_local! {
    static LETZTE: Cell<f64> = const { Cell::new(0.0) };
    static BELEGT: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
    /// Objekte, deren Kennung aus der Datei doppelt vergeben war: (Objekt-Id, alte Kennung) → neue Kennung
    static UMBENANNT: RefCell<HashMap<(u64, u64), u64>> = RefCell::new(HashMap::new());
}

/// Jetzt als Delphi-Datum (Tage seit 30.12.1899, ganze Millisekunden).
pub fn jetzt() -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    (chrono::Local::now().naive_local() - base).num_milliseconds() as f64 / 86_400_000.0
}

/// Neue Kennung: mindestens eine Millisekunde nach der zuletzt vergebenen, nie eine belegte.
pub fn neu() -> u64 {
    let mut t = jetzt().max(LETZTE.with(|l| l.get()) + MS);
    BELEGT.with(|b| {
        let mut b = b.borrow_mut();
        while b.contains(&t.to_bits()) {
            t += MS;
        }
        b.insert(t.to_bits());
    });
    LETZTE.with(|l| l.set(t));
    t.to_bits()
}

/// Kennung eines Blocks (Lage je Blockart), 0 = keine.
pub fn kennung_im_block(b: &Block) -> u64 {
    let at = match b.tag_str() {
        "OBJ" | "BTL" => 0,
        "VBG" => 40,
        "VSY" => 96,
        "SMS" => 32,
        "SYM" => 80,
        "BLT" => 72,
        "ANL" => crate::epr::ANL_ID_AT,
        _ => return 0,
    };
    b.data.get(at..at + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0)
}

fn belegen_block(b: &Block, set: &mut HashSet<u64>) {
    let k = kennung_im_block(b);
    if k != 0 {
        set.insert(k);
    }
    for k in &b.kids {
        belegen_block(k, set);
    }
}

/// Vor dem Speichern: alle Kennungen der Grundlage und der Originalblöcke im Projekt (auch solche, die beim
/// Bearbeiten neu gesetzt wurden, z. B. das zweite Teilstück einer an einer Klemmstelle geteilten Leitung) als
/// belegt merken. Doppelt vergebene Kennungen geladener Objekte (vom Nachbau vor dem 22.11.2026 geschriebene
/// Dateien) bekommen neue: zuerst Verbindungen, dann Symbole, zuletzt OBJ (Klemmstellen stehen auch im KFO).
pub fn beginnen(project: &Project, base: &EprFile) {
    let mut vorhanden: HashMap<u64, Vec<(u8, u64)>> = HashMap::new();
    for o in project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        let Some(r) = o.raw.as_deref() else { continue };
        let rang = match r.block.tag_str() {
            "VBG" => 0,
            "VSY" => 1,
            "OBJ" => 2,
            _ => continue,
        };
        let k = kennung_im_block(&r.block);
        if k != 0 {
            vorhanden.entry(k).or_default().push((rang, o.id));
        }
    }
    let mut doppelt: Vec<(u64, u64)> = vec![];
    for (k, v) in vorhanden.iter_mut().filter(|(_, v)| v.len() > 1) {
        // das Objekt mit dem höchsten Rang behält die Kennung
        v.sort();
        doppelt.extend(v[..v.len() - 1].iter().map(|(_, id)| (*id, *k)));
    }
    doppelt.sort();
    BELEGT.with(|s| {
        let mut s = s.borrow_mut();
        for r in &base.roots {
            belegen_block(r, &mut s);
        }
        for o in project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
            if let Some(r) = o.raw.as_deref() {
                belegen_block(&r.block, &mut s);
            }
        }
    });
    let neu: HashMap<(u64, u64), u64> = doppelt.into_iter().map(|x| (x, neu())).collect();
    UMBENANNT.with(|u| *u.borrow_mut() = neu);
}

/// Neue Kennung eines geladenen Objekts, dessen Kennung in der Datei doppelt vergeben war (siehe [`beginnen`]).
pub fn umbenannt(o: &Obj) -> Option<u64> {
    let alt = kennung_im_block(&o.raw.as_deref()?.block);
    UMBENANNT.with(|u| u.borrow().get(&(o.id, alt)).copied())
}

/// Kennung im Block eines umbenannten Objekts ersetzen.
pub fn umbenennen(o: &Obj, b: &mut Block) {
    let Some(k) = umbenannt(o) else { return };
    let at = match b.tag_str() {
        "OBJ" => 0,
        "VBG" => 40,
        "VSY" => 96,
        _ => return,
    };
    if b.data.len() >= at + 8 {
        b.data[at..at + 8].copy_from_slice(&k.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn streng_steigend_und_ohne_belegte() {
        let a = super::neu();
        super::BELEGT.with(|b| b.borrow_mut().insert((f64::from_bits(a) + super::MS).to_bits()));
        let b = super::neu();
        assert!(f64::from_bits(b) > f64::from_bits(a) + super::MS * 1.5);
        assert_ne!(a, b);
    }
}
