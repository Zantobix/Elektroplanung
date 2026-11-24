//! Fehlerbericht-Taste (todo N5, Nutzer 23.11.2026): F12 bzw. Hilfe › Fehlerbericht speichern legt einen Ordner
//! mit allem an, was zum genauen Nachstellen eines Befunds nötig ist:
//!
//! - `projekt.epj` – vollständige Kopie des Projekts im aktuellen Zustand (auch ungespeicherte Änderungen),
//! - `protokoll.txt` – Programmversion, Blatt, Ansicht, Raster und die letzten Eingaben (Tasten, Drücken/Loslassen
//!   mit Lage in mm und laufender Geste, beim Ziehen die Mausbahn),
//! - `bild.png` – Bild des Programmfensters.
//!
//! Der Ordner liegt unter `fehlerberichte` in den Programmeinstellungen; nichts wird verschickt.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

/// Höchstzahl der gemerkten Eingaben.
pub const EINTRAEGE: usize = 300;

/// Protokoll der letzten Eingaben.
pub struct Protokoll {
    pub eintraege: VecDeque<String>,
    start: Instant,
    /// Zeit des letzten Bahnpunkts beim Ziehen (s seit Start)
    bahn: f64,
}

impl Default for Protokoll {
    fn default() -> Self {
        Self { eintraege: VecDeque::new(), start: Instant::now(), bahn: 0.0 }
    }
}

impl Protokoll {
    fn zeit(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn merken(&mut self, text: String) {
        let t = self.zeit();
        self.eintraege.push_back(format!("{t:9.2}s  {text}"));
        while self.eintraege.len() > EINTRAEGE {
            self.eintraege.pop_front();
        }
    }
}

/// Kurzname der laufenden Geste.
fn geste(d: &crate::app::Drag) -> &'static str {
    use crate::app::Drag;
    match d {
        Drag::None => "-",
        Drag::Move { .. } => "Verschieben",
        Drag::Teilstueck { .. } => "Teilstück",
        Drag::Wire { .. } => "Leitung",
        Drag::Rubber { .. } => "Rahmen",
        Drag::Create { .. } => "Zeichnen",
        Drag::Scale { .. } => "Skalieren",
        Drag::Pan { .. } => "Schieben",
        _ => "andere",
    }
}

impl crate::app::App {
    /// Je Bild: Eingaben dieses Bildes ins Protokoll (nach der Verarbeitung, mit der Lage in mm).
    pub fn protokollieren(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui::{Event, PointerButton};
        let m = self.mouse_mm;
        let g = geste(&self.drag);
        let ereignisse: Vec<String> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    Event::PointerButton { button, pressed, modifiers, .. } => {
                        let b = match button {
                            PointerButton::Primary => "links",
                            PointerButton::Secondary => "rechts",
                            PointerButton::Middle => "Mitte",
                            _ => "Taste",
                        };
                        let mods = if modifiers.shift { "+Umschalt" } else if modifiers.ctrl || modifiers.command { "+Strg" } else { "" };
                        Some(format!("Maus {b}{mods} {} bei {:.1}/{:.1} mm", if *pressed { "gedrückt" } else { "losgelassen" }, m.x, m.y))
                    }
                    Event::Key { key, pressed: true, modifiers, repeat: false, .. } => Some(format!("Taste {}", crate::kuerzel::text(*modifiers, *key))),
                    Event::MouseWheel { delta, .. } => Some(format!("Mausrad {:.0}", delta.y)),
                    _ => None,
                })
                .collect()
        });
        for e in ereignisse {
            self.protokoll.merken(format!("{e}  [{g}]"));
        }
        // Mausbahn beim Ziehen etwa alle 0,1 s
        if !matches!(self.drag, crate::app::Drag::None) {
            let t = self.protokoll.zeit();
            if t - self.protokoll.bahn >= 0.1 {
                self.protokoll.bahn = t;
                self.protokoll.merken(format!("  Bahn {:.1}/{:.1} mm  [{g}]", m.x, m.y));
            }
        }
    }

    /// Fehlerbericht anlegen; liefert den Ordner.
    pub fn fehlerbericht(&mut self, ctx: &eframe::egui::Context) -> Result<PathBuf, String> {
        let dir = crate::config_dir().join("fehlerberichte").join(chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&self.project).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("projekt.epj"), json).map_err(|e| e.to_string())?;
        let mut t = String::new();
        t.push_str(&format!("Elektroplanung {} ({})\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS));
        t.push_str(&format!("Datei: {}\n", self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(neu)".into())));
        if let (Some(r), Some(s)) = (self.cur(), self.sheet()) {
            t.push_str(&format!("Blatt: Anlage {} Blatt {} „{}“ (Nr. {})\n", r.anlage, r.sheet, s.name, s.number));
        }
        t.push_str(&format!(
            "Ansicht: Zoom {:.3} px/mm, Versatz {:.1}/{:.1}; Raster {} mm, Rasterfang {}, Strompfadausrichtung {}, Auto-Verbindung {}\n",
            self.view.zoom,
            self.view.offset.x,
            self.view.offset.y,
            self.raster(),
            if self.grid { "ein" } else { "aus" },
            if self.path_align { "ein" } else { "aus" },
            if self.auto_conn { "ein" } else { "aus" }
        ));
        t.push_str(&format!("Markiert: {:?}\nGeste: {}\n\nLetzte Eingaben (älteste zuerst):\n", self.selection, geste(&self.drag)));
        for e in &self.protokoll.eintraege {
            t.push_str(e);
            t.push('\n');
        }
        std::fs::write(dir.join("protokoll.txt"), t).map_err(|e| e.to_string())?;
        // Bild des ganzen Fensters (kommt ein Bild später)
        let bild = dir.join("bild.png");
        ctx.request_screenshot(move |img| {
            let mut buf = Vec::with_capacity(img.pixels.len() * 4);
            for c in &img.pixels {
                buf.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
            }
            let _ = image::save_buffer(&bild, &buf, img.size[0] as u32, img.size[1] as u32, image::ExtendedColorType::Rgba8);
        });
        Ok(dir)
    }

    /// Fehlerbericht anlegen und den Ordner melden.
    pub fn fehlerbericht_melden(&mut self, ctx: &eframe::egui::Context) {
        match self.fehlerbericht(ctx) {
            Ok(d) => {
                self.info = format!("Fehlerbericht gespeichert: {}", d.display());
                self.dialog = Some(crate::app::Dialog::Message(format!(
                    "Fehlerbericht gespeichert:\n{}\n\nEnthält eine Kopie des Projekts (projekt.epj), das Protokoll der letzten Eingaben (protokoll.txt) und ein Bild des Fensters (bild.png). Der Ordner kann z. B. an eine Fehlermeldung angehängt werden – er enthält Ihre Projektdaten.",
                    d.display()
                )));
            }
            Err(e) => self.info = format!("Fehlerbericht fehlgeschlagen: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protokoll_begrenzt() {
        let mut p = Protokoll::default();
        for i in 0..EINTRAEGE + 20 {
            p.merken(format!("e{i}"));
        }
        assert_eq!(p.eintraege.len(), EINTRAEGE);
        assert!(p.eintraege.front().unwrap().ends_with("e20"));
    }
}
