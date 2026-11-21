//! Sammelstelle für die Zustände weiterer Dialoge (Parameter, Projektgrafik …), damit `App` und die
//! Oberfläche nur je eine Stelle dafür brauchen.

use crate::app::App;
use eframe::egui;

#[derive(Default)]
pub struct Zusatz {
    /// Blattmenü › Parameter / Alternativtext › Benutzerdefiniert (parameter.rs)
    pub param: Option<crate::parameter::ParamDlg>,
    /// Eigenschaften einer Projektgrafik (TGrafik, grafik.rs)
    pub grafik: Option<crate::grafik::GrafikDlg>,
    /// Datei › Vorlage öffnen: diese Vorlage wird gerade geladen (vorlage.rs)
    pub template: Option<std::path::PathBuf>,
    /// Kontaktspiegel (TSSpiegel, kontaktspiegel.rs)
    pub spiegel: Option<crate::kontaktspiegel::SpiegelDlg>,
}

impl App {
    /// Alle Dialoge dieser Sammelstelle zeichnen.
    pub fn zusatz_ui(&mut self, ctx: &egui::Context) {
        self.param_dialog_ui(ctx);
        self.grafik_dialog_ui(ctx);
        self.template_loaded();
        self.spiegel_ui(ctx);
        // Kontaktspiegel folgen ihren Kontakten (Lage, neue/gelöschte Kontakte); die X-Lage bleibt wie im Original
        // stehen, wenn die Spule verschoben wird (kontaktspiegel.rs)
        if self.sym_edit.is_none() {
            if crate::kontaktspiegel::nachfuehren(&mut self.project, &self.lib) {
                self.dirty = true;
            }
        }
    }

    /// Nach jeder Änderung (checkpoint): das aktuelle Blatt gilt als heute geändert (Blattdaten „Geändert“,
    /// Grundlage für Drucken › „Änderung seit …“).
    pub fn mark_sheet_changed(&mut self) {
        if self.sym_edit.is_some() {
            return;
        }
        if let Some(r) = self.cur() {
            if let Some(s) = self.project.sheet_mut(r) {
                s.geaendert = crate::model::today();
            }
        }
    }
}
