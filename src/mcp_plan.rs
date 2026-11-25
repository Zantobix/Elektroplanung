//! MCP-Werkzeuge für die Planobjekte, die `add_object` nicht abdeckt, und für Projektlisten: Sammelschiene,
//! Klemmstelle (teilt die Leitung wie das Werkzeug der Oberfläche), Verweis, Tabelle, Leitungsart und
//! Verbindungsbezeichnungen einer Leitung, Potentiale, Leitungsarten, Klemmleisten, Bauteile (anlegen, ändern,
//! löschen, Objekten zuordnen) und „Tools › Projekt prüfen“.
//!
//! Anlass: Nachzeichnen von W2 über MCP (25.11.2026) – 236 der 2052 Objekte (Schienen, Klemmstellen, Verweise,
//! Tabellen) und alle Bauteile ließen sich vorher nicht anlegen.

use crate::mcp::{arg_f32, arg_str, arg_usize, obj_json, points, Server};
use crate::model::*;
use serde_json::{json, Value};

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": props, "required": required } })
}

fn blatt() -> Value {
    json!({
        "anlage": { "type": "integer", "description": "Index der Anlage (0-basiert)" },
        "sheet": { "type": "integer", "description": "Index des Blattes in der Anlage (0-basiert)" }
    })
}

fn mit(mut a: Value, b: Value) -> Value {
    if let (Some(a), Value::Object(b)) = (a.as_object_mut(), b) {
        a.extend(b);
    }
    a
}

fn bauteil_props() -> Value {
    json!({
        "gruppe": {"type": "string"}, "name": {"type": "string"}, "typ": {"type": "string"}, "firma": {"type": "string"},
        "artikel": {"type": "string", "description": "Artikelnummer"}, "breite": {"type": "string"}, "hoehe": {"type": "string"},
        "tiefe": {"type": "string"}, "nummer": {"type": "string", "description": "interne Nummer"}
    })
}

