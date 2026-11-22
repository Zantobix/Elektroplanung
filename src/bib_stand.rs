//! Rückfrage beim Ablegen eines Bibliothekssymbols, das schon im Projekt steht (PaintBox1DragDrop 0x440bbe–0x440d6c).
//!
//! Das Original sucht beim Ablegen aus dem Bibliotheksbaum das Projektsymbol mit derselben Kennung (0x417ab0 mit
//! Symbol +0x50 = Kennung) und vergleicht das Änderungsdatum (Symbol +0x68, f64; 0x4260f4 „ungleich“). Ist es
//! verschieden, fragt es (MessageBox MB_YESNO|MB_ICONQUESTION, Überschrift „Achtung“):
//! „Das Symbol aus der Bibliothek ist älter  als das Symbol im Projekt.\nSoll es trotzdem übernommen werden?\n(Dies
//! wirkt sich auf das ganze Projekt aus)“ – „älter “ wenn das Bibliotheksdatum kleiner ist (0x441998), sonst
//! „ neuer “. Ja: die Bibliotheksfassung ersetzt das Projektsymbol im ganzen Projekt (0x4419bc) und wird gesetzt;
//! Nein: das Projektsymbol wird gesetzt. Gleiches Datum: ohne Frage das Projektsymbol.
//!
//! Daten: Symbole.tfi je Satz `[01][Kennung 8][01][Name 40][01][Haupt 8][01][Gruppe 8][01][Geändert f64]…`
//! (Kennung ab Byte 1, Geändert ab Byte 69), Projektsymbol SYM: Kennung @80, Geändert @104 – in W2 tragen alle aus
//! der Bibliothek stammenden Projektsymbole die Bibliothekskennung (z. B. „Sicherung“: Projekt 01.03.2025,
//! Bibliothek 05.03.2025 → „neuer“).

use crate::eprfile::Block;
use crate::model::Project;
use crate::symbols::SymDef;
use std::collections::HashMap;
use std::path::Path;

/// Kennung und Änderungsdatum (Delphi-Datum) je Bibliothekssymbol, Schlüssel wie `symbols::load_db` („db:…“).
pub fn bibliothek(dir: &Path) -> HashMap<String, (u64, f64)> {
    let mut out = HashMap::new();
    let Ok(s) = std::fs::read(dir.join("Symbole.tfi")) else { return out };
    let u32_at = |d: &[u8], p: usize| d.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let i64_at = |d: &[u8], p: usize| d.get(p..p + 8).map(|b| i64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0);
    let Some(mut p) = u32_at(&s, 0).map(|v| v as usize) else { return out };
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
        let name = crate::symbols::cstr(&rec[10..50]);
        let (haupt, gid) = (i64_at(rec, 51), i64_at(rec, 60));
        let id = u64::from_le_bytes(rec[1..9].try_into().unwrap());
        let datum = f64::from_le_bytes(rec[69..77].try_into().unwrap());
        out.insert(format!("db:{haupt}:{gid}:{name}"), (id, datum));
    }
    out
}

/// Änderungsdatum des Projektsymbols mit Kennung `id`: übernommener Bibliotheksstand, sonst SYM @104 der Datei.
pub fn projekt_datum(p: &Project, id: u64) -> Option<f64> {
    if let Some(d) = p.sym_geaendert.get(&id) {
        return Some(*d);
    }
    let prj = p.raw.as_deref()?.root("PRJ")?;
    let k = prj.kids_of("SYM").find(|k| k.data.get(80..88).map(|b| u64::from_le_bytes(b.try_into().unwrap())) == Some(id))?;
    k.data.get(104..112).map(|b| f64::from_le_bytes(b.try_into().unwrap()))
}

/// Text der Rückfrage: `aelter` = Bibliotheksfassung ist älter als die des Projekts.
pub fn frage_text(aelter: bool) -> String {
    use crate::meldungen::{BS_ALS, BS_ALTER, BS_BIBLIOTHEK, BS_NEUER};
    format!("{BS_BIBLIOTHEK}{}{BS_ALS}", if aelter { BS_ALTER } else { BS_NEUER })
}

