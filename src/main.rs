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
        .with_icon(Arc::new(app_icon()));
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
    eframe::run_native("Elektroplanung", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, shot.clone())))))
}

/// Kleines eigenes Programmsymbol (Blitz auf Blatt).
fn app_icon() -> egui::IconData {
    let n = 32usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            let inside = (3..29).contains(&x) && (2..30).contains(&y);
            let border = inside && (x == 3 || x == 28 || y == 2 || y == 29);
            let (fx, fy) = (x as f32, y as f32);
            let bolt = (fy > 5.0 && fy < 17.0 && (fx - (20.0 - (fy - 5.0) * 0.6)).abs() < 2.5)
                || (fy >= 15.0 && fy < 27.0 && (fx - (17.0 - (fy - 15.0) * 0.6)).abs() < 2.5);
            let c: [u8; 4] = if border {
                [40, 40, 40, 255]
            } else if bolt {
                [230, 180, 0, 255]
            } else if inside {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 0]
            };
            rgba[i..i + 4].copy_from_slice(&c);
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}
