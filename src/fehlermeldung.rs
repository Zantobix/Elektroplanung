//! Hilfe › „Fehler melden…“ / „Wunsch äußern…“ (Nutzerwunsch 22.11.2026, bewusste Erweiterung): Fehler und
//! Wünsche sollen sich mit möglichst wenig Aufwand melden lassen.
//!
//! Ein kleines Fenster mit einem Textfeld; das Programm hängt auf Wunsch Version und Betriebssystem an (sichtbar,
//! abwählbar – nie Projektinhalt). „Auf GitHub melden“ öffnet im Browser ein fertig ausgefülltes Issue im
//! öffentlichen Repo; dort genügt „Submit“ (GitHub-Konto nötig). „Text kopieren“ legt die Meldung in die
//! Zwischenablage (z. B. für eine E-Mail). Auf Wunsch wird ein Bild des aktuellen Blattes gespeichert, das man in
//! das GitHub-Fenster ziehen kann (über einen Link lassen sich keine Bilder mitgeben).

use crate::app::App;
use eframe::egui;

/// Öffentliches Repo, in dem die Meldungen landen.
pub const REPO: &str = "https://github.com/Zantobix/Elektroplanung";

/// Längste Adresse, die Browser und GitHub sicher annehmen; längere Texte werden gekürzt.
const MAX_URL: usize = 7000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    Fehler,
    Wunsch,
}

