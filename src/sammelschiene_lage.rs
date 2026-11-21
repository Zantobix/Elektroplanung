//! Lage der Sammelschienen auf dem Blatt und Kontextmenü „Sammelschiene › Nach Oben/Nach Unten“.
//!
//! Belegt aus dem Original:
//! - Zeichenroutine 0x438b94: vor dem Zeichnen setzt das Original die Y-Lage (OBJ Feld 7) jeder Sammelschiene ohne
//!   „feste Lage“ (Feld 15 Byte 0 Bit 1) neu. Die Schienen werden in der Reihenfolge des Blattes gestapelt: ohne
//!   Bit 0 von oben ab Rahmen oben + Strompfadleiste + 7 mm, mit Bit 0 („Unten“) von unten ab Rahmen unten −
//!   Schriftfeld − 7 mm. Abstand = Option „Sammelschienen › Abstand in mm“ (W2: 5 mm). Der Y-Versatz (Feld 11
//!   Wort 0, 0,1 mm) verschiebt die Schiene und alle folgenden derselben Richtung:
//!   oben `y = n + v; n += abstand + v`, unten `y = u + v; u -= abstand − v`.
//!   Mit W2.epr nachgerechnet (alle Blätter, z. B. L2 22 + 2,5 = 24,5, PE unten 183 + 2,5 = 185,5).
//! - Kontextmenü (PopupMenu1Popup 0x4190be): Beschriftung „Nach “ + („Oben“ bei Bit 0, sonst „Unten“).
//! - NachUnten1Click 0x433f30 (nur bei Vormarkierung Sammelschiene): Bit 0 umschalten (rückgängig machbar,
//!   0x4a7c20) und den Versatz auf 0 setzen (0x4a7138), danach neu zeichnen – die Schiene springt an das andere
//!   Ende des Stapels. Der unsichtbare Eintrag „Ausblenden“ hat denselben Handler, wirkt aber nicht auf die Lage.

use crate::app::App;
use crate::model::*;

/// Bit 0 von `lage`: von unten gestapelt
pub const UNTEN: u8 = 1;
/// Bit 1 von `lage`: feste Lage (kein „Auto“)
pub const FEST: u8 = 2;
/// Abstand vom Stapelanfang (0x46 = 7 mm)
const RAND: f32 = 7.0;

/// Beschriftung des Eintrags im Untermenü Sammelschiene.
pub fn beschriftung(lage: u8) -> &'static str {
    if lage & UNTEN != 0 {
        "Nach Oben"
    } else {
        "Nach Unten"
    }
}

/// Anfang des oberen bzw. unteren Stapels: (oben, unten) in mm.
pub fn stapel_anfang(s: &Sheet, pfad_hoehe: f32, schriftfeld: f32) -> (f32, f32) {
    let (_, fy0, _, fy1) = s.frame();
    (fy0 + pfad_hoehe + RAND, fy1 - schriftfeld - RAND)
}

/// Y-Lagen aller Sammelschienen ohne feste Lage wie in der Zeichenroutine des Originals: (Objekt-Id, Y).
pub fn stapeln(s: &Sheet, abstand: f32, oben: f32, unten: f32) -> Vec<(u64, f32)> {
    let (mut n, mut u) = (oben, unten);
    let mut v = vec![];
    for o in &s.objects {
        let Kind::Busbar { lage, versatz, .. } = &o.kind else { continue };
        if lage & FEST != 0 {
            continue;
        }
        if lage & UNTEN == 0 {
            v.push((o.id, n + versatz));
            n += abstand + versatz;
        } else {
            v.push((o.id, u + versatz));
            u -= abstand - versatz;
        }
    }
    v
}

/// Y-Lagen auf das Blatt übertragen; Leitungsenden auf einer verschobenen Schiene gehen mit (senkrecht
/// angeschlossene Leitungen behalten ihre Richtung). Rückgabe: ob sich etwas bewegt hat.
pub fn anwenden(s: &mut Sheet, lagen: &[(u64, f32)]) -> bool {
    let mut bewegt = false;
    for (id, y) in lagen {
        let Some(i) = s.objects.iter().position(|o| o.id == *id) else { continue };
        let Kind::Busbar { a, x2, .. } = s.objects[i].kind.clone() else { continue };
        let dy = y - a.y;
        if dy.abs() < 1e-4 {
            continue;
        }
        bewegt = true;
        let auf = |p: P| (p.y - a.y).abs() < 0.05 && p.x >= a.x - 0.05 && p.x <= x2 + 0.05;
        for o in s.objects.iter_mut() {
            if let Kind::Wire { pts, .. } = &mut o.kind {
                let ends = [pts.first().is_some_and(|p| auf(*p)), pts.last().is_some_and(|p| auf(*p))];
                if ends[0] || ends[1] {
                    *pts = crate::mitziehen::follow_wire(pts, ends, P::new(0.0, dy), &|p| p);
                }
            }
        }
        if let Kind::Busbar { a, .. } = &mut s.objects[i].kind {
            a.y = *y;
        }
    }
    bewegt
}

