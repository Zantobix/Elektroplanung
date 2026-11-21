//! Weitere berechnete Blätter: Artikelliste (Blattart 6) und Inhaltsverzeichnis (Blattart 1).
//!
//! Belegt aus dem Original: die Blattarten und ihre Zusätze (Summenliste, Summe+BMK, Einzelliste) sowie die
//! Spaltenköpfe der Artikelliste („Betriebsmittelkennzeichen“, „Menge“, „Bezeichnung“, „Typnummer“, „Hersteller“,
//! „Bestellnummer“ – Texte der Exe) und die Regel „zusammenhängende Blätter ergeben eine zusammenhängende Liste“
//! (Anleitung 4.2). Nicht belegt (kein Ausdruck vorhanden): Maße der Tabellen – sie folgen der
//! Sicherungsübersicht (Zeilen 4,95 mm) –, die Spalten des Inhaltsverzeichnisses (hier Blatt, Bezeichnung, Datum)
//! und der Zusatz je Blatt (die Artikelliste zeigt die Summenliste).

use crate::canvas::{draw_grid, natural_key, times, DrawCtx};
use crate::model::*;
use eframe::egui::{Align2, Color32, Stroke};

/// Artikelliste als Summenliste: je Bauteil Menge und Daten (Bauteile aller Symbole der Anlage).
pub fn artikel_rows(p: &Project, a: &Anlage) -> Vec<Vec<String>> {
    let mut n: std::collections::BTreeMap<usize, usize> = Default::default();
    for o in a.sheets.iter().flat_map(|s| &s.objects) {
        if let Some(i) = p.part_of.get(&o.id) {
            *n.entry(*i).or_default() += 1;
        }
    }
    let mut rows: Vec<Vec<String>> = n.into_iter().filter_map(|(i, c)| p.bauteile.get(i).map(|b| vec![c.to_string(), b.name.clone(), b.typ.clone(), b.firma.clone(), b.artikel.clone()])).collect();
    rows.sort_by_key(|r| natural_key(&r[1]));
    rows
}

/// Inhaltsverzeichnis: alle Blätter der Anlage (Nummer mit Subindex, Bezeichnung, Datum der letzten Änderung).
pub fn inhalt_rows(a: &Anlage) -> Vec<Vec<String>> {
    a.sheets
        .iter()
        .map(|s| {
            let nr = if s.subindex > 0 { format!("{}.{}", s.number, s.subindex) } else { s.number.to_string() };
            let datum = if s.geaendert.is_empty() || s.geaendert.starts_with('-') { s.erstellt.clone() } else { s.geaendert.clone() };
            vec![nr, s.name.clone(), datum]
        })
        .collect()
}

/// Berechnetes Blatt zeichnen; false, wenn die Blattart hier nicht behandelt wird.
pub fn draw(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage) -> bool {
    let (head, widths, rows): (Vec<&str>, Vec<f32>, Vec<Vec<String>>) = match sheet.art {
        6 => (vec!["Menge", "Bezeichnung", "Typnummer", "Hersteller", "Bestellnummer"], vec![15.0, 70.0, 50.0, 45.0], artikel_rows(dc.project, anlage)),
        1 => (vec!["Blatt", "Bezeichnung", "Datum"], vec![20.0, 150.0], inhalt_rows(anlage)),
        _ => return false,
    };
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (fx0, fy0, fx1, fy1) = sheet.frame();
    let stroke = Stroke::new((0.18 * z).max(1.0), Color32::BLACK);
    let (x0, x1) = (fx0 + 4.5, fx1 - 7.5);
    let (y0, y1) = (fy0 + 9.5, fy1 - dc.project.title_h() - 3.0);
    let rh = 4.95;
    let n = ((y1 - y0) / rh).floor().max(2.0) as usize;
    let mut w = widths;
    w.push((x1 - x0 - w.iter().sum::<f32>()).max(10.0));
    draw_grid(p, xf, P::new(x0, y0), P::new(x0 + w.iter().sum::<f32>(), y0 + n as f32 * rh), &w, &vec![rh; n], stroke);
    let off = crate::klemmplan::folge_index(anlage, sheet) * n.saturating_sub(1);
    let all: Vec<Vec<String>> = std::iter::once(head.iter().map(|h| h.to_string()).collect()).chain(rows.into_iter().skip(off)).collect();
    for (r, row) in all.iter().take(n).enumerate() {
        let mut x = x0;
        for (c, cw) in w.iter().enumerate() {
            if let Some(t) = row.get(c).filter(|t| !t.is_empty()) {
                p.text(xf.s(P::new(x + 0.8, y0 + (r as f32 + 0.5) * rh)), Align2::LEFT_CENTER, t, times(2.6 * z), Color32::BLACK);
            }
            x += cw;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_are_summed() {
        let mut p = Project::new_default();
        p.bauteile.push(Bauteil { name: "LS B16".into(), typ: "S201".into(), firma: "ABB".into(), artikel: "123".into(), ..Default::default() });
        for id in [5, 6] {
            p.anlagen[0].sheets[0].objects.push(Obj { id, kind: Kind::Line { a: P::default(), b: P::new(1.0, 1.0) }, pen: 0, fixed: false, group: None, raw: None });
            p.part_of.insert(id, 0);
        }
        assert_eq!(artikel_rows(&p, &p.anlagen[0]), vec![vec!["2", "LS B16", "S201", "ABB", "123"]]);
        assert_eq!(inhalt_rows(&p.anlagen[0])[0][..2], ["1".to_string(), "Blatt 1".to_string()]);
    }
}
