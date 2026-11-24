//! DXF-Import. Das Original (Änderung.txt 6.4.2) übernimmt nur Linien, Polygone, Kreise und Kreisabschnitte, Splines
//! als Linienzug, Einheiten 1:1 in mm, zu große Zeichnungen in den Rahmen eingepasst, als Gruppe.
//!
//! Nachbau-Erweiterung (Nutzer 24.11.2026): eigenes Importfenster (`dxf_dialog.rs`) mit
//! - **Ebenen** aus der LAYER-Tabelle (Name, Farbe, aus = negative Farbe), je Ebene an/aus,
//! - **Maßstab** nach der Einheit der Datei (`$INSUNITS`) oder fest (mm, cm, m, Zoll, Faktor),
//! - **Einpassen** in den Rahmen ja/nein, **Gruppieren** ja/nein (Vorgabe wie im Original: ja),
//! - **Farben**: Ebenenfarbe bzw. Elementfarbe als eigener Stift „DXF <Ebene>“,
//! - zusätzlich BLOCK/INSERT (aufgelöst mit Lage, Maßstab, Drehung), MTEXT, ELLIPSE.

use crate::model::*;
use std::collections::HashMap;

/// Element in Koordinaten der Datei (y nach oben).
#[derive(Clone, Debug, PartialEq)]
pub enum Ent {
    Lines(Vec<P>, bool),
    Circle(P, f32),
    /// Mittelpunkt, Radius, Start- und Endwinkel in Grad (gegen den Uhrzeigersinn)
    Arc(P, f32, f32, f32),
    /// Lage (links unten), Text, Höhe
    Text(P, String, f32),
}

/// Element mit Ebene und Farbe (AutoCAD-Farbindex; None = „von Ebene“).
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub ebene: String,
    pub farbe: Option<i32>,
    pub ent: Ent,
}

/// Ebene der Datei.
#[derive(Clone, Debug, PartialEq)]
pub struct Ebene {
    pub name: String,
    /// AutoCAD-Farbindex (1–255), 7 = weiß/schwarz
    pub farbe: i32,
    /// in der Datei ausgeschaltet (negative Farbe)
    pub aus: bool,
    /// Zahl der Elemente auf der Ebene
    pub anzahl: usize,
}

/// Gelesene Zeichnung.
#[derive(Clone, Debug, Default)]
pub struct Zeichnung {
    pub elemente: Vec<Element>,
    pub ebenen: Vec<Ebene>,
    /// `$INSUNITS` (0 ohne Angabe, 1 Zoll, 4 mm, 5 cm, 6 m …)
    pub einheit: i32,
}

/// Millimeter je Zeichnungseinheit nach `$INSUNITS` (ohne Angabe: mm wie das Original).
pub fn mm_je_einheit(insunits: i32) -> f32 {
    match insunits {
        1 => 25.4,
        2 => 304.8,
        5 => 10.0,
        6 => 1000.0,
        8 => 0.0000254,
        9 => 0.0254,
        10 => 914.4,
        14 => 100.0,
        _ => 1.0,
    }
}

/// Name der Einheit für die Anzeige.
pub fn einheit_name(insunits: i32) -> &'static str {
    match insunits {
        1 => "Zoll",
        2 => "Fuß",
        4 => "mm",
        5 => "cm",
        6 => "m",
        14 => "dm",
        _ => "ohne Angabe (mm)",
    }
}

fn pairs(src: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = vec![];
    let mut i = 0;
    while i + 1 < lines.len() {
        if let Ok(c) = lines[i].trim().parse::<i32>() {
            out.push((c, lines[i + 1].trim_end_matches('\r').trim_start().to_string()));
        }
        i += 2;
    }
    out
}

fn num(v: &str) -> f32 {
    v.trim().parse::<f32>().unwrap_or(0.0)
}

/// Ein Objekt der Datei: Typname und Gruppencodes.
struct Satz {
    art: String,
    f: Vec<(i32, String)>,
}

impl Satz {
    fn get(&self, code: i32) -> f32 {
        self.f.iter().find(|x| x.0 == code).map(|x| num(&x.1)).unwrap_or(0.0)
    }
    fn get_or(&self, code: i32, d: f32) -> f32 {
        self.f.iter().find(|x| x.0 == code).map(|x| num(&x.1)).unwrap_or(d)
    }
    fn text(&self, code: i32) -> Option<String> {
        self.f.iter().find(|x| x.0 == code).map(|x| x.1.clone())
    }
    fn all(&self, code: i32) -> Vec<f32> {
        self.f.iter().filter(|x| x.0 == code).map(|x| num(&x.1)).collect()
    }
}

