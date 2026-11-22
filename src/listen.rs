//! Listen Export (TListExport des Originals): Auswahl der Liste, Vorschau als Tabelle, Export als Textdatei.
//! Listen wie im Original: „Stückliste“, „Betriebsmittelkennzeichnung“, „Klemmenbezeichnung“; Dateiformate
//! „Tabulator“, „Trennzeichen(;)“, „Trennzeichen(#)“ (*.txt).
//! Die Spalten der Stückliste sind die des Originals (0x49b014); die der beiden anderen
//! Listen sind nicht belegt und so gewählt, dass sie zum Beschriften taugen: je Zeile ein Betriebsmittel bzw. eine
//! Klemme.

use crate::canvas::{bmk_ctx, natural_key};
use crate::model::*;
use eframe::egui::{self, RichText};

pub const LISTEN: &[&str] = &["Stückliste", "Betriebsmittelkennzeichnung", "Klemmenbezeichnung"];
pub const FORMATE: &[&str] = &["Tabulator", "Trennzeichen(;)", "Trennzeichen(#)"];
/// Spalten der Stückliste im Listen Export (0x49b014, Kopfzeile).
pub const STUECKLISTE: [&str; 9] = ["Betriebsmittel", "Anzahl", "Typ", "Interne Nr", "Bestellnummer", "Firma", "Breite", "Höhe", "Tiefe"];

/// Kopfzeile und Zeilen einer Liste.
pub fn liste(p: &Project, art: usize) -> (Vec<&'static str>, Vec<Vec<String>>) {
    match art {
        // Summenliste der zugeordneten Bauteile über alle Anlagen mit den Spalten des Originals (0x49b014 mit Art 1,
        // Aufruf aus TListExport.Auswahl1Change 0x4b9d38): Name des Bauteils, Anzahl der Verwendungen, Typ,
        // interne Nummer, Bestellnummer, Firma, Breite, Höhe, Tiefe; sortiert nach dem Namen
        0 => {
            let mut n: std::collections::BTreeMap<usize, usize> = Default::default();
            for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
                for i in crate::bauteilliste::liste(p, o.id) {
                    *n.entry(i).or_default() += 1;
                }
            }
            let mut rows: Vec<Vec<String>> = n
                .into_iter()
                .filter_map(|(i, c)| p.bauteile.get(i).map(|b| vec![b.name.clone(), c.to_string(), b.typ.clone(), b.nummer.clone(), b.artikel.clone(), b.firma.clone(), b.breite.clone(), b.hoehe.clone(), b.tiefe.clone()]))
                .collect();
            rows.sort_by_key(|r| natural_key(&r[0]));
            (STUECKLISTE.to_vec(), rows)
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

    /// Stückliste mit den Spalten des Originals: je Bauteil eine Zeile, Anzahl = Verwendungen über alle Anlagen.
    #[test]
    fn stueckliste_spalten_wie_original() {
        let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
        let mut p = crate::epr::import(std::path::Path::new(&f)).unwrap();
        p.bauteile = vec![
            Bauteil { name: "Schütz".into(), typ: "DILM12".into(), nummer: "7".into(), artikel: "276690".into(), firma: "Eaton".into(), breite: "45".into(), hoehe: "75".into(), tiefe: "90".into(), ..Default::default() },
            Bauteil { name: "Klemme".into(), typ: "UK5".into(), ..Default::default() },
        ];
        let ids: Vec<u64> = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| o.id).take(3).collect();
        for id in &ids[..2] {
            crate::bauteilliste::setzen(&mut p, *id, &[0]);
        }
        crate::bauteilliste::setzen(&mut p, ids[2], &[0, 1]);
        let (h, rows) = liste(&p, 0);
        assert_eq!(h, STUECKLISTE.to_vec());
        assert_eq!(rows, vec![vec!["Klemme", "1", "UK5", "", "", "", "", "", ""], vec!["Schütz", "3", "DILM12", "7", "276690", "Eaton", "45", "75", "90"]]);
    }
}
