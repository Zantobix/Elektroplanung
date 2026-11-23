//! Prüfung auf eine neuere Version (bewusste Erweiterung, Nutzerwunsch 23.11.2026).
//!
//! Fragt bei GitHub das neueste Release des Projekts ab (`/releases/latest`, Entwürfe und Vorabversionen zählen dort
//! nicht) und vergleicht dessen Tag `v<Version>` mit der eigenen Version. Beim Programmstart einmal im Hintergrund,
//! wenn die Option „Beim Start nach neuer Version suchen“ eingeschaltet ist (Standard), und jederzeit über
//! Hilfe › Nach neuer Version suchen. Es wird nichts heruntergeladen oder installiert: Bei einer neueren Version
//! erscheint ein Hinweis mit Link zur Release-Seite.
//!
//! Abgerufen wird mit dem `curl` des Systems (Windows 10/11, Linux und macOS bringen es mit), damit das Programm
//! keinen eigenen HTTP- und TLS-Stapel braucht. Gesendet wird nur die Anfrage selbst, keine Daten des Nutzers.

use std::sync::mpsc;

/// Gefundenes Release.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version ohne führendes „v“, z. B. „0.3.1“
    pub version: String,
    /// Seite des Releases auf GitHub
    pub url: String,
}

/// Stand der Prüfung.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stand {
    #[default]
    Keine,
    Laeuft,
    Aktuell(Release),
    Neu(Release),
    Fehler(String),
}

/// Prüfung im Hintergrund samt Anzeigezustand.
#[derive(Default)]
pub struct Pruefung {
    rx: Option<mpsc::Receiver<Result<Release, String>>>,
    pub stand: Stand,
    /// Hilfe › Nach neuer Version suchen: Ergebnis auch zeigen, wenn nichts Neues da ist oder die Abfrage scheitert
    pub manuell: bool,
    /// Hinweisfenster sichtbar
    pub fenster: bool,
}

/// API-Adresse des neuesten Releases aus der Repository-Adresse (`https://github.com/<besitzer>/<name>`).
pub fn api_url(repo: &str) -> Option<String> {
    let rest = repo.trim_end_matches('/').trim_end_matches(".git").strip_prefix("https://github.com/")?;
    let mut teile = rest.split('/');
    let (besitzer, name) = (teile.next()?, teile.next()?);
    if besitzer.is_empty() || name.is_empty() || teile.next().is_some() {
        return None;
    }
    Some(format!("https://api.github.com/repos/{besitzer}/{name}/releases/latest"))
}

/// Zahlenteile einer Version („v0.3.10-beta“ → [0, 3, 10]); Zusätze nach „-“ bzw. „+“ fallen weg.
pub fn version_teile(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let kern = v.split(['-', '+']).next().unwrap_or("");
    kern.split('.').map(|t| t.parse::<u64>().unwrap_or(0)).collect()
}

