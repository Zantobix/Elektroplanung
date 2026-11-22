//! MCP-Schnittstelle (Model Context Protocol, JSON-RPC 2.0 über stdio).
//!
//! Start: `Elektroplanung.exe --mcp`. Ein KI-Client kann damit Projekte öffnen (.epr/.epj),
//! Blätter und Objekte lesen und ändern, Symbole platzieren, im Originalformat speichern
//! und PDFs erzeugen – mit denselben Routinen wie die Oberfläche.

use crate::model::*;
use crate::symbols::{self, SymDef};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub const PROTOCOL: &str = "2025-06-18";

pub struct Server {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub lib: Vec<SymDef>,
    pub lib_info: String,
    pub dirty: bool,
    /// Arbeitskopie des Symboleditors (symbol_open/symbol_new … symbol_commit)
    pub sym: Option<SymDef>,
}

/// Symbolbibliothek wie in der Oberfläche: Original-Datenbank, sonst Zwischenspeicher, sonst Ersatzsymbole.
pub fn load_library(dir: Option<&Path>) -> (Vec<SymDef>, String) {
    for d in symbols::db_candidates(dir) {
        if let Ok(lib) = symbols::load_db(&d) {
            return (lib.clone(), format!("{} Symbole aus {}", lib.len(), d.display()));
        }
    }
    let cache = crate::config_dir().join("bibliothek_cache.json");
    if let Some(lib) = std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<Vec<SymDef>>(&s).ok()).filter(|l| !l.is_empty()) {
        return (lib.clone(), format!("{} Symbole (zwischengespeichert)", lib.len()));
    }
    (symbols::builtin(), "Ersatzsymbole (Original-Datenbank nicht gefunden)".into())
}

/// Projekt aus .epr (Original) oder .epj (JSON des Nachbaus) laden.
pub fn open_project(path: &Path) -> Result<Project, String> {
    let is_epr = crate::vorlage::is_epr_format(path);
    if is_epr {
        crate::epr::import(path)
    } else {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&s).map_err(|e| e.to_string())
    }
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": props, "required": required }
    })
}

fn sheet_props() -> Value {
    json!({
        "anlage": { "type": "integer", "description": "Index der Anlage (0-basiert)" },
        "sheet": { "type": "integer", "description": "Index des Blattes in der Anlage (0-basiert)" }
    })
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(a), Value::Object(b)) = (a.as_object_mut(), b) {
        a.extend(b);
    }
    a
}

