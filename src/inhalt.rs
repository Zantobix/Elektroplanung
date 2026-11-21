//! Tabelle › Daten „Inhaltsverzeichnis“ (TSTab, Tab_ArtChange): Vorschau im Dialog mit den Blättern der Anlage –
//! Spalten „Seite“, „Bezeichnung“ wie im Original (Stringtabelle 4000/4001) –, beginnend bei Blattnummer „Ab“.
//! Auf dem Blatt rechnet `berechnet::tabelle` den Inhalt bei jedem Zeichnen neu.

use crate::model::*;

pub const KOPF: [&str; 2] = crate::berechnet::KOPF_INHALT;

/// Alle Blätter der Anlage als Zeilen (Nummer, Zellen), nach Nummer und Subindex sortiert.
pub fn blaetter(p: &Project, anlage: usize) -> Vec<(u32, Vec<String>)> {
    p.anlagen.get(anlage).map(crate::berechnet::inhalt).unwrap_or_default()
}

/// Zellen der Tabelle: auf Wunsch eine Überschriftzeile, dann die Blätter ab Nummer `ab`.
pub fn tabelle(rows: &[(u32, Vec<String>)], ab: u32, header: bool) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = vec![];
    if header {
        out.push(KOPF.iter().map(|s| s.to_string()).collect());
    }
    out.extend(rows.iter().filter(|(n, _)| *n >= ab).map(|(_, r)| r.clone()));
    if out.is_empty() {
        out.push(vec![String::new(); KOPF.len()]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verzeichnis_ab_nummer() {
        let mut p = Project::new_default();
        let mut s = Sheet::new(3);
        s.name = "Küche".into();
        s.subindex = 1;
        p.anlagen[0].sheets.push(s);
        p.anlagen[0].sheets.push(Sheet::new(2));
        let rows = blaetter(&p, 0);
        assert_eq!(rows.iter().map(|r| r.1[0].clone()).collect::<Vec<_>>(), ["1", "2", "3.1"]);
        let t = tabelle(&rows, 2, true);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0][0], "Seite");
        assert_eq!(t[2][1], "Küche");
    }
}
