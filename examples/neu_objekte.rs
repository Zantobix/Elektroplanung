//! Testdatei für das Original: ein neues Projekt (aus der Vorlage) mit je einem neu angelegten Objekt jeder Art
//! (Linie, Kurve, Rechteck, Dreieck, Kreis, Polygon, Text, Textbox, Tabelle, Bild, Symbol, Verbindung, Verweis mit
//! Gegenverweis, Sammelschiene). `cargo run --example neu_objekte [-- <ziel.epr>]`, Vorgabe %TEMP%\ep_neu_objekte.epr.
use elektroplanung::{epr, epr_write, model::*};

fn main() {
    let ziel = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("ep_neu_objekte.epr"));
    let mut p = Project::new_default();
    p.name = "Neue Objekte".into();
    p.anlagen[0].sheets[0].name = "Neue Objekte".into();
    let mut s2 = Sheet::new(2);
    s2.name = "Gegenverweis".into();
    p.anlagen[0].sheets.push(s2);
    let def = elektroplanung::symbols::builtin().into_iter().next().expect("eingebautes Symbol");
    let key = def.key.clone();
    p.symdefs.push(def);
    // Projektbild: kleines Farbverlauf-JPEG
    let img = image::RgbImage::from_fn(160, 120, |x, y| image::Rgb([(x * 255 / 160) as u8, (y * 255 / 120) as u8, 128]));
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90).encode_image(&img).unwrap();
    p.bilder.push(Bild { name: "Verlauf".into(), nr: 1, jpeg });
    let (ida, idb) = (p.alloc_id(), p.alloc_id());
    let anl = p.anlagen[0].name.clone();
    let rect = (P::new(40.0, 95.0), P::new(100.0, 105.0));
    let text_style = TextStyle { bold: true, color: [200, 0, 0], font: "Arial".into(), rect: Some(rect), ..Default::default() };
    let kinds = vec![
        Kind::Line { a: P::new(30.0, 40.0), b: P::new(80.0, 40.0) },
        Kind::Curve { a: P::new(30.0, 60.0), c1: P::new(45.0, 45.0), c2: P::new(60.0, 75.0), b: P::new(80.0, 60.0) },
        Kind::Rect { a: P::new(30.0, 70.0), b: P::new(60.0, 85.0) },
        Kind::Triangle { a: P::new(65.0, 70.0), b: P::new(85.0, 85.0) },
        Kind::Circle { a: P::new(90.0, 70.0), b: P::new(105.0, 85.0) },
        Kind::Poly { pts: vec![P::new(110.0, 70.0), P::new(130.0, 70.0), P::new(120.0, 85.0)] },
        Kind::Text { pos: epr::text_anchor(rect.0, rect.1, Align::Left, false), text: "Text fett rot (Arial)".into(), size: 3.528, align: Align::Left, vertical: false, style: text_style },
        Kind::TextBox { a: P::new(110.0, 30.0), b: P::new(170.0, 55.0), text: "Textbox mit Umbruch\nzweite Zeile".into(), size: 3.528 },
        Kind::Table { a: P::new(30.0, 115.0), b: P::new(130.0, 139.0), rows: 3, cols: 3, cells: ["Nr", "Name", "Wert", "1", "Lampe", "60 W", "2", "Dose", "16 A"].map(String::from).to_vec(), header: true, fixed_w: false, art: 0, ab: 0 },
        Kind::Image { a: P::new(180.0, 30.0), b: P::new(260.0, 90.0), path: "bld:1".into(), mode: 2, frame: true },
        Kind::Symbol { key, pos: P::new(200.0, 120.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-Q1".into(), value: "16A".into(), scale: 1.0, option: 0, title: Some((P::new(6.0, -4.0), false)), flags: 0, ext: SymExt { funktion: "Neu".into(), ..Default::default() } },
        Kind::Wire { pts: vec![P::new(150.0, 150.0), P::new(150.0, 170.0), P::new(230.0, 170.0)], leitung: 0, bez: vec![(P::new(180.0, 169.0), "sw".into())], opts: 0 },
        Kind::Busbar { a: P::new(30.0, 150.0), x2: 140.0, name: "Neue Schiene".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new() },
    ];
    for kind in kinds {
        let id = p.alloc_id();
        p.anlagen[0].sheets[0].objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None });
    }
    // Textversatz zeigt auf die Textmitte (wie im Original): rechts neben das Dreieck, um die halbe Textbreite versetzt
    let v = |pos: P, text: &str, blatt: u32, obj: u64| {
        let dx = 3.0 + 0.7 * text.chars().count() as f32;
        Kind::Verweis { pos, text_pos: P::new(pos.x + dx, pos.y - 1.0), text: text.into(), dir: 0, vertical: false, ziel: Some(VerweisZiel { anlage: anl.clone(), blatt, obj }) }
    };
    p.anlagen[0].sheets[0].objects.push(Obj { id: ida, kind: v(P::new(240.0, 150.0), "nach Blatt 2", 2, idb), pen: 0, fixed: false, group: None, raw: None });
    p.anlagen[0].sheets[1].objects.push(Obj { id: idb, kind: v(P::new(40.0, 60.0), "von Blatt 1", 1, ida), pen: 0, fixed: false, group: None, raw: None });
    let fehlt = epr_write::unsupported(&p);
    epr_write::save(&p, &ziel).expect("Speichern");
    let back = epr::import(&ziel).expect("Lesen");
    let n: usize = back.anlagen.iter().flat_map(|a| &a.sheets).map(|s| s.objects.len()).sum();
    println!("{} geschrieben: {n} Objekte, nicht schreibbar: {fehlt:?}", ziel.display());
}
