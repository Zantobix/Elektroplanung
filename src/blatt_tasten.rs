//! Tasten der Zeichenfläche wie in FormKeyDown des Originals (0x43f054) und die Anzeige „Heute“ im Schriftfeld.
//!
//! FormKeyDown des Originals (nur, solange kein Eingabefeld die Tasten hat):
//! - Strg+Tab: nächster Reiter, nach dem letzten wieder der erste. Umschalt wird nicht geprüft – Strg+Umschalt+Tab
//!   blättert also ebenfalls vorwärts (kein „voriges Blatt“).
//! - Leertaste: beim Platzieren eines Symbols (Klick oder Ziehen aus der Bibliothek) dreht sie die Vorschau um 90°
//!   (Zustand +0xa68: 0 → 1 → 2 → 4 → 0 = Drehung 0/90/180/270°, beim Absetzen übernommen); sonst zurück zum zuletzt
//!   angezeigten Blatt (+0x8f4 = Reiter, den 0x476768 beim Verlassen merkt) – zweimal drücken wechselt hin und her.
//! - Strg+1 … Strg+9: markierte Klemmstellen bekommen diese Etage (Byte 0x2C, nur bis zur Etagenzahl ihrer
//!   Klemmleiste); sind Symbole markiert, schaltet die Taste deren Zusatzoption n (Bit n−1 der Optionen) um.
//! - F gehalten (+0x7c5): beim Verschieben springt der Bezugspunkt des gegriffenen Objekts auf den Rasterpunkt unter
//!   der Maus (statt den Abstand beim Greifen zu behalten), siehe `fang_versatz`.
//! - Enter: nichts. S/A gehalten, Pfeiltasten, Entf, Strg+A: siehe view.rs/raster.rs.
//!
//! Schriftfeld (Routine 0x4876d8, Fall %Geändert): Auf dem Bildschirm steht „Heute“ statt des Änderungsdatums, wenn
//! das Blatt in dieser Sitzung geändert wurde (Rückgängig-Liste enthält das Blatt; zusätzlich prüft das Original,
//! dass das Erstelldatum nicht genau der jetzige Augenblick ist – das speichert der Nachbau nicht); sonst das Datum
//! BLT @120 bzw. „---------“. %Datum ist immer das heutige Datum, %Erstellt
//! das Datum BLT @112. Druck/PDF zeigen das Datum (Kennzeichen des Bildschirm-Zeichnens, Zeichenkontext +8).

use crate::app::App;
use crate::model::*;
use eframe::egui::{self, Key};
use std::cell::Cell;
use std::collections::HashSet;

/// Zustand der Tasten und der Anzeige „Heute“.
#[derive(Default, Clone, Debug)]
pub struct Stand {
    /// zuletzt angezeigtes Blatt vor dem jetzigen (Leertaste)
    pub vorher: Option<SheetRef>,
    pub jetzt: Option<SheetRef>,
    /// Drehung (0–3) des zu platzierenden Symbols
    pub drehung: u8,
    /// in dieser Sitzung geänderte Blätter (Schriftfeld „Heute“)
    pub geaendert: HashSet<SheetRef>,
}

/// Tastendruck, der hier ausgewertet wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taste {
    StrgTab,
    Leertaste,
    StrgZiffer(u8),
}

/// Tasten dieses Bildes (ohne Alt; Strg+Ziffer nur 1–9).
pub fn tasten(i: &egui::InputState) -> Vec<Taste> {
    let m = i.modifiers;
    let strg = m.ctrl || (m.command && !m.mac_cmd);
    let mut v = vec![];
    if m.alt {
        return v;
    }
    if strg && i.key_pressed(Key::Tab) {
        v.push(Taste::StrgTab);
    }
    if !strg && !m.shift && i.key_pressed(Key::Space) {
        v.push(Taste::Leertaste);
    }
    if strg && !m.shift {
        let ziffern = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        for (n, k) in ziffern.into_iter().enumerate() {
            if i.key_pressed(k) {
                v.push(Taste::StrgZiffer(n as u8 + 1));
            }
        }
    }
    v
}

thread_local! {
    static HEUTE: Cell<bool> = const { Cell::new(false) };
}

/// Während `f` zeigt das Schriftfeld bei %Geändert „Heute“ (wenn `aktiv`) – nur für das Zeichnen auf dem Bildschirm.
pub fn mit_heute<R>(aktiv: bool, f: impl FnOnce() -> R) -> R {
    let alt = HEUTE.with(|h| h.replace(aktiv));
    let r = f();
    HEUTE.with(|h| h.set(alt));
    r
}

