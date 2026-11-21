//! Projektexplorer: Symbole der Baumknoten (eigene SVG-Grafiken `resources/icons/tree_*.svg`, erzeugt mit
//! tools/icons_erzeugen.py) und das Suchfeld darunter (Original: Komponente TSuchfeld mit Verlaufsliste,
//! Knopf „Aa“ für Groß-/Kleinschreibung und Fernglas für „weitersuchen“).

use crate::app::TNode;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// Werte für `TNode::icon`. 0–4 behalten ihre bisherige Bedeutung (Gruppe = Buch, Favoriten = Buch mit Stern);
/// Blätter und Anlagen haben wie im Original kein Symbol.
pub mod ico {
    pub const KEINS: u8 = 0;
    pub const BUCH: u8 = 1;
    pub const FAVORITEN: u8 = 2;
    pub const BLATT: u8 = 3;
    pub const ANLAGE: u8 = 4;
    pub const POTENTIALE: u8 = 5;
    pub const LEITUNGEN: u8 = 6;
    pub const OBJEKTKATALOG: u8 = 7;
    pub const STIFTE: u8 = 8;
    pub const GRAFIKEN: u8 = 9;
    pub const BAUTEILE: u8 = 10;
    pub const PLATZHALTER: u8 = 11;
    pub const SAMMELSCHIENEN: u8 = 12;
    pub const KLEMMLEISTEN: u8 = 13;
    pub const BLAETTER: u8 = 14;
}

/// Grafik eines Baumknotens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BaumIcon {
    Potentiale,
    Leitungen,
    Objektkatalog,
    Stifte,
    Grafiken,
    Bauteile,
    Platzhalter,
    Sammelschienen,
    Klemmleisten,
    Blaetter,
    /// Gruppe der Bibliothek, zugeklappt
    Buch,
    /// Gruppe der Bibliothek, aufgeklappt (Bücherstapel)
    Buecher,
}

/// Symbol zu einem Knotenwert; `offen` = Knoten aufgeklappt (Bibliotheksgruppen wechseln dann das Bild).
/// Zweiter Wert: zusätzlich den Favoriten-Stern zeichnen.
pub fn icon_fuer(code: u8, offen: bool) -> (Option<BaumIcon>, bool) {
    use BaumIcon::*;
    let buch = if offen { Buecher } else { Buch };
    let i = match code {
        ico::BUCH | ico::FAVORITEN => Some(buch),
        ico::POTENTIALE => Some(Potentiale),
        ico::LEITUNGEN => Some(Leitungen),
        ico::OBJEKTKATALOG => Some(Objektkatalog),
        ico::STIFTE => Some(Stifte),
        ico::GRAFIKEN => Some(Grafiken),
        ico::BAUTEILE => Some(Bauteile),
        ico::PLATZHALTER => Some(Platzhalter),
        ico::SAMMELSCHIENEN => Some(Sammelschienen),
        ico::KLEMMLEISTEN => Some(Klemmleisten),
        ico::BLAETTER => Some(Blaetter),
        _ => None,
    };
    (i, code == ico::FAVORITEN)
}

/// SVG-Quelle (im Programm eingebettet).
pub fn source(i: BaumIcon) -> egui::ImageSource<'static> {
    match i {
        BaumIcon::Potentiale => egui::include_image!("../resources/icons/tree_potentiale.svg"),
        BaumIcon::Leitungen => egui::include_image!("../resources/icons/tree_leitungen.svg"),
        BaumIcon::Objektkatalog => egui::include_image!("../resources/icons/tree_objektkatalog.svg"),
        BaumIcon::Stifte => egui::include_image!("../resources/icons/tree_stifte.svg"),
        BaumIcon::Grafiken => egui::include_image!("../resources/icons/tree_grafiken.svg"),
        BaumIcon::Bauteile => egui::include_image!("../resources/icons/tree_bauteile.svg"),
        BaumIcon::Platzhalter => egui::include_image!("../resources/icons/tree_platzhalter.svg"),
        BaumIcon::Sammelschienen => egui::include_image!("../resources/icons/tree_sammelschienen.svg"),
        BaumIcon::Klemmleisten => egui::include_image!("../resources/icons/tree_klemmleisten.svg"),
        BaumIcon::Blaetter => egui::include_image!("../resources/icons/tree_blaetter.svg"),
        BaumIcon::Buch => egui::include_image!("../resources/icons/tree_buch.svg"),
        BaumIcon::Buecher => egui::include_image!("../resources/icons/tree_buecher.svg"),
    }
}

/// Größe der Baumsymbole in Pixeln (wie im Original 16 × 16).
pub const GROESSE: f32 = 16.0;

/// Symbol eines Knotens links an `x` (senkrecht mittig zu `mid`) zeichnen; liefert die belegte Breite.
pub fn zeichne(ui: &egui::Ui, code: u8, offen: bool, x: f32, mid: f32) -> f32 {
    let (icon, stern) = icon_fuer(code, offen);
    let mut w = 0.0;
    if let Some(i) = icon {
        let r = Rect::from_min_size(Pos2::new(x, (mid - GROESSE / 2.0).round()), Vec2::splat(GROESSE));
        egui::Image::new(source(i)).fit_to_exact_size(r.size()).paint_at(ui, r);
        w += GROESSE + 1.0;
    }
    if stern {
        stern_zeichnen(ui.painter(), Pos2::new(x + w + 5.5, mid), 5.5);
        w += 12.0;
    }
    w
}