pub fn tool_list() -> Vec<Value> {
    let mut v = vec![
        tool("open_project", "Öffnet ein Projekt (*.epr Originalformat oder *.epj).", json!({"path": {"type": "string"}}), &["path"]),
        tool("new_project", "Legt ein neues, leeres Projekt an (wie Datei › Neu › Projekt).", json!({"name": {"type": "string"}}), &[]),
        tool("project_info", "Projektdaten: Name, Datum, Anlagen, Blätter, Potentiale, Leitungen, Symbolanzahl.", json!({}), &[]),
        tool("set_project_info", "Projektdaten ändern (Name, Kunde, Eigentümer, Von, Ort, Zeichnungsnummer).",
            json!({"name": {"type":"string"}, "kunde": {"type":"string"}, "owner": {"type":"string"}, "von": {"type":"string"}, "ort": {"type":"string"}, "znr": {"type":"string"}}), &[]),
        tool("list_sheets", "Alle Anlagen und Blätter mit Nummer, Name, Format und Objektanzahl.", json!({}), &[]),
        tool("get_sheet", "Alle Objekte eines Blattes (Id, Art, Koordinaten in mm, Texte, BMK).", sheet_props(), &["anlage", "sheet"]),
        tool("add_anlage", "Neue Anlage mit einem Blatt anlegen.", json!({"name": {"type": "string"}}), &[]),
        tool("add_sheet", "Neues Blatt in einer Anlage anlegen.", json!({"anlage": {"type": "integer"}, "name": {"type": "string"}, "portrait": {"type": "boolean"}}), &["anlage"]),
        tool("set_sheet", "Blatt umbenennen, Nummer oder Format ändern.",
            merge(sheet_props(), json!({"name": {"type":"string"}, "number": {"type":"integer"}, "portrait": {"type":"boolean"}})), &["anlage", "sheet"]),
        tool("add_object", "Zeichenobjekt einfügen. kind: line|rect|circle|curve|text|textbox|wire. points: [[x,y],…] in mm             (textbox: zwei Ecken; Vorgabe wie im Original Rahmen an, Umbruch aus, 8 pt).",
            merge(sheet_props(), json!({
                "kind": {"type": "string", "enum": ["line", "rect", "circle", "curve", "text", "textbox", "wire"]},
                "points": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}},
                "text": {"type": "string"}, "size": {"type": "number", "description": "Schrifthöhe in mm"},
                "vertical": {"type": "boolean"}, "pen": {"type": "integer"},
                "frame": {"type": "boolean", "description": "Rahmen (text, textbox)"}, "wrap": {"type": "boolean", "description": "Umbruch (textbox)"}
            })), &["anlage", "sheet", "kind", "points"]),
        tool("place_symbol", "Symbol aus Projekt oder Bibliothek platzieren (key aus list_symbols).",
            merge(sheet_props(), json!({
                "key": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"},
                "bmk": {"type": "string"}, "value": {"type": "string"}, "rot": {"type": "integer", "description": "Drehung in 90°-Schritten"}, "option": {"type": "integer", "description": "Bildvariante (Seite), z. B. 1 = Taster beim Schließer"},
                "flags": {"type": "integer", "description": "Zusatzoptionen als Bitmaske (z. B. Steckdose: 1 = Ports, 2 = SPD)"}
            })), &["anlage", "sheet", "key", "x", "y"]),
        tool("update_object", "Objekt ändern: verschieben (dx, dy), Text, BMK, Wert, Drehung.",
            merge(sheet_props(), json!({
                "id": {"type": "integer"}, "dx": {"type": "number"}, "dy": {"type": "number"},
                "text": {"type": "string"}, "bmk": {"type": "string"}, "value": {"type": "string"}, "rot": {"type": "integer"}
            })), &["anlage", "sheet", "id"]),
        tool("delete_object", "Objekt löschen.", merge(sheet_props(), json!({"id": {"type": "integer"}})), &["anlage", "sheet", "id"]),
        tool("list_symbols", "Symbole aus Projekt und Bibliothek suchen.", json!({"filter": {"type": "string"}, "limit": {"type": "integer"}}), &[]),
        tool("save_project", "Projekt speichern. format: epr (Original, Standard bei .epr) oder epj.",
            json!({"path": {"type": "string"}, "format": {"type": "string", "enum": ["epr", "epj"]}}), &[]),
        tool("export_pdf", "Alle Blätter als Vektor-PDF ausgeben (wie Datei › PDF Export).", json!({"path": {"type": "string"}}), &["path"]),
        tool("verify_epr_roundtrip", "Lädt eine .epr-Datei, schreibt sie mit dem Nachbau neu und vergleicht Byte für Byte.",
            json!({"path": {"type": "string"}}), &["path"]),
    ];
    v.extend(crate::mcp_symbol::tool_list());
    v
}

