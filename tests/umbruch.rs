//! Textbox-Dialog, Schalter „Umbruch“: Ablage im .epr (ZCN Byte 0x19) in neuen Projekten und in W2, dazu das
//! Zeichnen mit Umbruch an der Boxbreite.

use elektroplanung::{bildexport, epr, epr_write, eprfile::EprFile, model::*};
use std::path::PathBuf;

fn reload(p: &Project) -> Project {
    let bytes = epr_write::to_bytes(p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    epr::import_file(&f).unwrap()
}

fn textbox(wrap: bool, text: &str) -> Kind {
    let (mn, mx) = (P::new(20.0, 50.0), P::new(60.0, 80.0));
    let style = TextStyle { wrap, frame: true, bold: true, rect: Some((mn, mx)), ..Default::default() };
    Kind::Text { pos: epr::text_anchor(mn, mx, Align::Left, false), text: text.into(), size: 12.0 * 0.3528, align: Align::Left, vertical: false, style }
}

fn texte(p: &Project) -> Vec<(String, bool)> {
    p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter_map(|o| if let Kind::Text { text, style, .. } = &o.kind { Some((text.clone(), style.wrap)) } else { None }).collect()
}

#[test]
fn umbruch_rundreise_neues_projekt() {
    for wrap in [true, false] {
        let mut p = Project::new_default();
        let id = p.alloc_id();
        p.anlagen[0].sheets[0].objects.push(Obj { id, kind: textbox(wrap, "Umbruch Probe"), pen: 0, fixed: false, group: None, raw: None });
        let t = texte(&reload(&p));
        assert!(t.contains(&("Umbruch Probe".to_string(), wrap)), "{t:?}");
    }
}

#[test]
fn umbruch_rundreise_w2() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let mut p = epr::import(&path).unwrap();
    // ersten Text mit Box umschalten, alle anderen bleiben, wie sie waren
    let vorher = texte(&p);
    let mut ziel = None;
    'suche: for a in p.anlagen.iter_mut() {
        for s in a.sheets.iter_mut() {
            for o in s.objects.iter_mut() {
                if let Kind::Text { style, text, .. } = &mut o.kind {
                    if style.rect.is_some() && !text.is_empty() {
                        style.wrap = !style.wrap;
                        ziel = Some((text.clone(), style.wrap));
                        break 'suche;
                    }
                }
            }
        }
    }
    let (text, wrap) = ziel.expect("W2 ohne Textbox");
    let nachher = texte(&reload(&p));
    assert_eq!(nachher.len(), vorher.len());
    let i = vorher.iter().position(|(t, _)| *t == text).unwrap();
    assert_eq!(nachher[i], (text, wrap));
    for (k, (a, b)) in vorher.iter().zip(&nachher).enumerate() {
        if k != i {
            assert_eq!(a, b);
        }
    }
}

/// Spalten und Zeilen mit dunklen Pixeln im Bildexport: (rechteste Spalte, Anzahl Zeilen).
fn ausdehnung(p: &Project) -> (u32, usize) {
    let img = bildexport::render_headless(p, &[], SheetRef { anlage: 0, sheet: 0 }, 150, true).unwrap();
    let k = 150.0 / 25.4;
    // nur das Innere der Box (ohne Rahmen) und rechts daneben
    let (x0, y0, y1) = ((21.0 * k) as u32, (51.0 * k) as u32, (79.0 * k) as u32);
    let dunkel = |x: u32, y: u32| img.get_pixel(x, y).0[0] < 128;
    let rechts = (x0..img.width()).filter(|&x| (y0..y1).any(|y| dunkel(x, y)) && !(y0..y1).all(|y| dunkel(x, y))).max().unwrap_or(0);
    let zeilen = (y0..y1).filter(|&y| (x0..(59.0 * k) as u32).any(|x| dunkel(x, y))).count();
    (rechts, zeilen)
}

#[test]
fn umbruch_zeichnet_in_der_box() {
    let text = "eins zwei drei vier fuenf sechs sieben acht neun zehn";
    let bild = |wrap| {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(Obj { id: 1, kind: textbox(wrap, text), pen: 0, fixed: false, group: None, raw: None });
        ausdehnung(&p)
    };
    let (r_ohne, z_ohne) = bild(false);
    let (r_mit, z_mit) = bild(true);
    let k = 150.0 / 25.4;
    // ohne Umbruch läuft der Text über die Box (rechter Rand 60 mm) hinaus, mit Umbruch nicht
    assert!(r_ohne as f32 > 61.0 * k, "{r_ohne}");
    assert!((r_mit as f32) < 60.5 * k, "{r_mit}");
    assert!(z_mit > 2 * z_ohne, "{z_mit} / {z_ohne}");
}
