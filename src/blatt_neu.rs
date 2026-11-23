//! Blattdialog (TSBlatt) wie im Original:
//! - Hinzufügen › Blatt (Blatteinfgen1Click 0x461960) öffnet den Dialog mit Strompfade „-1“ (von der Anlage),
//!   Ausrichtung „Querformat“ (ItemIndex 1, unabhängig von der Projektvorgabe „Hochformat“, die nur für ohne
//!   Dialog angelegte Blätter gilt: 0x403ce4), Blattart „Schaltplan“ und „Auto+“ aus; die Anlage ist wählbar.
//! - Blattart oder Anlage wechseln (Blatt1Blatt_AnlageChange 0x4a3970) setzt die Vorgaben (TIn-Default, nur
//!   sichtbar/wirksam, solange nichts eingetragen ist): Bezeichnung = Name der Blattart („Deckblatt“,
//!   „Inhaltsverzeichnis“, „Querverweise“, „Klemmenplan“, „Sicherungsübersicht“, „Stückliste“; Schaltplan
//!   „Blatt “ + Blattnummer), Blattnummer 0 für Deckblatt/Inhaltsverzeichnis, sonst die nächste Nummer der Anlage;
//!   Subindex 0, beim Inhaltsverzeichnis der nächste nach dem letzten Inhaltsverzeichnis.
//! - „Auto+“ ist nur bei Inhaltsverzeichnis, Querverweisen, Klemmenplan, Sicherungsübersicht und Artikelliste
//!   wählbar (0x4a3970: -0xb2) und liegt in Blatt +0x80 Bit 0x20 (0x461e20 beim Anlegen, 0x44a43c beim Bearbeiten).
//! - Beim Bearbeiten eines vorhandenen Blattes sind Anlage und Blattart gesperrt (0x44a3da/0x44a3ed); der Nachbau
//!   lässt die Blattart wählbar (Listenblätter aus vorhandenen Blättern machen).

use crate::model::*;

/// Name der Blattart als Vorgabe der Bezeichnung (Strings aus 0x4a3970).
pub fn vorgabe_name(art: usize, nummer: &str) -> String {
    match art {
        0 => "Deckblatt".into(),
        1 => "Inhaltsverzeichnis".into(),
        2 => "Querverweise".into(),
        4 => "Klemmenplan".into(),
        5 => "Sicherungsübersicht".into(),
        6 => "Stückliste".into(),
        _ => format!("Blatt {nummer}"),
    }
}

/// „Auto+“ wählbar (Listenblätter außer Deckblatt und Schaltplan).
pub fn auto_plus_frei(art: usize) -> bool {
    matches!(art, 1 | 2 | 4 | 5 | 6)
}

/// Vorgaben [Bezeichnung, Blattnummer, Subindex] für Blattart `art` in Anlage `anlage`; `ohne` = das gerade
/// angelegte Blatt (zählt nicht mit).
pub fn vorgaben(p: &Project, anlage: usize, art: usize, ohne: SheetRef) -> [String; 3] {
    let blaetter: Vec<&Sheet> = p
        .anlagen
        .get(anlage)
        .map(|a| a.sheets.iter().enumerate().filter(|(i, _)| !(anlage == ohne.anlage && *i == ohne.sheet)).map(|(_, s)| s).collect())
        .unwrap_or_default();
    match art {
        0 => [vorgabe_name(0, ""), "0".into(), "0".into()],
        1 => {
            // nächster Subindex nach dem letzten Inhaltsverzeichnis unter Nummer 0
            let sub = blaetter.iter().filter(|s| s.number == 0 && s.blattart() == 1).map(|s| s.subindex as u32 + 1).max().unwrap_or(1);
            [vorgabe_name(1, ""), "0".into(), sub.to_string()]
        }
        _ => {
            let n = (blaetter.iter().map(|s| s.number).max().unwrap_or(0) + 1).to_string();
            [vorgabe_name(art, &n), n, "0".into()]
        }
    }
}

/// Neues Blatt `r` in die Anlage `ziel` umhängen (Anlagenwahl beim Anlegen); liefert die neue Lage.
pub fn umhaengen(p: &mut Project, r: SheetRef, ziel: usize) -> SheetRef {
    if r.anlage == ziel || ziel >= p.anlagen.len() || r.sheet >= p.anlagen.get(r.anlage).map(|a| a.sheets.len()).unwrap_or(0) {
        return r;
    }
    let s = p.anlagen[r.anlage].sheets.remove(r.sheet);
    let a = &mut p.anlagen[ziel];
    a.sheets.push(s);
    crate::rahmen::abgleichen_anlage(a);
    SheetRef { anlage: ziel, sheet: a.sheets.len() - 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vorgaben_je_blattart() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets.push(Sheet::new(4));
        let neu = SheetRef { anlage: 0, sheet: 1 };
        assert_eq!(vorgaben(&p, 0, 3, neu), ["Blatt 2".to_string(), "2".into(), "0".into()]);
        assert_eq!(vorgaben(&p, 0, 4, neu)[0], "Klemmenplan");
        assert_eq!(vorgaben(&p, 0, 0, neu), ["Deckblatt".to_string(), "0".into(), "0".into()]);
        let mut iv = Sheet::new(0);
        iv.art = 1;
        iv.art_fest = true;
        iv.subindex = 1;
        p.anlagen[0].sheets.insert(0, iv);
        assert_eq!(vorgaben(&p, 0, 1, SheetRef { anlage: 0, sheet: 2 })[2], "2");
        assert!(auto_plus_frei(4) && !auto_plus_frei(3) && !auto_plus_frei(0));
    }

    #[test]
    fn umhaengen_in_andere_anlage() {
        let mut p = Project::new_default();
        let mut b = p.anlagen[0].clone();
        b.name = "B".into();
        p.anlagen.push(b);
        p.anlagen[0].sheets.push(Sheet::new(9));
        let r = umhaengen(&mut p, SheetRef { anlage: 0, sheet: 1 }, 1);
        assert_eq!(r, SheetRef { anlage: 1, sheet: 1 });
        assert_eq!(p.anlagen[1].sheets[1].number, 9);
        assert_eq!(p.anlagen[0].sheets.len(), 1);
    }
}
