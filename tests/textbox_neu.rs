//! Neue Textbox (Werkzeug „Neue Textbox“, MCP add_object textbox): ein Text mit Box wie im Original (ZCN 11) mit den
//! Vorgaben des Originals (Rahmen an, Umbruch aus, 8 pt), Einstellungen des Textbox-Dialogs ändern (Umbruch an/aus,
//! Rahmen, Ausrichtung, Drehung, Box), Rundreise im neuen Projekt und in W2.

use elektroplanung::{epr, epr_write, eprfile::EprFile, mcp::Server, model::*, textbox};
use serde_json::json;
use std::path::PathBuf;

fn reload(p: &Project) -> (Project, EprFile) {
    let bytes = epr_write::to_bytes(p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    (epr::import_file(&f).unwrap(), f)
}

/// Text des Blocks und Box auf 0,1 mm gerundet vergleichen (die Datei speichert 0,1 mm).
fn gleich(a: &Kind, b: &Kind) -> bool {
    let r = |k: &Kind| {
        let mut k = k.clone();
        let q = |p: &mut P| *p = P::new((p.x * 10.0).round() / 10.0, (p.y * 10.0).round() / 10.0);
        if let Kind::Text { pos, size, style, .. } = &mut k {
            q(pos);
            *size = (*size / 0.3528).round();
            if let Some((a, b)) = style.rect.as_mut() {
                q(a);
                q(b);
            }
        }
        k
    };
    r(a) == r(b)
}

/// Einstellungen wie nach dem Textbox-Dialog (props.rs, Props::Text): Umbruch, Rahmen, Ausrichtung, Box.
fn dialog(k: &mut Kind, wrap: bool, frame: bool) {
    if let Kind::Text { pos, align, style, text, .. } = k {
        style.wrap = wrap;
        style.frame = frame;
        *align = Align::Center;
        *text = "eins zwei drei vier".into();
        let (mn, mx) = (P::new(30.0, 40.0), P::new(70.0, 55.0));
        style.rect = Some((mn, mx));
        *pos = epr::text_anchor(mn, mx, Align::Center, false);
    }
}

fn zcn_letztes(f: &EprFile) -> elektroplanung::eprfile::Block {
    f.root("PRJ").unwrap().kids_of("ANL").next().unwrap().kids_of("BLT").next().unwrap().kids.last().unwrap().clone()
}

#[test]
fn neue_textbox_im_neuen_projekt() {
    let mut p = Project::new_default();
    let id = p.alloc_id();
    p.anlagen[0].sheets[0].objects.push(Obj { id, kind: textbox::neu(P::new(20.0, 30.0), P::new(80.0, 45.0), "Text"), pen: 0, fixed: false, group: None, raw: None });
    let (back, f) = reload(&p);
    // Block wie das Original ihn für eine neue Textbox anlegt: Typ 11, Rahmen 1, Umbruch 0, 8 pt, links
    let b = zcn_letztes(&f);
    assert_eq!((b.data[0x3c], b.data[0x18], b.data[0x19], b.data[0x1a], b.i32(4) & 0xffff), (11, 0, 0, 1, 8));
    assert_eq!((b.i32(0), b.i32(1), b.i32(2), b.i32(3)), (200, 300, 800, 450));
    let got = &back.anlagen[0].sheets[0].objects.last().unwrap().kind;
    assert!(gleich(got, &p.anlagen[0].sheets[0].objects[0].kind), "{got:?}");

    // Dialog: Umbruch an/aus, Rahmen aus, Ausrichtung mittig, neue Box – Rundreise je Fall, auch zweimal
    for (wrap, frame) in [(true, false), (false, true), (true, true)] {
        let mut q = back.clone();
        let o = q.anlagen[0].sheets[0].objects.last_mut().unwrap();
        dialog(&mut o.kind, wrap, frame);
        let want = o.kind.clone();
        let (r1, f1) = reload(&q);
        let b = zcn_letztes(&f1);
        assert_eq!((b.data[0x18], b.data[0x19], b.data[0x1a]), (1, wrap as u8, frame as u8));
        let got = &r1.anlagen[0].sheets[0].objects.last().unwrap().kind;
        assert!(gleich(got, &want), "{got:?} / {want:?}");
        let (r2, _) = reload(&r1);
        assert!(gleich(&r2.anlagen[0].sheets[0].objects.last().unwrap().kind, &want));
    }
}

#[test]
fn neue_textbox_in_w2() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let orig = std::fs::read(&path).unwrap();
    let base = epr::import(&path).unwrap();
    // unverändert: byteidentisch (frühere Umwandlung in eine eigene Art TextBox entfällt)
    assert_eq!(epr_write::to_bytes(&base).unwrap(), orig);
    for wrap in [false, true] {
        let mut p = base.clone();
        let id = p.alloc_id();
        let mut k = textbox::neu(P::new(20.0, 30.0), P::new(80.0, 45.0), "Neue Box");
        dialog(&mut k, wrap, true);
        p.anlagen[0].sheets[0].objects.push(Obj { id, kind: k.clone(), pen: 0, fixed: false, group: None, raw: None });
        let (back, _) = reload(&p);
        let objs = &back.anlagen[0].sheets[0].objects;
        assert_eq!(objs.len(), base.anlagen[0].sheets[0].objects.len() + 1);
        assert!(gleich(&objs.last().unwrap().kind, &k), "{:?}", objs.last().unwrap().kind);
        // alle übrigen Objekte unverändert (Verweisziele zeigen auf Objekt-Ids, die beim Lesen neu vergeben werden)
        let ohne_id = |k: &Kind| {
            let mut k = k.clone();
            if let Kind::Verweis { ziel: Some(z), .. } = &mut k {
                z.obj = 0;
            }
            k
        };
        for (a, b) in base.anlagen[0].sheets[0].objects.iter().zip(objs) {
            assert_eq!(ohne_id(&a.kind), ohne_id(&b.kind));
        }
        // Umbruch der neuen Box umschalten, erneut speichern
        let mut q = back.clone();
        if let Kind::Text { style, .. } = &mut q.anlagen[0].sheets[0].objects.last_mut().unwrap().kind {
            style.wrap = !wrap;
        }
        let (again, _) = reload(&q);
        assert!(matches!(&again.anlagen[0].sheets[0].objects.last().unwrap().kind, Kind::Text { style, .. } if style.wrap == !wrap));
    }
}

#[test]
fn mcp_add_object_textbox() {
    let mut s = Server::new();
    s.call("new_project", &json!({"name": "Textbox"})).unwrap();
    let r = s.call("add_object", &json!({"anlage": 0, "sheet": 0, "kind": "textbox", "points": [[20, 30], [80, 45]], "text": "Hallo", "wrap": true})).unwrap();
    let id = r["id"].as_u64().unwrap();
    let sh = s.call("get_sheet", &json!({"anlage": 0, "sheet": 0})).unwrap();
    let o = sh.to_string();
    assert!(o.contains("\"Hallo\""), "{o}");
    let objs = sh["objects"].as_array().cloned().unwrap_or_default();
    let t = objs.iter().find(|x| x["id"].as_u64() == Some(id)).expect("Textbox in get_sheet");
    assert_eq!((t["wrap"].as_bool(), t["frame"].as_bool()), (Some(true), Some(true)));
    assert_eq!(t["box"], json!([[20.0, 30.0], [80.0, 45.0]]));
}
