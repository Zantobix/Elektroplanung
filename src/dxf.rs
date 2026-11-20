//! DXF-Import wie im Original (Änderung.txt 6.4.2): nur Linien, Polygone, Kreise und Kreisabschnitte,
//! Splines nur als Linie (Stützpunkte verbunden). Einheiten 1:1 in mm; zu große Zeichnungen werden in den
//! Rahmen eingepasst.

use crate::model::*;

enum Ent {
    Lines(Vec<P>, bool),
    Circle(P, f32),
    Arc(P, f32, f32, f32),
    Text(P, String, f32),
}

fn pairs(src: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = vec![];
    let mut i = 0;
    while i + 1 < lines.len() {
        if let Ok(c) = lines[i].trim().parse::<i32>() {
            out.push((c, lines[i + 1].trim().to_string()));
        }
        i += 2;
    }
    out
}

fn num(v: &str) -> f32 {
    v.parse::<f32>().unwrap_or(0.0)
}

pub fn import(path: &std::path::Path, frame: (f32, f32, f32, f32)) -> Result<Vec<Kind>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let src: String = bytes.iter().map(|&b| b as char).collect();
    let pr = pairs(&src);
    // Nur den ENTITIES-Abschnitt lesen
    let start = pr.windows(2).position(|w| w[0].0 == 2 && w[0].1 == "ENTITIES").ok_or("Kein ENTITIES-Abschnitt (keine ASCII-DXF-Datei?)")?;
    let mut ents: Vec<Ent> = vec![];
    let mut i = start + 1;
    let mut poly: Option<(Vec<P>, bool)> = None;
    while i < pr.len() {
        let (c, v) = (&pr[i].0, pr[i].1.as_str());
        if *c == 0 && v == "ENDSEC" {
            break;
        }
        if *c != 0 {
            i += 1;
            continue;
        }
        let kind = v.to_string();
        let mut j = i + 1;
        let mut f: Vec<(i32, String)> = vec![];
        while j < pr.len() && pr[j].0 != 0 {
            f.push(pr[j].clone());
            j += 1;
        }
        let get = |code: i32| f.iter().find(|x| x.0 == code).map(|x| num(&x.1)).unwrap_or(0.0);
        let all = |code: i32| -> Vec<f32> { f.iter().filter(|x| x.0 == code).map(|x| num(&x.1)).collect() };
        match kind.as_str() {
            "LINE" => ents.push(Ent::Lines(vec![P::new(get(10), get(20)), P::new(get(11), get(21))], false)),
            "LWPOLYLINE" | "SPLINE" => {
                let xs = all(10);
                let ys = all(20);
                let pts: Vec<P> = xs.iter().zip(ys.iter()).map(|(x, y)| P::new(*x, *y)).collect();
                let closed = (get(70) as i32) & 1 == 1;
                if pts.len() >= 2 {
                    ents.push(Ent::Lines(pts, closed && kind == "LWPOLYLINE"));
                }
            }
            "POLYLINE" => poly = Some((vec![], (get(70) as i32) & 1 == 1)),
            "VERTEX" => {
                if let Some((v, _)) = &mut poly {
                    v.push(P::new(get(10), get(20)));
                }
            }
            "SEQEND" => {
                if let Some((v, cl)) = poly.take() {
                    if v.len() >= 2 {
                        ents.push(Ent::Lines(v, cl));
                    }
                }
            }
            "CIRCLE" => ents.push(Ent::Circle(P::new(get(10), get(20)), get(40))),
            "ARC" => ents.push(Ent::Arc(P::new(get(10), get(20)), get(40), get(50), get(51))),
            "TEXT" => {
                let t = f.iter().find(|x| x.0 == 1).map(|x| x.1.clone()).unwrap_or_default();
                if !t.is_empty() {
                    ents.push(Ent::Text(P::new(get(10), get(20)), t, get(40).max(1.0)));
                }
            }
            _ => {}
        }
        i = j;
    }
    if ents.is_empty() {
        return Err("Keine unterstützten Elemente (Linien, Polygone, Kreise, Kreisabschnitte) gefunden".into());
    }
    // Umriss bestimmen (DXF: y nach oben)
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    let mut add = |p: P| {
        mn.x = mn.x.min(p.x);
        mn.y = mn.y.min(p.y);
        mx.x = mx.x.max(p.x);
        mx.y = mx.y.max(p.y);
    };
    for e in &ents {
        match e {
            Ent::Lines(v, _) => v.iter().for_each(|p| add(*p)),
            Ent::Circle(c, r) | Ent::Arc(c, r, _, _) => {
                add(P::new(c.x - r, c.y - r));
                add(P::new(c.x + r, c.y + r))
            }
            Ent::Text(p, _, h) => {
                add(*p);
                add(P::new(p.x, p.y + h))
            }
        }
    }
    let avail_w = frame.2 - frame.0 - 20.0;
    let avail_h = frame.3 - frame.1 - COLUMN_BAR_H - TITLE_H - 20.0;
    let (w, h) = ((mx.x - mn.x).max(0.001), (mx.y - mn.y).max(0.001));
    let s = (avail_w / w).min(avail_h / h).min(1.0);
    let ox = frame.0 + 10.0;
    let oy = frame.1 + COLUMN_BAR_H + 10.0;
    let tr = |p: P| P::new(ox + (p.x - mn.x) * s, oy + (mx.y - p.y) * s);
    let mut out = vec![];
    for e in ents {
        match e {
            Ent::Lines(v, closed) => {
                let mut pts: Vec<P> = v.into_iter().map(tr).collect();
                if closed {
                    out.push(Kind::Poly { pts });
                } else if pts.len() == 2 {
                    out.push(Kind::Line { a: pts[0], b: pts[1] });
                } else {
                    for w in pts.windows(2) {
                        out.push(Kind::Line { a: w[0], b: w[1] });
                    }
                    pts.clear();
                }
            }
            Ent::Circle(c, r) => {
                let c = tr(c);
                let r = r * s;
                out.push(Kind::Circle { a: P::new(c.x - r, c.y - r), b: P::new(c.x + r, c.y + r) });
            }
            Ent::Arc(c, r, a0, a1) => {
                let a1 = if a1 < a0 { a1 + 360.0 } else { a1 };
                let n = (((a1 - a0) / 10.0).ceil() as usize).max(2);
                let pts: Vec<P> = (0..=n)
                    .map(|k| {
                        let a = (a0 + (a1 - a0) * k as f32 / n as f32).to_radians();
                        tr(P::new(c.x + r * a.cos(), c.y + r * a.sin()))
                    })
                    .collect();
                for w in pts.windows(2) {
                    out.push(Kind::Line { a: w[0], b: w[1] });
                }
            }
            Ent::Text(p, t, hgt) => {
                let q = tr(p);
                out.push(Kind::Text { pos: P::new(q.x, q.y - hgt * s / 2.0), text: t, size: (hgt * s * 1.4).max(1.0), align: Align::Left, vertical: false, style: Default::default() });
            }
        }
    }
    Ok(out)
}
