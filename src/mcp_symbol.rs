//! MCP-Werkzeuge für den Symboleditor: Symbol öffnen/anlegen, Elemente auflisten/hinzufügen/ändern/löschen,
//! Seiten (Bildvarianten) und Zusatzoptionen verwalten, Alternativtexte, Übernehmen ins Projekt bzw. in die
//! Bibliothek. Bearbeitet wird eine Arbeitskopie (`Server::sym`) wie im Symboleditor der Oberfläche
//! ("Übernehmen" / "Abbruch").

use crate::mcp::Server;
use crate::model::{Align, P};
use crate::symbols::{Prim, SymDef};
use serde_json::{json, Value};

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": props, "required": required } })
}

/// Werkzeugbeschreibungen (werden an `tools/list` angehängt).
pub fn tool_list() -> Vec<Value> {
    let elem = json!({
        "type": {"type": "string", "description": "line, poly, bezier, ellipse, text, pin, title, value"},
        "points": {"type": "array", "description": "Punkte [[x, y], …] in mm relativ zum Symbolursprung (y nach unten); line 2, poly ≥ 3, bezier 4, ellipse/text 2 (Rechteck), pin/title/value 1"},
        "style": {"type": "integer", "description": "Stiftart 0 Standard, 1 Gestrichelt, 2 Strich/Punkt, 3 Gepunktet"},
        "page": {"type": "integer", "description": "Seite (Bildvariante), 0 = Grundbild"},
        "page_mode": {"type": "integer", "description": "0 = nur diese Seite, 1 = ab dieser Seite (Standard), 2 = bis zu dieser Seite"},
        "flag": {"type": "integer", "description": "Zusatzoption n (0 = immer sichtbar)"},
        "text": {"type": "string"},
        "align": {"type": "string", "description": "left, center, right"},
        "text_mode": {"type": "integer", "description": "0 waagerecht, 1 hochkant, 2 dreht mit dem Symbol"},
        "fill": {"type": "boolean"}
    });
    let mut upd = elem.clone();
    upd["index"] = json!({"type": "integer", "description": "Elementnummer aus symbol_get"});
    upd["pin"] = json!({"type": "integer", "description": "statt index: Nummer des Anschlusses"});
    vec![
        tool("symbol_open", "Symbol aus Projekt oder Bibliothek zum Bearbeiten öffnen (Schlüssel oder Name).", json!({"key": {"type": "string"}, "name": {"type": "string"}}), &[]),
        tool("symbol_new", "Neues leeres Symbol anlegen (wie Hinzufügen › Symbol).", json!({"name": {"type": "string"}, "bmk": {"type": "string", "description": "BMK-Vorlage, z. B. -Q%C"}}), &["name"]),
        tool("symbol_get", "Geöffnetes Symbol: Elemente, Anschlüsse, Titelanker, Seiten, Zusatzoptionen, Alternativtexte.", json!({}), &[]),
        tool("symbol_add_element", "Element hinzufügen (Linie, Polygon, Kurve, Ellipse, Text, Anschluss, Titel-/Wertanker).", elem, &["type", "points"]),
        tool("symbol_update_element", "Element bzw. Anschluss ändern (nur angegebene Felder).", upd, &[]),
        tool("symbol_delete_element", "Element (index) oder Anschluss (pin) löschen.", json!({"index": {"type": "integer"}, "pin": {"type": "integer"}}), &[]),
        tool("symbol_pages", "Seiten (Bildvarianten): list, add (name), rename (index, name), delete (index).",
            json!({"action": {"type": "string"}, "index": {"type": "integer"}, "name": {"type": "string"}}), &["action"]),
        tool("symbol_options", "Zusatzoptionen (OPT): list, add (name), rename (index ab 1, name), delete (index).",
            json!({"action": {"type": "string"}, "index": {"type": "integer"}, "name": {"type": "string"}}), &["action"]),
        tool("symbol_alt_texts", "Alternativtexte: list, set (set ab 1, text_nr, text), delete (set).",
            json!({"action": {"type": "string"}, "set": {"type": "integer"}, "text_nr": {"type": "integer"}, "text": {"type": "string"}}), &["action"]),
        tool("symbol_set_properties", "Symboleigenschaften: Name, BMK-Vorlage, Gruppe, Freigaben.",
            json!({"name": {"type": "string"}, "bmk": {"type": "string"}, "group": {"type": "string"}, "querverweise": {"type": "boolean"}, "sicherung": {"type": "boolean"}}), &[]),
        tool("symbol_commit", "Übernehmen: target 'project' (Objektkatalog; gesetzte Symbole zeigen die Änderung) oder 'library' (Benutzerbibliothek, library: 'user' oder 'standard').",
            json!({"target": {"type": "string"}, "library": {"type": "string"}}), &["target"]),
        tool("symbol_close", "Bearbeitung ohne Übernehmen beenden (Abbruch).", json!({}), &[]),
    ]
}

