#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Elektroplanung – Nachbau in Rust (eframe/egui). Mit `--mcp` startet statt der Oberfläche der MCP-Server (stdio).

use elektroplanung::app;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp") {
        elektroplanung::mcp::run_stdio();
        return Ok(());
    }
    let shot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(PathBuf::from);

    let mut vp = egui::ViewportBuilder::default()
        .with_title("Elektroplanung Neues Projekt")
        .with_min_inner_size([830.0, 600.0])
        // Projektdateien per Drag & Drop öffnen (datei_ablegen.rs)
        .with_drag_and_drop(true)
        .with_icon(Arc::new(app_icon()))
        // feste Kennung: GNOME/Wayland ordnet das Fenster darüber der .desktop-Datei mit dem Symbol zu
        // (desktop_eintrag.rs)
        .with_app_id(elektroplanung::desktop_eintrag::KENNUNG);
    if shot.is_some() {
        vp = vp.with_inner_size([1920.0, 1007.0]).with_position([0.0, 0.0]).with_active(false);
        #[cfg(windows)]
        {
            vp = vp.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    } else {
        vp = vp.with_inner_size([1280.0, 800.0]).with_maximized(true);
    }
    let mut options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    options.persist_window = false;
    eframe::run_native("Elektroplanung", options, Box::new(move |cc| {
        let mut a = app::App::new(cc, shot.clone());
        // beim Start einmal im Hintergrund nach einer neueren Version suchen (Option, nicht bei Bildschirmfotos)
        if shot.is_none() && a.opt.update_pruefen {
            a.update.starten(&cc.egui_ctx, false);
        }
        // Linux: Startmenü-Eintrag mit Symbol (AppImage oder direkt gestartet; Option, desktop_eintrag.rs)
        if shot.is_none() && a.opt.desktop_eintrag {
            if let Some(m) = elektroplanung::desktop_eintrag::einrichten() {
                a.info = m;
            }
        }
        // Absturzschutz: Sicherung eines abgestürzten Programms anbieten (wiederherstellung.rs)
        if shot.is_none() {
            a.wiederherstellen_anbieten();
        }
        Ok(Box::new(a))
    }))
}

/// Programmsymbol aus resources/elektroplanung.png (tools/symbol_erzeugen.py); ersatzweise eine blaue Fläche.
fn app_icon() -> egui::IconData {
    if let Some((rgba, width, height)) = elektroplanung::desktop_eintrag::fenster_symbol() {
        return egui::IconData { rgba, width, height };
    }
    let n = 32usize;
    let rgba = [30u8, 90, 160, 255].repeat(n * n);
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}
