//! Elektroplanung – Nachbau in Rust. Bibliothek mit Datenmodell, Dateiformaten, Zeichnen, GUI und MCP-Server.

pub mod app;
pub mod baum;
pub mod bauteil_darstellung;
pub mod berechnet;
pub mod bildexport;
pub mod blattnummer;
pub mod canvas;
pub mod dialoge;
pub mod dialoge3;
pub mod druck;
pub mod dxf;
pub mod epr;
pub mod epr_neu;
pub mod epr_projekt;
pub mod epr_write;
pub mod epr_zusatz;
pub mod eprfile;
pub mod extra;
pub mod groesse;
pub mod hover;
pub mod grafik;
pub mod icons;
pub mod inhalt;
pub mod klemmen;
pub mod klemmplan;
pub mod kontaktspiegel;
pub mod kuerzel;
pub mod leitung_stift;
pub mod listen;
pub mod mcp;
pub mod mcp_symbol;
pub mod mehrfach;
pub mod menus;
pub mod mitziehen;
pub mod model;
pub mod optionen;
pub mod parameter;
pub mod pdf;
pub mod pdf_schriften;
pub mod props;
pub mod pruefen;
pub mod schriften;
pub mod schriftmass;
pub mod schriftfeld_editor;
pub mod suche;
pub mod symbol_freigaben;
pub mod symbol_kopie;
pub mod symbol_stifte;
pub mod symbols;
pub mod textmass;
pub mod titleblock;
pub mod variantenrad;
pub mod verweise;
pub mod view;
pub mod vorlage;
pub mod vorschau;
pub mod zusatz;

use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

/// Schriftdatei laden. Windows: %WINDIR%\Fonts (Times New Roman, Segoe UI). Linux/macOS: Liberation (Serif als
/// metrisch gleicher Ersatz für Times New Roman, Sans für Arial/Segoe) bzw. DejaVu aus den üblichen Schriftordnern;
/// fehlt alles, bleibt die eingebaute egui-Schrift.
pub fn load_font(name: &str) -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        std::fs::read(PathBuf::from(&windir).join("Fonts").join(name)).ok()
    }
    #[cfg(not(windows))]
    {
        let alt: &[&str] = match name {
            "times.ttf" => &["LiberationSerif-Regular.ttf", "Tinos-Regular.ttf", "DejaVuSerif.ttf", "Times New Roman.ttf"],
            "timesbd.ttf" => &["LiberationSerif-Bold.ttf", "Tinos-Bold.ttf", "DejaVuSerif-Bold.ttf", "Times New Roman Bold.ttf"],
            "timesi.ttf" => &["LiberationSerif-Italic.ttf", "Tinos-Italic.ttf", "DejaVuSerif-Italic.ttf", "Times New Roman Italic.ttf"],
            "timesbi.ttf" => &["LiberationSerif-BoldItalic.ttf", "Tinos-BoldItalic.ttf", "DejaVuSerif-BoldItalic.ttf", "Times New Roman Bold Italic.ttf"],
            "segoeui.ttf" => &["LiberationSans-Regular.ttf", "DejaVuSans.ttf", "Arial.ttf"],
            "segoeuib.ttf" | "arialbd.ttf" => &["LiberationSans-Bold.ttf", "DejaVuSans-Bold.ttf", "Arial Bold.ttf"],
            _ => &[],
        };
        let mut dirs: Vec<PathBuf> = vec!["/usr/share/fonts".into(), "/usr/local/share/fonts".into(), "/Library/Fonts".into(), "/System/Library/Fonts/Supplemental".into()];
        if let Ok(h) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&h).join(".local/share/fonts"));
            dirs.push(PathBuf::from(&h).join(".fonts"));
        }
        for d in dirs {
            if let Some(p) = alt.iter().find_map(|f| find_file(&d, f, 4)) {
                return std::fs::read(p).ok();
            }
        }
        None
    }
}

