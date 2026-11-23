//! Fang beim Zeichnen und Platzieren (Runde 13, Nutzer 23.11.2026):
//!
//! - **Fluchten waagerecht und senkrecht:** Das Ende einer Leitung (und eine neue Klemmstelle abseits von Leitungen)
//!   fängt je Achse auf die nächste Flucht, wenn die Maus weniger als ein halbes Raster daneben liegt und die Flucht
//!   näher ist als der Rasterpunkt: Anfang bzw. letzter Knick der Leitung, Anschlüsse von Symbolen, Klemmstellen und
//!   Verweisen, Enden anderer Leitungen. So laufen Leitungen auch zu Anschlüssen abseits des Rasters gerade.
//! - **Fangkreuz:** Wie im Original zeigt ein kleines Kreuz, wo ein Klick bzw. das Loslassen tatsächlich setzt
//!   (Leitungsende, Klemmstelle – auf einer Leitung deren Lage auf der Leitung –, Zeichenobjekte).

use crate::app::{App, Drag, Tool};
use crate::canvas;
use crate::model::*;
use eframe::egui::{self, Color32, Pos2, Stroke};

/// Punkte, deren Fluchten fangen: Anschlüsse von Symbolen, Klemmstellen und Verweisen, Enden von Leitungen.
pub fn fluchtpunkte(sheet: &Sheet, lib: &[crate::symbols::SymDef]) -> Vec<P> {
    let mut v = vec![];
    for o in &sheet.objects {
        match &o.kind {
            Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Verweis { .. } | Kind::Wire { .. } => v.extend(canvas::obj_pins(o, lib)),
            _ => {}
        }
    }
    v
}

/// Rasterpunkt `q` der Maus `m` je Achse durch die nächste Flucht aus `punkte` ersetzen, wenn sie weniger als `halb`
/// von der Maus entfernt und näher als der Rasterwert liegt.
pub fn auf_fluchten(m: P, q: P, punkte: &[P], halb: f32) -> P {
    let beste = |mw: f32, qw: f32, werte: &mut dyn Iterator<Item = f32>| -> f32 {
        let mut best = qw;
        for w in werte {
            if (w - mw).abs() < halb && (w - mw).abs() < (best - mw).abs() - 1e-4 {
                best = w;
            }
        }
        best
    };
    P::new(beste(m.x, q.x, &mut punkte.iter().map(|p| p.x)), beste(m.y, q.y, &mut punkte.iter().map(|p| p.y)))
}

impl App {
    /// Rasterpunkt der Maus mit Fluchtfang (siehe Modulkopf); `bezug` = zusätzliche Fluchtpunkte (Anfang, Knicke).
    pub fn fang_mit_fluchten(&self, m: P, bezug: &[P]) -> P {
        let q = self.snap(m);
        if self.fang() <= 0.0 || self.no_snap {
            return q;
        }
        let Some(s) = self.sheet() else { return q };
        let mut pkt = fluchtpunkte(s, &self.lib);
        pkt.extend_from_slice(bezug);
        auf_fluchten(m, q, &pkt, self.raster() / 2.0)
    }

    /// Lage einer neuen Klemmstelle bei der Maus `m` (wie `klemmstelle_setzen`): auf einer Leitung deren Lage auf der
    /// Leitung, sonst der Rasterpunkt.
    pub fn klemmstelle_lage(&self, m: P, tol: f32) -> P {
        let treffer = self.sheet().and_then(|s| {
            s.objects.iter().rev().filter(|o| matches!(o.kind, Kind::Wire { .. }) && canvas::hit(o, m, tol, &self.lib)).find_map(|o| {
                let Kind::Wire { pts, .. } = &o.kind else { return None };
                let i = (0..pts.len().saturating_sub(1)).min_by(|x, y| seg_dist(m, pts[*x], pts[*x + 1]).total_cmp(&seg_dist(m, pts[*y], pts[*y + 1])))?;
                Some((pts[i], pts[i + 1]))
            })
        });
        match treffer {
            Some((a, b)) if self.sym_edit.is_none() => crate::klemme_leitung::lage(a, b, m, self.snap(m)),
            Some(_) => self.snap(m),
            None if self.sym_edit.is_none() => self.fang_mit_fluchten(m, &[]),
            None => self.snap(m),
        }
    }

    /// Wo ein Klick bzw. das Loslassen bei `m` (Zoom `z`) tatsächlich setzt – für das Fangkreuz; `None`, wenn nichts
    /// gesetzt wird.
    pub fn fangpunkt(&self, m: P, z: f32) -> Option<P> {
        if self.zoom_tool || self.ring.is_some() {
            return None;
        }
        match &self.drag {
            Drag::Wire { f } => Some(self.leitungsende(f, m, z)),
            Drag::Create { tool, .. } => Some(self.create_end(*tool, m, z)),
            Drag::None if self.placing.is_none() => match self.tool {
                Tool::Klemmstelle => Some(self.klemmstelle_lage(m, 6.0 / z)),
                Tool::Select | Tool::Image => None,
                t => Some(self.create_end(t, m, z)),
            },
            _ => None,
        }
    }
}

/// Kleines Fangkreuz („+“) bei `c` (Bildschirm).
pub fn kreuz(p: &egui::Painter, c: Pos2) {
    // wie im Original ein grünes „+“ am gefangenen Punkt
    let s = Stroke::new(1.0, Color32::from_rgb(0, 160, 0));
    let r = 4.0;
    p.line_segment([Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)], s);
    p.line_segment([Pos2::new(c.x, c.y - r), Pos2::new(c.x, c.y + r)], s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flucht_naeher_als_raster() {
        let pkt = [P::new(202.3, 40.0), P::new(10.0, 77.5)];
        // Maus bei 203: Raster 205 (Abstand 2), Flucht 202,3 (Abstand 0,7) → Flucht; y: Raster 80, Flucht 77,5 näher
        assert_eq!(auf_fluchten(P::new(203.0, 78.0), P::new(205.0, 80.0), &pkt, 2.5), P::new(202.3, 77.5));
        // Rasterpunkt näher an der Maus als die Flucht
        assert_eq!(auf_fluchten(P::new(204.6, 79.9), P::new(205.0, 80.0), &pkt, 2.5), P::new(205.0, 80.0));
        // Flucht weiter als ein halbes Raster: Raster
        assert_eq!(auf_fluchten(P::new(206.0, 90.0), P::new(205.0, 90.0), &pkt, 2.5), P::new(205.0, 90.0));
    }
}
