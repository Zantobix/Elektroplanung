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
