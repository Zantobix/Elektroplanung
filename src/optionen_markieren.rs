//! Optionen › Markieren (todo NR13, Nutzer 25.11.2026): alle Einstellungen zum Markieren an einer Stelle. Was eine
//! andere Einstellung wirkungslos macht, ist ausgegraut und nennt den Grund (Nutzerentscheidung „widersprechende
//! Einstellungen ausgrauen“). Die Taste zum Erweitern der Markierung steht unter Tastenkürzel (`dazu_zeile`).

use crate::auswahl::{Art, Filter};
use crate::optionen::Optionen;
use crate::props::bold;
use eframe::egui::{self, Color32, RichText};

fn mark(ui: &mut egui::Ui, v: &mut bool, t: &str) -> egui::Response {
    ui.checkbox(v, RichText::new(t).font(bold(13.0)))
}

/// Stufen des Mehrfachklicks für die Auswahlliste.
pub const STUFEN: &[(u8, &str)] = &[(1, "nur Teilstück"), (2, "bis zum Knoten"), (3, "Netz ohne Sammelschiene"), (4, "ganzes Potential mit Schiene")];

/// Grund, warum der Mehrfachklick auf Leitungen gerade nichts bewirkt (None = wirksam).
pub fn mehrfachklick_gesperrt(filter: &Filter) -> Option<&'static str> {
    filter.aus.contains(&Art::Leitung).then_some("Der Auswahlfilter dieser Sitzung nimmt Leitungen aus (Bearbeiten › Auswahl › Filter).")
}

pub fn reiter(ui: &mut egui::Ui, o: &mut Optionen, filter: &Filter) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(570.0);
        ui.label("Klicken und Rahmen");
        mark(ui, &mut o.markieren_cad, "Markierrichtung wie CAD (→ eingeschlossen, ← berührt; aus = Bezugspunkt im Rahmen wie im Original)");
        mark(ui, &mut o.vormarkierung, "Vormarkierung: Objekt unter der Maus hervorheben");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Markierfarbe").font(bold(13.0)));
            let mut c = Color32::from_rgb(o.markier_farbe[0], o.markier_farbe[1], o.markier_farbe[2]);
            if ui.color_edit_button_srgba(&mut c).changed() {
                o.markier_farbe = [c.r(), c.g(), c.b()];
            }
            if ui.small_button("Standard").clicked() {
                o.markier_farbe = [220, 0, 0];
            }
        });
        ui.label(RichText::new("Fixierte Objekte sind nicht markierbar, solange „Fixierung aktivieren“ eingeschaltet ist (Werkzeugleiste, Starteinstellung unter Programm).").small());
    });
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(570.0);
        ui.label("Mehrfachklick auf Leitungen");
        let gesperrt = mehrfachklick_gesperrt(filter);
        ui.add_enabled_ui(gesperrt.is_none(), |ui| {
            let r = mark(ui, &mut o.mehrfachklick_leitung, "Weitere schnelle Klicks erweitern die Markierung stufenweise (aus = Doppelklick öffnet die Eigenschaften wie im Original)");
            if let Some(g) = gesperrt {
                r.on_disabled_hover_text(g);
            }
            ui.add_enabled_ui(o.mehrfachklick_leitung, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("bis Stufe").font(bold(13.0)));
                    let cur = STUFEN.iter().find(|s| s.0 == o.mehrfachklick_stufen).map(|s| format!("{} – {}", s.0, s.1)).unwrap_or_default();
                    egui::ComboBox::from_id_salt("mkstufen").width(260.0).selected_text(cur).show_ui(ui, |ui| {
                        for (n, t) in STUFEN {
                            ui.selectable_value(&mut o.mehrfachklick_stufen, *n, format!("{n} – {t}"));
                        }
                    });
                    ui.label("danach wieder Stufe 1");
                })
                .response
                .on_disabled_hover_text("Nur mit eingeschaltetem Mehrfachklick.");
            });
        });
        if let Some(g) = gesperrt {
            ui.colored_label(Color32::from_rgb(150, 90, 0), g);
        }
    });
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(570.0);
        ui.label("Löschen");
        mark(ui, &mut o.loeschen_mit_leitungen, "Symbol löschen: angeschlossene Leitungen mitlöschen (wie im Original)");
    });
    if filter.aktiv() {
        ui.label(RichText::new(filter.hinweis()).color(Color32::from_rgb(150, 90, 0)));
    }
    ui.label(RichText::new("Auswahlbefehle (alle Leitungen, Symbole …, Gleiche wie markiert, Auswahl umkehren) und der Auswahlfilter: Bearbeiten › Auswahl und Kontextmenü der freien Fläche; Tastenkürzel unter „Tastenkürzel“.").small());
}

/// Zeile im Reiter Tastenkürzel: Taste zum Hinzunehmen/Herausnehmen beim Markieren (Strg oder Umschalt).
pub fn dazu_zeile(ui: &mut egui::Ui, o: &mut Optionen) {
    ui.label("Markierung erweitern (Taste beim Klicken)");
    let umschalt = crate::kuerzel::auswahl_dazu_umschalt(&o.tasten);
    let mut wahl = umschalt;
    egui::ComboBox::from_id_salt("auswdazu").width(110.0).selected_text(if umschalt { "Umschalt" } else { "Strg" }).show_ui(ui, |ui| {
        ui.selectable_value(&mut wahl, false, "Strg");
        ui.selectable_value(&mut wahl, true, "Umschalt");
    });
    if wahl != umschalt {
        if wahl {
            o.tasten.insert(crate::kuerzel::AUSWAHL_DAZU.into(), "Umschalt".into());
        } else {
            o.tasten.remove(crate::kuerzel::AUSWAHL_DAZU);
        }
    }
    if wahl {
        ui.label(RichText::new("Abzweig auf Leitung dann mit Strg+Umschalt").small()).on_hover_text("Umschalt beginnt sonst auf einer Leitung einen Abzweig; mit Umschalt als Erweiterungstaste übernimmt das Strg+Umschalt. Umschalt beim Ziehen schaltet weiterhin den Rasterfang ab.");
    } else {
        ui.label("");
    }
    ui.label("");
    ui.end_row();
}
