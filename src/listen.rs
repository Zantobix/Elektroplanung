//! Listen Export (TListExport des Originals): Auswahl der Liste, Vorschau als Tabelle, Export als Textdatei.
//! Listen wie im Original: „Stückliste“, „Betriebsmittelkennzeichnung“, „Klemmenbezeichnung“; Dateiformate
//! „Tabulator“, „Trennzeichen(;)“, „Trennzeichen(#)“ (*.txt).
//! Die Spalten der Stückliste sind die der Artikelliste des Originals (Texte der Exe); die der beiden anderen
//! Listen sind nicht belegt und so gewählt, dass sie zum Beschriften taugen: je Zeile ein Betriebsmittel bzw. eine
//! Klemme.

use crate::canvas::{bmk_ctx, natural_key};
use crate::model::*;
use eframe::egui::{self, RichText};

pub const LISTEN: &[&str] = &["Stückliste", "Betriebsmittelkennzeichnung", "Klemmenbezeichnung"];
pub const FORMATE: &[&str] = &["Tabulator", "Trennzeichen(;)", "Trennzeichen(#)"];

/// Kopfzeile und Zeilen einer Liste.
pub fn liste(p: &Project, art: usize) -> (Vec<&'static str>, Vec<Vec<String>>) {
    match art {
        // Summenliste der zugeordneten Bauteile (Spalten wie die Artikelliste des Originals)
        0 => {
            let mut rows: Vec<Vec<String>> = vec![];
            for a in &p.anlagen {
                for r in crate::berechnet::artikel_rows(p, a) {
                    match rows.iter_mut().find(|x| x[1..] == r[1..]) {
                        Some(x) => x[0] = (x[0].parse::<usize>().unwrap_or(0) + r[0].parse::<usize>().unwrap_or(0)).to_string(),
                        None => rows.push(r),
                    }
                }
            }
            (vec!["Menge", "Bezeichnung", "Typnummer", "Hersteller", "Bestellnummer"], rows)
        }
        // je Betriebsmittel (aufgelöste BMK) eine Zeile, erste Fundstelle
        1 => {
            let mut rows: Vec<Vec<String>> = vec![];
            for a in &p.anlagen {
                for s in &a.sheets {
                    let ctx = bmk_ctx(a, s);
                    for o in &s.objects {
                        if let Kind::Symbol { value, ext, pos, .. } = &o.kind {
                            // angezeigte BMK, bei Symbolen mit Eigentümer dessen Kennzeichnung
                            let b = crate::eigentuemer::anzeige_in(&p.anlagen, o, &ctx);
                            if b.is_empty() || rows.iter().any(|r| r[0] == a.name && r[1] == b) {
                                continue;
                            }
                            rows.push(vec![a.name.clone(), b, crate::klemmplan::pfad(s, pos.x), value.clone(), ext.funktion.clone()]);
                        }
                    }
                }
            }
            rows.sort_by(|x, y| x[0].cmp(&y[0]).then_with(|| natural_key(&x[1]).cmp(&natural_key(&y[1]))));
            (vec!["Anlage", "Kennzeichnung", "Pfad", "Wert", "Funktion"], rows)
        }
        // je Klemme (Leiste, Nummer, Etage) eine Zeile in der Reihenfolge des Klemmplans
        _ => {
            let mut rows: Vec<Vec<String>> = vec![];
            for a in &p.anlagen {
                let mut leisten: Vec<String> = a.klemmleisten.clone();
                for k in &a.klemmen {
                    if !leisten.contains(&k.name) {
                        leisten.push(k.name.clone());
                    }
                }
                for l in leisten {
                    for z in crate::klemmplan::plan_projekt(p, a, &l, &p.symdefs).zeilen {
                        rows.push(vec![a.name.clone(), l.clone(), z.label, z.funktion]);
                    }
                }
            }
            (vec!["Anlage", "Klemmleiste", "Klemme", "Funktion"], rows)
        }
    }
}

/// Liste als Text: Spalten durch Tabulator, ';' oder '#' getrennt, Zeilenende CRLF wie unter Windows.
pub fn text(head: &[&str], rows: &[Vec<String>], format: usize) -> String {
    let sep = match format {
        1 => ";",
        2 => "#",
        _ => "\t",
    };
    let clean = |s: &str| s.replace(['\r', '\n'], " ").replace(sep, " ");
    let mut out = head.join(sep);
    out.push_str("\r\n");
    for r in rows {
        out.push_str(&r.iter().map(|c| clean(c)).collect::<Vec<_>>().join(sep));
        out.push_str("\r\n");
    }
    out
}

/// Inhalt des Dialogs: Auswahl, Format, Vorschau und „Export“. Rückgabe: Meldung nach dem Export.
pub fn ui(ui: &mut egui::Ui, p: &Project, art: &mut usize, format: &mut usize) -> Option<String> {
    let mut msg = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Auswahl").strong());
        egui::ComboBox::from_id_salt("lexp_art").width(217.0).selected_text(LISTEN[(*art).min(LISTEN.len() - 1)]).show_ui(ui, |ui| {
            for (i, l) in LISTEN.iter().enumerate() {
                ui.selectable_value(art, i, *l);
            }
        });
    });
    let (head, rows) = liste(p, *art);
    egui::ScrollArea::both().id_salt("lexp_tab").max_height(400.0).show(ui, |ui| {
        egui::Grid::new("lexp_grid").striped(true).num_columns(head.len()).show(ui, |ui| {
            for h in &head {
                ui.label(RichText::new(*h).strong());
            }
            ui.end_row();
            for r in &rows {
                for c in r {
                    ui.label(c);
                }
                ui.end_row();
            }
        });
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("lexp_fmt").width(130.0).selected_text(FORMATE[(*format).min(FORMATE.len() - 1)]).show_ui(ui, |ui| {
            for (i, f) in FORMATE.iter().enumerate() {
                ui.selectable_value(format, i, *f);
            }
        });
        if ui.add_sized([100.0, 21.0], egui::Button::new("Export")).clicked() {
            let name = format!("{}.txt", LISTEN[(*art).min(LISTEN.len() - 1)]);
            if let Some(path) = rfd::FileDialog::new().add_filter(FORMATE[*format], &["txt"]).set_file_name(name).save_file() {
                let data = crate::eprfile::encode_cp1252(&text(&head, &rows, *format));
                msg = Some(match std::fs::write(&path, data) {
                    Ok(()) => format!("Liste exportiert: {}", path.display()),
                    Err(e) => format!("Fehler beim Schreiben: {e}"),
                });
            }
        }
    });
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_and_bmk_lists() {
        let mut p = Project::new_default();
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(Obj { id: 1, kind: Kind::Terminal { pos: P::new(50.0, 50.0), leiste: "X1".into(), nr: 2, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj { id: 2, kind: Kind::Terminal { pos: P::new(80.0, 50.0), leiste: "X1".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: crate::klemmen::KlemmOpt { funktion: "Licht".into(), ..Default::default() } }, pen: 0, fixed: false, group: None, raw: None });
        let (h, rows) = liste(&p, 2);
        assert_eq!(h.len(), 4);
        assert_eq!(rows, vec![vec!["Anlage 1", "X1", "X1.1", "Licht"], vec!["Anlage 1", "X1", "X1.2", ""]]);
        assert_eq!(text(&h, &rows[..1], 1), "Anlage;Klemmleiste;Klemme;Funktion\r\nAnlage 1;X1;X1.1;Licht\r\n");
    }
}
