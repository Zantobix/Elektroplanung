//! Kontextmenü des Blattes wie im Original (PopupMenu1 der Formularressource TFORM1). Welche Einträge erscheinen,
//! hängt wie im Original vom Objekt unter der Maus beim Rechtsklick ab – Regeln und Tests in `kontextmenue.rs`.

use crate::app::App;
use crate::kontextmenue::{self, Eintrag, MenueZiel};
use crate::model::*;
use crate::symbols;
use eframe::egui;

impl App {
    fn change_symbol(&mut self, id: u64, f: impl FnOnce(&mut Kind)) {
        self.checkpoint();
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                f(&mut o.kind);
            }
        });
        self.dirty = true;
    }

    /// Kontextmenü des Blattes: Einträge je nach Objekt unter der Maus beim Rechtsklick (`kontextmenue.rs`).
    pub fn canvas_menu(&mut self, ui: &mut egui::Ui) {
        let ziel = kontextmenue::gemerkt(ui.ctx());
        let lage = self.menue_lage(ziel);
        let at = self.mouse_ctx;
        let obj = ziel.id().and_then(|id| self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).cloned());
        for e in kontextmenue::eintraege(&lage) {
            match e {
                Eintrag::Trenner => {
                    ui.separator();
                }
                Eintrag::Loeschen => {
                    if ui.add(egui::Button::new("Löschen").shortcut_text("Del")).clicked() {
                        self.delete_selection();
                        ui.close();
                        return;
                    }
                }
                Eintrag::ObjektEinfuegen => {
                    ui.add_enabled_ui(lage.projektsymbole, |ui| {
                        ui.menu_button("Objekt einfügen", |ui| {
                            let defs: Vec<(String, String)> = self.project.symdefs.iter().map(|d| (d.key.clone(), d.name.clone())).collect();
                            egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                                for (k, n) in defs {
                                    if ui.button(n).clicked() {
                                        self.place_symbol(k, at);
                                        ui.close();
                                    }
                                }
                            });
                        });
                    });
                }
                Eintrag::VerweisEinfuegen => {
                    ui.menu_button("Verweis einfügen", |ui| {
                        for (n, start) in [("Start", true), ("Ziel", false)] {
                            if ui.button(n).clicked() {
                                self.xdlg = Some(crate::extra::XDialog::Verweis { text: String::new(), start });
                                ui.close();
                            }
                        }
                    });
                }
                // Beschriftung je Lage wie im Original („Nach Oben“ bei einer unten gestapelten Schiene)
                Eintrag::Sammelschiene => {
                    let lage = match obj.as_ref().map(|o| &o.kind) {
                        Some(Kind::Busbar { lage, .. }) => *lage,
                        _ => 0,
                    };
                    ui.menu_button("Sammelschiene", |ui| {
                        if ui.button(crate::sammelschiene_lage::beschriftung(lage)).clicked() {
                            if let Some(id) = ziel.id() {
                                self.sammelschiene_umschalten(id);
                            }
                            ui.close();
                        }
                    });
                }
                Eintrag::Ausrichtung => self.menu_ausrichtung(ui, ziel, obj.as_ref()),
                Eintrag::Eigenschaften => {
                    if ui.add_enabled(kontextmenue::eigenschaften_aktiv(&lage), egui::Button::new("Eigenschaften")).clicked() {
                        match ziel {
                            MenueZiel::Leer => {
                                if let Some(r) = self.cur() {
                                    self.open_sheet_props(r);
                                }
                            }
                            MenueZiel::Symboltext { id, wert } => self.symboltext_eigenschaften(id, wert),
                            z => {
                                if let Some(id) = z.id() {
                                    self.props_command(id);
                                }
                            }
                        }
                        ui.close();
                    }
                }
                Eintrag::ZielSetzen => {
                    if ui.button("Ziel setzen").clicked() {
                        if let Some(id) = ziel.id() {
                            self.open_props3(id);
                        }
                        ui.close();
                    }
                }
                Eintrag::BezeichnerSetzen => {
                    // wie im Original: danach den Text anklicken, der den Anschluss bezeichnet (bezeichner.rs)
                    if ui.button("Bezeichner setzen").clicked() {
                        if let MenueZiel::Anschluss(id) = ziel {
                            self.bezeichner_start(id);
                        }
                        ui.close();
                    }
                }
                Eintrag::Fixieren => {
                    let fest = obj.as_ref().is_some_and(|o| o.fixed);
                    ui.menu_button("Fixieren", |ui| {
                        for (n, v) in [("Ja", true), ("Nein", false)] {
                            if ui.radio(fest == v, n).clicked() {
                                self.set_fixed(v);
                                ui.close();
                            }
                        }
                    });
                }
                Eintrag::Bauteilliste => self.menu_bauteilliste(ui, ziel),
                Eintrag::Pfadbezeichnung => {
                    if ui.button("Pfadbezeichnung").clicked() {
                        self.open_columns_dialog();
                        ui.close();
                    }
                }
                e => self.menu_symbol(ui, e, obj.as_ref()),
            }
        }
    }

    /// Untermenü Ausrichtung für Symboltexte (Titel/Wert) und Texte, mit Häkchen wie im Original.
    fn menu_ausrichtung(&mut self, ui: &mut egui::Ui, ziel: MenueZiel, obj: Option<&Obj>) {
        let wert = match ziel {
            MenueZiel::Symboltext { wert, .. } => Some(wert),
            _ => None,
        };
        let haken = obj.map(|o| kontextmenue::ausrichtung_haken(&o.kind, wert)).unwrap_or_default();
        ui.menu_button("Ausrichtung", |ui| {
            let mut wahl: Option<(Option<u8>, Option<bool>)> = None;
            for (i, n) in ["Links", "Mittig", "Rechts"].iter().enumerate() {
                if ui.radio(haken[i], *n).clicked() {
                    wahl = Some((Some(i as u8), None));
                }
            }
            ui.separator();
            if ui.radio(haken[3], "Hochkant").clicked() {
                wahl = Some((None, Some(true)));
            }
            if ui.radio(haken[4], "Waagerecht").clicked() {
                wahl = Some((None, Some(false)));
            }
            let Some((al, hoch)) = wahl else { return };
            match ziel {
                MenueZiel::Symboltext { id, wert } => self.symboltext_ausrichten(id, wert, al, hoch),
                _ => self.set_text_align(al.map(|a| [Align::Left, Align::Center, Align::Right][a as usize]), hoch),
            }
            ui.close();
        });
    }

    /// Bauteilliste › Bearbeiten / Loeschen an Symbol oder Klemmstelle. „Loeschen“ entfernt wie im Original
    /// (Loeschen1Click 0x49d81c) die Zuordnung aller markierten Symbole und Klemmstellen (ohne Markierung die des
    /// angeklickten Objekts).
    fn menu_bauteilliste(&mut self, ui: &mut egui::Ui, ziel: MenueZiel) {
        let Some(id) = ziel.id() else { return };
        ui.menu_button("Bauteilliste", |ui| {
            if ui.button("Bearbeiten").clicked() {
                self.bauteilliste_oeffnen(id);
                ui.close();
            }
            if ui.button("Loeschen").clicked() {
                let mut ids: Vec<u64> = self.selected_objs().iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })).map(|o| o.id).collect();
                if ids.is_empty() {
                    ids.push(id);
                }
                if ids.iter().any(|i| self.project.part_of.contains_key(i)) {
                    self.checkpoint();
                    ids.iter().for_each(|i| {
                        self.project.part_of.remove(i);
                    });
                    self.dirty = true;
                }
                ui.close();
            }
        });
    }

    /// Einträge für Symbole: Bildindex, Optionen, Parameter, Alternativtext, Eigentümer, Kontaktspiegel,
    /// Symbol bearbeiten.
    fn menu_symbol(&mut self, ui: &mut egui::Ui, e: Eintrag, obj: Option<&Obj>) {
        let Some(so) = obj else { return };
        let Kind::Symbol { key, option, flags, ext, .. } = &so.kind else { return };
        let def = symbols::find(&self.lib, key).cloned();
        let id = so.id;
        match e {
            // Optionen: Zusatzoptionen zum An-/Abhaken (OPT-Blöcke)
            Eintrag::Optionen => {
                let names = def.as_ref().map(|d| d.flag_names.clone()).unwrap_or_default();
                let flags = *flags;
                ui.menu_button("Optionen", |ui| {
                    for (i, n) in names.iter().enumerate() {
                        if n.is_empty() {
                            continue;
                        }
                        let mut on = flags & (1 << i) != 0;
                        if ui.checkbox(&mut on, n.as_str()).clicked() {
                            self.change_symbol(id, |k| {
                                if let Kind::Symbol { flags, .. } = k {
                                    *flags ^= 1 << i;
                                }
                            });
                            ui.close();
                        }
                    }
                });
            }
            // Bildindex: Bildvarianten (SEI)
            Eintrag::Bildindex => {
                let bilder = def.as_ref().map(|d| d.options.clone()).unwrap_or_default();
                let option = *option;
                ui.menu_button("Bildindex", |ui| {
                    for (i, n) in bilder.iter().enumerate() {
                        if ui.radio(option as usize == i, n.as_str()).clicked() {
                            self.change_symbol(id, |k| {
                                if let Kind::Symbol { option, ext, .. } = k {
                                    *option = i as u16;
                                    ext.bounds = None;
                                }
                            });
                            ui.close();
                        }
                    }
                });
            }
            // Parameter: Werte der Symbolparameter (Tabelle wie TTextAbfrage, parameter.rs)
            Eintrag::Parameter => {
                if ui.button("Parameter").clicked() {
                    self.open_param_dialog(id, 2);
                    ui.close();
                }
            }
            // Alternativtext: Standard, die Sätze des Symbols, Benutzerdefiniert
            Eintrag::Alternativtext => {
                let sets = def.as_ref().map(|d| d.alt_sets.clone()).unwrap_or_default();
                let cur = if ext.texts.is_empty() { ext.alt } else { u16::MAX };
                ui.menu_button("Alternativtext", |ui| {
                    let mut items = vec!["Standard".to_string()];
                    items.extend(sets.iter().map(|s| symbols::alt_label(s)));
                    for (i, n) in items.iter().enumerate() {
                        if ui.radio(cur as usize == i, n.as_str()).clicked() {
                            self.change_symbol(id, |k| {
                                if let Kind::Symbol { ext, .. } = k {
                                    ext.alt = i as u16;
                                    // ein Satz ersetzt die benutzerdefinierten Texte
                                    ext.texts.clear();
                                }
                            });
                            ui.close();
                        }
                    }
                    if ui.radio(cur as usize >= items.len(), "Benutzerdefiniert").clicked() {
                        self.open_param_dialog(id, 1);
                        ui.close();
                    }
                });
            }
            // Eigentümer: Suchen (zum Eigentümer) nur aktiv, wenn einer gesetzt ist – wie im Original
            Eintrag::Eigentuemer => {
                let hat = ext.eigentuemer != 0;
                ui.menu_button("Eigentümer", |ui| {
                    if ui.button("Eigenschaften").clicked() {
                        self.edit_props(id);
                        ui.close();
                    }
                    let any_sym = self.selected_objs().iter().any(|o| matches!(o.kind, Kind::Symbol { .. }));
                    if ui.add_enabled(any_sym, egui::Button::new("Setzen")).clicked() {
                        self.start_owner_pick();
                        ui.close();
                    }
                    if ui.add_enabled(hat, egui::Button::new("Suchen")).clicked() {
                        self.eigentuemer_suchen(id);
                        ui.close();
                    }
                });
            }
            Eintrag::Kontaktspiegel => {
                if ui.button("Kontaktspiegel hinzufügen").clicked() {
                    self.open_spiegel();
                    ui.close();
                }
            }
            Eintrag::SymbolBearbeiten => {
                if ui.button("Symbol bearbeiten").clicked() {
                    self.open_symbol_editor_fuer(id);
                    ui.close();
                }
            }
            _ => {}
        }
    }
}

