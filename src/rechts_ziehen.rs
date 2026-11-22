//! Rechte Maustaste auf der Zeichenfläche wie im Original (MouseDown 0x40923c, MouseMove 0x41615f–0x4161fd,
//! Beobachtungen am Original 21.11.2026):
//! - Ziehen auf einem Symbol bzw. markierten Objekten: eine blaue Vorschau der Markierung folgt der Maus, beim
//!   Loslassen liegt eine Kopie an der neuen Stelle – mit derselben BMK (anders als das bewusst abweichende Einfügen
//!   des Nachbaus, das neu nummeriert), die Kopie ist markiert, kein Kontextmenü, ein Rückgängig-Schritt.
//!   Zeiger: Pfeil mit Plus (Ressource 5, Zeiger 3).
//! - Auf freier Fläche und auf Leitungen entsteht nichts (kein Abzweig); das Kontextmenü erscheint beim Loslassen,
//!   auch nach einer Mausbewegung (egui öffnet es sonst nur bei einem Klick ohne Ziehen).
//! - Ein Rechtsklick ohne Bewegung (weniger als 3 Bildpunkte oder kein Rasterschritt) öffnet wie bisher das Menü.
//! - Freie Fläche bei vergrößerter Ansicht (MouseMove 0x40f351 ff.: Shift-Prüfung 0x40b204 mit Bit 4 = ssRight,
//!   nichts getroffen, eine Bildlaufleiste hat einen Bereich): ab 3 Bildpunkten Weg verschiebt das Ziehen die
//!   Ansicht – Bildlaufposition = Position beim Drücken + Druckpunkt − Maus (das Blatt folgt der Maus), begrenzt auf
//!   den Bereich der Leisten; Merker +0x7b9. MouseUp (0x41fa5e) öffnet das Kontextmenü dann nicht. Ohne
//!   Bildlaufbereich (Blatt passt ins Fenster) bleibt es beim Menü nach dem Loslassen.

use crate::app::{App, Drag};
use crate::canvas::{self, DrawCtx};
use crate::kontextmenue::MenueZiel;
use crate::model::*;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;

/// Mindestweg in Bildpunkten, ab dem das Ziehen eine Kopie ablegt (wie die 3-Pixel-Grenze in MouseMove).
pub const MIN_WEG: f32 = 3.0;

/// Legt Ziehen mit der rechten Taste auf diesem Ziel eine Kopie ab? (Leitungen, Symboltexte, freie Fläche und
/// Kontaktspiegel nicht; Anschlüsse im Symboleditor nicht)
pub fn kopierbar(ziel: MenueZiel) -> bool {
    matches!(ziel, MenueZiel::Symbol(_) | MenueZiel::Klemme(_) | MenueZiel::Verweis(_) | MenueZiel::Sammelschiene(_) | MenueZiel::Text(_) | MenueZiel::Grafik(_))
}

/// Öffnen/Schließen des Kontextmenüs der Zeichenfläche in diesem Bild: wie `Response::context_menu` (Rechtsklick
/// öffnet, Linksklick schließt), dazu `los_ohne_kopie` = rechte Taste nach einem Ziehen ohne Kopie losgelassen.
/// Wurde eben eine Kopie abgelegt, bleibt das Menü zu.
pub fn menue_befehl(resp: &egui::Response, los_ohne_kopie: bool, kopiert: bool) -> Option<egui::SetOpenCommand> {
    if kopiert {
        None
    } else if resp.secondary_clicked() || los_ohne_kopie {
        Some(egui::SetOpenCommand::Bool(true))
    } else if resp.clicked() {
        Some(egui::SetOpenCommand::Bool(false))
    } else {
        None
    }
}

/// Ergebnis des Loslassens der rechten Taste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Los {
    /// keine rechte Taste gedrückt gewesen
    Nichts,
    /// Kontextmenü öffnen
    Menue,
    /// Kopie abgelegt
    Kopie,
    /// Ansicht verschoben (kein Kontextmenü)
    Verschoben,
}

/// Rand um das Blatt im Bildlaufbereich in Bildpunkten (wie die Bildlaufleisten in view.rs)
const RAND: f32 = 20.0;