/// Untermenü-Eintrag umschalten: Bit 0 kippen, Versatz auf 0.
pub fn umschalten(k: &mut Kind) {
    if let Kind::Busbar { lage, versatz, .. } = k {
        *lage ^= UNTEN;
        *versatz = 0.0;
    }
}

impl App {
    /// Kontextmenü „Sammelschiene › Nach Oben/Nach Unten“ auf der Sammelschiene `id` (NachUnten1Click).
    pub fn sammelschiene_umschalten(&mut self, id: u64) {
        if !self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.id == id && matches!(o.kind, Kind::Busbar { .. }))) {
            return;
        }
        self.checkpoint();
        let abstand = self.opt.sms_abstand;
        let schriftfeld = self.project.title_h();
        let pfad = self.cur().and_then(|r| self.project.anlagen.get(r.anlage)).map(|a| a.set.pfad_hoehe as f32 / 10.0).unwrap_or(COLUMN_BAR_H);
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                umschalten(&mut o.kind);
            }
            let (oben, unten) = stapel_anfang(s, pfad, schriftfeld);
            let lagen = stapeln(s, abstand, oben, unten);
            anwenden(s, &lagen);
        });
        crate::epr::update_busbar_xrefs(&mut self.project);
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schiene(id: u64, y: f32, lage: u8, versatz: f32) -> Obj {
        Obj {
            id,
            kind: Kind::Busbar { a: P::new(20.0, y), x2: 280.0, name: format!("S{id}"), label: P::default(), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage, versatz },
            pen: 0,
            fixed: false,
            group: None,
            raw: None,
        }
    }

    #[test]
    fn stapel_wie_im_original() {
        let mut s = Sheet::new(1);
        // Blatt „Sicherungen -F1 und -F2“ aus W2: L2 (+2,5), L3, PE unten (+2,5), N, -F1.L1 (+22,5)
        s.objects = vec![schiene(1, 0.0, 0, 2.5), schiene(2, 0.0, 0, 0.0), schiene(3, 0.0, UNTEN, 2.5), schiene(4, 0.0, 0, 0.0), schiene(5, 0.0, 0, 22.5), schiene(6, 50.0, FEST, 0.0)];
        let (oben, unten) = stapel_anfang(&s, 5.0, 15.0);
        assert_eq!((oben, unten), (22.0, 183.0));
        let l = stapeln(&s, 5.0, oben, unten);
        assert_eq!(l, vec![(1, 24.5), (2, 29.5), (3, 185.5), (4, 34.5), (5, 62.0)]);
    }

    #[test]
    fn nach_unten_und_zurueck() {
        let mut s = Sheet::new(1);
        s.objects = vec![schiene(1, 22.0, 0, 0.0), schiene(2, 29.5, 0, 2.5)];
        // Leitung senkrecht von der zweiten Schiene nach unten
        s.objects.push(Obj { id: 9, kind: Kind::Wire { pts: vec![P::new(100.0, 29.5), P::new(100.0, 80.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        assert_eq!(beschriftung(0), "Nach Unten");
        umschalten(&mut s.objects[1].kind);
        let Kind::Busbar { lage, versatz, .. } = s.objects[1].kind else { unreachable!() };
        assert_eq!((lage, versatz), (UNTEN, 0.0));
        assert_eq!(beschriftung(lage), "Nach Oben");
        let (oben, unten) = stapel_anfang(&s, 5.0, 15.0);
        let l = stapeln(&s, 5.0, oben, unten);
        assert!(anwenden(&mut s, &l));
        assert!(matches!(s.objects[1].kind, Kind::Busbar { a, .. } if a.y == 183.0));
        // das Leitungsende folgt der Schiene, das andere bleibt
        let Kind::Wire { pts, .. } = &s.objects[2].kind else { unreachable!() };
        assert_eq!((pts.first().unwrap().y, pts.last().unwrap().y), (183.0, 80.0));
        umschalten(&mut s.objects[1].kind);
        let l = stapeln(&s, 5.0, oben, unten);
        assert!(anwenden(&mut s, &l));
        assert!(matches!(s.objects[1].kind, Kind::Busbar { a, .. } if a.y == 27.0));
    }
}
