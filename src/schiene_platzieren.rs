//! Sammelschiene auf das Blatt bringen (Maus-Befund M6, 22.11.2026).
//!
//! Original (Gegenprobe unter Wine): Hinzufügen › Sammelschiene legt die Schiene nur im Projektbaum an; zieht man
//! sie aus dem Baum auf das Blatt, liegt sie oben unter den vorhandenen Schienen (gestapelt) über die Blattbreite.
//! Im Nachbau ging das Ziehen aus dem Baum bisher gar nicht. Lage wie in W2: Anfang 8 mm rechts vom Rahmen, Ende
//! 21 mm vor dem rechten Rahmen, Name 4 mm darüber, 5 mm Abstand zwischen den Schienen.

use crate::app::App;
use crate::model::*;

/// Y-Lage der nächsten Schiene: ab 12 mm unter dem oberen Rahmen die erste freie Lage im 5-mm-Abstand.
pub fn naechste_lage(oben: f32, vorhanden: &[f32]) -> f32 {
    let mut y = oben + 12.0;
    while vorhanden.iter().any(|v| (v - y).abs() < 2.5) {
        y += 5.0;
    }
    y
}

impl App {
    /// Schiene `name` auf das aktuelle Blatt legen (ohne Rückgängig-Schritt) und markieren.
    pub fn schiene_auf_blatt_nocp(&mut self, name: String) -> Option<u64> {
        let (fx0, fy0, fx1, _) = self.sheet().map(|s| s.frame())?;
        let vorhanden: Vec<f32> = self.sheet().map(|s| s.objects.iter().filter_map(|o| if let Kind::Busbar { a, .. } = &o.kind { Some(a.y) } else { None }).collect()).unwrap_or_default();
        let y = naechste_lage(fy0, &vorhanden);
        let id = self.project.alloc_id();
        let kind = Kind::Busbar { a: P::new(fx0 + 8.0, y), x2: fx1 - 21.0, name, label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 };
        self.with_sheet(|s, _| s.objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }));
        self.selection = vec![id];
        Some(id)
    }

    /// Aus dem Baum gezogene Schiene (Anlage `a`, Index `i`) auf dem aktuellen Blatt ablegen; nur innerhalb ihrer
    /// Anlage (Sammelschienen gehören zu einer Anlage).
    pub fn schiene_ablegen(&mut self, a: usize, i: usize) {
        if self.cur().map(|r| r.anlage) != Some(a) {
            self.info = "Sammelschiene gehört zu einer anderen Anlage".into();
            return;
        }
        let Some(name) = self.project.anlagen.get(a).and_then(|x| x.sammelschienen.get(i)).cloned() else { return };
        self.checkpoint();
        self.schiene_auf_blatt_nocp(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stapel_wie_w2() {
        // W2: L1 22, L2 27, L3 32, N 37 bei Rahmen oben 10
        assert_eq!(naechste_lage(10.0, &[]), 22.0);
        assert_eq!(naechste_lage(10.0, &[22.0, 27.0]), 32.0);
        assert_eq!(naechste_lage(10.0, &[27.0]), 22.0);
    }
}
