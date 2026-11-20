//! Prüfungen gegen das Beispielprojekt des Originals (W2.epr).
//! Pfad über EP_SAMPLE änderbar; fehlt die Datei, werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, eprfile::EprFile, mcp::Server, model::*};
use serde_json::json;
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if p.exists() {
        Some(p)
    } else {
        eprintln!("W2.epr nicht gefunden – Test übersprungen");
        None
    }
}

#[test]
fn block_layer_roundtrip_is_byte_identical() {
    let Some(p) = sample() else { return };
    let orig = std::fs::read(&p).unwrap();
    let f = EprFile::parse(&orig).unwrap();
    assert_eq!(f.bad_checksums, 0, "alle Blockprüfsummen müssen stimmen");
    assert!(f.trailer.is_empty(), "keine ungelesenen Bytes am Dateiende");
    assert_eq!(f.to_bytes(), orig);
}

#[test]
fn project_roundtrip_is_byte_identical() {
    let Some(p) = sample() else { return };
    let orig = std::fs::read(&p).unwrap();
    let project = epr::import(&p).unwrap();
    let out = epr_write::to_bytes(&project).unwrap();
    let diff = orig.iter().zip(&out).position(|(a, b)| a != b);
    assert_eq!((orig.len(), diff), (out.len(), None), "Laden + Speichern muss die Originaldatei exakt wiederherstellen");
}

#[test]
fn imported_content_matches_original_print() {
    let Some(p) = sample() else { return };
    let project = epr::import(&p).unwrap();
    assert!(project.total_sheets() >= 3);
    let names: Vec<&str> = project.anlagen.iter().flat_map(|a| &a.sheets).map(|s| s.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Altbau EG")), "Blatt 'Altbau EG' fehlt: {names:?}");
    let symbols = project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(o.kind, Kind::Symbol { .. })).count();
    assert!(symbols > 20, "zu wenige Symbole: {symbols}");
}

fn first_symbol(p: &Project) -> (SheetRef, u64, P) {
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                if let Kind::Symbol { pos, .. } = o.kind {
                    return (SheetRef { anlage: ai, sheet: si }, o.id, pos);
                }
            }
        }
    }
    panic!("kein Symbol")
}

#[test]
fn frame_follows_sheet_data_like_original_print() {
    let Some(p) = sample() else { return };
    let project = epr::import(&p).unwrap();
    let s = project.anlagen.iter().flat_map(|a| &a.sheets).find(|s| s.name == "Altbau EG").unwrap();
    // Original-Ausdruck Seite 3: Spaltenleiste beginnt 10 mm unter der Blattkante
    assert_eq!(s.frame(), (5.0, 10.0, 292.0, 205.0));
    let s = project.anlagen.iter().flat_map(|a| &a.sheets).find(|s| s.name == "Neubau EG").unwrap();
    assert!(s.portrait);
}

#[test]
fn edits_are_written_back_and_reloaded() {
    let Some(p) = sample() else { return };
    let mut project = epr::import(&p).unwrap();
    let (r, id, pos) = first_symbol(&project);
    {
        let s = project.sheet_mut(r).unwrap();
        let o = s.objects.iter_mut().find(|o| o.id == id).unwrap();
        o.kind.translate(P::new(10.0, 5.0));
        if let Kind::Symbol { bmk, .. } = &mut o.kind {
            *bmk = "-Q99".into();
        }
        s.name = "Geändert".into();
        let nid = 999_999;
        s.objects.push(Obj { id: nid, kind: Kind::Line { a: P::new(20.0, 30.0), b: P::new(120.0, 30.0) }, pen: 0, fixed: false, group: None, raw: None });
        s.objects.push(Obj { id: nid + 1, kind: Kind::Text { pos: P::new(40.0, 50.0), text: "Neuer Text".into(), size: 3.5, align: Align::Left, vertical: false, style: Default::default() }, pen: 0, fixed: false, group: None, raw: None });
    }
    project.anlagen[r.anlage].sheets[r.sheet].objects.retain(|o| !matches!(&o.kind, Kind::Text { text, .. } if text.is_empty()));
    let tmp = std::env::temp_dir().join("ep_edit_test.epr");
    epr_write::save(&project, &tmp).unwrap();
    let again = epr::import(&tmp).unwrap();
    assert_eq!(EprFile::read(&tmp).unwrap().bad_checksums, 0);
    let s = again.sheet(r).unwrap();
    assert_eq!(s.name, "Geändert");
    let moved = s.objects.iter().find_map(|o| match &o.kind {
        Kind::Symbol { pos: q, bmk, .. } if bmk == "-Q99" => Some(*q),
        _ => None,
    });
    let q = moved.expect("geändertes Symbol fehlt");
    assert!((q.x - pos.x - 10.0).abs() < 0.11 && (q.y - pos.y - 5.0).abs() < 0.11, "Lage {q:?} statt {pos:?}+(10,5)");
    assert!(s.objects.iter().any(|o| matches!(&o.kind, Kind::Line { a, b } if (a.x - 20.0).abs() < 0.11 && (b.x - 120.0).abs() < 0.11 && (a.y - 30.0).abs() < 0.11)));
    assert!(s.objects.iter().any(|o| matches!(&o.kind, Kind::Text { text, .. } if text == "Neuer Text")));
    // Andere Blätter bleiben unverändert
    let other = project.anlagen.iter().flat_map(|a| &a.sheets).count();
    assert_eq!(other, again.total_sheets());
}

