//! Rechte Maustaste auf der Zeichenfläche wie im Original (MouseDown 0x40923c, MouseMove 0x41615f–0x4161fd,
//! Beobachtungen am Original 21.11.2026):
//! - Ziehen auf einem Symbol bzw. markierten Objekten: eine blaue Vorschau der Markierung folgt der Maus, beim
//!   Loslassen liegt eine Kopie an der neuen Stelle – mit derselben BMK (anders als das bewusst abweichende Einfügen
//!   des Nachbaus, das neu nummeriert), die Kopie ist markiert, kein Kontextmenü, ein Rückgängig-Schritt.
//!   Zeiger: Pfeil mit Plus (Ressource 5, Zeiger 3).
//! - Auf freier Fläche und auf Leitungen entsteht nichts (kein Abzweig); das Kontextmenü erscheint beim Loslassen,
//!   auch nach einer Mausbewegung (egui öffnet es sonst nur bei einem Klick ohne Ziehen).
//! - Ein Rechtsklick ohne Bewegung (weniger als 3 Bildpunkte oder kein Rasterschritt) öffnet wie bisher das Menü.

use crate::app::{App, Drag};
use crate::canvas::{self, DrawCtx};
use crate::kontextmenue::MenueZiel;
use crate::model::*;
use eframe::egui::{self, Color32, Pos2};
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
}

impl App {
    /// Rechte Taste gedrückt (nach dem Markieren des Ziels in handle_canvas_input).
    pub fn rechts_druck(&mut self, ziel: MenueZiel, m: P, h: Pos2) {
        let kopie = kopierbar(ziel) && !self.selection.is_empty();
        self.drag = Drag::Rechts { start: self.snap(m), druck: h, kopie };
    }

    /// Verschiebung der Kopie beim Ziehen bis `m` (None = noch keine: zu kurzer Weg oder kein Rasterschritt).
    pub fn kopie_versatz(&self, m: P, h: Pos2) -> Option<P> {
        let Drag::Rechts { start, druck, kopie: true } = self.drag else { return None };
        let d = self.snap(m).sub(start);
        (h.distance(druck) >= MIN_WEG && (d.x.abs() > 1e-4 || d.y.abs() > 1e-4)).then_some(d)
    }

    /// Rechte Taste losgelassen bei `m` (mm) / `h` (Bildschirm).
    pub fn rechts_los(&mut self, m: P, h: Pos2) -> Los {
        if !matches!(self.drag, Drag::Rechts { .. }) {
            return Los::Nichts;
        }
        let d = self.kopie_versatz(m, h);
        self.drag = Drag::None;
        match d {
            Some(d) if !self.kopie_ablegen(d).is_empty() => Los::Kopie,
            _ => Los::Menue,
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
}
