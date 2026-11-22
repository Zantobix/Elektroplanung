//! Beschriftung der Blattreiter: „[Nummer] Name“ wie im Original. Kommt eine Beschriftung unter den offenen
//! Reitern mehrfach vor (z. B. „[1] Blatt 1“ in zwei Anlagen), wird die Anlagen-Kurzform (ohne Kurzform der
//! Anlagenname) vorangestellt – bewusste Abweichung vom Original, das die Reiter dann nicht unterscheidet (Maus-Befund M8).

use crate::model::{Project, SheetRef};

/// Grundbeschriftung eines Blattes ohne Anlagenzusatz.
pub fn grund(p: &Project, r: SheetRef) -> String {
    p.sheet(r).map(|s| format!("[{}] {}", s.number, s.name)).unwrap_or_default()
}

/// Beschriftungen aller Reiter `tabs` in derselben Reihenfolge.
pub fn beschriftungen(p: &Project, tabs: &[SheetRef]) -> Vec<String> {
    let basis: Vec<String> = tabs.iter().map(|r| grund(p, *r)).collect();
    tabs.iter()
        .zip(&basis)
        .map(|(r, b)| {
            // nur bei gleicher Beschriftung aus einer anderen Anlage unterscheiden
            let doppelt = tabs.iter().zip(&basis).any(|(r2, b2)| b2 == b && r2.anlage != r.anlage);
            match p.anlagen.get(r.anlage).filter(|_| doppelt) {
                Some(a) => {
                    let k = a.set.kurz.trim();
                    format!("{} {b}", if k.is_empty() { a.name.as_str() } else { k })
                }
                None => b.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn projekt() -> Project {
        let mut p = Project::new_default();
        let mut a2 = p.anlagen[0].clone();
        a2.name = "Anlage 2".into();
        a2.set.kurz = "UV1".into();
        p.anlagen.push(a2);
        p
    }

    #[test]
    fn eindeutige_reiter_bleiben_wie_im_original() {
        let p = projekt();
        let r = SheetRef { anlage: 0, sheet: 0 };
        assert_eq!(beschriftungen(&p, &[r]), vec![grund(&p, r)]);
        assert!(grund(&p, r).starts_with("[1] "));
    }

    #[test]
    fn gleiche_reiter_zweier_anlagen_bekommen_die_anlage() {
        let p = projekt();
        let t = [SheetRef { anlage: 0, sheet: 0 }, SheetRef { anlage: 1, sheet: 0 }];
        let g = grund(&p, t[0]);
        let b = beschriftungen(&p, &t);
        assert_eq!(b[0], format!("{} {g}", p.anlagen[0].name));
        assert_eq!(b[1], format!("UV1 {g}"));
    }
}