impl App {
    /// Werkzeugfenster "Ausrichtung" (Formular TAusrichtung: Referenzpunkt, Horizontal, Vertikal,
    /// Abstand Horizontal, Abstand Vertikal; bleibt im Vordergrund wie fsStayOnTop).
    pub fn align_window(&mut self, ctx: &egui::Context) {
        if !self.align_win {
            return;
        }
        let objs = self.selected_objs();
        // Referenzpunkt wie im Original (TAuswahl "Referenz"): fester Bezug innerhalb der Auswahl, kein Einzelobjekt –
        // am Original beobachtet: "Links/Oben" + Horizontal setzt alle Objekte auf die oberste Kante der Auswahl
        let names: Vec<String> = REFERENZPUNKTE.iter().map(|s| s.to_string()).collect();
        let mut open = true;
        let mut action: Option<u8> = None;
        egui::Window::new("Ausrichtung")
            .id(egui::Id::unique("ausrichtung"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([164.0, 156.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Referenzpunkt");
                let cur = names.get(self.align_ref).cloned().unwrap_or_default();
                egui::ComboBox::from_id_salt("refp").width(158.0).selected_text(cur).show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut self.align_ref, i, n);
                    }
                });
                ui.add_space(6.0);
                let enabled = objs.len() >= 2;
                // vier Knöpfe wie im Original (TAusrichtung), mit eigenen Vektor-Icons
                use crate::icons::Icon;
                let glyphs: [(u8, &str, Icon); 4] = [
                    (0, "Horizontal", Icon::AlignHor),
                    (2, "Abstand Horizontal", Icon::SpaceHor),
                    (1, "Vertikal", Icon::AlignVer),
                    (3, "Abstand Vertikal", Icon::SpaceVer),
                ];
                for row in glyphs.chunks(2) {
                    ui.horizontal(|ui| {
                        for (code, hint, icon) in row {
                            let btn = egui::Button::image(crate::icons::image(*icon, 29.0));
                            if ui.add_enabled(enabled, btn.min_size(egui::Vec2::splat(37.0))).on_hover_text(*hint).clicked() {
                                action = Some(*code);
                            }
                            ui.add_space(12.0);
                        }
                    });
                }
            });
        if !open {
            self.align_win = false;
        }
        if let Some(a) = action {
            self.align_to_reference(a);
        }
    }

    /// Markierte Objekte am Referenzobjekt ausrichten bzw. gleichmäßig verteilen (Umrissrechtecke wie im Original).
    pub fn align_to_reference(&mut self, mode: u8) {
        let objs = self.selected_objs();
        if objs.len() < 2 {
            return;
        }
        let lib = self.lib.clone();
        let bounds: Vec<(u64, P, P)> = objs.iter().map(|o| {
            let (a, b) = crate::canvas::obj_bounds(o, &lib);
            (o.id, a, b)
        }).collect();
        let moves = ausrichten(&bounds, mode, self.align_ref);
        self.checkpoint();
        self.with_sheet(|s, _| {
            for (id, d) in moves {
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == id && !o.fixed) {
                    o.kind.translate(d);
                }
            }
        });
        self.dirty = true;
    }
}

