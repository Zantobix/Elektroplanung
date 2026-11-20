//! MCP-Werkzeuge des Symboleditors: anlegen, Elemente, Seiten, Zusatzoptionen, Alternativtexte, Übernehmen
//! ins Projekt (und Schreiben/Lesen im Originalformat) bzw. in die Benutzerbibliothek.

use elektroplanung::{epr, mcp::Server, model::*};
use serde_json::{json, Value};
use std::path::PathBuf;

fn call(s: &mut Server, name: &str, a: Value) -> Value {
    s.call(name, &a).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ep_mcp_symbol_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

#[test]
fn tools_are_listed() {
    let names: Vec<String> = elektroplanung::mcp::tool_list().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    for n in ["symbol_open", "symbol_new", "symbol_get", "symbol_add_element", "symbol_update_element", "symbol_delete_element", "symbol_pages", "symbol_options", "symbol_alt_texts", "symbol_commit", "symbol_close"] {
        assert!(names.iter().any(|x| x == n), "{n} fehlt");
    }
}

#[test]
fn new_symbol_roundtrip_through_epr() {
    let mut s = Server::new();
    call(&mut s, "new_project", json!({"name": "Symboltest"}));
    call(&mut s, "symbol_new", json!({"name": "Schütz X", "bmk": "-K%C"}));
    call(&mut s, "symbol_add_element", json!({"type": "poly", "points": [[-5, -5], [5, -5], [5, 5], [-5, 5]]}));
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[0, -5], [0, -10]]}));
    call(&mut s, "symbol_add_element", json!({"type": "pin", "points": [[0, -10]]}));
    call(&mut s, "symbol_add_element", json!({"type": "pin", "points": [[0, 10]]}));
    call(&mut s, "symbol_add_element", json!({"type": "title", "points": [[-7, 0]]}));
    let t = call(&mut s, "symbol_add_element", json!({"type": "text", "points": [[-4, -2], [4, 2]], "text": "K", "align": "center", "text_mode": 2}));
    let ti = t["added"]["index"].as_u64().unwrap();
    // Seite (Bildvariante) mit eigenem Element, Zusatzoption mit eigenem Element
    let pg = call(&mut s, "symbol_pages", json!({"action": "add", "name": "Mit Spule"}));
    assert_eq!(pg["page"], 1);
    call(&mut s, "symbol_add_element", json!({"type": "ellipse", "points": [[-2, -2], [2, 2]], "page": 1, "page_mode": 0, "fill": true}));
    let op = call(&mut s, "symbol_options", json!({"action": "add", "name": "LED"}));
    assert_eq!(op["option"], 1);
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[6, -6], [8, -8]], "flag": 1}));
    // Alternativtext für den Text "K"
    let nr = call(&mut s, "symbol_get", json!({}))["elements"][ti as usize]["text_nr"].as_u64().unwrap();
    call(&mut s, "symbol_alt_texts", json!({"action": "set", "set": 1, "text_nr": nr, "text": "Q"}));
    // ändern und löschen
    let u = call(&mut s, "symbol_update_element", json!({"index": 1, "points": [[0, -5], [0, -12]], "style": 1}));
    assert_eq!(u["style"], 1);
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[9, 9], [10, 10]]}));
    let before = call(&mut s, "symbol_get", json!({}))["elements"].as_array().unwrap().len();
    call(&mut s, "symbol_delete_element", json!({"index": before - 1}));
    let g = call(&mut s, "symbol_get", json!({}));
    assert_eq!(g["elements"].as_array().unwrap().len(), before - 1);
    assert_eq!(g["pins"].as_array().unwrap().len(), 2);
    assert_eq!(g["pages"], json!(["Standard", "Mit Spule"]));
    assert_eq!(g["options"], json!(["LED"]));
    assert_eq!(g["alt_sets"][0]["texts"][0]["text"], "Q");

    // ins Projekt übernehmen, setzen, als .epr speichern und wieder laden
    let c = call(&mut s, "symbol_commit", json!({"target": "project"}));
    let key = c["key"].as_str().unwrap().to_string();
    call(&mut s, "place_symbol", json!({"anlage": 0, "sheet": 0, "key": key, "x": 100, "y": 80}));
    let path = tmp("neu.epr");
    call(&mut s, "save_project", json!({"path": path.to_str().unwrap(), "format": "epr"}));
    let p = epr::import(&path).unwrap();
    let d = p.symdefs.iter().find(|d| d.name == "Schütz X").expect("Symbol im gespeicherten Projekt");
    assert_eq!(d.bmk, "-K%C");
    assert_eq!(d.pins.len(), 2);
    assert_eq!(d.options, ["Standard", "Mit Spule"]);
    assert_eq!(d.flag_names, ["LED"]);
    assert!(d.vis.iter().any(|v| *v == (1, 0)), "Element nur auf Seite 1");
    assert!(d.flag.contains(&1), "Element der Zusatzoption");
    assert!(d.fill.contains(&true), "gefüllte Ellipse");
    let ti = d.prims.iter().position(|p| matches!(p, elektroplanung::symbols::Prim::TextBox(_, _, t, _) if t == "K")).expect("Text K");
    assert_eq!(d.text_mode[ti], 2);
    assert_eq!(d.alt_sets.first().and_then(|s| s.first()).map(|(_, t)| t.as_str()), Some("Q"));
    assert!(matches!(&d.prims[1], elektroplanung::symbols::Prim::Line(_, b, 1) if (b.y + 12.0).abs() < 0.01));
}