/// Gelber Favoriten-Stern.
fn stern_zeichnen(p: &egui::Painter, c: Pos2, r: f32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let rr = if k % 2 == 0 { r } else { r * 0.45 };
            Pos2::new(c.x + rr * a.cos(), c.y + rr * a.sin())
        })
        .collect();
    // Sternpolygon ist nicht konvex: als Fächer aus Dreiecken um die Mitte füllen
    for k in 0..10 {
        p.add(egui::Shape::convex_polygon(vec![c, pts[k], pts[(k + 1) % 10]], Color32::from_rgb(255, 215, 0), Stroke::NONE));
    }
    let mut ring = pts.clone();
    ring.push(pts[0]);
    p.add(egui::Shape::line(ring, Stroke::new(0.8, Color32::from_rgb(120, 90, 0))));
}

/// Aufklapp-Pfeil (gefülltes Dreieck nach unten) mittig in `r` – gezeichnet, weil die Oberflächenschrift
/// Zeichen wie „▾“ nicht enthält.
pub fn pfeil_unten(p: &egui::Painter, r: Rect, farbe: Color32) {
    let c = r.center();
    p.add(egui::Shape::convex_polygon(
        vec![Pos2::new(c.x - 3.5, c.y - 1.5), Pos2::new(c.x + 3.5, c.y - 1.5), Pos2::new(c.x, c.y + 2.0)],
        farbe,
        Stroke::NONE,
    ));
}

// ------------------------------------------------------------------ Suchfeld

/// Zustand des Suchfelds (der Suchtext selbst steht in `App::search`).
#[derive(Default)]
pub struct SuchFeld {
    /// Knopf „Aa“ gedrückt: Groß-/Kleinschreibung beachten
    pub gross_klein: bool,
    /// zuletzt benutzte Suchbegriffe, neuester zuerst
    pub verlauf: Vec<String>,
    /// Knoten, zu dem der Baum beim nächsten Zeichnen rollen soll
    pub scroll_to: Option<String>,
    /// letzte Suche ohne Treffer (Feld rot hinterlegt)
    pub nicht_gefunden: bool,
}

/// Länge der Verlaufsliste (Original TSuchfeld::TextExit: bei mehr als 5 Einträgen fällt der älteste weg).
pub const VERLAUF_MAX: usize = 6;

/// Suchbegriff vorn in den Verlauf aufnehmen (doppelte Einträge werden nach vorn verschoben).
pub fn verlauf_merken(verlauf: &mut Vec<String>, text: &str) {
    let t = text.trim();
    if t.is_empty() {
        return;
    }
    verlauf.retain(|v| v != t);
    verlauf.insert(0, t.to_string());
    verlauf.truncate(VERLAUF_MAX);
}

/// Güte eines Treffers: ganzer Text gleich, Text beginnt mit dem Suchbegriff (wie TSuchfeld::Vergleich im
/// Original) oder – als Ergänzung, z. B. für Blätter mit vorangestellter Nummer – enthält ihn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Treffer {
    Enthaelt,
    Anfang,
    Gleich,
}

pub fn vergleich(label: &str, such: &str, gross_klein: bool) -> Option<Treffer> {
    if such.is_empty() {
        return None;
    }
    let (l, s) = if gross_klein { (label.to_string(), such.to_string()) } else { (label.to_lowercase(), such.to_lowercase()) };
    if l == s {
        Some(Treffer::Gleich)
    } else if l.starts_with(&s) {
        Some(Treffer::Anfang)
    } else if l.contains(&s) {
        Some(Treffer::Enthaelt)
    } else {
        None
    }
}

/// Alle Knoten in Anzeigereihenfolge (auch zugeklappte) mit dem Pfad ihrer Vorfahren.
fn flach<'a>(nodes: &'a [TNode], pfad: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'a TNode)>) {
    for n in nodes {
        out.push((pfad.clone(), n));
        pfad.push(n.id.clone());
        flach(&n.children, pfad, out);
        pfad.pop();
    }
}

