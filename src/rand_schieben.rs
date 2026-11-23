//! Ansicht beim Ziehen am Fensterrand mitschieben (Nutzerwunsch 24.11.2026: „wenn ein Objekt oder eine Leitung
//! außerhalb des Fensterrands gezogen wird, soll der Sichtbereich entsprechend mit verschoben werden“).
//!
//! Solange eine Ziehgeste läuft (Objekt verschieben, Leitung ziehen, Teilstück, Rahmen, Rechts-Ziehen …) und die Maus
//! näher als `RAND` Punkte am Rand der Zeichenfläche oder schon außerhalb liegt, wandert die Ansicht in jedem Bild in
//! diese Richtung – umso schneller, je weiter die Maus über den Rand hinaus ist. Die gezogenen Objekte folgen von
//! selbst, weil die Geste die Mauslage in jedem Bild neu in mm umrechnet.

use eframe::egui::{Pos2, Rect, Vec2};

/// Randzone innerhalb der Zeichenfläche (Punkte), ab der mitgeschoben wird
pub const RAND: f32 = 24.0;
/// Verschiebung je Bild am Beginn der Randzone bzw. höchstens (Punkte)
pub const MIN_SCHRITT: f32 = 2.0;
pub const MAX_SCHRITT: f32 = 30.0;

/// Änderung des Ansichtsversatzes für die Maus bei `m` über der Zeichenfläche `area` (Null außerhalb der Randzone).
/// Liegt die Maus rechts am Rand, wandert der Inhalt nach links (Versatz wird kleiner), usw.
pub fn schritt(area: Rect, m: Pos2) -> Vec2 {
    let achse = |m: f32, lo: f32, hi: f32| -> f32 {
        let tiefe = if m < lo + RAND {
            lo + RAND - m
        } else if m > hi - RAND {
            -(m - (hi - RAND))
        } else {
            return 0.0;
        };
        let betrag = (MIN_SCHRITT + tiefe.abs() / 2.0).min(MAX_SCHRITT);
        betrag * tiefe.signum()
    };
    Vec2::new(achse(m.x, area.min.x, area.max.x), achse(m.y, area.min.y, area.max.y))
}

/// Überstand der Ansicht über den Blattrand hinaus (Punkte), bis zu dem mitgeschoben wird
pub const UEBERSTAND: f32 = 20.0;

/// Neuer Ansichtsversatz nach dem Schritt `v`: nur so weit, dass das Blatt (Größe `blatt` in Punkten) höchstens
/// `UEBERSTAND` vom Rand der Zeichenfläche (Größe `flaeche`) abrückt; lag die Ansicht schon weiter draußen, bleibt sie
/// dort (kein Zurückspringen).
pub fn begrenzt(offset: Vec2, v: Vec2, flaeche: Vec2, blatt: Vec2) -> Vec2 {
    let achse = |o: f32, v: f32, f: f32, b: f32| -> f32 {
        let (lo, hi) = ((f - b - UEBERSTAND).min(UEBERSTAND), UEBERSTAND.max(f - b - UEBERSTAND));
        (o + v).clamp(lo.min(o), hi.max(o))
    };
    Vec2::new(achse(offset.x, v.x, flaeche.x, blatt.x), achse(offset.y, v.y, flaeche.y, blatt.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mitte_ruhig_rand_schiebt() {
        let a = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        assert_eq!(schritt(a, Pos2::new(400.0, 300.0)), Vec2::ZERO);
        // rechts hinaus: Inhalt nach links, weiter draußen schneller, gedeckelt
        let r1 = schritt(a, Pos2::new(790.0, 300.0));
        let r2 = schritt(a, Pos2::new(850.0, 300.0));
        assert!(r1.x < 0.0 && r2.x < r1.x && r1.y == 0.0);
        assert!(schritt(a, Pos2::new(5000.0, 300.0)).x >= -MAX_SCHRITT);
        // links oben: Inhalt nach rechts unten
        let lo = schritt(a, Pos2::new(-10.0, 5.0));
        assert!(lo.x > 0.0 && lo.y > 0.0);
    }

    #[test]
    fn nicht_ueber_das_blatt_hinaus() {
        let (f, b) = (Vec2::new(800.0, 600.0), Vec2::new(2000.0, 1400.0));
        // nach links schieben (rechten Teil zeigen): höchstens bis der rechte Blattrand 20 px vor dem Rand steht
        let mut o = Vec2::new(0.0, 0.0);
        for _ in 0..1000 {
            o = begrenzt(o, Vec2::new(-30.0, 0.0), f, b);
        }
        assert_eq!(o.x, 800.0 - 2000.0 - UEBERSTAND);
        // nach rechts: höchstens 20 px Abstand links
        for _ in 0..1000 {
            o = begrenzt(o, Vec2::new(30.0, 0.0), f, b);
        }
        assert_eq!(o.x, UEBERSTAND);
    }
}