#[test]
fn edit_existing_epr_symbol_and_library_commit() {
    let sample = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !sample.exists() {
        return;
    }
    let mut s = Server::new();
    call(&mut s, "open_project", json!({"path": sample.to_str().unwrap()}));
    let g = call(&mut s, "symbol_open", json!({"name": "Verbindung"}));
    let ti = g["elements"].as_array().unwrap().iter().position(|e| e["text"] == "N").expect("Text N im Symbol Verbindung");
    call(&mut s, "symbol_update_element", json!({"index": ti, "text": "NN"}));
    call(&mut s, "symbol_commit", json!({"target": "project"}));
    let path = tmp("w2_geaendert.epr");
    call(&mut s, "save_project", json!({"path": path.to_str().unwrap(), "format": "epr"}));
    let p = epr::import(&path).unwrap();
    let d = p.symdefs.iter().find(|d| d.name == "Verbindung").unwrap();
    assert!(d.prims.iter().any(|p| matches!(p, elektroplanung::symbols::Prim::TextBox(_, _, t, _) if t == "NN")), "Änderung geschrieben");
    assert_eq!(d.alt_sets.len(), 5, "Alternativtexte bleiben erhalten");
    // alle anderen Symbole bleiben unverändert
    let orig = epr::import(&sample).unwrap();
    for o in orig.symdefs.iter().filter(|d| d.name != "Verbindung") {
        let n = p.symdefs.iter().find(|d| d.key == o.key).unwrap();
        assert_eq!(n.prims, o.prims, "{} verändert", o.name);
    }
    // Benutzerbibliothek (Ablage umgelenkt)
    std::env::set_var("EP_APPDIR", tmp("appdir"));
    let c = call(&mut s, "symbol_commit", json!({"target": "library", "library": "user"}));
    assert_eq!(c["key"], "user:200:Verbindung");
    let lib = std::fs::read_to_string(tmp("appdir").join("benutzer_symbole.json")).unwrap();
    assert!(lib.contains("\"Verbindung\"") && lib.contains("NN"));
    assert!(s.lib.iter().any(|d| d.key == "user:200:Verbindung"));
    call(&mut s, "symbol_close", json!({}));
    assert!(s.call("symbol_get", &json!({})).is_err());
}

#[test]
fn delete_page_and_option_renumber() {
    let mut s = Server::new();
    call(&mut s, "symbol_new", json!({"name": "Seiten"}));
    call(&mut s, "symbol_pages", json!({"action": "add", "name": "A"}));
    call(&mut s, "symbol_pages", json!({"action": "add", "name": "B"}));
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[0, 0], [1, 0]], "page": 1, "page_mode": 0}));
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[0, 0], [2, 0]], "page": 2, "page_mode": 0}));
    call(&mut s, "symbol_pages", json!({"action": "rename", "index": 2, "name": "B2"}));
    let r = call(&mut s, "symbol_pages", json!({"action": "delete", "index": 1}));
    assert_eq!(r["pages"], json!(["Standard", "B2"]));
    let g = call(&mut s, "symbol_get", json!({}));
    assert_eq!(g["elements"].as_array().unwrap().len(), 1);
    assert_eq!(g["elements"][0]["page"], 1, "Seite B rückt auf 1 vor");
    assert!(s.call("symbol_pages", &json!({"action": "delete", "index": 0})).is_err(), "Grundbild bleibt");
    call(&mut s, "symbol_options", json!({"action": "add", "name": "X"}));
    call(&mut s, "symbol_options", json!({"action": "add", "name": "Y"}));
    call(&mut s, "symbol_add_element", json!({"type": "line", "points": [[0, 0], [3, 0]], "flag": 2}));
    call(&mut s, "symbol_options", json!({"action": "delete", "index": 1}));
    let g = call(&mut s, "symbol_get", json!({}));
    assert_eq!(g["options"], json!(["Y"]));
    assert_eq!(g["elements"][1]["flag"], 1);
    let _ = Kind::Line { a: P::default(), b: P::default() };
}