fn obj_json(o: &Obj) -> Value {
    let p = |p: &P| json!([p.x, p.y]);
    let mut v = json!({ "id": o.id, "pen": o.pen, "kind": crate::epr_write::kind_name(&o.kind), "from_epr": o.raw.is_some() });
    let m = v.as_object_mut().unwrap();
    match &o.kind {
        Kind::Line { a, b } | Kind::Rect { a, b } | Kind::Triangle { a, b } | Kind::Circle { a, b } => {
            m.insert("points".into(), json!([p(a), p(b)]));
        }
        Kind::Curve { a, c1, c2, b } => {
            m.insert("points".into(), json!([p(a), p(c1), p(c2), p(b)]));
        }
        Kind::Text { pos, text, size, vertical, style, .. } => {
            m.insert("points".into(), json!([p(pos)]));
            m.insert("text".into(), json!(text));
            m.insert("size".into(), json!(size));
            m.insert("vertical".into(), json!(vertical));
            // Textbox (ZCN 11): Box, Rahmen, Umbruch
            if let Some((a, b)) = &style.rect {
                m.insert("box".into(), json!([p(a), p(b)]));
                m.insert("frame".into(), json!(style.frame));
                m.insert("wrap".into(), json!(style.wrap));
            }
        }
        Kind::Busbar { a, x2, name, .. } => {
            m.insert("points".into(), json!([p(a), [x2, a.y]]));
            m.insert("text".into(), json!(name));
        }
        Kind::Spiegel { pos, owner, rows, .. } => {
            m.insert("points".into(), json!([p(pos)]));
            m.insert("owner".into(), json!(owner));
            m.insert("contacts".into(), json!(rows.iter().map(|z| z.referenz).collect::<Vec<_>>()));
        }
        Kind::Table { a, b, rows, cols, .. } => {
            m.insert("points".into(), json!([p(a), p(b)]));
            m.insert("rows".into(), json!(rows));
            m.insert("cols".into(), json!(cols));
        }
        Kind::Image { a, b, path, .. } => {
            m.insert("points".into(), json!([p(a), p(b)]));
            m.insert("path".into(), json!(path));
        }
        Kind::Symbol { key, pos, rot, mirror_h, mirror_v, bmk, value, scale, option, flags, .. } => {
            m.insert("points".into(), json!([p(pos)]));
            m.insert("key".into(), json!(key));
            m.insert("bmk".into(), json!(bmk));
            m.insert("value".into(), json!(value));
            m.insert("rot".into(), json!(rot));
            m.insert("mirror_h".into(), json!(mirror_h));
            m.insert("mirror_v".into(), json!(mirror_v));
            m.insert("scale".into(), json!(scale));
            m.insert("option".into(), json!(option));
            m.insert("flags".into(), json!(flags));
        }
        Kind::Poly { pts } | Kind::Wire { pts, .. } => {
            m.insert("points".into(), Value::Array(pts.iter().map(p).collect()));
        }
        Kind::Verweis { pos, text_pos, text, dir, vertical, spitze, ziel } => {
            m.insert("points".into(), json!([p(pos), p(text_pos)]));
            m.insert("text".into(), json!(text));
            m.insert("dir".into(), json!(dir));
            m.insert("vertical".into(), json!(vertical));
            m.insert("spitze".into(), json!(spitze));
            if let Some(z) = ziel {
                m.insert("ziel".into(), json!({ "anlage": z.anlage, "blatt": z.blatt, "obj": z.obj }));
            }
        }
        Kind::Terminal { pos, leiste, nr, .. } => {
            m.insert("points".into(), json!([p(pos)]));
            m.insert("leiste".into(), json!(leiste));
            m.insert("nr".into(), json!(nr));
        }
    }
    v
}

fn arg_usize(a: &Value, k: &str) -> Result<usize, String> {
    a.get(k).and_then(Value::as_u64).map(|v| v as usize).ok_or_else(|| format!("Parameter '{k}' fehlt"))
}
fn arg_f32(a: &Value, k: &str) -> Option<f32> {
    a.get(k).and_then(Value::as_f64).map(|v| v as f32)
}
fn arg_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

