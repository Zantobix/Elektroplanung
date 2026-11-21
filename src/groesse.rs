//! Größe ändern und Beschriftungen verschieben auf der Zeichenfläche.
//!
//! - Rahmen zur Größenänderung: Symbole (`scale`, VSY Feld 19 als f32) und Texte (`size`) skalieren proportional
//!   mit. Ein einzeln markiertes Symbol bzw. ein einzelner Text bekommt einen Rahmen mit vier Eckgriffen
//!   (proportional, Festpunkt = gegenüberliegende Ecke); bei Mehrfachmarkierung gilt für Symbole und Texte der
//!   kleinere der beiden Achsenfaktoren.
//! - Titel (BMK, VSY Feld 10/11) und Wert (VSY Feld 13/14, `SymExt::value_off`) eines markierten Symbols lassen
//!   sich einzeln mit der Maus relativ zum Symbol verschieben.

use crate::app::{App, Drag};
use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

/// Abstand des Rahmens um ein einzelnes Symbol bzw. einen Text (mm), damit die Eckgriffe nicht auf Anschlüssen liegen
pub const RAND: f32 = 1.0;
/// kleinster und größter Symbolmaßstab (wie beim Lesen des .epr)
pub const MIN_SCALE: f32 = 0.05;
pub const MAX_SCALE: f32 = 20.0;
/// Schrifthöhen der Beschriftungen (wie canvas.rs)
pub const TITEL_H: f32 = 2.5;
pub const WERT_H: f32 = 2.2;

/// Wird die Markierung proportional skaliert (ein einzelnes Symbol bzw. ein einzelner Text)?
pub fn proportional(objs: &[Obj]) -> bool {
    objs.len() == 1 && matches!(objs[0].kind, Kind::Symbol { .. } | Kind::Text { .. })
}

/// Rahmen zur Größenänderung (min, max) für die markierten Objekte: einzelnes Symbol/Text mit `RAND`, sonst ab zwei
/// Objekten, sofern nicht nur Klemmstellen und Verweise markiert sind; None = kein Rahmen.
pub fn rahmen(objs: &[Obj], lib: &[SymDef]) -> Option<(P, P)> {
    let skalierbar = |o: &Obj| !matches!(o.kind, Kind::Terminal { .. } | Kind::Verweis { .. });
    if proportional(objs) {
        let (a, b) = canvas::obj_bounds(&objs[0], lib);
        return Some((P::new(a.x - RAND, a.y - RAND), P::new(b.x + RAND, b.y + RAND)));
    }
    if objs.len() < 2 || !objs.iter().any(skalierbar) {
        return None;
    }
    let pts: Vec<P> = objs.iter().flat_map(|o| {
        let (a, b) = canvas::obj_bounds(o, lib);
        [a, b]
    }).collect();
    Some(canvas::pts_bounds(&pts))
}

/// Proportionaler Faktor beim Ziehen des Eckgriffs `handle` (0, 2, 4, 6) nach `cur`: Projektion der Maus auf die
/// Diagonale vom Festpunkt (gegenüberliegende Ecke) zum Griff. Rückgabe (Festpunkt, Faktor).
pub fn eck_faktor(handle: usize, mn: P, mx: P, cur: P) -> (P, f32) {
    let h = crate::app::frame_handles(mn, mx);
    let g = h[handle % 8];
    let anchor = h[(handle + 4) % 8];
    let (dx, dy) = (g.x - anchor.x, g.y - anchor.y);
    let l2 = dx * dx + dy * dy;
    if l2 < 1e-6 {
        return (anchor, 1.0);
    }
    let f = ((cur.x - anchor.x) * dx + (cur.y - anchor.y) * dy) / l2;
    (anchor, f.max(0.05))
}

/// Objekt um `anchor` mit `sx`/`sy` skalieren: wie `Kind::scale_about`, zusätzlich wachsen Symbole (Maßstab,
/// Lage von Titel und Wert, Umriss) und Texte (Schrifthöhe) mit dem kleineren Faktor.
pub fn skaliere(k: &mut Kind, anchor: P, sx: f32, sy: f32) {
    k.scale_about(anchor, sx, sy);
    let f = sx.min(sy);
    match k {
        Kind::Symbol { scale, title, ext, .. } => {
            let neu = (*scale * f).clamp(MIN_SCALE, MAX_SCALE);
            let f = neu / *scale;
            *scale = neu;
            let mul = |p: &mut P| *p = P::new(p.x * f, p.y * f);
            if let Some((t, _)) = title {
                mul(t);
            }
            if let Some(v) = &mut ext.value_off {
                mul(v);
            }
            if let Some((a, b)) = &mut ext.bounds {
                mul(a);
                mul(b);
            }
        }
        Kind::Text { size, .. } => *size = (*size * f).clamp(0.3, 200.0),
        _ => {}
    }
}

