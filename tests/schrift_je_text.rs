//! Schrift je Text: Texte mit eigenem Schriftnamen werden in dieser Schrift gezeichnet (Bild) bzw. als passende
//! PDF-Standardschrift ausgegeben.

use elektroplanung::model::*;
use elektroplanung::{bildexport, pdf};

fn projekt(font: &str, text: &str) -> Project {
    let mut p = Project::new_default();
    let style = TextStyle { font: font.into(), bold: true, ..Default::default() };
    p.anlagen[0].sheets[0].objects.push(Obj {
        id: 1,
        kind: Kind::Text { pos: P::new(20.0, 60.0), text: text.into(), size: 10.0, align: Align::Left, vertical: false, style },
        pen: 0,
        fixed: false,
        group: None,
        raw: None,
    });
    p
}

#[test]
fn pdf_waehlt_standardschrift_nach_namen() {
    let inhalt = |font: &str| String::from_utf8_lossy(&pdf::render_headless(&projekt(font, "Schrifttest"), &[]).0).to_string();
    // Fett: Times-Bold = F2, Helvetica-Bold = F6, Courier-Bold = F10
    let zeile = |s: &str| s.lines().find(|l| l.contains("(Schrifttest) Tj")).map(|l| l.to_string()).unwrap_or_default();
    assert!(zeile(&inhalt("")).starts_with("BT /F2 "), "{}", zeile(&inhalt("")));
    assert!(zeile(&inhalt("Arial")).starts_with("BT /F6 "), "{}", zeile(&inhalt("Arial")));
    assert!(zeile(&inhalt("Courier New")).starts_with("BT /F10 "));
    assert!(inhalt("Arial").contains("/BaseFont /Helvetica-Bold"));
}

/// Breite der dunklen Pixel (Spalten) einer Zeile im Bildexport.
fn breite(p: &Project) -> usize {
    let img = bildexport::render_headless(p, &[], SheetRef { anlage: 0, sheet: 0 }, 150, true).unwrap();
    let k = 150.0 / 25.4;
    let (y0, y1) = ((50.0 * k) as u32, (72.0 * k) as u32);
    (0..img.width()).filter(|&x| (y0..y1).any(|y| img.get_pixel(x, y).0[0] < 128)).count()
}

#[test]
fn bildexport_zeichnet_eigene_schrift() {
    if elektroplanung::schriften::finden("Arial", true, false).is_none() {
        return; // ohne installierte Arial (oder Ersatz) nichts zu prüfen
    }
    let times = breite(&projekt("", "Wiiiiiiiiiiiiiiiiiiide"));
    let arial = breite(&projekt("Arial", "Wiiiiiiiiiiiiiiiiiiide"));
    let unbekannt = breite(&projekt("Gibt es nicht 123", "Wiiiiiiiiiiiiiiiiiiide"));
    assert!(times > 100, "Text fehlt");
    assert_ne!(times, arial, "Arial wird wie Times gezeichnet");
    // unbekannte Schrift: Times wie bisher
    assert_eq!(times, unbekannt);
}

/// Ohne eigenen Schriftnamen gilt die Standardschrift des Projekts.
#[test]
fn projektschrift_gilt_ohne_eigenen_namen() {
    if elektroplanung::schriften::finden("Arial", true, false).is_none() {
        return;
    }
    let mut p = projekt("", "Wiiiiiiiiiiiiiiiiiiide");
    let times = breite(&p);
    p.schrift = "Arial".into();
    assert_eq!(breite(&p), breite(&projekt("Arial", "Wiiiiiiiiiiiiiiiiiiide")));
    assert_ne!(breite(&p), times);
}
