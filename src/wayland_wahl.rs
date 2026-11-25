//! Linux unter Wayland: Drag & Drop von Dateien (datei_ablegen.rs) braucht derzeit X11 – winit 0.30 meldet unter
//! Wayland keine abgelegten Dateien (Nutzer 25.11.2026: „das Drag-and-Drop geht gerade unter Linux nicht“). Daher
//! startet das Programm unter Wayland standardmäßig über XWayland, wenn es verfügbar ist (`DISPLAY` gesetzt).
//! Optionen › Programmverhalten schaltet auf echtes Wayland um (dann ohne Datei-Drag-and-Drop); die Wahl liegt als
//! kleine Datei im Einstellungsordner, weil sie vor dem Öffnen des Fensters gelesen werden muss, und wirkt nach dem
//! nächsten Start.

use std::path::PathBuf;

/// Merkdatei „echtes Wayland“ (vorhanden = Wayland statt XWayland).
pub fn datei() -> Option<PathBuf> {
    let basis = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(basis.join("elektroplanung").join("wayland_nativ"))
}

/// Echtes Wayland gewählt?
pub fn nativ() -> bool {
    datei().is_some_and(|d| d.exists())
}

/// Wahl speichern (wirkt beim nächsten Start).
pub fn setzen(nativ: bool) {
    let Some(d) = datei() else { return };
    if nativ {
        if let Some(p) = d.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&d, b"1\n");
    } else {
        let _ = std::fs::remove_file(&d);
    }
}

/// Vor dem Öffnen des Fensters: unter Wayland mit verfügbarem XWayland über X11 starten (außer echtes Wayland ist
/// gewählt). Läuft das Programm schon unter X11, ändert sich nichts.
pub fn vor_dem_start() {
    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
        let x11 = std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
        if wayland && x11 && !nativ() {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    }
}

/// Läuft gerade eine Wayland-Sitzung (für die Anzeige der Option)?
pub fn wayland_sitzung() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland")) || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