/// Grob geschätzter Umriss einer Beschriftung (mm) ohne Schriftmaße: Breite ≈ halbe Schrifthöhe je Zeichen.
fn text_rect(at: P, text: &str, h: f32, align: u8, vert: bool) -> (P, P) {
    let w = text.chars().count().max(1) as f32 * h * 0.5;
    canvas::title_rect(at, w, h * 1.15, align, vert)
}

/// Umrisse (mm) von Titel und Wert eines Symbols (None = nicht sichtbar oder ohne gespeicherte Titellage).
pub fn beschriftungen(k: &Kind) -> [Option<(P, P)>; 2] {
    let Kind::Symbol { pos, bmk, value, title: Some((off, vert)), ext, .. } = k else { return [None, None] };
    let t = (!bmk.is_empty()).then(|| text_rect(pos.add(*off), bmk, TITEL_H, ext.title_align, *vert));
    let w = (!value.is_empty()).then(|| text_rect(pos.add(wert_off(k).unwrap_or_default()), value, WERT_H, ext.value_align, *vert));
    [t, w]
}

/// Lage des Wertes relativ zum Symbol (gespeichert oder wie beim Zeichnen 2,8 mm unter dem Titel).
pub fn wert_off(k: &Kind) -> Option<P> {
    let Kind::Symbol { title: Some((off, _)), ext, .. } = k else { return None };
    Some(ext.value_off.unwrap_or(P::new(off.x, off.y + 2.8)))
}

/// Beschriftung `wert` (false = Titel, true = Wert) eines Symbols auf die Lage `off` (relativ zum Symbol) setzen.
pub fn setze_beschriftung(k: &mut Kind, wert: bool, off: P) {
    let Kind::Symbol { title: Some((t, _)), ext, .. } = k else { return };
    if wert {
        ext.value_off = Some(off);
    } else {
        // der Wert bleibt an seiner Stelle, auch wenn er bisher dem Titel folgte
        if ext.value_off.is_none() {
            ext.value_off = Some(P::new(t.x, t.y + 2.8));
        }
        *t = off;
    }
}

/// Beschriftung unter `m`: (wert, bisherige Lage relativ zum Symbol).
pub fn beschriftung_bei(k: &Kind, m: P, tol: f32) -> Option<(bool, P)> {
    let Kind::Symbol { title: Some((off, _)), .. } = k else { return None };
    // Wert zuerst (liegt meist unter dem Titel und ist kleiner)
    for (i, r) in beschriftungen(k).iter().enumerate().rev() {
        if let Some((a, b)) = r {
            if m.x >= a.x - tol && m.x <= b.x + tol && m.y >= a.y - tol && m.y <= b.y + tol {
                return Some(if i == 1 { (true, wert_off(k).unwrap_or(*off)) } else { (false, *off) });
            }
        }
    }
    None
}

/// auf 0,1 mm runden
fn r01(p: P) -> P {
    P::new((p.x * 10.0).round() / 10.0, (p.y * 10.0).round() / 10.0)
}

impl App {
    /// Druck auf die Beschriftung eines markierten Symbols: Ziehen der Beschriftung beginnen. true = behandelt.
    pub fn beschriftung_greifen(&mut self, m: P, tol: f32) -> bool {
        let tol = tol * 0.4;
        let hit = self.selected_objs().into_iter().rev().filter(|o| !(o.fixed && self.fix_active)).find_map(|o| beschriftung_bei(&o.kind, m, tol).map(|(w, off)| (o.id, w, off)));
        let Some((id, wert, orig)) = hit else { return false };
        self.checkpoint();
        self.drag = Drag::Beschriftung { id, wert, start: m, orig };
        true
    }