/// Einträge der Liste "Referenzpunkt" im Werkzeugfenster Ausrichtung (wie im Original).
pub const REFERENZPUNKTE: [&str; 3] = ["Mitte", "Links/Oben", "Rechts/Unten"];

/// Verschiebungen für Ausrichten (0 Horizontal = gleiche Höhe, 1 Vertikal = gleiche x-Lage) bzw. gleichmäßiges
/// Verteilen (2 Abstand horizontal, 3 Abstand vertikal). `referenz`: 0 Mitte, 1 Links/Oben, 2 Rechts/Unten –
/// Bezug ist die gemeinsame Umrandung der Auswahl bzw. beim Verteilen die jeweilige Kante/Mitte jedes Objekts.
pub fn ausrichten(bounds: &[(u64, P, P)], mode: u8, referenz: usize) -> Vec<(u64, P)> {
    if bounds.len() < 2 {
        return vec![];
    }
    let horiz = mode == 0 || mode == 3; // Lage entlang y
    let k = |a: &P, b: &P| -> f32 {
        let (lo, hi) = if horiz { (a.y, b.y) } else { (a.x, b.x) };
        match referenz {
            1 => lo,
            2 => hi,
            _ => (lo + hi) / 2.0,
        }
    };
    let verschiebung = |d: f32| if horiz { P::new(0.0, d) } else { P::new(d, 0.0) };
    match mode {
        0 | 1 => {
            let lo = bounds.iter().map(|(_, a, _)| if horiz { a.y } else { a.x }).fold(f32::MAX, f32::min);
            let hi = bounds.iter().map(|(_, _, b)| if horiz { b.y } else { b.x }).fold(f32::MIN, f32::max);
            let ziel = match referenz {
                1 => lo,
                2 => hi,
                _ => (lo + hi) / 2.0,
            };
            bounds.iter().map(|(id, a, b)| (*id, verschiebung(ziel - k(a, b)))).collect()
        }
        _ => {
            let mut v: Vec<&(u64, P, P)> = bounds.iter().collect();
            v.sort_by(|x, y| k(&x.1, &x.2).partial_cmp(&k(&y.1, &y.2)).unwrap_or(std::cmp::Ordering::Equal));
            let first = k(&v[0].1, &v[0].2);
            let last = k(&v[v.len() - 1].1, &v[v.len() - 1].2);
            let step = (last - first) / (v.len() - 1) as f32;
            v.iter().enumerate().map(|(i, (id, a, b))| (*id, verschiebung(first + step * i as f32 - k(a, b)))).collect()
        }
    }
}

#[cfg(test)]
mod ausrichten_tests {
    use super::*;

    #[test]
    fn links_oben_setzt_auf_oberste_kante() {
        let b = [(1, P::new(0.0, 10.0), P::new(4.0, 14.0)), (2, P::new(20.0, 30.0), P::new(24.0, 34.0))];
        let m = ausrichten(&b, 0, 1);
        assert_eq!(m[0].1, P::new(0.0, 0.0));
        assert_eq!(m[1].1, P::new(0.0, -20.0));
        let m = ausrichten(&b, 1, 2);
        assert_eq!(m[0].1, P::new(20.0, 0.0));
        assert_eq!(m[1].1, P::new(0.0, 0.0));
    }

    #[test]
    fn verteilen_gleichmaessig() {
        let b = [(1, P::new(0.0, 0.0), P::new(2.0, 2.0)), (2, P::new(3.0, 0.0), P::new(5.0, 2.0)), (3, P::new(20.0, 0.0), P::new(22.0, 2.0))];
        let m = ausrichten(&b, 2, 0);
        assert_eq!(m[1].1, P::new(7.0, 0.0));
    }
}