/// Ist `kandidat` neuer als `aktuell`? Fehlende Teile zählen als 0 („0.3“ = „0.3.0“).
pub fn ist_neuer(kandidat: &str, aktuell: &str) -> bool {
    let (mut a, mut b) = (version_teile(kandidat), version_teile(aktuell));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

/// Antwort von `/releases/latest` auswerten.
pub fn auswerten(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Antwort nicht lesbar: {e}"))?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).ok_or_else(|| "Antwort ohne Versionsangabe".to_string())?;
    let url = v.get("html_url").and_then(|t| t.as_str()).unwrap_or(crate::REPO_URL).to_string();
    Ok(Release { version: tag.trim_start_matches(['v', 'V']).to_string(), url })
}

/// Ergebnis einer Abfrage in einen Stand übersetzen.
pub fn stand_aus(erg: Result<Release, String>, aktuell: &str) -> Stand {
    match erg {
        Ok(r) if ist_neuer(&r.version, aktuell) => Stand::Neu(r),
        Ok(r) => Stand::Aktuell(r),
        Err(e) => Stand::Fehler(e),
    }
}

fn abrufen(url: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "15", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: Elektroplanung-Update-Check", url]);
    #[cfg(windows)]
    {
        // kein Konsolenfenster aufblitzen lassen
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("curl nicht gefunden ({e})"))?;
    if !out.status.success() {
        let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if text.is_empty() { "keine Verbindung zu GitHub".into() } else { text });
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

impl Pruefung {
    /// Abfrage im Hintergrund starten (eine laufende wird nicht doppelt gestartet).
    pub fn starten(&mut self, ctx: &eframe::egui::Context, manuell: bool) {
        self.manuell |= manuell;
        if manuell {
            self.fenster = true;
        }
        if self.rx.is_some() {
            return;
        }
        let Some(url) = api_url(crate::REPO_URL) else {
            self.stand = Stand::Fehler("keine GitHub-Adresse in Cargo.toml".into());
            return;
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let erg = abrufen(&url).and_then(|j| auswerten(&j));
            let _ = tx.send(erg);
            ctx.request_repaint();
        });
        self.rx = Some(rx);
        self.stand = Stand::Laeuft;
    }

    /// Ergebnis abholen, wenn da; öffnet den Hinweis bei einer neuen Version (bzw. immer bei manueller Prüfung).
    pub fn abholen(&mut self) {
        let Some(rx) = &self.rx else { return };
        let Ok(erg) = rx.try_recv() else { return };
        self.rx = None;
        self.stand = stand_aus(erg, env!("CARGO_PKG_VERSION"));
        if matches!(self.stand, Stand::Neu(_)) || self.manuell {
            self.fenster = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_der_api() {
        assert_eq!(api_url("https://github.com/Zantobix/Elektroplanung").as_deref(), Some("https://api.github.com/repos/Zantobix/Elektroplanung/releases/latest"));
        assert_eq!(api_url("https://github.com/a/b.git/").as_deref(), Some("https://api.github.com/repos/a/b/releases/latest"));
        assert_eq!(api_url("https://gitlab.com/a/b"), None);
        assert_eq!(api_url("https://github.com/a"), None);
    }

    #[test]
    fn versionen_vergleichen() {
        assert!(ist_neuer("v0.3.1", "0.3.0"));
        assert!(ist_neuer("0.10.0", "0.9.9"));
        assert!(ist_neuer("1.0", "0.99.99"));
        assert!(!ist_neuer("v0.3.0", "0.3.0"));
        assert!(!ist_neuer("0.3", "0.3.0"));
        assert!(!ist_neuer("0.2.9", "0.3.0"));
        assert!(!ist_neuer("v0.3.0-beta", "0.3.0"));
    }

    #[test]
    fn antwort_auswerten() {
        let j = r#"{"tag_name":"v0.4.0","html_url":"https://github.com/Zantobix/Elektroplanung/releases/tag/v0.4.0","draft":false}"#;
        let r = auswerten(j).unwrap();
        assert_eq!(r, Release { version: "0.4.0".into(), url: "https://github.com/Zantobix/Elektroplanung/releases/tag/v0.4.0".into() });
        assert!(matches!(stand_aus(Ok(r.clone()), "0.3.0"), Stand::Neu(_)));
        assert!(matches!(stand_aus(Ok(r), "0.4.0"), Stand::Aktuell(_)));
        assert!(auswerten(r#"{"message":"Not Found"}"#).is_err());
        assert!(auswerten("kein json").is_err());
    }
}

impl crate::app::App {
    /// Ergebnis der Prüfung abholen und den Hinweis zeigen (bei neuer Version bzw. nach Hilfe › Nach neuer Version
    /// suchen auch „aktuell“ oder den Fehler).
    pub fn update_hinweis(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui;
        self.update.abholen();
        if !self.update.fenster {
            return;
        }
        let aktuell = env!("CARGO_PKG_VERSION");
        let mut offen = true;
        let mut schliessen = false;
        egui::Window::new("Programmversion").collapsible(false).resizable(false).open(&mut offen).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
            match &self.update.stand {
                Stand::Neu(r) => {
                    ui.label(egui::RichText::new(format!("Elektroplanung {} ist erschienen.", r.version)).strong());
                    ui.label(format!("Installiert ist Version {aktuell}."));
                    ui.hyperlink_to("Release-Seite mit Download und Änderungen öffnen", &r.url);
                }
                Stand::Aktuell(r) => {
                    ui.label(format!("Version {aktuell} ist aktuell (neuestes Release: {}).", r.version));
                }
                Stand::Laeuft => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Suche nach neuer Version …");
                    });
                }
                Stand::Fehler(e) => {
                    ui.label("Die Suche nach einer neuen Version ist fehlgeschlagen:");
                    ui.label(e.as_str());
                }
                Stand::Keine => {}
            }
            ui.add_space(6.0);
            ui.checkbox(&mut self.opt.update_pruefen, "Beim Start nach neuer Version suchen");
            ui.add_space(4.0);
            if ui.button("    OK    ").clicked() {
                schliessen = true;
            }
        });
        if !offen || schliessen {
            self.update.fenster = false;
            self.update.manuell = false;
        }
    }
}