    /// Beschriftung beim Ziehen nachführen (Verschiebung seit dem Drücken, auf 0,1 mm gerundet).
    pub fn beschriftung_ziehen(&mut self, m: P) {
        let Drag::Beschriftung { id, wert, start, orig } = self.drag.clone() else { return };
        let off = r01(orig.add(m.sub(start)));
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                setze_beschriftung(&mut o.kind, wert, off);
            }
        });
    }

    /// Rahmen ziehen: Objekte aus dem Zustand beim Drücken (`orig`) neu skalieren. Einzelnes Symbol bzw. einzelner
    /// Text: proportional über den Eckgriff.
    pub fn rahmen_ziehen(&mut self, handle: usize, start: (P, P), orig: &[Obj], cur: P) {
        let (anchor, sx, sy) = if proportional(orig) {
            let (a, f) = eck_faktor(handle, start.0, start.1, cur);
            (a, f, f)
        } else {
            crate::app::frame_scale(handle, start.0, start.1, cur)
        };
        let fix = self.fix_active;
        self.with_sheet(|s, _| {
            for o0 in orig.iter().filter(|o| !(o.fixed && fix)) {
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == o0.id) {
                    let mut k = o0.kind.clone();
                    skaliere(&mut k, anchor, sx, sy);
                    o.kind = k;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(pos: P) -> Kind {
        Kind::Symbol { key: "k".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: "-Q1".into(), value: "10A".into(), scale: 1.0, option: 0, title: Some((P::new(-6.0, -2.0), false)), flags: 0, ext: Default::default() }
    }

    fn obj(kind: Kind) -> Obj {
        Obj { id: 1, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn symbol_und_text_proportional() {
        // Symbol um die Ecke (0,0) verdoppeln: Lage, Maßstab und Titellage wachsen mit
        let mut k = sym(P::new(10.0, 20.0));
        skaliere(&mut k, P::new(0.0, 0.0), 2.0, 2.0);
        let Kind::Symbol { pos, scale, title, .. } = &k else { unreachable!() };
        assert_eq!((*pos, *scale), (P::new(20.0, 40.0), 2.0));
        assert_eq!(title.unwrap().0, P::new(-12.0, -4.0));
        // ungleiche Achsen (Mehrfachmarkierung): kleinerer Faktor für Maßstab und Schrift
        let mut t = Kind::Text { pos: P::new(10.0, 10.0), text: "x".into(), size: 3.0, align: Align::Left, vertical: false, style: Default::default() };
        skaliere(&mut t, P::new(0.0, 0.0), 3.0, 0.5);
        let Kind::Text { pos, size, .. } = &t else { unreachable!() };
        assert_eq!((*pos, *size), (P::new(30.0, 5.0), 1.5));
        // Maßstab bleibt in den Grenzen des Formats
        let mut k = sym(P::new(0.0, 0.0));
        skaliere(&mut k, P::new(0.0, 0.0), 0.001, 0.001);
        assert!(matches!(k, Kind::Symbol { scale, .. } if scale == MIN_SCALE));
    }

    #[test]
    fn eckgriff_projektion() {
        let (mn, mx) = (P::new(0.0, 0.0), P::new(10.0, 10.0));
        // Griff 4 (rechts unten) nach (20, 20): doppelt, Festpunkt links oben
        assert_eq!(eck_faktor(4, mn, mx, P::new(20.0, 20.0)), (mn, 2.0));
        // nur waagerecht gezogen: halber Weg auf der Diagonale
        assert_eq!(eck_faktor(4, mn, mx, P::new(20.0, 10.0)).1, 1.5);
        // Griff 0 (links oben) nach (5, 5): halb so groß, Festpunkt rechts unten
        assert_eq!(eck_faktor(0, mn, mx, P::new(5.0, 5.0)), (mx, 0.5));
        assert!(eck_faktor(0, mn, mx, P::new(50.0, 50.0)).1 > 0.0);
        // Rahmen: einzelnes Symbol mit Rand, einzelne Klemmstelle ohne Rahmen
        assert!(proportional(&[obj(sym(P::new(0.0, 0.0)))]));
        assert!(rahmen(&[obj(Kind::Terminal { pos: P::default(), leiste: "X".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() })], &[]).is_none());
    }

    #[test]
    fn titel_und_wert_verschieben() {
        let mut k = sym(P::new(100.0, 50.0));
        // Titel "-Q1" liegt ab (94, 48), Wert 2,8 mm darunter
        let (wert, off) = beschriftung_bei(&k, P::new(95.0, 49.0), 0.1).unwrap();
        assert_eq!((wert, off), (false, P::new(-6.0, -2.0)));
        let (wert, off) = beschriftung_bei(&k, P::new(95.0, 51.5), 0.1).unwrap();
        assert!(wert && off.dist(P::new(-6.0, 0.8)) < 1e-4);
        assert!(beschriftung_bei(&k, P::new(120.0, 50.0), 0.1).is_none());
        // Titel verschieben: der Wert bleibt stehen
        setze_beschriftung(&mut k, false, P::new(3.0, -5.0));
        let Kind::Symbol { title, ext, .. } = &k else { unreachable!() };
        assert_eq!(title.unwrap().0, P::new(3.0, -5.0));
        assert!(ext.value_off.unwrap().dist(P::new(-6.0, 0.8)) < 1e-4);
        // Wert verschieben
        setze_beschriftung(&mut k, true, P::new(4.0, 6.0));
        assert_eq!(wert_off(&k), Some(P::new(4.0, 6.0)));
    }
}
