//! Standard-Schriftfeld. Layout aus FUN_00401e40 (0x401e40) des Originals:
//! Koordinaten normiert auf 0..10000 in beiden Richtungen des Schriftfeld-Rechtecks.

use crate::model::Align;
use serde::{Deserialize, Serialize};

/// Schriftfeld-Element zum Speichern im Projekt (z. B. aus einer .epr-Datei übernommen).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TbOwned {
    Line(i32, i32, i32, i32),
    Text(i32, i32, i32, i32, String, Align, u8),
    Logo(i32, i32, i32, i32),
}

pub fn default_owned() -> Vec<TbOwned> {
    elements()
        .into_iter()
        .map(|e| match e {
            TbElem::Line(a, b, c, d) => TbOwned::Line(a, b, c, d),
            TbElem::Text(a, b, c, d, s, al, l) => TbOwned::Text(a, b, c, d, s.to_string(), al, l),
            TbElem::Logo(a, b, c, d) => TbOwned::Logo(a, b, c, d),
        })
        .collect()
}

pub enum TbElem {
    Line(i32, i32, i32, i32),
    /// x1, y1, x2, y2, Textvorlage, Ausrichtung, Zeilen
    Text(i32, i32, i32, i32, &'static str, Align, u8),
    Logo(i32, i32, i32, i32),
}

pub fn elements() -> Vec<TbElem> {
    use Align::*;
    use TbElem::*;
    vec![
        Line(0, 0, 10000, 0),
        Line(0, 3333, 2536, 3333),
        Line(0, 6666, 2536, 6666),
        Line(7292, 6000, 8567, 6000),
        Line(2536, 0, 2536, 10000),
        Line(7292, 0, 7292, 10000),
        Line(8571, 0, 8571, 10000),
        Line(9160, 2000, 9160, 10000),
        Line(1289, 0, 1289, 10000),
        Line(3386, 0, 3386, 10000),
        Line(4261, 0, 4261, 10000),
        Line(7296, 2000, 10000, 2000),
        Line(8571, 7333, 10000, 7333),
        Line(8571, 4666, 9369, 4666),
        Line(9369, 2000, 9369, 7333),
        Text(4261, 466, 7292, 4000, "%Projekt", Center, 1),
        Text(641, 0, 1289, 3333, "%Erstellt", Center, 1),
        Text(153, 0, 606, 3333, "Erstellt:", Right, 1),
        Text(641, 3333, 1289, 6666, "%Geändert", Center, 1),
        Text(0, 3333, 599, 6666, "Geändert:", Right, 1),
        Text(4261, 4000, 7292, 10000, "%Name", Center, 1),
        Text(1567, 66, 2536, 3400, "%Von", Left, 1),
        Text(9369, 2333, 10000, 7000, "%Seite", Center, 1),
        Text(9160, 2000, 9369, 4666, "%Anlage", Center, 1),
        Text(8571, 0, 10000, 2000, "Blatt", Center, 1),
        Text(8797, 4933, 9156, 6933, "%Next", Left, 1),
        Text(7296, 6333, 8564, 9666, "%Status", Center, 1),
        Text(9160, 4666, 9369, 7333, "%Ort", Left, 1),
        Text(3414, 0, 4226, 10000, "%Owner\n\n\n", Left, 4),
        Text(2554, 0, 3365, 10000, "%Kunde\n\n\n", Left, 4),
        Text(8797, 2333, 9156, 4333, "%Prev", Left, 1),
        Text(9477, 7733, 9996, 9733, "%Anzahl", Left, 1),
        Text(9163, 7733, 9466, 9733, "Anzahl:", Right, 1),
        Text(8571, 4933, 8770, 6933, "nach:", Right, 1),
        Text(8571, 2333, 8770, 4333, "vor:", Right, 1),
        Text(8797, 7666, 9156, 9666, "%Max", Left, 1),
        Text(8571, 7666, 8770, 9666, "Max:", Right, 1),
        Text(7655, 2333, 8571, 5666, "%ZNR", Left, 1),
        Text(7296, 2333, 7634, 5666, "ZNr.:", Right, 1),
        Text(7292, 0, 8567, 2000, "Stromlaufplan", Center, 1),
        Text(641, 6666, 1289, 10000, "%Datum", Center, 1),
        Text(0, 6666, 599, 10000, "Datum:", Right, 1),
        Text(1289, 66, 1560, 3400, "Von:", Right, 1),
        Text(1289, 3400, 1560, 6733, "Von:", Right, 1),
        Text(1289, 6733, 1560, 10066, "Gepr.:", Right, 1),
        Logo(3393, 133, 4257, 9933),
    ]
}

/// Werte für die Textmakros eines Blattes.
pub struct MacroValues {
    pub projekt: String,
    pub erstellt: String,
    pub geaendert: String,
    pub name: String,
    pub von: String,
    pub seite: String,
    pub anlage: String,
    pub next: String,
    pub prev: String,
    pub status: String,
    pub ort: String,
    pub owner: String,
    pub kunde: String,
    pub anzahl: String,
    pub max: String,
    pub znr: String,
    pub datum: String,
}

pub fn expand(template: &str, v: &MacroValues) -> String {
    // Längere Makros zuerst ersetzen.
    let pairs: [(&str, &str); 18] = [
        ("%Heute", &v.datum),
        ("%Projekt", &v.projekt),
        ("%Erstellt", &v.erstellt),
        ("%Geändert", &v.geaendert),
        ("%Anlage", &v.anlage),
        ("%Anzahl", &v.anzahl),
        ("%Status", &v.status),
        ("%Owner", &v.owner),
        ("%Kunde", &v.kunde),
        ("%Datum", &v.datum),
        ("%Seite", &v.seite),
        ("%Name", &v.name),
        ("%Next", &v.next),
        ("%Prev", &v.prev),
        ("%Max", &v.max),
        ("%ZNR", &v.znr),
        ("%Von", &v.von),
        ("%Ort", &v.ort),
    ];
    let mut s = template.to_string();
    for (k, val) in pairs {
        s = s.replace(k, val);
    }
    s
}

/// Breite des Schriftfelds bei Maßangaben „Absolut“ in 0,1 mm (Rahmenbreite A4 quer). Abgeleitet aus einer vom
/// Original umgeschalteten Datei: x absolut = x relativ · 2870 / 10000, y absolut = y relativ · Höhe / 10000
/// (Höhe in 0,1 mm, FUS Feld 0), jeweils gerundet.
pub const ABS_BREITE: i32 = 2870;

fn umrechnen(tb: &[TbOwned], fx: f64, fy: f64) -> Vec<TbOwned> {
    let x = |v: i32| (v as f64 * fx).round() as i32;
    let y = |v: i32| (v as f64 * fy).round() as i32;
    tb.iter()
        .map(|e| match e {
            TbOwned::Line(a, b, c, d) => TbOwned::Line(x(*a), y(*b), x(*c), y(*d)),
            TbOwned::Logo(a, b, c, d) => TbOwned::Logo(x(*a), y(*b), x(*c), y(*d)),
            TbOwned::Text(a, b, c, d, t, al, n) => TbOwned::Text(x(*a), y(*b), x(*c), y(*d), t.clone(), *al, *n),
        })
        .collect()
}

/// Relative Koordinaten (0..10000) in absolute (0,1 mm) umrechnen, wie das Original beim Umschalten auf „Absolut“.
pub fn nach_absolut(tb: &[TbOwned], hoehe: i32) -> Vec<TbOwned> {
    umrechnen(tb, ABS_BREITE as f64 / 10000.0, hoehe.max(1) as f64 / 10000.0)
}

/// Absolute Koordinaten (0,1 mm) in relative (0..10000) umrechnen. Für ganzzahlige absolute Werte verlustfrei:
/// `nach_absolut(nach_relativ(x)) == x`.
pub fn nach_relativ(tb: &[TbOwned], hoehe: i32) -> Vec<TbOwned> {
    umrechnen(tb, 10000.0 / ABS_BREITE as f64, 10000.0 / hoehe.max(1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolut_wie_im_original() {
        // Werte aus W2 (relativ) und der vom Original auf „Absolut“ umgeschalteten Datei, Höhe 150
        let rel = vec![TbOwned::Line(0, 3333, 1306, 3333), TbOwned::Line(9195, 333, 9195, 7333), TbOwned::Text(4958, 466, 7989, 4000, "%Name".into(), Align::Center, 1), TbOwned::Logo(8689, 7333, 9958, 10666)];
        let abs = nach_absolut(&rel, 150);
        assert_eq!(abs[0], TbOwned::Line(0, 50, 375, 50));
        assert_eq!(abs[1], TbOwned::Line(2639, 5, 2639, 110));
        assert_eq!(abs[2], TbOwned::Text(1423, 7, 2293, 60, "%Name".into(), Align::Center, 1));
        assert_eq!(abs[3], TbOwned::Logo(2494, 110, 2858, 160));
        // zurück und wieder hin: dieselben absoluten Werte
        for h in [150, 200, 87] {
            let a = nach_absolut(&default_owned(), h);
            assert_eq!(nach_absolut(&nach_relativ(&a, h), h), a);
        }
    }
}