#[test]
fn new_project_saves_as_valid_epr() {
    let mut p = Project::new_default();
    p.name = "Testprojekt".into();
    let id = p.alloc_id();
    p.anlagen[0].sheets[0].objects.push(Obj { id, kind: Kind::Rect { a: P::new(30.0, 40.0), b: P::new(80.0, 70.0) }, pen: 1, fixed: false, group: None, raw: None });
    let tmp = std::env::temp_dir().join("ep_new_test.epr");
    epr_write::save(&p, &tmp).unwrap();
    let back = epr::import(&tmp).unwrap();
    assert_eq!(back.name, "Testprojekt");
    assert_eq!(back.total_sheets(), 1);
    assert!(!back.anlagen[0].sheets[0].portrait, "neues Blatt muss Querformat bleiben");
    assert_eq!(back.anlagen[0].sheets[0].frame().1, 20.0, "Rahmen oben wie ein neues Blatt im Original");
    let o = &back.anlagen[0].sheets[0].objects;
    assert!(o.iter().any(|o| matches!(o.kind, Kind::Rect { a, b } if (a.x - 30.0).abs() < 0.11 && (b.y - 70.0).abs() < 0.11) && o.pen == 1), "{o:?}");
    assert_eq!(back.potentiale, p.potentiale);
}

#[test]
fn mcp_tools_work_end_to_end() {
    let Some(p) = sample() else { return };
    let mut s = Server::new();
    let init = s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "elektroplanung");
    let tools = s.handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
    assert!(tools["result"]["tools"].as_array().unwrap().len() >= 15);
    let info = s.call("open_project", &json!({"path": p})).unwrap();
    assert!(info["sheets_total"].as_u64().unwrap() >= 3);
    let sheet = s.call("get_sheet", &json!({"anlage":0,"sheet":0})).unwrap();
    assert!(!sheet["objects"].as_array().unwrap().is_empty());
    let added = s.call("add_object", &json!({"anlage":0,"sheet":0,"kind":"text","points":[[50,60]],"text":"per MCP"})).unwrap();
    let id = added["id"].as_u64().unwrap();
    s.call("update_object", &json!({"anlage":0,"sheet":0,"id":id,"dx":5,"dy":0})).unwrap();
    let out = std::env::temp_dir().join("ep_mcp_test.epr");
    let saved = s.call("save_project", &json!({"path": out, "format": "epr"})).unwrap();
    assert!(saved["not_written"].as_array().unwrap().is_empty());
    let pdf = std::env::temp_dir().join("ep_mcp_test.pdf");
    let r = s.call("export_pdf", &json!({"path": pdf})).unwrap();
    assert_eq!(r["pages"].as_u64(), info["sheets_total"].as_u64());
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let rt = s.call("verify_epr_roundtrip", &json!({"path": p})).unwrap();
    assert_eq!(rt["identical"], true);
    let back = epr::import(&out).unwrap();
    assert!(back.anlagen[0].sheets[0].objects.iter().any(|o| matches!(&o.kind, Kind::Text { text, pos, .. } if text == "per MCP" && (pos.x - 55.0).abs() < 0.2)));
}