/// Neuer Ansichtsversatz beim Verschieben mit der rechten Taste: `o0` = Versatz beim Drücken, `d` = Mausweg,
/// `blatt` = Blattgröße in Bildpunkten, `area` = Zeichenfläche. `None`, wenn keine Bildlaufleiste einen Bereich hat
/// (das Blatt samt Rand liegt ganz im Fenster). Je Richtung bleibt der Versatz im Bereich der Leisten: das Blatt
/// wird nicht weiter hinausgeschoben, als die Leiste reicht (eine schon weiter verschobene Lage bleibt erreichbar).
pub fn schiebe_versatz(o0: Vec2, d: Vec2, blatt: Vec2, area: Rect) -> Option<Vec2> {
    let achse = |o: f32, d: f32, l: f32, a0: f32, a: f32| -> (bool, f32) {
        let s0 = a0 + o - RAND;
        let l = l + 2.0 * RAND;
        // Bildlaufbereich wie die Leisten: Blatt samt Rand vereinigt mit dem Fenster, größer als das Fenster?
        let bereich = (s0 + l).max(a0 + a) - s0.min(a0) > a + 0.5;
        if !bereich {
            return (false, o);
        }
        let lo = a0.min(a0 + a - l).min(s0);
        let hi = a0.max(a0 + a - l).max(s0);
        (true, (s0 + d).clamp(lo, hi) - a0 + RAND)
    };
    let (bx, x) = achse(o0.x, d.x, blatt.x, area.min.x, area.width());
    let (by, y) = achse(o0.y, d.y, blatt.y, area.min.y, area.height());
    (bx || by).then_some(Vec2::new(x, y))
}

impl App {
    /// Rechte Taste gedrückt (nach dem Markieren des Ziels in handle_canvas_input).
    pub fn rechts_druck(&mut self, ziel: MenueZiel, m: P, h: Pos2) {
        let kopie = kopierbar(ziel) && !self.selection.is_empty();
        let schieben = (ziel == MenueZiel::Leer).then_some(self.view.offset);
        self.drag = Drag::Rechts { start: self.snap(m), druck: h, kopie, schieben, geschoben: false };
    }

    /// Verschiebung der Kopie beim Ziehen bis `m` (None = noch keine: zu kurzer Weg oder kein Rasterschritt).
    pub fn kopie_versatz(&self, m: P, h: Pos2) -> Option<P> {
        let Drag::Rechts { start, druck, kopie: true, .. } = self.drag else { return None };
        let d = self.snap(m).sub(start);
        (h.distance(druck) >= MIN_WEG && (d.x.abs() > 1e-4 || d.y.abs() > 1e-4)).then_some(d)
    }

    /// Rechte Taste losgelassen bei `m` (mm) / `h` (Bildschirm).
    pub fn rechts_los(&mut self, m: P, h: Pos2) -> Los {
        if !matches!(self.drag, Drag::Rechts { .. }) {
            return Los::Nichts;
        }
        if let Drag::Rechts { geschoben: true, .. } = self.drag {
            self.drag = Drag::None;
            return Los::Verschoben;
        }
        let d = self.kopie_versatz(m, h);
        self.drag = Drag::None;
        match d {
            Some(d) if !self.kopie_ablegen(d).is_empty() => Los::Kopie,
            _ => Los::Menue,
        }
    }

    /// Rechte Taste gedrückt bei `h` (Bildschirm): auf freier Fläche die vergrößerte Ansicht verschieben (siehe
    /// Modulbeschreibung). `area` = Zeichenfläche.
    pub fn rechts_schieben(&mut self, h: Pos2, area: Rect) {
        let Drag::Rechts { druck, schieben: Some(o0), geschoben, .. } = self.drag else { return };
        if !geschoben && h.distance(druck) < MIN_WEG {
            return;
        }
        let (sw, sh) = if self.sym_edit.is_some() { (SHEET_W, SHEET_H) } else { self.sheet().map(|s| s.dims()).unwrap_or((SHEET_W, SHEET_H)) };
        let blatt = Vec2::new(sw, sh) * self.view.zoom;
        let Some(o) = schiebe_versatz(o0, h - druck, blatt, area) else { return };
        self.view.offset = o;
        self.view.user = true;
        if let Drag::Rechts { geschoben, .. } = &mut self.drag {
            *geschoben = true;
        }
    }