impl Art {
    fn vorlage(self) -> &'static str {
        match self {
            Art::Fehler => "fehler.md",
            Art::Wunsch => "wunsch.md",
        }
    }
    fn praefix(self) -> &'static str {
        match self {
            Art::Fehler => "Fehler: ",
            Art::Wunsch => "Wunsch: ",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Meldung {
    pub art: Art,
    pub titel: String,
    pub text: String,
    pub system: bool,
    pub bild: bool,
    /// Rückmeldung im Fenster (z. B. wo das Bild liegt)
    pub hinweis: String,
}

impl Meldung {
    pub fn neu(art: Art) -> Self {
        Meldung { art, titel: String::new(), text: String::new(), system: true, bild: false, hinweis: String::new() }
    }
}

/// Angaben zum System (ohne Projektinhalt).
pub fn systemangaben() -> String {
    format!("Version {} · {} {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH)
}

/// Titel der Meldung (aus dem Titelfeld bzw. der ersten Zeile des Textes).
pub fn titel(m: &Meldung) -> String {
    let t = if m.titel.trim().is_empty() { m.text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim() } else { m.titel.trim() };
    let t: String = t.chars().take(80).collect();
    format!("{}{}", m.art.praefix(), t)
}

/// Text der Meldung (Beschreibung und, wenn gewählt, Systemangaben).
pub fn inhalt(m: &Meldung) -> String {
    let mut s = String::new();
    s.push_str(match m.art {
        Art::Fehler => "### Was ist passiert?\n\n",
        Art::Wunsch => "### Was wünschen Sie sich?\n\n",
    });
    s.push_str(m.text.trim());
    s.push('\n');
    if m.system {
        s.push_str("\n---\n");
        s.push_str(&systemangaben());
        s.push('\n');
    }
    s
}

/// Prozentkodierung für Adressparameter (alles außer A–Z a–z 0–9 - _ . ~).
pub fn kodieren(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Adresse eines fertig ausgefüllten neuen Issues; zu lange Texte werden gekürzt (Hinweis am Ende).
pub fn adresse(m: &Meldung) -> String {
    let kopf = format!("{REPO}/issues/new?template={}&title={}&body=", m.art.vorlage(), kodieren(&titel(m)));
    let mut body = inhalt(m);
    let mut url = format!("{kopf}{}", kodieren(&body));
    while url.len() > MAX_URL && !body.is_empty() {
        let n = body.chars().count();
        body = body.chars().take(n * 9 / 10).collect::<String>();
        url = format!("{kopf}{}", kodieren(&format!("{body}\n\n(gekürzt – vollständiger Text in der Zwischenablage)")));
    }
    url
}

impl App {
    /// Fenster „Fehler melden / Wunsch äußern“.
    pub fn meldung_ui(&mut self, ctx: &egui::Context) {
        let Some(mut m) = self.meldung.take() else { return };
        let mut offen = true;
        let mut schliessen = false;
        egui::Window::new(match m.art {
            Art::Fehler => "Fehler melden",
            Art::Wunsch => "Wunsch äußern",
        })
        .open(&mut offen)
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        // mittig öffnen (Nutzer 24.11.2026: „Fehler melden“ erschien am Rand); neue Kennung, damit eine früher gespeicherte Lage am Rand nicht mehr gilt
        .id(egui::Id::unique("fehlermeldung_mitte"))
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut m.art, Art::Fehler, "Fehler");
                ui.radio_value(&mut m.art, Art::Wunsch, "Wunsch / Idee");
            });
            ui.add_space(4.0);
            ui.label("Kurzer Titel (optional):");
            ui.add(egui::TextEdit::singleline(&mut m.titel).desired_width(440.0));
            ui.add_space(4.0);
            ui.label(match m.art {
                Art::Fehler => "Was ist passiert? Was hätten Sie erwartet?",
                Art::Wunsch => "Was wünschen Sie sich?",
            });
            let te = ui.add(egui::TextEdit::multiline(&mut m.text).desired_width(440.0).desired_rows(8));
            if m.text.is_empty() && m.titel.is_empty() && !te.has_focus() {
                te.request_focus();
            }
            ui.add_space(4.0);
            ui.checkbox(&mut m.system, format!("Systemangaben anhängen ({})", systemangaben()));
            ui.checkbox(&mut m.bild, "Bild des aktuellen Blattes speichern (zum Hineinziehen in GitHub)");
            if !m.hinweis.is_empty() {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(0, 90, 160), &m.hinweis);
            }
            ui.add_space(6.0);
            ui.small("Zum Absenden auf GitHub ist ein (kostenloses) GitHub-Konto nötig. Ohne Konto: „Text kopieren“ und per E-Mail schicken.");
            ui.add_space(6.0);
            let leer = m.text.trim().is_empty() && m.titel.trim().is_empty();
            ui.horizontal(|ui| {
                if ui.add_enabled(!leer, egui::Button::new("Auf GitHub melden")).clicked() {
                    // vollständiger Text auch in die Zwischenablage (falls die Adresse gekürzt wurde)
                    ctx.copy_text(format!("{}\n\n{}", titel(&m), inhalt(&m)));
                    let bild = if m.bild { self.meldung_bild() } else { None };
                    ctx.open_url(egui::OpenUrl::new_tab(adresse(&m)));
                    match bild {
                        Some(p) => {
                            m.hinweis = format!("Browser geöffnet. Bild gespeichert unter\n{}\n– bitte in das GitHub-Fenster ziehen.", p.display());
                        }
                        None => schliessen = true,
                    }
                }
                if ui.add_enabled(!leer, egui::Button::new("Text kopieren")).clicked() {
                    ctx.copy_text(format!("{}\n\n{}", titel(&m), inhalt(&m)));
                    m.hinweis = "Text liegt in der Zwischenablage.".into();
                    if m.bild {
                        if let Some(p) = self.meldung_bild() {
                            m.hinweis = format!("Text liegt in der Zwischenablage, Bild unter\n{}", p.display());
                        }
                    }
                }
                if ui.button("Schließen").clicked() {
                    schliessen = true;
                }
            });
        });
        if offen && !schliessen {
            self.meldung = Some(m);
        }
    }

    /// Bild des aktuellen Blattes in den Temp-Ordner speichern.
    fn meldung_bild(&self) -> Option<std::path::PathBuf> {
        let r = self.cur().filter(|_| self.sym_edit.is_none())?;
        let img = crate::bildexport::render_headless(&self.project, &self.lib, r, 150, true)?;
        let bytes = crate::bildexport::png_bytes(&img, 150).ok()?;
        let p = std::env::temp_dir().join("elektroplanung_meldung.png");
        std::fs::write(&p, bytes).ok()?;
        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_ausgefuellt_und_kodiert() {
        let mut m = Meldung::neu(Art::Fehler);
        m.text = "Leitung reißt ab\nbeim Verschieben & so".into();
        let u = adresse(&m);
        assert!(u.starts_with("https://github.com/Zantobix/Elektroplanung/issues/new?template=fehler.md&title=Fehler%3A%20Leitung%20rei%C3%9Ft%20ab&body="), "{u}");
        assert!(u.contains("%26%20so") && !u.contains(' ') && !u.contains('\n'));
        assert!(u.contains(&kodieren(env!("CARGO_PKG_VERSION"))));
        m.system = false;
        assert!(!adresse(&m).contains("Version"));
    }

    #[test]
    fn wunsch_mit_titel() {
        let mut m = Meldung::neu(Art::Wunsch);
        m.titel = "Dunkles Design".into();
        assert_eq!(titel(&m), "Wunsch: Dunkles Design");
        assert!(adresse(&m).contains("template=wunsch.md"));
    }

    #[test]
    fn lange_texte_gekuerzt() {
        let mut m = Meldung::neu(Art::Fehler);
        m.text = "ä".repeat(20000);
        let u = adresse(&m);
        assert!(u.len() <= MAX_URL + 200, "{}", u.len());
        assert!(u.contains(&kodieren("(gekürzt")));
    }
}
