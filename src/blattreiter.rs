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

/// Erster sichtbarer Reiter: bei einem Blattwechsel (`wechsel`) so, dass der aktive ganz in `platz` passt (nach links
/// nur so weit wie nötig); sonst die bisherige Lage, begrenzt auf den größten sinnvollen Anfang.
pub fn erster_sichtbar(breiten: &[f32], platz: f32, erster: usize, aktiv: usize, wechsel: bool) -> usize {
    let n = breiten.len();
    if n == 0 {
        return 0;
    }
    let mut e = erster.min(groesster_erster(breiten, platz));
    if wechsel && aktiv < n {
        if aktiv < e {
            e = aktiv;
        }
        while e < aktiv && breiten[e..=aktiv].iter().sum::<f32>() > platz {
            e += 1;
        }
    }
    e
}

/// Größter sinnvoller erster Reiter: ab ihm passen alle folgenden in `platz`.
pub fn groesster_erster(breiten: &[f32], platz: f32) -> usize {
    let mut summe = 0.0;
    for i in (0..breiten.len()).rev() {
        summe += breiten[i];
        if summe > platz {
            return (i + 1).min(breiten.len().saturating_sub(1));
        }
    }
    0
}

#[cfg(test)]
mod leiste_tests {
    use super::*;

    #[test]
    fn aktiver_reiter_bleibt_sichtbar() {
        let b = vec![100.0; 10];
        assert_eq!(groesster_erster(&b, 350.0), 7);
        // Wechsel auf Reiter 6: Leiste rückt nach, bis 6 passt
        assert_eq!(erster_sichtbar(&b, 350.0, 0, 6, true), 4);
        // zurück auf 1: nach links
        assert_eq!(erster_sichtbar(&b, 350.0, 4, 1, true), 1);
        // ohne Wechsel (Leiste von Hand gerollt): bleibt
        assert_eq!(erster_sichtbar(&b, 350.0, 5, 1, false), 5);
        assert_eq!(groesster_erster(&b, 2000.0), 0);
    }
}