    /// Markierte Objekte um `d` versetzt kopieren: gleiche BMK, neue Kennungen (ohne Originalblock, damit die Datei
    /// keine Kennung doppelt enthält), Gruppen der Kopie neu, Kontaktspiegel nicht (wie Kopieren). Die Kopie wird
    /// markiert; ein Rückgängig-Schritt. Rückgabe: Ids der Kopien.
    pub fn kopie_ablegen(&mut self, d: P) -> Vec<u64> {
        let objs: Vec<Obj> = self.selected_objs().into_iter().filter(|o| !matches!(o.kind, Kind::Spiegel { .. })).collect();
        if objs.is_empty() {
            return vec![];
        }
        self.checkpoint();
        let mut neu = vec![];
        let mut gruppen: HashMap<u64, u64> = HashMap::new();
        for mut o in objs {
            o.id = self.project.alloc_id();
            o.raw = None;
            if let Some(g) = o.group {
                o.group = Some(*gruppen.entry(g).or_insert_with(|| 1_000_000 + o.id));
            }
            o.kind.translate(d);
            neu.push(o.id);
            let id = o.id;
            self.with_sheet(|s, _| s.objects.push(o));
            self.edit_tag_new(id);
        }
        self.selection = neu.clone();
        self.info = format!("{} Objekt(e) kopiert", neu.len());
        neu
    }

