//! Projektdatei per Drag & Drop ins Fenster ziehen (Nutzerwunsch 25.11.2026, Nachbau-Erweiterung): öffnet sie wie
//! Datei › Projekt öffnen. Ungespeicherte Änderungen am offenen Projekt fragen vorher wie gewohnt nach dem Speichern
//! (`guarded`); danach wird das bisherige Projekt geschlossen und das abgelegte geöffnet. Beim Darüberziehen zeigt
//! ein Hinweis, was beim Loslassen passiert.

use crate::app::{App, Pending};
use eframe::egui;
use std::path::{Path, PathBuf};

/// Endungen, die als Projekt geöffnet werden (wie im Öffnen-Dialog).
pub const ENDUNGEN: &[&str] = &["epr", "epj", "epv"];

/// Ist die Datei ein Projekt?
pub fn ist_projekt(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| ENDUNGEN.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Erstes Projekt unter den abgelegten Dateien; `Err` mit Hinweis, wenn nur andere Dateien dabei sind.
pub fn auswahl(pfade: &[PathBuf]) -> Option<Result<PathBuf, String>> {
    if pfade.is_empty() {
        return None;
    }
    Some(pfade.iter().find(|p| ist_projekt(p)).cloned().ok_or_else(|| format!("Nur Projekte ({}) lassen sich durch Ablegen öffnen.", ENDUNGEN.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", "))))
}

impl App {
    /// Abgelegte Dateien dieses Bildes verarbeiten und beim Darüberziehen einen Hinweis zeigen.
    pub fn dateien_ablegen(&mut self, ctx: &egui::Context) {
        let (abgelegt, schwebt): (Vec<PathBuf>, Vec<PathBuf>) = ctx.input(|i| {
            (i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect(), i.raw.hovered_files.iter().filter_map(|f| f.path.clone()).collect())
        });
        if !schwebt.is_empty() || ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let text = match auswahl(&schwebt) {
                Some(Ok(p)) => format!("Loslassen öffnet {}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                Some(Err(t)) => t,
                None => "Projekt hier ablegen".into(),
            };
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("ep_ablegen")));
            p.rect_filled(r, 0.0, egui::Color32::from_black_alpha(110));
            p.text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(22.0), egui::Color32::WHITE);
        }
        match auswahl(&abgelegt) {
            Some(Ok(p)) => self.guarded(Pending::OpenPath(p)),
            Some(Err(t)) => self.info = t,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nur_projekte() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(auswahl(&[]), None);
        assert_eq!(auswahl(&[p("/a/plan.dxf"), p("/a/W2.EPR")]), Some(Ok(p("/a/W2.EPR"))));
        assert!(matches!(auswahl(&[p("bild.png")]), Some(Err(_))));
        assert!(ist_projekt(&p("x.epj")) && !ist_projekt(&p("x")));
    }
}
