//! Linux: Startmenü-Eintrag mit Programmsymbol für das AppImage (Nutzer 24.11.2026: „das heruntergeladene
//! Linux-Programm hat kein Icon“).
//!
//! Unter GNOME/Wayland zeigt die Oberfläche das Symbol eines Fensters nur, wenn eine `.desktop`-Datei zur
//! Programmkennung (`app_id` = `elektroplanung`, main.rs) passt – ein AppImage bringt keine mit. Läuft das Programm als
//! AppImage (Umgebungsvariable `APPIMAGE`), legt es deshalb beim Start im Benutzerordner an bzw. aktualisiert:
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

/// Eintrag anlegen bzw. aktualisieren, wenn das Programm als AppImage läuft. Liefert eine Meldung, wenn etwas
/// geschrieben wurde.
pub fn einrichten() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let appimage = std::env::var("APPIMAGE").ok().filter(|p| !p.is_empty())?;
    let basis = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?;
    let desktop = basis.join("applications").join(format!("{KENNUNG}.desktop"));
    let symbol = basis.join("icons/hicolor/256x256/apps").join(format!("{KENNUNG}.png"));
    let text = desktop_text(&appimage);
    let mut geschrieben = false;
    if std::fs::read_to_string(&desktop).ok().as_deref() != Some(text.as_str()) {
        std::fs::create_dir_all(desktop.parent()?).ok()?;
        std::fs::write(&desktop, &text).ok()?;
        geschrieben = true;
    }
    if std::fs::read(&symbol).ok().as_deref() != Some(SYMBOL) {
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
    fn symbol_ist_lesbar() {
        let (px, w, h) = fenster_symbol().expect("PNG");
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
    }
}
