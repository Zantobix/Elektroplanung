//! Blattrahmen aus den Rändern der Anlage wie im Original.
//!
//! Die Ränder sind Eigenschaften der Anlage (ANL Feld 40–43 = +0xa0, +0xa4, +0xa8, +0xac in 0,1 mm; Anlagendialog
//! „Blattränder in mm (Hochformat)“: Oben, Unten, Links, Rechts). Die Zeichenroutine 0x438b94 liest sie je nach
//! Blattlage (BLT +0x80 Bit 0):
//! - hoch: Breite +0x98, Höhe +0x9c, links +0xa8, oben +0xa0, unten +0xa4, rechts +0xac – wie im Dialog;
//! - quer: Breite +0x9c, Höhe +0x98, links +0xa4, oben +0xa8, unten +0xac, rechts +0xa0 – das hochformatige Blatt
//!   ist im Uhrzeigersinn gedreht, der Heftrand „Links“ liegt also oben.
//!
//! W2.epr hat überall 50/50/100/50 (quer oben 10 mm, sonst 5 mm; hoch links 10 mm). Neue Anlagen bekommen im
//! Original die Projektvorgaben aus Form1 (+0x9cc…+0x9d8, ohne INI 50/50/200/50: Links 20 mm, Bild orig_03).
//!
//! `Sheet::frame` braucht die Ränder ohne Zugriff auf die Anlage; deshalb trägt jedes Blatt eine nicht
//! gespeicherte Kopie (`Sheet::raender`), die `abgleichen` nach dem Laden und vor jedem Bild aus der Anlage holt.

use crate::model::{Anlage, Project};

/// Ränder von W2.epr und der Vorlage (Oben, Unten, Links, Rechts im Hochformat; 0,1 mm).
pub const RAENDER_W2: [i32; 4] = [50, 50, 100, 50];
/// Vorgabe des Originals für neue Anlagen (Projektvorgaben ohne INI, Form1 +0x9cc…+0x9d8).
pub const RAENDER_NEU: [i32; 4] = [50, 50, 200, 50];

/// Reihenfolge in `AnlSettings::raender`
pub const OBEN: usize = 0;
pub const UNTEN: usize = 1;
pub const LINKS: usize = 2;
pub const RECHTS: usize = 3;

/// Rahmen (x0, y0, x1, y1) in mm eines Blattes `w` × `h` mm aus den Rändern `r` (0,1 mm, Reihenfolge wie im
/// Anlagendialog) – Zeichenroutine 0x438b94.
pub fn rahmen(w: f32, h: f32, hoch: bool, r: [i32; 4]) -> (f32, f32, f32, f32) {
    let mm = |i: usize| r[i] as f32 / 10.0;
    let (links, oben, rechts, unten) = if hoch { (mm(LINKS), mm(OBEN), mm(RECHTS), mm(UNTEN)) } else { (mm(UNTEN), mm(LINKS), mm(OBEN), mm(RECHTS)) };
    (links, oben, w - rechts, h - unten)
}

/// Ränder der Anlage in die Blätter übernehmen (Kopie für `Sheet::frame`).
pub fn abgleichen_anlage(a: &mut Anlage) {
    let r = a.set.raender;
    for s in &mut a.sheets {
        s.raender = r;
        s.ab_null = a.set.ab_null;
    }
}

/// Ränder aller Anlagen in ihre Blätter übernehmen – nach dem Laden und vor jedem Bild, damit eine Änderung in den
/// Anlagen-Eigenschaften den Rahmen sofort ändert.
pub fn abgleichen(p: &mut Project) {
    for a in &mut p.anlagen {
        abgleichen_anlage(a);
    }
}

/// Vorgabe für `Sheet::raender` (Blätter aus der Zwischenablage oder dem Rückgängig-Speicher bis zum nächsten Abgleich)
pub fn raender_w2() -> [i32; 4] {
    RAENDER_W2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Sheet, SHEET_H, SHEET_W};

    #[test]
    fn w2_quer_und_hoch() {
        // W2: quer oben 10 mm, sonst 5 mm; hoch links 10 mm, sonst 5 mm (Original-Ausdrucke)
        assert_eq!(rahmen(SHEET_W, SHEET_H, false, RAENDER_W2), (5.0, 10.0, 292.0, 205.0));
        assert_eq!(rahmen(SHEET_H, SHEET_W, true, RAENDER_W2), (10.0, 5.0, 205.0, 292.0));
    }

    #[test]
    fn vorgabe_neu_heftrand_links_20() {
        // hoch: Heftrand links 20 mm; quer liegt er oben
        assert_eq!(rahmen(SHEET_H, SHEET_W, true, RAENDER_NEU), (20.0, 5.0, 205.0, 292.0));
        assert_eq!(rahmen(SHEET_W, SHEET_H, false, RAENDER_NEU), (5.0, 20.0, 292.0, 205.0));
    }

    #[test]
    fn jeder_rand_an_seiner_seite() {
        // Oben 1, Unten 2, Links 3, Rechts 4 mm
        let r = [10, 20, 30, 40];
        // hoch wie im Dialog
        assert_eq!(rahmen(210.0, 297.0, true, r), (3.0, 1.0, 206.0, 295.0));
        // quer gedreht: links = Unten, oben = Links, rechts = Oben, unten = Rechts
        assert_eq!(rahmen(297.0, 210.0, false, r), (2.0, 3.0, 296.0, 206.0));
    }

    #[test]
    fn neue_anlage_mit_vorgabe_des_originals() {
        let mut app = crate::blatt_tasten::test_app();
        app.opt = crate::optionen::Optionen::default();
        app.new_project();
        assert_eq!(app.project.anlagen[0].set.raender, RAENDER_NEU);
        assert_eq!(app.project.anlagen[0].sheets[0].frame(), (5.0, 20.0, 292.0, 205.0));
        app.new_anlage();
        let a = app.project.anlagen.last().unwrap();
        assert_eq!(a.set.raender, RAENDER_NEU);
        assert_eq!(a.sheets[0].frame().1, 20.0);
        app.new_sheet();
        assert!(app.project.anlagen.last().unwrap().sheets.iter().all(|s| s.frame().1 == 20.0));
    }

    #[test]
    fn blatt_folgt_der_anlage() {
        let mut p = Project::new_default();
        let mut s = Sheet::new(2);
        s.portrait = true;
        p.anlagen[0].sheets.push(s);
        assert_eq!(p.anlagen[0].sheets[0].frame(), (5.0, 10.0, 292.0, 205.0));
        p.anlagen[0].set.raender = [70, 60, 250, 80];
        abgleichen(&mut p);
        assert_eq!(p.anlagen[0].sheets[0].frame(), (6.0, 25.0, 290.0, 202.0));
        assert_eq!(p.anlagen[0].sheets[1].frame(), (25.0, 7.0, 202.0, 291.0));
    }
}
