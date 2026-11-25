//! Projektdatei per Drag & Drop ins Fenster (datei_ablegen.rs): öffnet sie; mit ungespeicherten Änderungen kommt
//! zuerst die Rückfrage zum Speichern und das alte Projekt bleibt bis dahin offen.
use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use elektroplanung::app::{App, Pending};
use elektroplanung::blatt_tasten::test_app;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
struct Datei(PathBuf);
impl egui::DroppedFile for Datei {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

fn bild(ctx: &egui::Context, frame: &mut eframe::Frame, app: &mut App, abgelegt: Vec<PathBuf>) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 950.0))),
        dropped_files: abgelegt.into_iter().map(|p| Arc::new(Datei(p)) as egui::DroppedFileHandle).collect(),
        ..Default::default()
    };
    let _ = ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
}

#[test]
fn projekt_ablegen() {
    let Some(w2) = std::env::var("EP_SAMPLE").ok().filter(|p| Path::new(p).exists()) else { return };
    elektroplanung::meldungen::test_antworten(&[]);
    let (ctx, mut frame) = (egui::Context::default(), eframe::Frame::_new_kittest());
    let mut app = test_app();
    bild(&ctx, &mut frame, &mut app, vec![]);
    // ein Bild ohne Projektdatei: nur Hinweis
    bild(&ctx, &mut frame, &mut app, vec![PathBuf::from("/tmp/bild.png")]);
    assert!(app.info.contains("Nur Projekte"), "{}", app.info);
    // ohne Änderungen: gleich geöffnet
    bild(&ctx, &mut frame, &mut app, vec![PathBuf::from(&w2)]);
    assert_eq!(app.project.name, "Wohnung 2");
    // mit Änderungen: erst die Rückfrage, das Projekt bleibt offen
    app.dirty = true;
    let kopie = std::env::temp_dir().join("ep_ablegen.epr");
    std::fs::copy(&w2, &kopie).unwrap();
    bild(&ctx, &mut frame, &mut app, vec![kopie.clone()]);
    assert!(matches!(&app.ask_save, Some(Pending::OpenPath(p)) if *p == kopie));
}