/// Bibliotheksfassung als Projektsymbol: Zeichnung, Texte, Optionen aus der Bibliothek; Schlüssel, Gruppe und
/// Herkunft bleiben die des Projektsymbols (alle Instanzen zeigen danach die neue Fassung).
pub fn uebernehmen(projekt: &SymDef, bib: &SymDef) -> SymDef {
    let mut d = bib.clone();
    d.key = projekt.key.clone();
    d.group = projekt.group.clone();
    d.haupt = projekt.haupt;
    d.fav = projekt.fav;
    d.raw = projekt.raw.clone();
    if d.params.is_empty() {
        d.params = projekt.params.clone();
    }
    d
}

/// Beim Schreiben: übernommenes Änderungsdatum in den SYM-Block (@104). `geaendert` = schon neu geschriebener Block.
pub fn sym_block(p: &Project, k: &Block, geaendert: Option<Block>) -> Option<Block> {
    let id = k.data.get(80..88).map(|b| u64::from_le_bytes(b.try_into().unwrap()))?;
    let Some(datum) = p.sym_geaendert.get(&id) else { return geaendert };
    let mut b = geaendert.unwrap_or_else(|| k.clone());
    if b.data.len() >= 112 {
        b.data[104..112].copy_from_slice(&datum.to_le_bytes());
    }
    Some(b)
}

impl crate::app::App {
    /// Vor dem Ablegen eines Symbols aus der Bibliothek: Schlüssel, unter dem es gesetzt wird. Steht das Symbol
    /// (gleiche Kennung) schon im Projekt, ist es das Projektsymbol; bei anderem Änderungsdatum entscheidet die
    /// Rückfrage des Originals, ob die Bibliotheksfassung das Projektsymbol ersetzt.
    pub fn bibliothekssymbol_ablegen(&mut self, key: String) -> String {
        if !key.starts_with("db:") {
            return key;
        }
        let Some(dir) = self.lib_dir.clone() else { return key };
        let Some(&(id, bib_datum)) = bibliothek(&dir).get(&key) else { return key };
        let pkey = format!("epr:{id:016x}");
        if !self.project.symdefs.iter().any(|d| d.key == pkey) {
            return key;
        }
        let Some(prj_datum) = projekt_datum(&self.project, id) else { return pkey };
        if bib_datum == prj_datum {
            return pkey;
        }
        let ja = crate::meldungen::frage(crate::meldungen::T_ACHTUNG, &frage_text(bib_datum < prj_datum));
        if ja {
            let Some(bib) = crate::symbols::find(&self.lib, &key).cloned() else { return pkey };
            self.checkpoint();
            if let Some(d) = self.project.symdefs.iter_mut().find(|d| d.key == pkey) {
                *d = uebernehmen(d, &bib);
                let neu = d.clone();
                self.lib.retain(|x| x.key != pkey);
                self.lib.push(neu);
            }
            self.project.sym_geaendert.insert(id, bib_datum);
        }
        pkey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frage_wie_original() {
        // Wortlaut 0x5baad0 + 0x5baaf3 / 0x5baafa + 0x5bab02 (doppeltes Leerzeichen bei „neuer“ wie im Original)
        assert_eq!(
            frage_text(true),
            "Das Symbol aus der Bibliothek ist älter als das Symbol im Projekt.\nSoll es trotzdem übernommen werden?\n(Dies wirkt sich auf das ganze Projekt aus)"
        );
        assert!(frage_text(false).starts_with("Das Symbol aus der Bibliothek ist  neuer als das Symbol"));
    }

    #[test]
    fn uebernehmen_behaelt_schluessel() {
        let mut p = SymDef::empty("epr:0000000000000001", "Lampe", "-E%C");
        p.params = vec![(1, "P".into(), "x".into())];
        let mut b = SymDef::empty("db:200:5:Lampe", "Lampe", "-E%C");
        b.prims = vec![crate::symbols::Prim::Line(crate::model::P::new(0.0, 0.0), crate::model::P::new(1.0, 0.0), 0)];
        let d = uebernehmen(&p, &b);
        assert_eq!(d.key, p.key);
        assert_eq!(d.prims, b.prims);
        assert_eq!(d.params, p.params);
    }
}