/// Knoten suchen. Start beim markierten Knoten `ab` (bei `weiter` erst beim nächsten, wie Fernglas/F3 im
/// Original), am Ende geht es oben weiter. Bevorzugt wird der erste ganz gleiche Text, dann der erste, der mit
/// dem Suchbegriff beginnt, zuletzt der erste, der ihn enthält. Ergebnis: Vorfahren (zum Aufklappen) und Knoten-ID.
pub fn suchen(nodes: &[TNode], such: &str, gross_klein: bool, ab: Option<&str>, weiter: bool) -> Option<(Vec<String>, String)> {
    let mut alle = vec![];
    flach(nodes, &mut vec![], &mut alle);
    let n = alle.len();
    if n == 0 || such.is_empty() {
        return None;
    }
    let start = match ab.and_then(|id| alle.iter().position(|(_, k)| k.id == id)) {
        Some(p) if weiter => p + 1,
        Some(p) => p,
        None => 0,
    };
    let mut best: Option<(Treffer, usize)> = None;
    for k in 0..n {
        let i = (start + k) % n;
        if let Some(t) = vergleich(&alle[i].1.label, such, gross_klein) {
            if best.is_none_or(|(b, _)| t > b) {
                best = Some((t, i));
                if t == Treffer::Gleich {
                    break;
                }
            }
        }
    }
    best.map(|(_, i)| (alle[i].0.clone(), alle[i].1.id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::TAction;

    fn k(id: &str, label: &str, children: Vec<TNode>) -> TNode {
        TNode { id: id.into(), label: label.into(), bold: !children.is_empty(), icon: 0, action: TAction::None, children }
    }

    fn baum() -> Vec<TNode> {
        vec![
            k("a", "Potentiale", vec![k("a/1", "L1", vec![]), k("a/2", "N", vec![])]),
            k("b", "Unterverteilung 1", vec![k("b/bl", "Blätter", vec![k("b/bl/0", "[1] Sicherungsplan", vec![]), k("b/bl/1", "[2] Schaltplan -X1", vec![])])]),
            k("c", "Schaltplan", vec![]),
        ]
    }

    #[test]
    fn verlauf_neuester_zuerst_ohne_doppelte() {
        let mut v = vec![];
        for t in ["a", "b", " ", "a", "c", "d", "e", "f", "g"] {
            verlauf_merken(&mut v, t);
        }
        assert_eq!(v, ["g", "f", "e", "d", "c", "a"]);
        verlauf_merken(&mut v, "d");
        assert_eq!(v[0], "d");
        assert_eq!(v.len(), VERLAUF_MAX);
        assert_eq!(v.iter().filter(|x| *x == "d").count(), 1);
    }

    #[test]
    fn vergleich_gross_klein() {
        assert_eq!(vergleich("Schaltplan", "schaltplan", false), Some(Treffer::Gleich));
        assert_eq!(vergleich("Schaltplan", "schaltplan", true), None);
        assert_eq!(vergleich("Schaltplan", "Schalt", true), Some(Treffer::Anfang));
        assert_eq!(vergleich("Übersicht", "ü", false), Some(Treffer::Anfang));
        assert_eq!(vergleich("[2] Schaltplan", "schalt", false), Some(Treffer::Enthaelt));
        assert_eq!(vergleich("x", "", false), None);
    }

    #[test]
    fn suchen_mit_vorfahren_und_weiter() {
        let b = baum();
        // gleicher Text hat Vorrang vor Teiltreffern
        assert_eq!(suchen(&b, "schaltplan", false, None, false), Some((vec![], "c".into())));
        // Anfang vor „enthält“; Vorfahren zum Aufklappen
        let (pfad, id) = suchen(&b, "N", true, None, false).unwrap();
        assert_eq!((pfad, id.as_str()), (vec!["a".to_string()], "a/2"));
        // „enthält“ findet Blätter mit Nummer
        let (pfad, id) = suchen(&b, "Sicherung", false, None, false).unwrap();
        assert_eq!(id, "b/bl/0");
        assert_eq!(pfad, ["b", "b/bl"]);
        // weiter: ab dem nächsten Knoten, am Ende von vorn
        assert_eq!(suchen(&b, "plan", false, Some("b/bl/0"), true).unwrap().1, "b/bl/1");
        assert_eq!(suchen(&b, "plan", false, Some("b/bl/0"), false).unwrap().1, "b/bl/0");
        assert_eq!(suchen(&b, "Potent", false, Some("c"), true).unwrap().1, "a");
        assert_eq!(suchen(&b, "gibt es nicht", false, None, false), None);
    }

    #[test]
    fn symbole_der_knoten() {
        assert_eq!(icon_fuer(ico::BUCH, false), (Some(BaumIcon::Buch), false));
        assert_eq!(icon_fuer(ico::BUCH, true), (Some(BaumIcon::Buecher), false));
        assert_eq!(icon_fuer(ico::FAVORITEN, true), (Some(BaumIcon::Buecher), true));
        assert_eq!(icon_fuer(ico::BLATT, false), (None, false));
        assert_eq!(icon_fuer(ico::ANLAGE, false), (None, false));
        assert_eq!(icon_fuer(ico::KLEMMLEISTEN, false), (Some(BaumIcon::Klemmleisten), false));
    }

    #[test]
    fn svg_eingebettet() {
        use BaumIcon::*;
        for i in [Potentiale, Leitungen, Objektkatalog, Stifte, Grafiken, Bauteile, Platzhalter, Sammelschienen, Klemmleisten, Blaetter, Buch, Buecher] {
            match source(i) {
                egui::ImageSource::Bytes { bytes, .. } => {
                    let t = String::from_utf8_lossy(bytes.as_ref()).to_string();
                    assert!(t.starts_with("<svg") && t.contains(r#"viewBox="0 0 16 16""#), "{i:?}");
                }
                _ => panic!("{i:?} nicht eingebettet"),
            }
        }
    }
}