    /// Blaue Vorschau der Kopie (markierte Objekte, um den jetzigen Versatz verschoben).
    pub fn kopie_vorschau(&self, dc: &DrawCtx, m: P, h: Pos2, farbe: Color32) {
        let Some(d) = self.kopie_versatz(m, h) else { return };
        for mut o in self.selected_objs().into_iter().filter(|o| !matches!(o.kind, Kind::Spiegel { .. })) {
            o.kind.translate(d);
            canvas::draw_object(dc, &o, Some(farbe));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blatt_tasten::test_app;

    fn symbol(id: u64, x: f32, bmk: &str) -> Obj {
        Obj {
            id,
            kind: Kind::Symbol { key: "k".into(), pos: P::new(x, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
            pen: 0,
            fixed: false,
            group: None,
            raw: None,
        }
    }

    #[test]
    fn ziele() {
        assert!(kopierbar(MenueZiel::Symbol(1)));
        assert!(kopierbar(MenueZiel::Grafik(1)));
        assert!(!kopierbar(MenueZiel::Leitung(1)));
        assert!(!kopierbar(MenueZiel::Leer));
        assert!(!kopierbar(MenueZiel::Symboltext { id: 1, wert: false }));
        assert!(!kopierbar(MenueZiel::Kontaktspiegel(1)));
    }

    #[test]
    fn kopie_mit_gleicher_bmk_und_rueckgaengig() {
        let mut app = test_app();
        app.project = Project::new_default();
        app.project.next_id = 100;
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        app.with_sheet(|s, _| s.objects.push(symbol(1, 50.0, "-E2.1")));
        app.selection = vec![1];
        let start = P::new(50.0, 50.0);
        app.rechts_druck(MenueZiel::Symbol(1), start, Pos2::new(100.0, 100.0));
        // kaum bewegt: Menü statt Kopie
        assert_eq!(app.kopie_versatz(P::new(50.2, 50.0), Pos2::new(101.0, 100.0)), None);
        let m = start.add(P::new(20.0, 10.0));
        assert!(app.kopie_versatz(m, Pos2::new(180.0, 140.0)).is_some());
        assert_eq!(app.rechts_los(m, Pos2::new(180.0, 140.0)), Los::Kopie);
        let s = app.sheet().unwrap();
        assert_eq!(s.objects.len(), 2);
        let k = &s.objects[1];
        assert_eq!(app.selection, vec![k.id]);
        match &k.kind {
            Kind::Symbol { bmk, pos, .. } => {
                assert_eq!(bmk, "-E2.1", "gleiche BMK wie im Original");
                assert!(pos.dist(P::new(50.0, 50.0)) > 5.0);
            }
            _ => panic!(),
        }
        assert!(matches!(app.drag, Drag::None));
        app.do_undo();
        assert_eq!(app.sheet().unwrap().objects.len(), 1);
    }

    #[test]
    fn freie_flaeche_und_leitung_oeffnen_das_menue() {
        let mut app = test_app();
        app.project = Project::new_default();
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        app.rechts_druck(MenueZiel::Leer, P::new(10.0, 10.0), Pos2::new(10.0, 10.0));
        assert_eq!(app.rechts_los(P::new(80.0, 40.0), Pos2::new(300.0, 200.0)), Los::Menue);
        app.with_sheet(|s, _| s.objects.push(Obj { id: 5, kind: Kind::Wire { pts: vec![P::new(0.0, 0.0), P::new(50.0, 0.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }));
        app.selection = vec![5];
        app.rechts_druck(MenueZiel::Leitung(5), P::new(10.0, 0.0), Pos2::new(10.0, 10.0));
        assert_eq!(app.rechts_los(P::new(80.0, 40.0), Pos2::new(300.0, 200.0)), Los::Menue);
        assert_eq!(app.sheet().unwrap().objects.len(), 1, "kein Abzweig, keine Kopie");
        assert_eq!(app.rechts_los(P::new(80.0, 40.0), Pos2::new(300.0, 200.0)), Los::Nichts);
    }

    #[test]
    fn schiebe_versatz_im_bereich_der_leisten() {
        let area = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
        // Blatt passt ins Fenster: kein Bildlaufbereich, nichts zu verschieben
        assert_eq!(schiebe_versatz(Vec2::new(100.0, 50.0), Vec2::new(30.0, 30.0), Vec2::new(400.0, 300.0), area), None);
        // vergrößert (2000 × 1500 Bildpunkte): das Blatt folgt der Maus
        let blatt = Vec2::new(2000.0, 1500.0);
        let o0 = Vec2::new(-500.0, -400.0);
        assert_eq!(schiebe_versatz(o0, Vec2::new(30.0, -20.0), blatt, area), Some(Vec2::new(-470.0, -420.0)));
        // begrenzt: linke/obere Blattkante (samt Rand) höchstens an der Fensterkante …
        assert_eq!(schiebe_versatz(o0, Vec2::new(5000.0, 5000.0), blatt, area), Some(Vec2::new(20.0, 20.0)));
        // … rechte/untere höchstens an der rechten/unteren Fensterkante
        assert_eq!(schiebe_versatz(o0, Vec2::new(-5000.0, -5000.0), blatt, area), Some(Vec2::new(800.0 - 2020.0, 600.0 - 1520.0)));
        // nur waagerecht zu groß: senkrecht bleibt der Versatz
        assert_eq!(schiebe_versatz(Vec2::new(-100.0, 50.0), Vec2::new(10.0, 10.0), Vec2::new(2000.0, 300.0), area), Some(Vec2::new(-90.0, 50.0)));
    }

    #[test]
    fn rechts_ziehen_auf_freier_flaeche_verschiebt_die_ansicht() {
        let mut app = test_app();
        app.project = Project::new_default();
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        let area = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
        // vergrößert: 10 Bildpunkte je mm, das Blatt ist größer als das Fenster
        app.view.zoom = 10.0;
        app.view.offset = Vec2::new(-1000.0, -800.0);
        let druck = Pos2::new(400.0, 300.0);
        app.rechts_druck(MenueZiel::Leer, P::new(10.0, 10.0), druck);
        // unter 3 Bildpunkten: nichts
        app.rechts_schieben(Pos2::new(401.0, 301.0), area);
        assert_eq!(app.view.offset, Vec2::new(-1000.0, -800.0));
        app.rechts_schieben(Pos2::new(450.0, 280.0), area);
        assert_eq!(app.view.offset, Vec2::new(-950.0, -820.0));
        assert_eq!(app.rechts_los(P::new(10.0, 10.0), Pos2::new(450.0, 280.0)), Los::Verschoben, "kein Kontextmenü");
        // auf einem Objekt verschiebt die rechte Taste die Ansicht nicht
        app.with_sheet(|s, _| s.objects.push(symbol(1, 50.0, "-E1")));
        app.selection = vec![1];
        app.rechts_druck(MenueZiel::Symbol(1), P::new(50.0, 50.0), druck);
        app.rechts_schieben(Pos2::new(480.0, 300.0), area);
        assert_eq!(app.view.offset, Vec2::new(-950.0, -820.0));
        // nicht vergrößert (Blatt passt ins Fenster): kein Verschieben, Menü beim Loslassen
        app.rechts_los(P::new(50.0, 50.0), druck);
        app.view.zoom = 1.0;
        app.view.offset = Vec2::new(100.0, 100.0);
        app.rechts_druck(MenueZiel::Leer, P::new(10.0, 10.0), druck);
        app.rechts_schieben(Pos2::new(480.0, 350.0), area);
        assert_eq!(app.view.offset, Vec2::new(100.0, 100.0));
        assert_eq!(app.rechts_los(P::new(10.0, 10.0), Pos2::new(480.0, 350.0)), Los::Menue);
    }
}