fn pts(a: &Value) -> Result<Vec<P>, String> {
    a.get("points")
        .and_then(Value::as_array)
        .ok_or("Parameter 'points' fehlt")?
        .iter()
        .map(|p| {
            let x = p.get(0).and_then(Value::as_f64).ok_or("Punkt braucht [x, y]")?;
            let y = p.get(1).and_then(Value::as_f64).ok_or("Punkt braucht [x, y]")?;
            Ok(P::new(x as f32, y as f32))
        })
        .collect()
}
fn int(a: &Value, k: &str) -> Option<i64> {
    a.get(k).and_then(Value::as_i64)
}
fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}
fn align(a: &Value) -> Option<Align> {
    s(a, "align").map(|v| match v {
        "center" | "mittig" => Align::Center,
        "right" | "rechts" => Align::Right,
        _ => Align::Left,
    })
}
fn al_str(al: Align) -> &'static str {
    match al {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}
fn pj(p: &P) -> Value {
    json!([p.x, p.y])
}

/// Grundelement aus Typ und Punkten.
fn make_prim(ty: &str, p: &[P], style: u8, text: &str, al: Align) -> Result<Prim, String> {
    let need = |n: usize| if p.len() < n { Err(format!("'{ty}' braucht {n} Punkte")) } else { Ok(()) };
    Ok(match ty {
        "line" => {
            need(2)?;
            Prim::Line(p[0], p[1], style)
        }
        "poly" | "rect" | "triangle" => {
            need(3)?;
            Prim::Poly(p.to_vec(), style)
        }
        "bezier" | "curve" => {
            need(4)?;
            Prim::Bezier(p[0], p[1], p[2], p[3], style)
        }
        "ellipse" | "circle" => {
            need(2)?;
            Prim::Ellipse(p[0], p[1], style)
        }
        "text" => {
            need(2)?;
            Prim::TextBox(p[0], p[1], text.into(), al)
        }
        _ => return Err(format!("unbekannter Elementtyp '{ty}'")),
    })
}

fn prim_json(d: &SymDef, i: usize) -> Value {
    let (ty, points, style, text, al): (&str, Vec<P>, u8, Option<&str>, Option<Align>) = match &d.prims[i] {
        Prim::Line(a, b, st) => ("line", vec![*a, *b], *st, None, None),
        Prim::Poly(v, st) => ("poly", v.clone(), *st, None, None),
        Prim::Bezier(a, b, c, e, st) => ("bezier", vec![*a, *b, *c, *e], *st, None, None),
        Prim::Ellipse(a, b, st) => ("ellipse", vec![*a, *b], *st, None, None),
        Prim::Arc(c, r, _, _) => ("arc", vec![*c, P::new(c.x + r, c.y)], 0, None, None),
        Prim::Text(c, t, _) => ("text", vec![*c], 0, Some(t.as_str()), None),
        Prim::TextBox(a, b, t, al) => ("text", vec![*a, *b], 0, Some(t.as_str()), Some(*al)),
    };
    let v = d.vis.get(i).copied().unwrap_or((0, 1));
    let mut o = json!({
        "index": i, "type": ty, "points": points.iter().map(pj).collect::<Vec<_>>(), "style": style,
        "page": v.0, "page_mode": v.1, "flag": d.flag.get(i).copied().unwrap_or(0),
        "fill": d.fill.get(i).copied().unwrap_or(false),
    });
    if let Some(t) = text {
        o["text"] = json!(t);
        o["text_nr"] = json!(d.text_idx.get(i).copied().unwrap_or(0));
        o["text_mode"] = json!(d.text_mode.get(i).copied().unwrap_or(0));
        if let Some(al) = al {
            o["align"] = json!(al_str(al));
        }
    }
    o
}