fn points(a: &Value) -> Result<Vec<P>, String> {
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

impl Server {
    pub fn new() -> Self {
        let (lib, lib_info) = load_library(None);
        Server { project: Project::new_default(), path: None, lib, lib_info, dirty: false, sym: None }
    }

    fn sheet_ref(&self, a: &Value) -> Result<SheetRef, String> {
        let r = SheetRef { anlage: arg_usize(a, "anlage")?, sheet: arg_usize(a, "sheet")? };
        self.project.sheet(r).ok_or("Blatt nicht vorhanden")?;
        Ok(r)
    }

    fn all_symbols(&self) -> Vec<&SymDef> {
        let mut v: Vec<&SymDef> = self.project.symdefs.iter().collect();
        v.extend(self.lib.iter().filter(|d| !self.project.symdefs.iter().any(|p| p.key == d.key)));
        v
    }

    /// Ein Werkzeug ausführen; Ergebnis als JSON-Wert.
    pub fn call(&mut self, name: &str, a: &Value) -> Result<Value, String> {
        if let Some(r) = self.symbol_call(name, a) {
            return r;
        }
        match name {
            "open_project" => {
                let path = PathBuf::from(arg_str(a, "path").ok_or("Parameter 'path' fehlt")?);
                self.project = open_project(&path)?;
                self.path = Some(path);
                self.dirty = false;
                self.call("project_info", &json!({}))
            }
            "new_project" => {
                self.project = Project::new_default();
                if let Some(n) = arg_str(a, "name") {
                    self.project.name = n.into();
                }
                self.path = None;
                self.dirty = true;
                self.call("project_info", &json!({}))
            }
            "project_info" => {
                let p = &self.project;
                Ok(json!({
                    "path": self.path.as_ref().map(|p| p.display().to_string()),
                    "name": p.name, "kunde": p.kunde, "owner": p.owner, "von": p.von, "ort": p.ort, "znr": p.znr,
                    "erstellt": p.erstellt, "geaendert": p.geaendert, "abgeschlossen": p.abgeschlossen,
                    "anlagen": p.anlagen.iter().map(|a| json!({"name": a.name, "sheets": a.sheets.len(), "sammelschienen": a.sammelschienen, "klemmleisten": a.klemmleisten})).collect::<Vec<_>>(),
                    "sheets_total": p.total_sheets(),
                    "potentiale": p.potentiale,
                    "leitungen": p.leitungen.iter().map(|l| l.name.clone()).collect::<Vec<_>>(),
                    "project_symbols": p.symdefs.len(),
                    "library": self.lib_info,
                    "epr_source": p.raw.is_some(),
                    "unsaved_changes": self.dirty,
                }))
            }
            "set_project_info" => {
                let p = &mut self.project;
                for (k, f) in [("name", &mut p.name), ("kunde", &mut p.kunde), ("owner", &mut p.owner), ("von", &mut p.von), ("ort", &mut p.ort), ("znr", &mut p.znr)] {
                    if let Some(v) = arg_str(a, k) {
                        *f = v.into();
                    }
                }
                self.dirty = true;
                self.call("project_info", &json!({}))
            }
            "list_sheets" => Ok(Value::Array(
                self.project
                    .anlagen
                    .iter()
                    .enumerate()
                    .map(|(ai, an)| {
                        json!({
                            "anlage": ai, "name": an.name,
                            "sheets": an.sheets.iter().enumerate().map(|(si, s)| json!({
                                "sheet": si, "number": s.number, "name": s.name, "portrait": s.portrait, "objects": s.objects.len()
                            })).collect::<Vec<_>>()
                        })
                    })
                    .collect(),
            )),
            "get_sheet" => {
                let r = self.sheet_ref(a)?;
                let s = self.project.sheet(r).unwrap();
                let (w, h) = s.dims();
                Ok(json!({ "number": s.number, "name": s.name, "width_mm": w, "height_mm": h, "frame_mm": s.frame(), "columns": s.columns,
                    "objects": s.objects.iter().map(obj_json).collect::<Vec<_>>() }))
            }
            "add_anlage" => {
                let n = self.project.anlagen.len() + 1;
                let name = arg_str(a, "name").map(String::from).unwrap_or_else(|| format!("Anlage {n}"));
                self.project.anlagen.push(Anlage { name, sheets: vec![Sheet::new(1)], sammelschienen: vec![], klemmleisten: crate::klemmen::erste_leiste().0, klemmen: crate::klemmen::erste_leiste().1, schienen: Default::default(), set: Default::default(), raw: None });
                self.dirty = true;
                Ok(json!({ "anlage": n - 1 }))
            }
            "add_sheet" => {
                let ai = arg_usize(a, "anlage")?;
                let an = self.project.anlagen.get_mut(ai).ok_or("Anlage nicht vorhanden")?;
                let number = an.sheets.iter().map(|s| s.number).max().unwrap_or(0) + 1;
                let mut s = Sheet::new(number);
                if let Some(n) = arg_str(a, "name") {
                    s.name = n.into();
                }
                s.portrait = a.get("portrait").and_then(Value::as_bool).unwrap_or(false);
                an.sheets.push(s);
                self.dirty = true;
                Ok(json!({ "anlage": ai, "sheet": an.sheets.len() - 1, "number": number }))
            }
            "set_sheet" => {
                let r = self.sheet_ref(a)?;
                let s = self.project.sheet_mut(r).unwrap();
                if let Some(n) = arg_str(a, "name") {
                    s.name = n.into();
                }
                if let Some(n) = a.get("number").and_then(Value::as_u64) {
                    s.number = n as u32;
                }
                if let Some(p) = a.get("portrait").and_then(Value::as_bool) {
                    s.portrait = p;
                }
                self.dirty = true;
                Ok(json!({ "ok": true }))
            }
            "add_object" => {
                let r = self.sheet_ref(a)?;
                let pts = points(a)?;
                let need = |n: usize| if pts.len() < n { Err(format!("mindestens {n} Punkte nötig")) } else { Ok(()) };
                let kind = match arg_str(a, "kind").ok_or("Parameter 'kind' fehlt")? {
                    "line" => {
                        need(2)?;
                        Kind::Line { a: pts[0], b: pts[1] }
                    }
                    "rect" => {
                        need(2)?;
                        Kind::Rect { a: pts[0], b: pts[1] }
                    }
                    "circle" => {
                        need(2)?;
                        Kind::Circle { a: pts[0], b: pts[1] }
                    }
                    "curve" => {
                        need(4)?;
                        Kind::Curve { a: pts[0], c1: pts[1], c2: pts[2], b: pts[3] }
                    }
                    "wire" => {
                        need(2)?;
                        Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 }
                    }
                    "text" => {
                        need(1)?;
                        Kind::Text {
                            pos: pts[0],
                            text: arg_str(a, "text").unwrap_or_default().into(),
                            size: arg_f32(a, "size").unwrap_or(3.5),
                            align: match arg_str(a, "align") {
                                Some("center") | Some("mittig") => Align::Center,
                                Some("right") | Some("rechts") => Align::Right,
                                _ => Align::Left,
                            },
                            vertical: a.get("vertical").and_then(Value::as_bool).unwrap_or(false),
                            style: TextStyle {
                                frame: a.get("frame").and_then(Value::as_bool).unwrap_or(false),
                                bold: a.get("bold").and_then(Value::as_bool).unwrap_or(false),
                                italic: a.get("italic").and_then(Value::as_bool).unwrap_or(false),
                                rotation: a.get("rotation").and_then(Value::as_i64).unwrap_or(0) as i16,
                                ..Default::default()
                            },
                        }
                    }
                    "textbox" => {
                        need(2)?;
                        let flag = |k: &str| a.get(k).and_then(Value::as_bool);
                        let mut k = crate::textbox::neu(pts[0], pts[1], arg_str(a, "text").unwrap_or("Text"));
                        if let Kind::Text { size, style, .. } = &mut k {
                            *size = arg_f32(a, "size").unwrap_or(*size);
                            style.frame = flag("frame").unwrap_or(style.frame);
                            style.wrap = flag("wrap").unwrap_or(style.wrap);
                        }
                        k
                    }
                    k => return Err(format!("unbekannte Art '{k}'")),
                };
                let pen = a.get("pen").and_then(Value::as_u64).unwrap_or(0) as usize;
                let id = self.project.alloc_id();
                self.project.sheet_mut(r).unwrap().objects.push(Obj { id, kind, pen, fixed: false, group: None, raw: None });
                self.dirty = true;
                Ok(json!({ "id": id }))
            }
            "place_symbol" => {
                let r = self.sheet_ref(a)?;
                let key = arg_str(a, "key").ok_or("Parameter 'key' fehlt")?;
                let def = self.all_symbols().into_iter().find(|d| d.key == key).cloned().ok_or_else(|| format!("Symbol '{key}' nicht gefunden"))?;
                let pos = P::new(arg_f32(a, "x").ok_or("x fehlt")?, arg_f32(a, "y").ok_or("y fehlt")?);
                let sheet_no = self.project.sheet(r).unwrap().number;
                let n = self.project.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(&o.kind, Kind::Symbol { key: k, .. } if *k == def.key)).count() as u32 + 1;
                let bmk = arg_str(a, "bmk").map(String::from).unwrap_or_else(|| symbols::expand_bmk(&def.bmk, n, sheet_no));
                if !self.project.symdefs.iter().any(|d| d.key == def.key) {
                    self.project.symdefs.push(def.clone());
                }
                let kind = Kind::Symbol {
                    key: def.key.clone(),
                    pos,
                    rot: a.get("rot").and_then(Value::as_u64).unwrap_or(0) as u8 % 4,
                    mirror_h: false,
                    mirror_v: false,
                    bmk: bmk.clone(),
                    value: arg_str(a, "value").unwrap_or_default().into(),
                    scale: 1.0,
                    option: a.get("option").and_then(Value::as_u64).unwrap_or(0) as u16,
                    title: Some((crate::app::symbol_title_rot(&def, (a.get("rot").and_then(Value::as_u64).unwrap_or(0) % 4) as u8, a.get("option").and_then(Value::as_u64).unwrap_or(0) as u16).0, false)),
                    flags: a.get("flags").and_then(Value::as_u64).unwrap_or(0) as u32,
                    ext: SymExt { alt: a.get("alt").and_then(Value::as_u64).unwrap_or(0) as u16, title_align: crate::app::symbol_title_rot(&def, (a.get("rot").and_then(Value::as_u64).unwrap_or(0) % 4) as u8, a.get("option").and_then(Value::as_u64).unwrap_or(0) as u16).1, ..Default::default() },
                };
                let id = self.project.alloc_id();
                self.project.sheet_mut(r).unwrap().objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None });
                self.dirty = true;
                Ok(json!({ "id": id, "bmk": bmk, "name": def.name }))
            }
            "update_object" => {
                let r = self.sheet_ref(a)?;
                let id = a.get("id").and_then(Value::as_u64).ok_or("Parameter 'id' fehlt")?;
                let o = self.project.sheet_mut(r).unwrap().objects.iter_mut().find(|o| o.id == id).ok_or("Objekt nicht gefunden")?;
                let (dx, dy) = (arg_f32(a, "dx").unwrap_or(0.0), arg_f32(a, "dy").unwrap_or(0.0));
                if dx != 0.0 || dy != 0.0 {
                    o.kind.translate(P::new(dx, dy));
                }
                match &mut o.kind {
                    Kind::Text { text, .. } | Kind::Verweis { text, .. } => {
                        if let Some(t) = arg_str(a, "text") {
                            *text = t.into();
                        }
                    }
                    Kind::Symbol { bmk, value, rot, .. } => {
                        if let Some(t) = arg_str(a, "bmk") {
                            *bmk = t.into();
                        }
                        if let Some(t) = arg_str(a, "value") {
                            *value = t.into();
                        }
                        if let Some(rr) = a.get("rot").and_then(Value::as_u64) {
                            *rot = rr as u8 % 4;
                        }
                    }
                    _ => {}
                }
                let v = obj_json(o);
                self.dirty = true;
                Ok(v)
            }
            "delete_object" => {
                let r = self.sheet_ref(a)?;
                let id = a.get("id").and_then(Value::as_u64).ok_or("Parameter 'id' fehlt")?;
                let s = self.project.sheet_mut(r).unwrap();
                let before = s.objects.len();
                s.objects.retain(|o| o.id != id);
                if s.objects.len() == before {
                    return Err("Objekt nicht gefunden".into());
                }
                self.dirty = true;
                Ok(json!({ "deleted": id }))
            }
            "list_symbols" => {
                let f = arg_str(a, "filter").unwrap_or("").to_lowercase();
                let limit = a.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
                let all = self.all_symbols();
                let hits: Vec<Value> = all
                    .iter()
                    .filter(|d| f.is_empty() || d.name.to_lowercase().contains(&f) || d.group.join("/").to_lowercase().contains(&f))
                    .take(limit)
                    .map(|d| json!({ "key": d.key, "name": d.name, "group": d.group.join(" / "), "bmk": d.bmk, "variants": d.options, "flag_options": d.flag_names, "in_project": self.project.symdefs.iter().any(|p| p.key == d.key) }))
                    .collect();
                Ok(json!({ "total": all.len(), "symbols": hits }))
            }
            "save_project" => {
                let path = arg_str(a, "path").map(PathBuf::from).or_else(|| self.path.clone()).ok_or("Kein Dateiname")?;
                let fmt = arg_str(a, "format").map(String::from).unwrap_or_else(|| path.extension().and_then(|e| e.to_str()).unwrap_or("epj").to_lowercase());
                let mut warnings = vec![];
                if fmt == "epr" {
                    warnings = crate::epr_write::unsupported(&self.project);
                    warnings.extend(crate::epr_projekt::nicht_gespeichert(&self.project));
                    crate::epr_write::save(&self.project, &path)?;
                } else {
                    let s = serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string())?;
                    std::fs::write(&path, s).map_err(|e| e.to_string())?;
                }
                self.path = Some(path.clone());
                self.dirty = false;
                Ok(json!({ "saved": path.display().to_string(), "format": fmt, "not_written": warnings }))
            }
            "export_pdf" => {
                let path = PathBuf::from(arg_str(a, "path").ok_or("Parameter 'path' fehlt")?);
                let mut lib = self.lib.clone();
                lib.extend(self.project.symdefs.iter().filter(|d| !self.lib.iter().any(|l| l.key == d.key)).cloned());
                let (bytes, n) = crate::pdf::render_headless(&self.project, &lib);
                std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
                Ok(json!({ "pdf": path.display().to_string(), "pages": n, "bytes": bytes.len() }))
            }
            "verify_epr_roundtrip" => {
                let path = PathBuf::from(arg_str(a, "path").ok_or("Parameter 'path' fehlt")?);
                let orig = std::fs::read(&path).map_err(|e| e.to_string())?;
                let p = crate::epr::import(&path)?;
                let out = crate::epr_write::to_bytes(&p)?;
                let first_diff = orig.iter().zip(&out).position(|(x, y)| x != y).or(if orig.len() != out.len() { Some(orig.len().min(out.len())) } else { None });
                Ok(json!({
                    "identical": first_diff.is_none(), "original_bytes": orig.len(), "written_bytes": out.len(), "first_difference_at": first_diff,
                    "blocks": p.raw.as_ref().map(|f| f.roots.iter().map(|b| b.count()).sum::<usize>()),
                    "sheets": p.total_sheets(), "objects": p.anlagen.iter().flat_map(|a| &a.sheets).map(|s| s.objects.len()).sum::<usize>(),
                }))
            }
            _ => Err(format!("Unbekanntes Werkzeug '{name}'")),
        }
    }

    /// Eine JSON-RPC-Nachricht verarbeiten; None bei Benachrichtigungen.
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        id.as_ref()?;
        let id = id.unwrap();
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": msg.pointer("/params/protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "elektroplanung", "title": "Elektroplanung", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Elektroplanung: Stromlauf- und Installationspläne. Erst open_project oder new_project, dann list_sheets/get_sheet. Koordinaten in mm auf dem Blatt (A4 quer 297×210). Speichern im Originalformat mit save_project format=epr."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_list() })),
            "tools/call" => {
                let name = msg.pointer("/params/name").and_then(Value::as_str).unwrap_or("");
                let args = msg.pointer("/params/arguments").cloned().unwrap_or(json!({}));
                Ok(match self.call(name, &args) {
                    Ok(v) => json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap() }], "structuredContent": if v.is_object() { v.clone() } else { json!({ "result": v }) }, "isError": false }),
                    Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                })
            }
            _ => Err(json!({ "code": -32601, "message": format!("Methode '{method}' nicht unterstützt") })),
        };
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
        })
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP-Server auf stdin/stdout (eine JSON-Nachricht je Zeile).
pub fn run_stdio() {
    let mut server = Server::new();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let r: Vec<Value> = batch.iter().filter_map(|m| server.handle(m)).collect();
                (!r.is_empty()).then(|| Value::Array(r))
            }
            Ok(m) => server.handle(&m),
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": e.to_string() } })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}