/// %Geändert ohne Änderungsdatum des Blattes (BLT @120 = 0, z. B. alle Klemmpläne in W2): das Original greift nicht
/// auf das Projektdatum zurück, sondern zeigt diese Striche (0x4876d8 Fall 8).
pub const OHNE_DATUM: &str = "---------";

/// Datum für %Geändert: BLT @120 des Blattes, sonst `OHNE_DATUM`.
pub fn geaendert_datum(sheet: &Sheet) -> String {
    if sheet.geaendert.trim().is_empty() {
        OHNE_DATUM.into()
    } else {
        sheet.geaendert.clone()
    }
}

/// Wert für %Geändert: „Heute“ beim Bildschirm-Zeichnen eines in dieser Sitzung geänderten Blattes, sonst das Datum.
pub fn geaendert_text(datum: String) -> String {
    if HEUTE.with(|h| h.get()) {
        "Heute".into()
    } else {
        datum
    }
}

impl App {
    /// Je Bild: Blattwechsel für die Leertaste merken, Drehung nach dem Platzieren zurücksetzen.
    pub fn blatt_verfolgen(&mut self) {
        let jetzt = self.cur();
        if jetzt != self.bt.jetzt {
            if self.bt.jetzt.is_some() {
                self.bt.vorher = self.bt.jetzt;
            }
            self.bt.jetzt = jetzt;
        }
        if self.placing.is_none() && self.ring.is_none() {
            self.bt.drehung = 0;
        }
    }

    /// Zeigt das Schriftfeld des Blattes `r` auf dem Bildschirm „Heute“?
    pub fn zeigt_heute(&self, r: SheetRef) -> bool {
        self.sym_edit.is_none() && self.bt.geaendert.contains(&r)
    }

    /// Tastendruck ausführen (Aufrufer prüft, dass kein Eingabefeld und kein Dialog die Tasten hat).
    pub fn blatt_taste(&mut self, t: Taste) {
        match t {
            Taste::StrgTab => {
                if !self.tabs.is_empty() {
                    self.active = if self.active + 1 < self.tabs.len() { self.active + 1 } else { 0 };
                    self.selection.clear();
                }
            }
            Taste::Leertaste => {
                if self.placing.is_some() || self.ring.is_some() || self.lib_drag {
                    self.bt.drehung = (self.bt.drehung + 1) % 4;
                } else if let Some(i) = self.bt.vorher.and_then(|v| self.tabs.iter().position(|r| *r == v)) {
                    if i != self.active {
                        self.active = i;
                        self.selection.clear();
                    }
                }
            }
            Taste::StrgZiffer(n) => self.strg_ziffer(n),
        }
    }

    /// Strg+Ziffer: Etage der markierten Klemmstellen bzw. Zusatzoption n der markierten Symbole (siehe oben).
    pub fn strg_ziffer(&mut self, n: u8) {
        let objs = self.selected_objs();
        let symbole: Vec<u64> = objs.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| o.id).collect();
        let Some(r) = self.cur() else { return };
        if self.sym_edit.is_some() {
            return;
        }
        if !symbole.is_empty() {
            let bit = 1u32 << (n - 1);
            self.checkpoint();
            self.with_sheet(|s, _| {
                for o in s.objects.iter_mut().filter(|o| symbole.contains(&o.id)) {
                    if let Kind::Symbol { flags, .. } = &mut o.kind {
                        *flags ^= bit;
                    }
                }
            });
            self.info = format!("Option {n} umgeschaltet");
            return;
        }
        let Some(a) = self.project.anlagen.get(r.anlage) else { return };
        let ziele: Vec<u64> = objs
            .iter()
            .filter(|o| match &o.kind {
                Kind::Terminal { leiste, level, .. } => {
                    let e = crate::klemmen::etagen(a, leiste);
                    e > 1 && n as i32 <= e && *level != n
                }
                _ => false,
            })
            .map(|o| o.id)
            .collect();
        if ziele.is_empty() {
            return;
        }
        self.checkpoint();
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| ziele.contains(&o.id)) {
                if let Kind::Terminal { level, .. } = &mut o.kind {
                    *level = n;
                }
            }
        });
        self.info = format!("Etage {n} gesetzt");
    }

    /// Taste F beim Verschieben: Verschiebung, mit der der Bezugspunkt `bezug` (Lage vor dem Ziehen) auf dem
    /// Rasterpunkt unter der Maus `m` liegt.
    pub fn fang_versatz(&self, bezug: P, m: P) -> P {
        self.snap(m).sub(bezug)
    }
}

