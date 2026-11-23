//! Schriftfelddatei (.eff) wie im Original: Datei › „Schriftfeld speichern“ schreibt das Schriftfeld in eine
//! Datei, „Aus Datei“ (Hinzufügen › Schriftfeld, Projektvorgaben) liest sie wieder.
//!
//! Aufbau aus Schriftfeldspeichern1Click 0x4737a8 → Schreibroutine 0x47362c und Leseroutine 0x4732d4:
//! Kennung „Fuss02“ (6 Byte), zwei Merker ('c' = Teil folgt, 0x0b = leer) für Elemente und Texte, der Kopf des
//! Schriftfelds (16 Byte, wie die Daten des FUS-Blocks: Höhe in 0,1 mm, Feld 1 Bit 1 = relative Maße), dann
//! je vorhandenem Teil die Anzahl (u32) und die Elemente zu 0x50 Byte (wie GRS) bzw. die Länge (u32) und die
//! Texte (wie STR). „Fuss01“ wird ebenfalls gelesen: dort sind die Elementarten ab 2 um eins kleiner.
//! Fehlermeldungen wie im Original (Überschrift „Achtung Fehler!“).

use crate::eprfile::Block;
use crate::model::Project;
use crate::titleblock::TbOwned;

pub const T_FEHLER: &str = "Achtung Fehler!";
pub const FALSCHES_FORMAT: &str = "Datenformat der Datei ist nicht korrekt";
pub const FALSCHE_VERSION: &str = "Falsche Version der Datendatei";
pub const LESEFEHLER: &str = "Fehler beim laden der Schriftfeld-Daten";

const LEER: u8 = 0x0b;
const DA: u8 = b'c';

/// FUS-Block (Kopf + GRS + STR) als Schriftfelddatei.
pub fn schreiben(fus: &Block) -> Vec<u8> {
    let grs: Vec<u8> = fus.kids_of("GRS").flat_map(|g| g.data.iter().copied()).collect();
    let strs: Vec<u8> = fus.kids_of("STR").flat_map(|g| g.data.iter().copied()).collect();
    let n = grs.len() / 0x50;
    let mut out = b"Fuss02".to_vec();
    out.push(if n > 0 { DA } else { LEER });
    out.push(if strs.is_empty() { LEER } else { DA });
    let mut kopf = fus.data.clone();
    kopf.resize(16, 0);
    out.extend_from_slice(&kopf[..16]);
    if n > 0 {
        out.extend_from_slice(&(n as u32).to_le_bytes());
        out.extend_from_slice(&grs[..n * 0x50]);
    }
    if !strs.is_empty() {
        out.extend_from_slice(&(strs.len() as u32).to_le_bytes());
        out.extend_from_slice(&strs);
    }
    out
}

/// Schriftfelddatei lesen; Ergebnis als FUS-Block mit GRS und STR. Fehler = Meldungstext des Originals.
pub fn lesen(d: &[u8]) -> Result<Block, &'static str> {
    if d.len() < 6 || &d[0..4] != b"Fuss" {
        return Err(FALSCHES_FORMAT);
    }
    let version = (d[4] as i32 - 0x30) * 10 + d[5] as i32 - 0x30;
    if version != 1 && version != 2 {
        return Err(FALSCHE_VERSION);
    }
    let mut p = 6;
    let mut nimm = |n: usize| -> Result<&[u8], &'static str> {
        let s = d.get(p..p + n).ok_or(LESEFEHLER)?;
        p += n;
        Ok(s)
    };
    let merker = nimm(2)?.to_vec();
    let kopf = nimm(16)?.to_vec();
    let u32_lesen = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize;
    let mut grs = vec![];
    match merker[0] {
        DA => {
            let n = u32_lesen(nimm(4)?);
            grs = nimm(n.checked_mul(0x50).ok_or(LESEFEHLER)?)?.to_vec();
            if version == 1 {
                for e in grs.chunks_exact_mut(0x50) {
                    if e[0x3c] > 1 {
                        e[0x3c] += 1;
                    }
                }
            }
        }
        LEER => {}
        _ => return Err(LESEFEHLER),
    }
    let mut strs = vec![];
    match merker[1] {
        DA => {
            let n = u32_lesen(nimm(4)?);
            strs = nimm(n)?.to_vec();
        }
        LEER => {}
        _ => return Err(LESEFEHLER),
    }
    let mut fus = Block::new("FUS", kopf);
    fus.kids = vec![Block::new("GRS", grs), Block::new("STR", strs)];
    Ok(fus)
}