/// Werkzeugbeschreibungen (werden an `tools/list` angehängt).
pub fn tool_list() -> Vec<Value> {
    vec![
        tool("add_busbar", "Sammelschiene auf ein Blatt legen. name = Kürzel (z. B. L1, PE); eine neue Schiene wird in der Anlage angelegt. Ohne x1/x2 über die Blattbreite wie der Werkzeugknopf; auto (Standard) stapelt die Höhe wie im Original, sonst y.",
            mit(blatt(), json!({
                "name": {"type": "string"}, "title": {"type": "string", "description": "Name der neuen Schiene (Standard: wie Kürzel)"},
                "x1": {"type": "number"}, "x2": {"type": "number"}, "y": {"type": "number"},
                "auto": {"type": "boolean"}, "unten": {"type": "boolean", "description": "Auto-Stapel von unten (z. B. PE)"},
                "potential": {"type": "string"},
                "versatz": {"type": "number", "description": "Y-Versatz im Auto-Stapel in mm"},
                "label": {"type": "array", "description": "Lage der Beschriftung [dx, dy] zum Anfang, Standard [0, -4]"}
            })), &["anlage", "sheet", "name"]),
        tool("add_terminal", "Klemmstelle setzen; liegt sie auf einer Leitung, wird die Leitung wie in der Oberfläche geteilt. Ohne nr die nächste freie Nummer der Leiste.",
            mit(blatt(), json!({"x": {"type": "number"}, "y": {"type": "number"}, "leiste": {"type": "string", "description": "Klemmleiste, Standard die erste der Anlage"}, "nr": {"type": "integer"}, "split": {"type": "boolean", "description": "Leitung teilen (Standard an)"}})),
            &["anlage", "sheet", "x", "y"]),
        tool("add_reference", "Verweis (Dreieck mit Text) setzen; optional Ziel auf ein Objekt eines Blattes.",
            mit(blatt(), json!({"x": {"type": "number"}, "y": {"type": "number"}, "text": {"type": "string"}, "dir": {"type": "integer", "description": "Richtung der Spitze in Grad: 0, 90, 180, 270"},
                "text_x": {"type": "number"}, "text_y": {"type": "number"}, "ziel_anlage": {"type": "string", "description": "Name der Zielanlage"}, "ziel_blatt": {"type": "integer", "description": "Blattnummer"}, "ziel_obj": {"type": "integer"}})),
            &["anlage", "sheet", "x", "y", "text"]),
        tool("add_table", "Tabelle einfügen: points = zwei Ecken, cells zeilenweise.",
            mit(blatt(), json!({"points": {"type": "array"}, "rows": {"type": "integer"}, "cols": {"type": "integer"}, "cells": {"type": "array", "items": {"type": "string"}}, "header": {"type": "boolean"}})),
            &["anlage", "sheet", "points", "rows", "cols"]),
        tool("set_wire", "Leitung ändern: Leitungsart (Index oder Name aus project_info.leitungen), Verbindungsbezeichnungen bez = [[x, y, Text], …] (z. B. Aderfarbe), Punkte.",
            mit(blatt(), json!({"id": {"type": "integer"}, "leitung": {"description": "Index oder Name"}, "bez": {"type": "array"}, "points": {"type": "array"}, "opts": {"type": "integer", "description": "Bits: Zielpfeil Anfang/Ende, Brücke, Hochkant, Unterstrich, Markierung (wie get_sheet)"}})),
            &["anlage", "sheet", "id"]),
        tool("add_potential", "Potential anlegen (Projekt › Potentiale).", json!({"name": {"type": "string"}}), &["name"]),
        tool("add_leitung", "Leitungsart anlegen (Projekt › Leitungen): Farbe [r, g, b], Breite in mm, Strichart 0–3, Adern.",
            json!({"name": {"type": "string"}, "color": {"type": "array"}, "width": {"type": "number"}, "style": {"type": "integer"}, "quer": {"type": "number"}, "adern": {"type": "integer"}}), &["name"]),
        tool("add_klemmleiste", "Klemmleiste in einer Anlage anlegen.", json!({"anlage": {"type": "integer"}, "name": {"type": "string"}}), &["anlage"]),
        tool("list_parts", "Bauteile des Projekts (Index, Daten) und ihre Zuordnung zu Objekten.", json!({}), &[]),
        tool("list_part_catalog", "Bauteilgruppen und Hersteller der Bibliothek: nur diese Werte speichert das Originalformat (.epr) bei gruppe/firma.", json!({}), &[]),
        tool("add_part", "Bauteil anlegen (Dialog Bauteil: Gruppe, Name, Typ, Firma, Artikel, Maße, Nummer).", bauteil_props(), &["name"]),
        tool("update_part", "Bauteil ändern (nur angegebene Felder).", mit(json!({"index": {"type": "integer"}}), bauteil_props()), &["index"]),
        tool("delete_part", "Bauteil löschen (auch aus allen Zuordnungen).", json!({"index": {"type": "integer"}}), &["index"]),
        tool("assign_parts", "Bauteile einem Objekt (Symbol, Klemmstelle) zuordnen: parts = Liste von Bauteil-Indizes in Listenreihenfolge, leer = Zuordnung entfernen.",
            json!({"id": {"type": "integer"}, "parts": {"type": "array", "items": {"type": "integer"}}}), &["id", "parts"]),
        tool("add_contact_mirror", "Kontaktspiegel unter eine Spule setzen: owner = Objekt-Id der Spule; Zeilen = Kontakte gleicher Kennzeichnung (auto führt sie nach). Ohne owner frei an x/y.",
            mit(blatt(), json!({"owner": {"type": "integer"}, "x": {"type": "number"}, "y": {"type": "number"}, "auto": {"type": "boolean"}})), &["anlage", "sheet"]),
        tool("check_project", "Tools › Projekt prüfen: offene Anschlüsse, doppelte BMK, gleiches Potential getrennt.", json!({}), &[]),
    ]
}

fn ja(a: &Value, k: &str) -> Option<bool> {
    a.get(k).and_then(Value::as_bool)
}