/// App ohne Fenster für Tests (Bibliothek aus dem Zwischenspeicher, sonst leer).
#[doc(hidden)]
pub fn test_app() -> App {
    let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
    App::new(&cc, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heute_nur_innerhalb() {
        let mut s = Sheet::new(1);
        s.geaendert.clear();
        assert_eq!(geaendert_datum(&s), "---------");
        s.geaendert = "03.04.2025".into();
        assert_eq!(geaendert_datum(&s), "03.04.2025");
        assert_eq!(geaendert_text("01.02.2026".into()), "01.02.2026");
        assert_eq!(mit_heute(true, || geaendert_text("01.02.2026".into())), "Heute");
        assert_eq!(geaendert_text("01.02.2026".into()), "01.02.2026");
    }

    fn app_mit_blaettern(n: usize) -> App {
        let mut app = test_app();
        app.project = Project::new_default();
        for i in 1..n {
            app.project.anlagen[0].sheets.push(Sheet::new(i as u32 + 1));
        }
        app.tabs = (0..n).map(|s| SheetRef { anlage: 0, sheet: s }).collect();
        app.active = 0;
        app
    }

    #[test]
    fn strg_tab_und_leertaste() {
        let mut app = app_mit_blaettern(3);
        app.blatt_verfolgen();
        app.blatt_taste(Taste::StrgTab);
        app.blatt_verfolgen();
        assert_eq!(app.active, 1);
        app.blatt_taste(Taste::StrgTab);
        app.blatt_verfolgen();
        app.blatt_taste(Taste::StrgTab);
        app.blatt_verfolgen();
        assert_eq!(app.active, 0, "nach dem letzten Reiter wieder der erste");
        // Leertaste: zurück zum zuletzt angezeigten Blatt und wieder hin
        app.blatt_taste(Taste::Leertaste);
        app.blatt_verfolgen();
        assert_eq!(app.active, 2);
        app.blatt_taste(Taste::Leertaste);
        app.blatt_verfolgen();
        assert_eq!(app.active, 0);
    }

    #[test]
    fn leertaste_dreht_beim_platzieren() {
        let mut app = app_mit_blaettern(2);
        app.placing = Some("x".into());
        for soll in [1, 2, 3, 0] {
            app.blatt_taste(Taste::Leertaste);
            assert_eq!(app.bt.drehung, soll);
        }
        assert_eq!(app.active, 0);
        app.bt.drehung = 2;
        app.placing = None;
        app.blatt_verfolgen();
        assert_eq!(app.bt.drehung, 0);
    }

    #[test]
    fn strg_ziffer_option_und_etage() {
        let mut app = app_mit_blaettern(1);
        let leiste = "-X9".to_string();
        app.project.anlagen[0].klemmen.push(KlemmInfo { id: 99, name: leiste.clone(), etagen: 3, ..Default::default() });
        app.with_sheet(|s, _| {
            s.objects.push(Obj { id: 1, kind: Kind::Terminal { pos: P::new(10.0, 10.0), leiste: leiste.clone(), nr: 1, level: 1, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        });
        app.selection = vec![1];
        app.strg_ziffer(2);
        let lv = |app: &App| match &app.sheet().unwrap().objects[0].kind {
            Kind::Terminal { level, .. } => *level,
            _ => 0,
        };
        assert_eq!(lv(&app), 2);
        app.strg_ziffer(5);
        assert_eq!(lv(&app), 2, "nicht über die Etagenzahl der Leiste");
        app.do_undo();
        assert_eq!(lv(&app), 1);
        // markiertes Symbol: Strg+2 schaltet Option 2 (Bit 1) um
        app.with_sheet(|s, _| {
            s.objects.push(Obj { id: 2, kind: Kind::Symbol { key: "k".into(), pos: P::new(50.0, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-K1".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 1, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        });
        app.selection = vec![2];
        app.strg_ziffer(2);
        assert!(matches!(app.sheet().unwrap().objects[1].kind, Kind::Symbol { flags: 3, .. }));
        app.strg_ziffer(2);
        assert!(matches!(app.sheet().unwrap().objects[1].kind, Kind::Symbol { flags: 1, .. }));
    }

    #[test]
    fn heute_nach_aenderung() {
        let mut app = app_mit_blaettern(2);
        let r = SheetRef { anlage: 0, sheet: 0 };
        assert!(!app.zeigt_heute(r));
        app.checkpoint();
        assert!(app.zeigt_heute(r));
        assert!(!app.zeigt_heute(SheetRef { anlage: 0, sheet: 1 }));
        app.new_project();
        assert!(!app.zeigt_heute(r), "neu geladenes Projekt: wieder das Datum");
    }
}