/// Abschnitte der Datei: Name → Folge der Sätze.
fn abschnitte(pr: &[(i32, String)]) -> HashMap<String, Vec<Satz>> {
    let mut out: HashMap<String, Vec<Satz>> = HashMap::new();
    let mut i = 0;
    while i < pr.len() {
        if pr[i].0 == 0 && pr[i].1 == "SECTION" && i + 1 < pr.len() && pr[i + 1].0 == 2 {
            let name = pr[i + 1].1.clone();
            let mut j = i + 2;
            let mut saetze = vec![];
            // Kopf des Abschnitts (HEADER hat keine 0-Sätze, nur 9-Variablen) als eigener Satz
            let mut kopf = Satz { art: String::new(), f: vec![] };
            while j < pr.len() && pr[j].0 != 0 {
                kopf.f.push(pr[j].clone());
                j += 1;
            }
            saetze.push(kopf);
            while j < pr.len() && !(pr[j].0 == 0 && pr[j].1 == "ENDSEC") {
                let art = pr[j].1.clone();
                let mut f = vec![];
                j += 1;
                while j < pr.len() && pr[j].0 != 0 {
                    f.push(pr[j].clone());
                    j += 1;
                }
                saetze.push(Satz { art, f });
            }
            out.insert(name, saetze);
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// MTEXT-Formatierung entfernen: \P = neue Zeile, {\f…;…} und \H…; usw. weg, Klammern weg.
fn mtext_rein(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' => match it.next() {
                Some('P') | Some('p') => out.push('\n'),
                Some('~') => out.push(' '),
                Some('\\') => out.push('\\'),
                Some('{') => out.push('{'),
                Some('}') => out.push('}'),
                // \L \l \O \o \K \k: Schalter ohne Wert
                Some('L' | 'l' | 'O' | 'o' | 'K' | 'k') => {}
                // \f… \H… \W… \C… \Q… \T… \A… \S…: bis ';'
                Some(_) => {
                    for d in it.by_ref() {
                        if d == ';' {
                            break;
                        }
                    }
                }
                None => {}
            },
            '{' | '}' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Element aus einem Satz (ohne INSERT/POLYLINE-Folgen).
fn element(s: &Satz) -> Option<Ent> {
    Some(match s.art.as_str() {
        "LINE" => Ent::Lines(vec![P::new(s.get(10), s.get(20)), P::new(s.get(11), s.get(21))], false),
        "LWPOLYLINE" | "SPLINE" => {
            // SPLINE: Stützpunkte (11/21), sonst Kontrollpunkte (10/20) als Linienzug
            let (xs, ys) = if s.art == "SPLINE" && !s.all(11).is_empty() { (s.all(11), s.all(21)) } else { (s.all(10), s.all(20)) };
            let pts: Vec<P> = xs.iter().zip(ys.iter()).map(|(x, y)| P::new(*x, *y)).collect();
            if pts.len() < 2 {
                return None;
            }
            Ent::Lines(pts, (s.get(70) as i32) & 1 == 1 && s.art == "LWPOLYLINE")
        }
        "CIRCLE" => Ent::Circle(P::new(s.get(10), s.get(20)), s.get(40)),
        "ARC" => Ent::Arc(P::new(s.get(10), s.get(20)), s.get(40), s.get(50), s.get(51)),
        "ELLIPSE" => {
            // Mittelpunkt, Endpunkt der großen Achse (relativ), Verhältnis, Parameter von/bis (Bogenmaß)
            let c = P::new(s.get(10), s.get(20));
            let ma = P::new(s.get(11), s.get(21));
            let k = s.get_or(40, 1.0);
            let (t0, t1) = (s.get_or(41, 0.0), s.get_or(42, std::f32::consts::TAU));
            let t1 = if t1 <= t0 { t1 + std::f32::consts::TAU } else { t1 };
            let mi = P::new(-ma.y * k, ma.x * k);
            let n = (((t1 - t0) / std::f32::consts::TAU * 48.0).ceil() as usize).max(4);
            let pts: Vec<P> = (0..=n)
                .map(|i| {
                    let t = t0 + (t1 - t0) * i as f32 / n as f32;
                    P::new(c.x + ma.x * t.cos() + mi.x * t.sin(), c.y + ma.y * t.cos() + mi.y * t.sin())
                })
                .collect();
            Ent::Lines(pts, false)
        }
        "TEXT" => {
            let t = s.text(1).unwrap_or_default();
            if t.is_empty() {
                return None;
            }
            Ent::Text(P::new(s.get(10), s.get(20)), t, s.get(40).max(0.1))
        }
        "MTEXT" => {
            // Text in 3-Gruppen (je 250 Zeichen) und 1 am Ende
            let mut roh: String = s.f.iter().filter(|x| x.0 == 3).map(|x| x.1.as_str()).collect();
            roh.push_str(&s.text(1).unwrap_or_default());
            let t = mtext_rein(&roh);
            if t.trim().is_empty() {
                return None;
            }
            // MTEXT liegt mit der linken oberen Ecke bei 10/20: Lage der ersten Grundlinie ≈ eine Höhe tiefer
            let h = s.get(40).max(0.1);
            Ent::Text(P::new(s.get(10), s.get(20) - h), t, h)
        }
        _ => return None,
    })
}

/// Lage, Maßstab, Drehung eines eingefügten Blocks auf einen Punkt anwenden.
#[derive(Clone, Copy)]
struct Einfuegung {
    basis: P,
    lage: P,
    sx: f32,
    sy: f32,
    dreh: f32,
}

impl Einfuegung {
    fn p(&self, q: P) -> P {
        let (x, y) = ((q.x - self.basis.x) * self.sx, (q.y - self.basis.y) * self.sy);
        let (s, c) = self.dreh.to_radians().sin_cos();
        P::new(self.lage.x + x * c - y * s, self.lage.y + x * s + y * c)
    }
    fn m(&self) -> f32 {
        (self.sx.abs() + self.sy.abs()) / 2.0
    }
    fn ent(&self, e: &Ent) -> Ent {
        match e {
            Ent::Lines(v, c) => Ent::Lines(v.iter().map(|q| self.p(*q)).collect(), *c),
            Ent::Circle(c, r) => Ent::Circle(self.p(*c), r * self.m()),
            Ent::Arc(c, r, a0, a1) => {
                let gespiegelt = self.sx * self.sy < 0.0;
                let (a0, a1) = if gespiegelt { (180.0 - a1, 180.0 - a0) } else { (*a0, *a1) };
                Ent::Arc(self.p(*c), r * self.m(), a0 + self.dreh, a1 + self.dreh)
            }
            Ent::Text(q, t, h) => Ent::Text(self.p(*q), t.clone(), h * self.sy.abs()),
        }
    }
}

/// Folge von Sätzen (ENTITIES bzw. Inhalt eines Blocks) in Elemente; INSERT wird mit `bloecke` aufgelöst.
fn folge(saetze: &[Satz], bloecke: &HashMap<String, (P, Vec<Satz>)>, tiefe: usize, out: &mut Vec<Element>) {
    let mut poly: Option<(Vec<P>, bool, String, Option<i32>)> = None;
    for s in saetze {
        let ebene = s.text(8).unwrap_or_else(|| "0".into());
        let farbe = s.f.iter().find(|x| x.0 == 62).map(|x| num(&x.1) as i32).filter(|c| *c != 256);
        match s.art.as_str() {
            "POLYLINE" => poly = Some((vec![], (s.get(70) as i32) & 1 == 1, ebene, farbe)),
            "VERTEX" => {
                if let Some((v, ..)) = &mut poly {
                    v.push(P::new(s.get(10), s.get(20)));
                }
            }
            "SEQEND" => {
                if let Some((v, cl, eb, fa)) = poly.take() {
                    if v.len() >= 2 {
                        out.push(Element { ebene: eb, farbe: fa, ent: Ent::Lines(v, cl) });
                    }
                }
            }
            "INSERT" if tiefe < 8 => {
                let Some(name) = s.text(2) else { continue };
                let Some((basis, inhalt)) = bloecke.get(&name) else { continue };
                let e = Einfuegung { basis: *basis, lage: P::new(s.get(10), s.get(20)), sx: s.get_or(41, 1.0), sy: s.get_or(42, 1.0), dreh: s.get(50) };
                let mut innen = vec![];
                folge(inhalt, bloecke, tiefe + 1, &mut innen);
                for mut el in innen {
                    el.ent = e.ent(&el.ent);
                    // Elemente auf Ebene 0 bzw. mit Farbe „von Block“ erben vom INSERT
                    if el.ebene == "0" {
                        el.ebene = ebene.clone();
                    }
                    if el.farbe == Some(0) {
                        el.farbe = farbe;
                    }
                    out.push(el);
                }
            }
            _ => {
                if let Some(ent) = element(s) {
                    out.push(Element { ebene, farbe, ent });
                }
            }
        }
    }
}

/// DXF (ASCII) lesen.
pub fn lesen(bytes: &[u8]) -> Result<Zeichnung, String> {
    // Windows-1252 bzw. UTF-8: Bytes einzeln (wie bisher), UTF-8 wenn gültig
    let src: String = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };
    let pr = pairs(&src);
    let ab = abschnitte(&pr);
    let ents = ab.get("ENTITIES").ok_or("Kein ENTITIES-Abschnitt (keine ASCII-DXF-Datei?)")?;
    // Einheit aus dem Kopf: 9 $INSUNITS, dann 70 Wert
    let mut einheit = 0;
    if let Some(h) = ab.get("HEADER").and_then(|v| v.first()) {
        if let Some(i) = h.f.iter().position(|x| x.0 == 9 && x.1 == "$INSUNITS") {
            einheit = h.f.get(i + 1).map(|x| num(&x.1) as i32).unwrap_or(0);
        }
    }
    // Ebenen aus der LAYER-Tabelle
    let mut ebenen: Vec<Ebene> = vec![];
    if let Some(t) = ab.get("TABLES") {
        for s in t.iter().filter(|s| s.art == "LAYER") {
            if let Some(n) = s.text(2) {
                let f = s.get_or(62, 7.0) as i32;
                ebenen.push(Ebene { name: n, farbe: f.abs().max(1), aus: f < 0, anzahl: 0 });
            }
        }
    }
    // Blöcke: Name → (Basispunkt, Inhalt)
    let mut bloecke: HashMap<String, (P, Vec<Satz>)> = HashMap::new();
    if let Some(b) = ab.get("BLOCKS") {
        let mut akt: Option<(String, P, Vec<Satz>)> = None;
        for s in b.iter().skip(1) {
            match s.art.as_str() {
                "BLOCK" => akt = Some((s.text(2).unwrap_or_default(), P::new(s.get(10), s.get(20)), vec![])),
                "ENDBLK" => {
                    if let Some((n, p, v)) = akt.take() {
                        bloecke.insert(n, (p, v));
                    }
                }
                _ => {
                    if let Some((_, _, v)) = &mut akt {
                        v.push(Satz { art: s.art.clone(), f: s.f.clone() });
                    }
                }
            }
        }
    }
    let mut elemente = vec![];
    folge(&ents[1..], &bloecke, 0, &mut elemente);
    if elemente.is_empty() {
        return Err("Keine unterstützten Elemente (Linien, Polygone, Kreise, Kreisabschnitte) gefunden".into());
    }
    for e in &elemente {
        match ebenen.iter_mut().find(|x| x.name == e.ebene) {
            Some(x) => x.anzahl += 1,
            None => ebenen.push(Ebene { name: e.ebene.clone(), farbe: 7, aus: false, anzahl: 1 }),
        }
    }
    ebenen.retain(|e| e.anzahl > 0);
    Ok(Zeichnung { elemente, ebenen, einheit })
}

/// Einstellungen des Imports.
#[derive(Clone, Debug, PartialEq)]
pub struct Einstellungen {
    /// Namen der übernommenen Ebenen
    pub ebenen: Vec<String>,
    /// mm je Zeichnungseinheit
    pub massstab: f32,
    /// in den Rahmen einpassen (nur verkleinern)
    pub einpassen: bool,
    pub gruppieren: bool,
    /// Ebenen-/Elementfarben als Stifte übernehmen
    pub farben: bool,
}

impl Einstellungen {
    /// Vorgaben wie das Original: alle eingeschalteten Ebenen, Einheit der Datei, einpassen, gruppieren, ohne Farben.
    pub fn vorgabe(z: &Zeichnung) -> Self {
        Einstellungen { ebenen: z.ebenen.iter().filter(|e| !e.aus).map(|e| e.name.clone()).collect(), massstab: mm_je_einheit(z.einheit), einpassen: true, gruppieren: true, farben: false }
    }
}

/// Farbe eines AutoCAD-Farbindex (Grundfarben genau, sonst angenähert); 7 = schwarz auf weißem Blatt.
pub fn aci_rgb(i: i32) -> [u8; 3] {
    match i {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 => [0, 0, 0],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        250..=255 => {
            let g = [51, 80, 105, 130, 190, 255][(i - 250) as usize];
            [g, g, g]
        }
        10..=249 => {
            // Farbton in 24 Stufen, gerade = kräftig, ungerade = blasser; Helligkeit nach der Zehnerstelle
            let h = ((i - 10) / 10) as f32 * 15.0;
            let hell = [1.0, 1.0, 0.8, 0.8, 0.6, 0.6, 0.5, 0.5, 0.3, 0.3][((i - 10) % 10) as usize];
            let blass = (i % 2) == 1;
            let (r, g, b) = hsv(h, if blass { 0.5 } else { 1.0 }, hell);
            [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
        }
        _ => [0, 0, 0],
    }
}

fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

/// Umriss der übernommenen Elemente in Dateikoordinaten.
fn umriss(el: &[&Element]) -> Option<(P, P)> {
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    let mut add = |p: P| {
        mn = P::new(mn.x.min(p.x), mn.y.min(p.y));
        mx = P::new(mx.x.max(p.x), mx.y.max(p.y));
    };
    for e in el {
        match &e.ent {
            Ent::Lines(v, _) => v.iter().for_each(|p| add(*p)),
            Ent::Circle(c, r) => {
                add(P::new(c.x - r, c.y - r));
                add(P::new(c.x + r, c.y + r))
            }
            // Bogen: nur der gezeichnete Teil (sonst rückte z. B. eine Tür die Zeichnung um einen Radius)
            Ent::Arc(c, r, a0, a1) => {
                let a1 = if a1 < a0 { a1 + 360.0 } else { *a1 };
                for k in 0..=36 {
                    let a = (a0 + (a1 - a0) * k as f32 / 36.0).to_radians();
                    add(P::new(c.x + r * a.cos(), c.y + r * a.sin()));
                }
            }
            Ent::Text(p, _, h) => {
                add(*p);
                add(P::new(p.x, p.y + h))
            }
        }
    }
    (mn.x <= mx.x).then_some((mn, mx))
}

/// Ergebnis der Umsetzung: Objektart und Farbe (für einen Stift) je Element.
pub type Umgesetzt = Vec<(Kind, Option<[u8; 3]>, String)>;

/// Zeichnung nach `e` in Objekte des Blattes umsetzen (`frame` = Zeichnungsrahmen x0, y0, x1, y1).
pub fn umsetzen(z: &Zeichnung, e: &Einstellungen, frame: (f32, f32, f32, f32)) -> Umgesetzt {
    let el: Vec<&Element> = z.elemente.iter().filter(|x| e.ebenen.contains(&x.ebene)).collect();
    let Some((mn, mx)) = umriss(&el) else { return vec![] };
    let avail_w = frame.2 - frame.0 - 20.0;
    let avail_h = frame.3 - frame.1 - COLUMN_BAR_H - TITLE_H - 20.0;
    let (w, h) = (((mx.x - mn.x) * e.massstab).max(0.001), ((mx.y - mn.y) * e.massstab).max(0.001));
    let s = e.massstab * if e.einpassen { (avail_w / w).min(avail_h / h).min(1.0) } else { 1.0 };
    let ox = frame.0 + 10.0;
    let oy = frame.1 + COLUMN_BAR_H + 10.0;
    let tr = |p: P| P::new(ox + (p.x - mn.x) * s, oy + (mx.y - p.y) * s);
    let ebenenfarbe: HashMap<&str, i32> = z.ebenen.iter().map(|x| (x.name.as_str(), x.farbe)).collect();
    let mut out = vec![];
    for x in el {
        let farbe = e.farben.then(|| aci_rgb(x.farbe.filter(|c| *c > 0).unwrap_or_else(|| ebenenfarbe.get(x.ebene.as_str()).copied().unwrap_or(7))));
        let mut push = |k: Kind| out.push((k, farbe, x.ebene.clone()));
        match &x.ent {
            Ent::Lines(v, closed) => {
                let pts: Vec<P> = v.iter().map(|p| tr(*p)).collect();
                if *closed {
                    push(Kind::Poly { pts });
                } else {
                    for w in pts.windows(2) {
                        push(Kind::Line { a: w[0], b: w[1] });
                    }
                }
            }
            Ent::Circle(c, r) => {
                let c = tr(*c);
                let r = r * s;
                push(Kind::Circle { a: P::new(c.x - r, c.y - r), b: P::new(c.x + r, c.y + r) });
            }
            Ent::Arc(c, r, a0, a1) => {
                let a1 = if a1 < a0 { a1 + 360.0 } else { *a1 };
                let n = (((a1 - a0) / 10.0).ceil() as usize).max(2);
                let pts: Vec<P> = (0..=n)
                    .map(|k| {
                        let a = (a0 + (a1 - a0) * k as f32 / n as f32).to_radians();
                        tr(P::new(c.x + r * a.cos(), c.y + r * a.sin()))
                    })
                    .collect();
                for w in pts.windows(2) {
                    push(Kind::Line { a: w[0], b: w[1] });
                }
            }
            Ent::Text(p, t, hgt) => {
                let q = tr(*p);
                push(Kind::Text { pos: P::new(q.x, q.y - hgt * s / 2.0), text: t.clone(), size: (hgt * s * 1.4).max(1.0), align: Align::Left, vertical: false, style: Default::default() });
            }
        }
    }
    out
}

/// Wie das Original: alle Ebenen, Einheit der Datei, eingepasst (für Aufrufer ohne Importfenster).
pub fn import(path: &std::path::Path, frame: (f32, f32, f32, f32)) -> Result<Vec<Kind>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let z = lesen(&bytes)?;
    Ok(umsetzen(&z, &Einstellungen::vorgabe(&z), frame).into_iter().map(|(k, ..)| k).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kleine DXF-Datei: zwei Ebenen (Wand rot, Moebel aus), ein Block „TUER“ (Bogen + Linie) zweimal eingefügt,
    /// MTEXT mit Formatierung, ELLIPSE, Einheit cm.
    pub fn beispiel() -> String {
        let z = |v: &[(&str, &str)]| v.iter().map(|(c, w)| format!("{c}\n{w}\n")).collect::<String>();
        let mut s = String::new();
        s += &z(&[("0", "SECTION"), ("2", "HEADER"), ("9", "$INSUNITS"), ("70", "5"), ("0", "ENDSEC")]);
        s += &z(&[("0", "SECTION"), ("2", "TABLES"), ("0", "TABLE"), ("2", "LAYER")]);
        s += &z(&[("0", "LAYER"), ("2", "Wand"), ("62", "1"), ("0", "LAYER"), ("2", "Moebel"), ("62", "-3"), ("0", "ENDTAB"), ("0", "ENDSEC")]);
        s += &z(&[("0", "SECTION"), ("2", "BLOCKS"), ("0", "BLOCK"), ("2", "TUER"), ("10", "0"), ("20", "0")]);
        s += &z(&[("0", "ARC"), ("8", "0"), ("10", "0"), ("20", "0"), ("40", "80"), ("50", "0"), ("51", "90")]);
        s += &z(&[("0", "LINE"), ("8", "0"), ("10", "0"), ("20", "0"), ("11", "0"), ("21", "80")]);
        s += &z(&[("0", "ENDBLK"), ("0", "ENDSEC")]);
        s += &z(&[("0", "SECTION"), ("2", "ENTITIES")]);
        s += &z(&[("0", "LWPOLYLINE"), ("8", "Wand"), ("70", "1"), ("10", "0"), ("20", "0"), ("10", "500"), ("20", "0"), ("10", "500"), ("20", "400"), ("10", "0"), ("20", "400")]);
        s += &z(&[("0", "INSERT"), ("8", "Wand"), ("2", "TUER"), ("10", "100"), ("20", "0")]);
        s += &z(&[("0", "INSERT"), ("8", "Wand"), ("2", "TUER"), ("10", "300"), ("20", "0"), ("50", "90")]);
        s += &z(&[("0", "MTEXT"), ("8", "Wand"), ("10", "200"), ("20", "200"), ("40", "20"), ("1", "{\\fArial|b0;Raum}\\P11")]);
        s += &z(&[("0", "ELLIPSE"), ("8", "Moebel"), ("10", "250"), ("20", "300"), ("11", "50"), ("21", "0"), ("40", "0.5")]);
        s += &z(&[("0", "CIRCLE"), ("8", "Moebel"), ("62", "5"), ("10", "100"), ("20", "300"), ("40", "20")]);
        s += &z(&[("0", "ENDSEC"), ("0", "EOF")]);
        s
    }

    #[test]
    fn ebenen_bloecke_texte() {
        let z = lesen(beispiel().as_bytes()).unwrap();
        assert_eq!(z.einheit, 5);
        let n = |name: &str| z.ebenen.iter().find(|e| e.name == name).unwrap().clone();
        // Wand: Rechteck + 2×(Bogen + Linie) + MTEXT = 6; Moebel: Ellipse + Kreis, ausgeschaltet
        assert_eq!((n("Wand").anzahl, n("Wand").farbe, n("Wand").aus), (6, 1, false));
        assert_eq!((n("Moebel").anzahl, n("Moebel").aus), (2, true));
        // Block aufgelöst: Linie der zweiten Tür (90° gedreht) liegt waagerecht von (300,0) nach (220,0)
        let linien: Vec<&Ent> = z.elemente.iter().map(|e| &e.ent).filter(|e| matches!(e, Ent::Lines(v, false) if v.len() == 2)).collect();
        assert!(linien.iter().any(|e| matches!(e, Ent::Lines(v, _) if v[0].dist(P::new(300.0, 0.0)) < 0.01 && v[1].dist(P::new(220.0, 0.0)) < 0.01)), "{linien:?}");
        // MTEXT ohne Formatcodes, Zeilenumbruch erhalten
        assert!(z.elemente.iter().any(|e| matches!(&e.ent, Ent::Text(_, t, _) if t == "Raum\n11")));
    }

    #[test]
    fn umsetzen_mit_einstellungen() {
        let z = lesen(beispiel().as_bytes()).unwrap();
        let frame = (5.0, 5.0, 292.0, 205.0);
        // Vorgabe: nur eingeschaltete Ebenen (Wand), cm → mm, eingepasst
        let e = Einstellungen::vorgabe(&z);
        assert_eq!(e.ebenen, vec!["Wand".to_string()]);
        assert_eq!(e.massstab, 10.0);
        let k = umsetzen(&z, &e, frame);
        assert!(k.iter().all(|(_, f, eb)| f.is_none() && eb == "Wand"));
        // 5 m × 4 m passen nicht: eingepasst in die Höhe 205 − 5 − 5 − 15 − 20 = 160 mm
        let hoehe = k.iter().filter_map(|(k, ..)| if let Kind::Poly { pts } = k { Some(pts.iter().map(|p| p.y).fold(f32::MIN, f32::max) - pts.iter().map(|p| p.y).fold(f32::MAX, f32::min)) } else { None }).next().unwrap();
        assert!((hoehe - 160.0).abs() < 0.5, "{hoehe}");
        // ohne Einpassen, Maßstab 0,1 (1 cm → 1 mm): 400 → 40 mm
        let e2 = Einstellungen { massstab: 0.1, einpassen: false, ..e.clone() };
        let k2 = umsetzen(&z, &e2, frame);
        let h2 = k2.iter().filter_map(|(k, ..)| if let Kind::Poly { pts } = k { Some(pts.iter().map(|p| p.y).fold(f32::MIN, f32::max) - pts.iter().map(|p| p.y).fold(f32::MAX, f32::min)) } else { None }).next().unwrap();
        assert!((h2 - 40.0).abs() < 0.01, "{h2}");
        // mit Möbeln und Farben: Kreis blau (Elementfarbe 5), Ellipse grün (Ebenenfarbe 3)
        let e3 = Einstellungen { ebenen: vec!["Wand".into(), "Moebel".into()], farben: true, ..e };
        let k3 = umsetzen(&z, &e3, frame);
        assert!(k3.iter().any(|(k, f, _)| matches!(k, Kind::Circle { .. }) && *f == Some([0, 0, 255])));
        assert!(k3.iter().any(|(k, f, eb)| matches!(k, Kind::Line { .. }) && eb == "Moebel" && *f == Some([0, 255, 0])));
        assert!(k3.iter().any(|(_, f, eb)| eb == "Wand" && *f == Some([255, 0, 0])));
    }

    #[test]
    fn aci_farben() {
        assert_eq!(aci_rgb(1), [255, 0, 0]);
        assert_eq!(aci_rgb(7), [0, 0, 0]);
        assert_eq!(aci_rgb(250), [51, 51, 51]);
        let _ = aci_rgb(123);
    }
}