/// Übersicht des Symbols wie es der Symboleditor zeigt.
pub fn symbol_json(d: &SymDef) -> Value {
    json!({
        "key": d.key, "name": d.name, "bmk": d.bmk, "group": d.group,
        "elements": (0..d.prims.len()).map(|i| prim_json(d, i)).collect::<Vec<_>>(),
        "pins": d.pins.iter().enumerate().map(|(i, p)| {
            let v = d.pin_vis.get(i).copied().unwrap_or((0, 1));
            json!({"pin": i, "point": pj(p), "page": v.0, "page_mode": v.1})
        }).collect::<Vec<_>>(),
        "title": d.title.as_ref().map(pj), "value": d.value_pos.as_ref().map(pj),
        "pages": d.options, "options": d.flag_names,
        "alt_sets": d.alt_sets.iter().enumerate().map(|(i, s)| json!({"set": i + 1, "label": crate::symbols::alt_label(s), "texts": s.iter().map(|(n, t)| json!({"text_nr": n, "text": t})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "params": d.params.iter().map(|(n, name, v)| json!({"nr": n, "name": name, "default": v})).collect::<Vec<_>>(),
        "querverweise": d.frg.0, "sicherung": d.frg.1,
    })
}

impl Server {
    fn sym_mut(&mut self) -> Result<&mut SymDef, String> {
        self.sym.as_mut().ok_or_else(|| "Kein Symbol geöffnet (symbol_open oder symbol_new)".to_string())
    }

    /// Symboleditor-Werkzeuge; `None`, wenn der Name keines davon ist.
    pub fn symbol_call(&mut self, name: &str, a: &Value) -> Option<Result<Value, String>> {
        if !name.starts_with("symbol_") {
            return None;
        }
        Some(self.symbol_call_inner(name, a))
    }

    fn symbol_call_inner(&mut self, name: &str, a: &Value) -> Result<Value, String> {
        match name {
            "symbol_open" => {
                let key = s(a, "key");
                let nm = s(a, "name");
                let found = self
                    .project
                    .symdefs
                    .iter()
                    .chain(self.lib.iter())
                    .find(|d| key.map(|k| d.key == k).unwrap_or(false) || nm.map(|n| d.name == n).unwrap_or(false))
                    .cloned()
                    .ok_or("Symbol nicht gefunden")?;
                let mut d = found;
                d.normalize();
                self.sym = Some(d);
                self.call("symbol_get", &json!({}))
            }
            "symbol_new" => {
                let nm = s(a, "name").ok_or("Parameter 'name' fehlt")?;
                let d = SymDef::empty(&format!("proj:{nm}"), nm, s(a, "bmk").unwrap_or("-A%C"));
                self.sym = Some(d);
                self.call("symbol_get", &json!({}))
            }
            "symbol_get" => Ok(symbol_json(self.sym.as_ref().ok_or("Kein Symbol geöffnet")?)),
            "symbol_add_element" => {
                let ty = s(a, "type").ok_or("Parameter 'type' fehlt")?.to_string();
                let p = pts(a)?;
                let d = self.sym_mut()?;
                let vis = (int(a, "page").unwrap_or(0).clamp(0, 255) as u16, int(a, "page_mode").unwrap_or(1).clamp(0, 2) as u16);
                let first = *p.first().ok_or("mindestens ein Punkt")?;
                let idx = match ty.as_str() {
                    "pin" => {
                        d.normalize();
                        d.pins.push(first);
                        d.pin_vis.push(vis);
                        d.raw.clear();
                        json!({"pin": d.pins.len() - 1})
                    }
                    "title" => {
                        d.title = Some(first);
                        d.raw.clear();
                        json!({"title": pj(&first)})
                    }
                    "value" => {
                        d.value_pos = Some(first);
                        d.raw.clear();
                        json!({"value": pj(&first)})
                    }
                    _ => {
                        let pr = make_prim(&ty, &p, int(a, "style").unwrap_or(0).clamp(0, 3) as u8, s(a, "text").unwrap_or(""), align(a).unwrap_or(Align::Center))?;
                        let i = d.push_prim(pr, vis, int(a, "flag").unwrap_or(0).max(0) as u32, int(a, "text_mode").unwrap_or(0).clamp(0, 2) as u8, a.get("fill").and_then(Value::as_bool).unwrap_or(false));
                        json!({"index": i})
                    }
                };
                Ok(json!({"added": idx, "symbol": symbol_json(self.sym.as_ref().unwrap())}))
            }
            "symbol_update_element" => {
                let d = self.sym_mut()?;
                d.normalize();
                if let Some(k) = int(a, "pin") {
                    let k = k as usize;
                    if k >= d.pins.len() {
                        return Err(format!("Anschluss {k} gibt es nicht"));
                    }
                    if let Ok(p) = pts(a) {
                        d.pins[k] = *p.first().ok_or("Punkt fehlt")?;
                    }
                    if let Some(pg) = int(a, "page") {
                        d.pin_vis[k].0 = pg.clamp(0, 255) as u16;
                    }
                    if let Some(m) = int(a, "page_mode") {
                        d.pin_vis[k].1 = m.clamp(0, 2) as u16;
                    }
                    d.raw.clear();
                    return Ok(symbol_json(d));
                }
                let i = int(a, "index").ok_or("Parameter 'index' oder 'pin' fehlt")? as usize;
                if i >= d.prims.len() {
                    return Err(format!("Element {i} gibt es nicht"));
                }
                let style = int(a, "style").map(|v| v.clamp(0, 3) as u8);
                let new_pts = pts(a).ok();
                let cur = prim_json(d, i);
                let ty = cur["type"].as_str().unwrap_or("line").to_string();
                let old_pts: Vec<P> = cur["points"].as_array().unwrap().iter().map(|v| P::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)).collect();
                let text = s(a, "text").map(String::from).or_else(|| cur.get("text").and_then(Value::as_str).map(String::from)).unwrap_or_default();
                let al = align(a).or_else(|| match &d.prims[i] {
                    Prim::TextBox(_, _, _, al) => Some(*al),
                    _ => None,
                });
                if ty != "arc" {
                    let st = style.unwrap_or(cur["style"].as_u64().unwrap_or(0) as u8);
                    d.prims[i] = make_prim(&ty, new_pts.as_deref().unwrap_or(&old_pts), st, &text, al.unwrap_or(Align::Center))?;
                }
                if let Some(pg) = int(a, "page") {
                    d.vis[i].0 = pg.clamp(0, 255) as u16;
                }
                if let Some(m) = int(a, "page_mode") {
                    d.vis[i].1 = m.clamp(0, 2) as u16;
                }
                if let Some(f) = int(a, "flag") {
                    d.flag[i] = f.max(0) as u32;
                }
                if let Some(m) = int(a, "text_mode") {
                    d.text_mode[i] = m.clamp(0, 2) as u8;
                }
                if let Some(f) = a.get("fill").and_then(Value::as_bool) {
                    d.fill[i] = f;
                }
                d.raw.clear();
                Ok(prim_json(d, i))
            }
            "symbol_delete_element" => {
                let d = self.sym_mut()?;
                d.normalize();
                if let Some(k) = int(a, "pin") {
                    let k = k as usize;
                    if k >= d.pins.len() {
                        return Err(format!("Anschluss {k} gibt es nicht"));
                    }
                    d.pins.remove(k);
                    d.pin_vis.remove(k);
                    d.raw.clear();
                } else {
                    d.remove_prim(int(a, "index").ok_or("Parameter 'index' oder 'pin' fehlt")? as usize)?;
                }
                Ok(symbol_json(d))
            }
            "symbol_pages" => {
                let d = self.sym_mut()?;
                match s(a, "action").unwrap_or("list") {
                    "add" => {
                        let n = d.add_page(s(a, "name").unwrap_or("Seite"));
                        Ok(json!({"page": n, "pages": d.options}))
                    }
                    "rename" => {
                        let i = int(a, "index").ok_or("Parameter 'index' fehlt")? as usize;
                        let o = d.options.get_mut(i).ok_or("Seite nicht vorhanden")?;
                        *o = s(a, "name").ok_or("Parameter 'name' fehlt")?.into();
                        d.raw.clear();
                        Ok(json!({"pages": d.options}))
                    }
                    "delete" => {
                        d.delete_page(int(a, "index").ok_or("Parameter 'index' fehlt")? as u16)?;
                        Ok(json!({"pages": d.options}))
                    }
                    _ => Ok(json!({"pages": d.options})),
                }
            }
            "symbol_options" => {
                let d = self.sym_mut()?;
                match s(a, "action").unwrap_or("list") {
                    "add" => {
                        let n = d.add_flag(s(a, "name").unwrap_or("Option"))?;
                        Ok(json!({"option": n, "options": d.flag_names}))
                    }
                    "rename" => {
                        let i = int(a, "index").ok_or("Parameter 'index' fehlt")? as usize;
                        let o = d.flag_names.get_mut(i.wrapping_sub(1)).ok_or("Zusatzoption nicht vorhanden")?;
                        *o = s(a, "name").ok_or("Parameter 'name' fehlt")?.into();
                        d.raw.clear();
                        Ok(json!({"options": d.flag_names}))
                    }
                    "delete" => {
                        d.delete_flag(int(a, "index").ok_or("Parameter 'index' fehlt")? as u32)?;
                        Ok(json!({"options": d.flag_names}))
                    }
                    _ => Ok(json!({"options": d.flag_names})),
                }
            }
            "symbol_alt_texts" => {
                let d = self.sym_mut()?;
                match s(a, "action").unwrap_or("list") {
                    "set" => {
                        let set = int(a, "set").ok_or("Parameter 'set' fehlt")?.max(0) as usize;
                        let nr = int(a, "text_nr").ok_or("Parameter 'text_nr' fehlt")?.max(0) as u32;
                        d.set_alt_text(set, nr, s(a, "text").unwrap_or(""));
                    }
                    "delete" => d.delete_alt_set(int(a, "set").ok_or("Parameter 'set' fehlt")?.max(0) as usize)?,
                    _ => {}
                }
                Ok(symbol_json(d)["alt_sets"].clone())
            }
            "symbol_set_properties" => {
                let d = self.sym_mut()?;
                if let Some(v) = s(a, "name") {
                    d.name = v.into();
                }
                if let Some(v) = s(a, "bmk") {
                    d.bmk = v.into();
                }
                if let Some(v) = s(a, "group") {
                    d.group = v.split('/').map(String::from).collect();
                }
                if let Some(v) = a.get("querverweise").and_then(Value::as_bool) {
                    d.frg.0 = v;
                }
                if let Some(v) = a.get("sicherung").and_then(Value::as_bool) {
                    d.frg.1 = v;
                }
                d.raw.clear();
                Ok(symbol_json(d))
            }
            "symbol_commit" => {
                let mut d = self.sym.clone().ok_or("Kein Symbol geöffnet")?;
                d.normalize();
                match s(a, "target").unwrap_or("project") {
                    "library" => {
                        d.haupt = if s(a, "library") == Some("standard") { 300 } else { 200 };
                        d.key = format!("user:{}:{}", d.haupt, d.name);
                        if d.group.is_empty() || d.group[0] == "Projekt" {
                            d.group = vec!["Sonstiges".into()];
                        }
                        self.lib.retain(|x| x.key != d.key);
                        self.lib.push(d.clone());
                        let own: Vec<&SymDef> = self.lib.iter().filter(|d| d.key.starts_with("user:")).collect();
                        let path = crate::extra::appdir().join("benutzer_symbole.json");
                        std::fs::write(&path, serde_json::to_string(&own).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                        Ok(json!({"key": d.key, "library": path.display().to_string()}))
                    }
                    _ => {
                        match self.project.symdefs.iter_mut().find(|x| x.key == d.key) {
                            Some(x) => *x = d.clone(),
                            None => self.project.symdefs.push(d.clone()),
                        }
                        self.dirty = true;
                        Ok(json!({"key": d.key, "project_symbols": self.project.symdefs.len()}))
                    }
                }
            }
            "symbol_close" => {
                self.sym = None;
                Ok(json!({"closed": true}))
            }
            _ => Err(format!("unbekanntes Werkzeug {name}")),
        }
    }
}
