//! Prüfprojekt für die PDF-Ausgabe: verschiedene Schriften und Schnitte, Umlaute, Zeichen außerhalb von WinAnsi,
//! gedrehter Text, ein Bildobjekt und ein Blatt im Hochformat. Mit `EP_PDF_OUT=<datei.pdf>` wird das PDF zum
//! Ansehen (pdftoppm, pdffonts, pdftotext) zusätzlich gespeichert.

use elektroplanung::model::*;
use elektroplanung::pdf;

fn text(id: u64, x: f32, y: f32, s: &str, font: &str, bold: bool, italic: bool) -> Obj {
    let style = TextStyle { font: font.into(), bold, italic, ..Default::default() };
    Obj { id, kind: Kind::Text { pos: P::new(x, y), text: s.into(), size: 4.0, align: Align::Left, vertical: false, style }, pen: 0, fixed: false, group: None, raw: None }
}

/// Kleines PNG (rot/blau, halb durchsichtige Ecke) im Temp-Ordner
fn bild() -> String {
    let p = std::env::temp_dir().join("ep_pdf_pruefung.png");
    let img = image::RgbaImage::from_fn(40, 20, |x, y| match (x < 20, x > 34 && y < 5) {
        (_, true) => image::Rgba([0, 160, 0, 100]),
        (true, _) => image::Rgba([220, 0, 0, 255]),
        _ => image::Rgba([0, 0, 220, 255]),
    });
    img.save(&p).unwrap();
    p.to_string_lossy().to_string()
}

fn projekt() -> Project {
    let mut p = Project::new_default();
    let s = &mut p.anlagen[0].sheets[0];
    let mut id = 100;
    let mut y = 30.0;
    for font in ["", "Arial", "Courier New", "Segoe UI", "Verdana", "Comic Sans MS", "Gibt es nicht"] {
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            id += 1;
            let x = 20.0 + (bold as u8 as f32) * 60.0 + (italic as u8 as f32) * 120.0;
            let name = if font.is_empty() { "Times" } else { font };
            s.objects.push(text(id, x, y, &format!("{name} Äöüß €"), font, bold, italic));
        }
        y += 8.0;
    }
    s.objects.push(text(200, 20.0, y + 4.0, "Sonderzeichen: Ω µ ² ≥ → „Zitat“ … – —", "", false, false));
    s.objects.push(text(201, 20.0, y + 12.0, "Sonderzeichen Arial: Ω µ ² ≥ → „Zitat“ … – —", "Arial", false, false));
    let mut senkrecht = text(202, 270.0, 150.0, "Senkrecht Prüfung", "", false, false);
    if let Kind::Text { vertical, .. } = &mut senkrecht.kind {
        *vertical = true;
    }
    s.objects.push(senkrecht);
    s.objects.push(Obj {
        id: 300,
        kind: Kind::Image { a: P::new(200.0, 110.0), b: P::new(260.0, 140.0), path: bild(), mode: 0, frame: true },
        pen: 0,
        fixed: false,
        group: None,
        raw: None,
    });
    let mut hoch = p.anlagen[0].sheets[0].clone();
    hoch.portrait = true;
    hoch.objects = vec![text(400, 20.0, 40.0, "Hochformat Seite 2 – Übersicht", "Arial", true, false)];
    hoch.number = 2;
    p.anlagen[0].sheets.push(hoch);
    p
}

#[test]
fn pdf_pruefprojekt() {
    let (bytes, n) = pdf::render_headless(&projekt(), &[]);
    assert_eq!(n, 2);
    if let Ok(out) = std::env::var("EP_PDF_OUT") {
        std::fs::write(&out, &bytes).unwrap();
        // Vergleichsbilder der Bildschirmdarstellung (Bildexport) daneben
        for si in 0..2 {
            let img = elektroplanung::bildexport::render_headless(&projekt(), &[], SheetRef { anlage: 0, sheet: si }, 110, true).unwrap();
            img.save(format!("{out}.bild-{}.png", si + 1)).unwrap();
        }
    }
    let s = String::from_utf8_lossy(&bytes).to_string();
    // Seitengrößen: A4 quer, dann hoch
    assert!(s.contains("/MediaBox [0 0 841.89 595.28]") && s.contains("/MediaBox [0 0 595.28 841.89]"), "Seitengrößen");
    // Bild als XObject mit Transparenzmaske, im Inhalt gezeichnet
    assert!(s.contains("/Subtype /Image /Width 40 /Height 20") && s.contains("/SMask "), "Bild eingebettet");
    assert!(s.contains("/XObject << /Im1 ") && s.contains("/Im1 Do"), "Bild gezeichnet");
    // Times-Text mit Ω/≥/→: diese Zeichen aus der eingebetteten Times New Roman, kein „?“ (wenn installiert).
    // Kein ✓ o. Ä. im Prüftext: ob eine Ersatzschrift es hat, hängt von den installierten Schriften ab (auf dem
    // Linux-Runner fehlte es am 25.11.2026 und wurde zu „?“).
    if elektroplanung::schriften::finden("Times New Roman", false, false).is_some() {
        let fragezeichen: Vec<&str> = s.lines().filter(|l| l.contains(") Tj") && l.contains('?')).collect();
        assert!(fragezeichen.is_empty(), "{fragezeichen:?}");
        assert!(s.contains("/FontFile2"), "Times New Roman eingebettet");
    }
    // Typografische Zeichen bleiben in WinAnsi: „ (0x84), “ (0x93), … (0x85), – (0x96), — (0x97)
    assert!(s.contains("\\204Zitat\\223 \\205 \\226 \\227"), "WinAnsi-Sonderzeichen");
}
