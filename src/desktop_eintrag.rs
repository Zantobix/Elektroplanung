//! Linux: Startmenü-Eintrag mit Programmsymbol für das AppImage (Nutzer 24.11.2026: „das heruntergeladene
//! Linux-Programm hat kein Icon“).
//!
//! Unter GNOME/Wayland zeigt die Oberfläche das Symbol eines Fensters nur, wenn eine `.desktop`-Datei zur
//! Programmkennung (`app_id` = `elektroplanung`, main.rs) passt – ein AppImage bringt keine mit. Läuft das Programm als
//! AppImage (Umgebungsvariable `APPIMAGE`) oder direkt gestartet ohne vorhandenen Eintrag, legt es deshalb beim Start
//! im Benutzerordner an bzw. aktualisiert:
//! - `~/.local/share/applications/elektroplanung.desktop` (Exec = Pfad des AppImage, StartupWMClass = Kennung),
//! - `~/.local/share/icons/hicolor/256x256/apps/elektroplanung.png`.
//! Nichts außerhalb des Benutzerordners; abschaltbar (Optionen › Programmverhalten). Geschrieben wird nur, wenn sich
//! etwas ändert (z. B. das AppImage wurde verschoben).

/// Programmkennung (Wayland app_id / X11 WM_CLASS) und Name der .desktop-Datei.
pub const KENNUNG: &str = "elektroplanung";

const SYMBOL: &[u8] = include_bytes!("../resources/elektroplanung.png");

/// Inhalt der .desktop-Datei für das AppImage unter `appimage`.
pub fn desktop_text(appimage: &str) -> String {
    let exec = appimage.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[Desktop Entry]\nType=Application\nName=Elektroplanung\nComment=Schaltpläne mit Blättern, Symbolen, Leitungen und Schriftfeld\nExec=\"{exec}\" %f\nIcon={KENNUNG}\nCategories=Graphics;Engineering;\nTerminal=false\nStartupWMClass={KENNUNG}\n"
    )
}

/// Was mit der .desktop-Datei geschehen soll: neuer Inhalt oder `None` (lassen). `vorhanden` = bisheriger Inhalt,
/// `appimage` = Pfad des laufenden AppImage, `exe` = Pfad der laufenden Programmdatei.
pub fn neuer_eintrag(vorhanden: Option<&str>, appimage: Option<&str>, exe: Option<&str>) -> Option<String> {
    if let Some(a) = appimage {
        let t = desktop_text(a);
        return (vorhanden != Some(t.as_str())).then_some(t);
    }
    // Programm direkt gestartet (Nutzer 24.11.2026: „wenn ich die Software manuell öffne, ist das Standard-Icon“):
    // ohne .desktop-Datei zeigt GNOME/Wayland das Ersatzsymbol. Fehlt sie, mit diesem Programm anlegen; eine
    // vorhandene (z. B. für das AppImage) bleibt, solange ihr Programm noch existiert. Nicht für Kopien in einem
    // temporären Ordner (Testinstanzen).
    let exe = exe?;
    let tmp = std::env::temp_dir();
    if std::path::Path::new(exe).starts_with(&tmp) || exe.starts_with("/tmp/") {
        return None;
    }
    let gueltig = vorhanden.is_some_and(|v| {
        v.lines().find_map(|l| l.strip_prefix("Exec=")).map(|e| e.trim_end_matches(" %f").trim_matches('"').to_string()).is_some_and(|e| std::path::Path::new(&e).exists())
    });
    if gueltig {
        return None;
    }
    Some(desktop_text(exe))
}

/// Eintrag und Symbol im Benutzerordner anlegen bzw. aktualisieren (Linux). Liefert eine Meldung, wenn etwas
/// geschrieben wurde.
pub fn einrichten() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let appimage = std::env::var("APPIMAGE").ok().filter(|p| !p.is_empty());
    let exe = std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned());
    let basis = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?;
    let desktop = basis.join("applications").join(format!("{KENNUNG}.desktop"));
    let symbol = basis.join("icons/hicolor/256x256/apps").join(format!("{KENNUNG}.png"));
    let mut geschrieben = false;
    let vorhanden = std::fs::read_to_string(&desktop).ok();
    if let Some(text) = neuer_eintrag(vorhanden.as_deref(), appimage.as_deref(), exe.as_deref()) {
        std::fs::create_dir_all(desktop.parent()?).ok()?;
        std::fs::write(&desktop, &text).ok()?;
        geschrieben = true;
    }
    // das Symbol immer aktuell halten (auch bei vorhandenem Eintrag), sobald es einen Eintrag gibt
    if desktop.exists() && std::fs::read(&symbol).ok().as_deref() != Some(SYMBOL) {
        std::fs::create_dir_all(symbol.parent()?).ok()?;
        std::fs::write(&symbol, SYMBOL).ok()?;
        geschrieben = true;
    }
    geschrieben.then(|| "Startmenü-Eintrag mit Programmsymbol angelegt (~/.local/share/applications)".to_string())
}

/// Programmsymbol für das Fenster (X11, Windows) aus der mitgelieferten PNG-Datei.
pub fn fenster_symbol() -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory_with_format(SYMBOL, image::ImageFormat::Png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_datei_passt_zur_kennung() {
        let t = desktop_text("/home/x/Downloads/Elektroplanung-0.5.1-x86_64.AppImage");
        assert!(t.contains("Exec=\"/home/x/Downloads/Elektroplanung-0.5.1-x86_64.AppImage\" %f"));
        assert!(t.contains("Icon=elektroplanung\n"));
        assert!(t.contains("StartupWMClass=elektroplanung\n"));
    }

    #[test]
    fn eintrag_beim_direkten_start() {
        let exe = std::env::current_exe().unwrap().to_string_lossy().into_owned();
        // AppImage: immer dessen Pfad
        assert!(neuer_eintrag(None, Some("/x/E.AppImage"), Some(&exe)).unwrap().contains("Exec=\"/x/E.AppImage\""));
        assert!(neuer_eintrag(Some(&desktop_text("/x/E.AppImage")), Some("/x/E.AppImage"), None).is_none());
        // direkt gestartet, kein Eintrag: anlegen (außer aus einem temporären Ordner)
        assert!(neuer_eintrag(None, None, Some("/opt/ep/Elektroplanung")).unwrap().contains("Exec=\"/opt/ep/Elektroplanung\""));
        assert!(neuer_eintrag(None, None, Some("/tmp/ep_test/nachbau/Elektroplanung")).is_none());
        // vorhandener Eintrag mit existierendem Programm bleibt, mit verschwundenem wird ersetzt
        assert!(neuer_eintrag(Some(&desktop_text(&exe)), None, Some("/opt/ep/Elektroplanung")).is_none());
        assert!(neuer_eintrag(Some(&desktop_text("/gibt/es/nicht")), None, Some("/opt/ep/Elektroplanung")).is_some());
    }

    #[test]
    fn symbol_ist_lesbar() {
        let (px, w, h) = fenster_symbol().expect("PNG");
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
    }
}