/// FUS-Block des Projekts, wie ihn das Speichern im Originalformat schreibt.
pub fn fus_des_projekts(p: &Project) -> Result<Block, String> {
    let d = crate::epr_write::to_bytes(p)?;
    let f = crate::eprfile::EprFile::parse(&d)?;
    f.root("PRJ").and_then(|prj| prj.kid("FUS")).cloned().ok_or_else(|| "kein Schriftfeld im Projekt".to_string())
}

/// Schriftfeld (relativ, wie im Modell) aus einer Schriftfelddatei.
pub fn schriftfeld_aus(d: &[u8]) -> Result<Vec<TbOwned>, &'static str> {
    let fus = lesen(d)?;
    let tb = crate::epr_projekt::schriftfeld_aus(&fus);
    // Feld 1 Bit 1 gelöscht = Maßangaben absolut (0,1 mm), dann auf die Höhe aus Feld 0 beziehen
    let absolut = fus.data.len() >= 8 && fus.i32(1) & 2 == 0;
    Ok(if absolut { crate::titleblock::nach_relativ(&tb, fus.i32(0).max(1)) } else { tb })
}

/// Dateiendung .eff?
pub fn ist_eff(p: &std::path::Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("eff")).unwrap_or(false)
}

impl crate::app::App {
    /// Datei › Schriftfeld speichern (0x4737a8): Speichern-Dialog „Schriftfelddatei|*.eff“, Endung „eff“.
    pub fn schriftfeld_speichern(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Schriftfelddatei", &["eff"]).save_file() else { return };
        let p = if p.extension().is_none() { p.with_extension("eff") } else { p };
        match fus_des_projekts(&self.project).and_then(|f| std::fs::write(&p, schreiben(&f)).map_err(|e| e.to_string())) {
            Ok(()) => self.info = format!("Schriftfeld gespeichert: {}", p.display()),
            Err(e) => crate::meldungen::warnung(T_FEHLER, &format!("Schriftfeld konnte nicht gespeichert werden: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rundreise_und_fehler() {
        let mut p = Project::new_default();
        p.titleblock = crate::titleblock::default_owned();
        let fus = fus_des_projekts(&p).unwrap();
        let d = schreiben(&fus);
        assert_eq!(&d[0..6], b"Fuss02");
        let zurueck = lesen(&d).unwrap();
        assert_eq!(zurueck.data, fus.data[..16].to_vec());
        assert_eq!(crate::epr_projekt::schriftfeld_aus(&zurueck), crate::epr_projekt::schriftfeld_aus(&fus));
        assert_eq!(schriftfeld_aus(&d).unwrap().len(), crate::epr_projekt::schriftfeld_aus(&fus).len());
        assert_eq!(lesen(b"EP12xx").unwrap_err(), FALSCHES_FORMAT);
        assert_eq!(lesen(b"Fuss03").unwrap_err(), FALSCHE_VERSION);
        assert_eq!(lesen(&d[..30]).unwrap_err(), LESEFEHLER);
        // leeres Schriftfeld: beide Merker 0x0b, nur der Kopf
        let leer = schreiben(&Block::new("FUS", vec![150, 0, 0, 0, 2, 0, 0, 0]));
        assert_eq!((leer.len(), leer[6], leer[7]), (24, LEER, LEER));
        assert!(lesen(&leer).unwrap().kids.iter().all(|k| k.data.is_empty()));
    }

    /// W2: Elemente und Texte des FUS-Blocks kommen unverändert aus der Datei zurück.
    #[test]
    fn w2_schriftfeld_rundreise() {
        let Ok(pfad) = std::env::var("EP_SAMPLE") else { return };
        let f = crate::eprfile::EprFile::read(std::path::Path::new(&pfad)).unwrap();
        let fus = f.root("PRJ").and_then(|p| p.kid("FUS")).unwrap();
        let d = schreiben(fus);
        let n = fus.kid("GRS").unwrap().data.len() / 0x50;
        assert_eq!(d.len(), 8 + 16 + 4 + n * 0x50 + 4 + fus.kid("STR").unwrap().data.len());
        let z = lesen(&d).unwrap();
        assert_eq!(z.data, fus.data);
        assert_eq!(z.kid("GRS").unwrap().data, fus.kid("GRS").unwrap().data);
        assert_eq!(z.kid("STR").unwrap().data, fus.kid("STR").unwrap().data);
        let p = crate::epr::import(std::path::Path::new(&pfad)).unwrap();
        assert_eq!(schriftfeld_aus(&d).unwrap(), p.titleblock);
    }

    #[test]
    fn version_1_verschiebt_arten() {
        let mut e = vec![0u8; 0x50];
        e[0x3c] = 5;
        let mut d = b"Fuss01".to_vec();
        d.extend_from_slice(&[DA, LEER]);
        d.extend_from_slice(&[0; 16]);
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&e);
        let f = lesen(&d).unwrap();
        assert_eq!(f.kids[0].data[0x3c], 6);
    }
}