impl Server {
    /// Planobjekt- und Listenwerkzeuge; `None`, wenn der Name keines davon ist.
    pub fn plan_call(&mut self, name: &str, a: &Value) -> Option<Result<Value, String>> {
        let r = match name {
            "add_busbar" => self.busbar(a),
            "add_terminal" => self.terminal(a),
            "add_reference" => self.reference(a),
            "add_table" => self.table(a),
            "set_wire" => self.set_wire(a),
            "add_potential" => {
                let n = arg_str(a, "name").unwrap_or_default().trim().to_string();
                if n.is_empty() {
                    return Some(Err("Name fehlt".into()));
                }
                if !self.project.potentiale.contains(&n) {
                    self.project.potentiale.push(n);
                }
                self.dirty = true;
                Ok(json!({ "potentiale": self.project.potentiale }))
            }
            "add_leitung" => {
                let c = a.get("color").and_then(Value::as_array).map(|v| v.iter().map(|x| x.as_u64().unwrap_or(0).min(255) as u8).collect::<Vec<_>>()).unwrap_or_default();
                let l = Leitung {
                    name: arg_str(a, "name").unwrap_or("Leitung").into(),
                    color: [c.first().copied().unwrap_or(0), c.get(1).copied().unwrap_or(0), c.get(2).copied().unwrap_or(0)],
                    width_mm: arg_f32(a, "width").unwrap_or(0.25),
                    quer: arg_f32(a, "quer").unwrap_or(0.0),
                    adern: a.get("adern").and_then(Value::as_u64).unwrap_or(0) as u32,
                    kennz: 0,
                    style: a.get("style").and_then(Value::as_u64).unwrap_or(0).min(3) as u8,
                    id: self.project.leitungen.iter().map(|l| l.id).max().unwrap_or(0) + 1,
                };
                self.project.leitungen.push(l);
                self.dirty = true;
                Ok(json!({ "index": self.project.leitungen.len() - 1 }))
            }
            "add_klemmleiste" => (|| {
                let ai = arg_usize(a, "anlage")?;
                self.project.anlagen.get(ai).ok_or("Anlage nicht vorhanden")?;
                let i = crate::klemmen::new_leiste(&mut self.project, ai).ok_or("Klemmleiste nicht angelegt")?;
                if let Some(n) = arg_str(a, "name").filter(|n| !n.trim().is_empty()) {
                    let an = &mut self.project.anlagen[ai];
                    an.klemmleisten[i] = n.trim().into();
                    an.klemmen[i].name = n.trim().into();
                }
                self.dirty = true;
                Ok(json!({ "klemmleisten": self.project.anlagen[ai].klemmleisten }))
            })(),
            "list_parts" => {
                let p = &self.project;
                let zu: Vec<Value> = p.part_of.keys().chain(p.weitere_bauteile.keys()).collect::<std::collections::BTreeSet<_>>().into_iter().map(|id| json!({ "id": id, "parts": crate::bauteilliste::liste(p, *id) })).collect();
                Ok(json!({ "parts": p.bauteile.iter().enumerate().map(|(i, b)| teil_json(i, b)).collect::<Vec<_>>(), "assigned": zu }))
            }
            "list_part_catalog" => {
                let (g, f) = crate::epr_zusatz::katalog();
                Ok(json!({ "gruppen": g, "hersteller": f }))
            }
            "add_part" => {
                let mut b = Bauteil::default();
                teil_setzen(&mut b, a);
                self.project.bauteile.push(b);
                self.dirty = true;
                let i = self.project.bauteile.len() - 1;
                Ok(mit(teil_json(i, &self.project.bauteile[i]), json!({ "warnings": crate::epr_zusatz::bauteil_warnungen(&self.project.bauteile[i]) })))
            }
            "update_part" => (|| {
                let i = arg_usize(a, "index")?;
                let b = self.project.bauteile.get_mut(i).ok_or("Bauteil nicht vorhanden")?;
                teil_setzen(b, a);
                self.dirty = true;
                Ok(mit(teil_json(i, &self.project.bauteile[i]), json!({ "warnings": crate::epr_zusatz::bauteil_warnungen(&self.project.bauteile[i]) })))
            })(),
            "delete_part" => (|| {
                let i = arg_usize(a, "index")?;
                if i >= self.project.bauteile.len() {
                    return Err("Bauteil nicht vorhanden".into());
                }
                self.project.bauteile.remove(i);
                crate::bauteilliste::bauteil_entfernt(&mut self.project, i);
                self.dirty = true;
                Ok(json!({ "deleted": i, "parts": self.project.bauteile.len() }))
            })(),
            "assign_parts" => (|| {
                let id = a.get("id").and_then(Value::as_u64).ok_or("Parameter 'id' fehlt")?;
                let teile: Vec<usize> = a.get("parts").and_then(Value::as_array).ok_or("Parameter 'parts' fehlt")?.iter().filter_map(|v| v.as_u64().map(|v| v as usize)).collect();
                if let Some(t) = teile.iter().find(|t| **t >= self.project.bauteile.len()) {
                    return Err(format!("Bauteil {t} nicht vorhanden"));
                }
                let da = self.project.anlagen.iter().flat_map(|an| &an.sheets).flat_map(|s| &s.objects).any(|o| o.id == id && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. }));
                if !da {
                    return Err("Objekt nicht gefunden (Symbol oder Klemmstelle)".into());
                }
                crate::bauteilliste::setzen(&mut self.project, id, &teile);
                self.dirty = true;
                Ok(json!({ "id": id, "parts": crate::bauteilliste::liste(&self.project, id) }))
            })(),
            "add_contact_mirror" => self.spiegel(a),
            "check_project" => {
                let mut lib = self.lib.clone();
                lib.extend(self.project.symdefs.iter().filter(|d| !self.lib.iter().any(|l| l.key == d.key)).cloned());
                let h = crate::pruefen::pruefen(&self.project, &lib);
                Ok(json!({ "count": h.len(), "hints": h.iter().map(|x| json!({ "anlage": x.sheet.anlage, "sheet": x.sheet.sheet, "id": x.id, "art": x.art.name(), "text": x.text, "pos": [x.pos.x, x.pos.y] })).collect::<Vec<_>>() }))
            }
            _ => return None,
        };
        Some(r)
    }

    fn neu(&mut self, r: SheetRef, kind: Kind) -> u64 {
        let id = self.project.alloc_id();
        self.project.sheet_mut(r).unwrap().objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None });
        self.dirty = true;
        id
    }

    fn busbar(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let name = arg_str(a, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or("Parameter 'name' fehlt")?.to_string();
        let auto = ja(a, "auto").unwrap_or(a.get("y").is_none());
        let unten = ja(a, "unten").unwrap_or(false);
        let an = &mut self.project.anlagen[r.anlage];
        if !an.sammelschienen.contains(&name) {
            an.sammelschienen.push(name.clone());
            let title = arg_str(a, "title").unwrap_or(&name).to_string();
            an.schienen.insert(name.clone(), SchieneSet { name: title, potential: arg_str(a, "potential").unwrap_or_default().into(), pfeile: false, ..Default::default() });
        }
        let pfad = an.set.pfad_hoehe as f32 / 10.0;
        let schriftfeld = self.project.title_h();
        let s = self.project.sheet(r).unwrap();
        let (x1d, x2d) = crate::schiene_neu::werkzeug_lage(s);
        let (x1, x2) = (arg_f32(a, "x1").unwrap_or(x1d), arg_f32(a, "x2").unwrap_or(x2d));
        let y = arg_f32(a, "y").unwrap_or(s.frame().1);
        let lage = if auto { 0 } else { crate::sammelschiene_lage::FEST } | if unten { crate::sammelschiene_lage::UNTEN } else { 0 };
        let label = a.get("label").and_then(Value::as_array).and_then(|v| Some(P::new(v.first()?.as_f64()? as f32, v.get(1)?.as_f64()? as f32))).unwrap_or(P::new(0.0, -4.0));
        let versatz = arg_f32(a, "versatz").unwrap_or(0.0);
        let id = self.neu(r, Kind::Busbar { a: P::new(x1.min(x2), y), x2: x1.max(x2), name, label, style: 0, sms: 0, xin: String::new(), xout: String::new(), lage, versatz });
        let abstand = crate::optionen::Optionen::default().sms_abstand;
        let s = self.project.sheet_mut(r).unwrap();
        let (oben, u) = crate::sammelschiene_lage::stapel_anfang(s, pfad, schriftfeld);
        let lagen = crate::sammelschiene_lage::stapeln(s, abstand, oben, u);
        crate::sammelschiene_lage::anwenden(s, &lagen);
        crate::epr::update_busbar_xrefs(&mut self.project);
        Ok(self.objekt(r, id))
    }

    fn terminal(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let pos = P::new(arg_f32(a, "x").ok_or("x fehlt")?, arg_f32(a, "y").ok_or("y fehlt")?);
        let an = &self.project.anlagen[r.anlage];
        let leiste = arg_str(a, "leiste").map(String::from).or_else(|| an.klemmleisten.first().cloned()).unwrap_or_else(|| crate::klemmen::vorgabe_leiste(1).name);
        if !an.klemmleisten.contains(&leiste) {
            return Err(format!("Klemmleiste '{leiste}' nicht in der Anlage (add_klemmleiste)"));
        }
        let nr = match a.get("nr").and_then(Value::as_u64) {
            Some(n) => n as u32,
            None => an.sheets.iter().flat_map(|s| &s.objects).filter_map(|o| if let Kind::Terminal { leiste: l, nr, .. } = &o.kind { (*l == leiste).then_some(*nr) } else { None }).max().unwrap_or(0) + 1,
        };
        // Leitung unter der Klemme teilen wie klemme_leitung::klemmstelle_setzen
        if ja(a, "split").unwrap_or(true) {
            let s = self.project.sheet(r).unwrap();
            let treffer = s.objects.iter().rev().find_map(|o| {
                let Kind::Wire { pts, .. } = &o.kind else { return None };
                let i = (0..pts.len().saturating_sub(1)).find(|&i| seg_dist(pos, pts[i], pts[i + 1]) < 0.05 && pts[i].dist(pos) > 0.05 && pts[i + 1].dist(pos) > 0.05)?;
                Some((o.id, i))
            });
            if let Some((wid, i)) = treffer {
                let neu_id = self.project.alloc_id();
                let s = self.project.sheet_mut(r).unwrap();
                let o = s.objects.iter_mut().find(|o| o.id == wid).unwrap();
                if let Kind::Wire { pts, leitung, bez, opts } = &mut o.kind {
                    let (p1, p2) = crate::klemme_leitung::teilen(pts, i, pos);
                    let zweite = Kind::Wire { pts: p2, leitung: *leitung, bez: vec![], opts: *opts };
                    *pts = p1;
                    let _ = bez;
                    let (pen, fixed) = (o.pen, o.fixed);
                    s.objects.push(Obj { id: neu_id, kind: zweite, pen, fixed, group: None, raw: None });
                }
            }
        }
        let id = self.neu(r, Kind::Terminal { pos, leiste: leiste.clone(), nr, level: 0, text_off: Some(P::new(-1.0, -2.0)), tpl: String::new(), opt: Default::default() });
        Ok(self.objekt(r, id))
    }

    fn reference(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let pos = P::new(arg_f32(a, "x").ok_or("x fehlt")?, arg_f32(a, "y").ok_or("y fehlt")?);
        let dir = a.get("dir").and_then(Value::as_u64).unwrap_or(0) as u16 % 360;
        let vertical = dir == 90 || dir == 270;
        let vorgabe = if vertical { P::new(pos.x + 2.0, pos.y - 10.0) } else { P::new(pos.x + 7.0, pos.y - 1.0) };
        let text_pos = P::new(arg_f32(a, "text_x").unwrap_or(vorgabe.x), arg_f32(a, "text_y").unwrap_or(vorgabe.y));
        let ziel = match (arg_str(a, "ziel_anlage"), a.get("ziel_blatt").and_then(Value::as_u64)) {
            (Some(an), Some(b)) => Some(VerweisZiel { anlage: an.into(), blatt: b as u32, obj: a.get("ziel_obj").and_then(Value::as_u64).unwrap_or(0) }),
            _ => None,
        };
        let id = self.neu(r, Kind::Verweis { pos, text_pos, text: arg_str(a, "text").unwrap_or_default().into(), dir, vertical, spitze: false, ziel });
        Ok(self.objekt(r, id))
    }

    fn table(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let pts = points(a)?;
        if pts.len() < 2 {
            return Err("zwei Ecken nötig".into());
        }
        let (rows, cols) = (arg_usize(a, "rows")?.max(1), arg_usize(a, "cols")?.max(1));
        let mut cells: Vec<String> = a.get("cells").and_then(Value::as_array).map(|v| v.iter().map(|c| c.as_str().unwrap_or_default().to_string()).collect()).unwrap_or_default();
        cells.resize(rows * cols, String::new());
        let id = self.neu(r, Kind::Table { a: pts[0], b: pts[1], rows, cols, cells, header: ja(a, "header").unwrap_or(true), fixed_w: true, art: 0, ab: 0 });
        Ok(self.objekt(r, id))
    }

    fn set_wire(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let id = a.get("id").and_then(Value::as_u64).ok_or("Parameter 'id' fehlt")?;
        let li = match a.get("leitung") {
            Some(Value::Number(n)) => Some(n.as_u64().unwrap_or(0) as usize),
            Some(Value::String(n)) => Some(self.project.leitungen.iter().position(|l| l.name == *n).ok_or_else(|| format!("Leitungsart '{n}' nicht vorhanden"))?),
            _ => None,
        };
        if li.is_some_and(|i| i >= self.project.leitungen.len()) {
            return Err("Leitungsart nicht vorhanden".into());
        }
        let neue_pts = if a.get("points").is_some() { Some(points(a)?) } else { None };
        let o = self.project.sheet_mut(r).unwrap().objects.iter_mut().find(|o| o.id == id).ok_or("Objekt nicht gefunden")?;
        let Kind::Wire { pts, leitung, bez, opts } = &mut o.kind else { return Err("Objekt ist keine Leitung".into()) };
        if let Some(v) = a.get("opts").and_then(Value::as_u64) {
            *opts = v as _;
        }
        if let Some(i) = li {
            *leitung = i;
        }
        if let Some(b) = a.get("bez").and_then(Value::as_array) {
            *bez = b.iter().filter_map(|e| Some((P::new(e.get(0)?.as_f64()? as f32, e.get(1)?.as_f64()? as f32), e.get(2)?.as_str()?.to_string()))).collect();
        }
        if let Some(p) = neue_pts.filter(|p| p.len() >= 2) {
            *pts = p;
        }
        self.dirty = true;
        Ok(obj_json(o))
    }

    fn spiegel(&mut self, a: &Value) -> Result<Value, String> {
        let r = self.sheet_ref(a)?;
        let owner = a.get("owner").and_then(Value::as_u64).unwrap_or(0);
        let mut lib = self.lib.clone();
        lib.extend(self.project.symdefs.iter().filter(|d| !self.lib.iter().any(|l| l.key == d.key)).cloned());
        let s = self.project.sheet(r).unwrap();
        let spule = s.objects.iter().find(|o| o.id == owner && matches!(o.kind, Kind::Symbol { .. }));
        if owner != 0 && spule.is_none() {
            return Err("Spule (owner) nicht auf diesem Blatt".into());
        }
        let x = arg_f32(a, "x").or_else(|| spule.and_then(|o| if let Kind::Symbol { pos, .. } = &o.kind { Some(pos.x) } else { None })).ok_or("x oder owner nötig")?;
        let y = arg_f32(a, "y").unwrap_or_else(|| crate::kontaktspiegel::start_y(&self.project, s));
        let rows = if owner != 0 { crate::kontaktspiegel::kontakte(&self.project, &lib, owner).into_iter().map(|(referenz, art)| crate::kontaktspiegel::Zeile { art, referenz, ..Default::default() }).collect() } else { vec![] };
        let auto = ja(a, "auto").unwrap_or(owner != 0);
        let id = self.neu(r, Kind::Spiegel { pos: P::new(x, y), owner, rows, auto });
        if owner != 0 {
            crate::kontaktspiegel::nachfuehren(&mut self.project, &lib);
        }
        Ok(self.objekt(r, id))
    }

    fn objekt(&self, r: SheetRef, id: u64) -> Value {
        self.project.sheet(r).and_then(|s| s.objects.iter().find(|o| o.id == id)).map(obj_json).unwrap_or(json!({ "id": id }))
    }
}

fn teil_json(i: usize, b: &Bauteil) -> Value {
    json!({ "index": i, "gruppe": b.gruppe, "name": b.name, "typ": b.typ, "firma": b.firma, "artikel": b.artikel, "breite": b.breite, "hoehe": b.hoehe, "tiefe": b.tiefe, "nummer": b.nummer })
}

fn teil_setzen(b: &mut Bauteil, a: &Value) {
    for (k, f) in [("gruppe", &mut b.gruppe), ("name", &mut b.name), ("typ", &mut b.typ), ("firma", &mut b.firma), ("artikel", &mut b.artikel), ("breite", &mut b.breite), ("hoehe", &mut b.hoehe), ("tiefe", &mut b.tiefe), ("nummer", &mut b.nummer)] {
        if let Some(v) = arg_str(a, k) {
            *f = v.into();
        }
    }
}