/// Datei `name` in `dir` und Unterordnern (bis `depth` Ebenen) suchen.
#[cfg(not(windows))]
fn find_file(dir: &std::path::Path, name: &str, depth: usize) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(dir).ok()?.flatten().filter(|e| e.path().is_dir()).find_map(|e| find_file(&e.path(), name, depth - 1))
}

/// Projektseite (Info-Dialog, Cargo.toml)
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Ablageordner für Einstellungen, Zwischenspeicher und Benutzerbibliothek: Windows %APPDATA%\Elektroplanung,
/// sonst $XDG_CONFIG_HOME bzw. ~/.config/elektroplanung.
pub fn config_dir() -> PathBuf {
    if let Ok(d) = std::env::var("EP_APPDIR") {
        return PathBuf::from(d);
    }
    #[cfg(windows)]
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    #[cfg(not(windows))]
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join(if cfg!(windows) { "Elektroplanung" } else { "elektroplanung" })
}

/// Schriften (Segoe UI für die Oberfläche, Times New Roman für das Blatt) und helles Windows-Aussehen.
pub fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let load = load_font;
    let defaults_prop = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    let mut times_list = vec![];
    if let Some(d) = load("times.ttf") {
        fonts.font_data.insert("times".into(), Arc::new(egui::FontData::from_owned(d)));
        times_list.push("times".to_string());
    }
    // Schnitte für Texteinstellungen Fett/Kursiv (Original: Schriftdialog)
    for (file, fam) in [("timesbd.ttf", "times_b"), ("timesi.ttf", "times_i"), ("timesbi.ttf", "times_bi")] {
        let mut list = vec![];
        if let Some(d) = load(file) {
            fonts.font_data.insert(fam.into(), Arc::new(egui::FontData::from_owned(d)));
            list.push(fam.to_string());
        }
        list.extend(times_list.iter().cloned());
        fonts.families.insert(egui::FontFamily::Name(fam.into()), list);
    }
    let mut ui_list = vec![];
    if let Some(d) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), Arc::new(egui::FontData::from_owned(d)));
        ui_list.push("segoe".to_string());
    }
    let mut bold_list = vec![];
    if let Some(d) = load("segoeuib.ttf").or_else(|| load("arialbd.ttf")) {
        fonts.font_data.insert("segoe_bold".into(), Arc::new(egui::FontData::from_owned(d)));
        bold_list.push("segoe_bold".to_string());
    }
    let mut prop = ui_list.clone();
    prop.extend(defaults_prop.iter().cloned());
    fonts.families.insert(egui::FontFamily::Proportional, prop.clone());
    times_list.extend(prop.iter().cloned());
    fonts.families.insert(egui::FontFamily::Name("times".into()), times_list.clone());
    for fam in ["times_b", "times_i", "times_bi"] {
        if let Some(l) = fonts.families.get_mut(&egui::FontFamily::Name(fam.into())) {
            l.extend(prop.iter().cloned());
        }
    }
    bold_list.extend(prop.iter().cloned());
    fonts.families.insert(egui::FontFamily::Name("bold".into()), bold_list);
    // Schriften je Text (Familien ep:<Name>|<Schnitt>) beim ersten Gebrauch nachladen, Ersatz Times
    schriften::einrichten(ctx, &fonts, &prop);
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|st| {
        st.visuals = egui::Visuals::light();
        st.visuals.panel_fill = egui::Color32::from_rgb(240, 240, 240);
        st.visuals.window_fill = egui::Color32::from_rgb(240, 240, 240);
        st.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::BLACK;
        st.visuals.widgets.inactive.fg_stroke.color = egui::Color32::BLACK;
        st.visuals.override_text_color = Some(egui::Color32::BLACK);
        st.spacing.menu_margin = egui::Margin::same(3);
        st.interaction.tooltip_delay = 0.4;
        use egui::{FontFamily, FontId, TextStyle};
        st.text_styles.insert(TextStyle::Body, FontId::new(12.0, FontFamily::Proportional));
        st.text_styles.insert(TextStyle::Button, FontId::new(12.0, FontFamily::Proportional));
        st.text_styles.insert(TextStyle::Small, FontId::new(10.0, FontFamily::Proportional));
        st.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional));
    });
}

