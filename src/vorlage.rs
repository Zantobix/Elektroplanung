//! Vorlagen wie im Original: Datei › „Vorlage speichern“ / „Vorlage öffnen“.
//!
//! Das Original speichert Vorlagen im Projektformat unter der Endung `.epv` („Elektroplan Vorlage|*.epv“,
//! Routine 0x416cfc mit eigenem Dateifilter und Titel „Elektroplanung Vorlage“); „Vorlage öffnen“ nutzt
//! denselben Öffnen-Handler wie „Projekt öffnen“ mit dem Filter `*.epr|*.epv`.
//! Der Nachbau speichert die Vorlage als Kopie (das Projekt behält seinen Dateinamen) und öffnet eine Vorlage
//! als neues, unbenanntes Projekt, damit die Vorlage beim nächsten Speichern nicht überschrieben wird.

use crate::app::{App, Dialog};
use std::path::Path;

/// Projektdatei im Originalformat (.epr oder Vorlage .epv)?
pub fn is_epr_format(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("epr") || e.eq_ignore_ascii_case("epv")).unwrap_or(false)
}

impl App {
    /// Datei › Vorlage speichern: Kopie des Projekts als .epv.
    pub fn save_template(&mut self) {
        let stem = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Vorlage".into());
        let Some(p) = rfd::FileDialog::new().add_filter("Elektroplan Vorlage", &["epv"]).set_file_name(format!("{stem}.epv")).save_file() else { return };
        let p = if p.extension().is_none() { p.with_extension("epv") } else { p };
        let skipped = crate::epr_write::unsupported(&self.project);
        match crate::epr_write::save(&self.project, &p) {
            Ok(()) => {
                self.info = format!("Vorlage gespeichert: {}", p.display());
                if !skipped.is_empty() {
                    self.dialog = Some(Dialog::Message(format!("Nicht in der Vorlage gespeichert:\n{}", skipped.join("\n"))));
                }
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Vorlage konnte nicht gespeichert werden: {e}"))),
        }
    }

    /// Datei › Vorlage öffnen: Vorlage (.epv) oder Projekt wählen und als neues Projekt öffnen.
    pub fn open_template(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Elektroplan Vorlage", &["epv", "epr"]).pick_file() else { return };
        self.zusatz.template = Some(p.clone());
        self.guarded(crate::app::Pending::OpenPath(p));
    }

    /// Nach dem Laden einer Vorlage den Dateinamen vergessen (neues Projekt aus der Vorlage).
    pub fn template_loaded(&mut self) {
        let Some(t) = self.zusatz.template.clone() else { return };
        if self.path.as_ref() == Some(&t) {
            self.path = None;
            self.zusatz.template = None;
            self.info = format!("Neues Projekt aus Vorlage {}", t.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
        } else if self.ask_save.is_none() && self.dialog.is_none() {
            // Laden abgebrochen oder fehlgeschlagen
            self.zusatz.template = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vorlage_ist_originalformat() {
        assert!(is_epr_format(Path::new("a/b.EPV")));
        assert!(is_epr_format(Path::new("x.epr")));
        assert!(!is_epr_format(Path::new("x.epj")));
    }
}
