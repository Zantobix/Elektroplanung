//! MCP-Werkzeuge für Planobjekte und Bauteile (src/mcp_plan.rs): Sammelschiene mit Auto-Stapel, Klemmstelle teilt
//! die Leitung, Verweis, Tabelle, Leitungsart/Bezeichnung, Kontaktspiegel, Bauteile anlegen/zuordnen/löschen und
//! die .epr-Rundreise (Gruppe/Hersteller nur aus der Bibliothek).
use elektroplanung::mcp::Server;
use serde_json::{json, Value};

fn ok(s: &mut Server, name: &str, a: Value) -> Value {
    s.call(name, &a).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn planobjekte_ueber_mcp() {
    let mut s = Server::new();
    ok(&mut s, "new_project", json!({"name": "MCP"}));
    let b = json!({"anlage": 0, "sheet": 0});
    let mit = |x: Value| { let mut v = b.clone(); v.as_object_mut().unwrap().extend(x.as_object().unwrap().clone()); v };
    // zwei Schienen im Auto-Stapel: L1 oben, PE unten; Lage über die Blattbreite wie der Werkzeugknopf
    let l1 = ok(&mut s, "add_busbar", mit(json!({"name": "L1"})));
    let pe = ok(&mut s, "add_busbar", mit(json!({"name": "PE", "unten": true})));
    assert_eq!(l1["points"][0][0], json!(13.0));
    assert!(pe["points"][0][1].as_f64().unwrap() > l1["points"][0][1].as_f64().unwrap() + 100.0, "{pe} unter {l1}");
    assert_eq!((pe["auto"].clone(), pe["unten"].clone()), (json!(true), json!(true)));
    // Leitung von L1 nach unten, Klemmstelle mitten darauf teilt sie in zwei
    let y1 = l1["points"][0][1].as_f64().unwrap();
    let w = ok(&mut s, "add_object", mit(json!({"kind": "wire", "points": [[50.0, y1], [50.0, 120.0]]})));
    let k = ok(&mut s, "add_terminal", mit(json!({"x": 50.0, "y": 80.0})));
    assert_eq!((k["leiste"].clone(), k["nr"].clone()), (json!("X1"), json!(1)));
    let blatt = ok(&mut s, "get_sheet", b.clone());
    let leitungen: Vec<&Value> = blatt["objects"].as_array().unwrap().iter().filter(|o| o["kind"] == "Verbindung").collect();
    assert_eq!(leitungen.len(), 2, "Leitung an der Klemme geteilt");
    // Leitungsart und Aderfarbe
    let li = ok(&mut s, "add_leitung", json!({"name": "NYM 3x1,5", "color": [0, 0, 255], "width": 0.35}))["index"].as_u64().unwrap();
    let wv = ok(&mut s, "set_wire", mit(json!({"id": w["id"], "leitung": "NYM 3x1,5", "bez": [[51.0, 60.0, "sw"]]})));
    assert_eq!((wv["leitung"].as_u64(), wv["bez"][0][2].clone()), (Some(li), json!("sw")));
    // Verweis, Tabelle, Kontaktspiegel ohne Eigentümer
    let v = ok(&mut s, "add_reference", mit(json!({"x": 280.0, "y": y1, "text": "/2.1", "dir": 0})));
    assert_eq!(v["text"], json!("/2.1"));
    let t = ok(&mut s, "add_table", mit(json!({"points": [[20.0, 150.0], [120.0, 180.0]], "rows": 2, "cols": 2, "cells": ["Nr", "Name", "1", "LS"]})));
    assert_eq!(t["cells"][3], json!("LS"));
    ok(&mut s, "add_contact_mirror", mit(json!({"x": 200.0, "y": 150.0})));
    // Bauteile: Gruppe/Hersteller nur aus der Bibliothek speicherbar (ohne Bibliothek: Warnung)
    let p0 = ok(&mut s, "add_part", json!({"name": "LS B16", "typ": "5SL6116-6", "artikel": "4001869161441", "nummer": "LS1"}));
    assert_eq!(p0["index"], json!(0));
    ok(&mut s, "add_part", json!({"name": "Klemme", "typ": "PT 4"}));
    ok(&mut s, "update_part", json!({"index": 1, "typ": "PT 4-TWIN"}));
    ok(&mut s, "assign_parts", json!({"id": k["id"], "parts": [1, 0]}));
    assert!(s.call("assign_parts", &json!({"id": k["id"], "parts": [5]})).is_err());
    ok(&mut s, "delete_part", json!({"index": 0}));
    let l = ok(&mut s, "list_parts", json!({}));
    assert_eq!(l["assigned"][0]["parts"], json!([0]), "Index rückt nach dem Löschen nach");
    assert_eq!(l["parts"][0]["typ"], json!("PT 4-TWIN"));
    // speichern und wieder laden: alles da, Rundreise byteidentisch
    let pfad = std::env::temp_dir().join("ep_mcp_plan_test.epr");
    ok(&mut s, "save_project", json!({"path": pfad.to_string_lossy(), "format": "epr"}));
    let mut s2 = Server::new();
    ok(&mut s2, "open_project", json!({"path": pfad.to_string_lossy()}));
    let n = ok(&mut s2, "get_sheet", b.clone());
    let arten: Vec<&str> = n["objects"].as_array().unwrap().iter().map(|o| o["kind"].as_str().unwrap()).collect();
    for a in ["Sammelschiene", "Klemmstelle", "Verweis", "Tabelle", "Kontaktspiegel", "Verbindung"] {
        assert!(arten.contains(&a), "{a} fehlt nach dem Laden: {arten:?}");
    }
    let l2 = ok(&mut s2, "list_parts", json!({}));
    assert_eq!((l2["parts"].as_array().unwrap().len(), l2["parts"][0]["name"].clone()), (1, json!("Klemme")));
    assert_eq!(l2["assigned"].as_array().unwrap().len(), 1);
    assert_eq!(ok(&mut s2, "verify_epr_roundtrip", json!({"path": pfad.to_string_lossy()}))["identical"], json!(true));
    let _ = std::fs::remove_file(pfad);
}
