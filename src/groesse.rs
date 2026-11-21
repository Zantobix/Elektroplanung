//! Größe ändern und Beschriftungen verschieben auf der Zeichenfläche.
//!
//! - Rahmen zur Größenänderung: Symbole (`scale`, VSY Feld 19 als f32) und Texte (`size`, Textbox) skalieren
//!   proportional mit; Titel und Wert eines Symbols bleiben dabei wie im Original an ihrer Stelle auf dem Blatt. Ein einzeln markiertes Symbol bzw. ein einzelner Text bekommt einen Rahmen mit vier Eckgriffen
//!   (proportional, Festpunkt = gegenüberliegende Ecke); bei Mehrfachmarkierung gilt für Symbole und Texte der
//!   kleinere der beiden Achsenfaktoren.
//! - Titel (BMK, VSY Feld 10/11) und Wert (VSY Feld 13/14, `SymExt::value_off`) eines Symbols lassen sich wie im
//!   Original direkt (ohne vorheriges Markieren) mit der Maus relativ zum Symbol verschieben.

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
/// Umriss) und Texte (Schrifthöhe, Textbox um ihren Bezugspunkt) mit dem kleineren Faktor. Titel und Wert eines
/// Symbols bleiben an ihrer Stelle auf dem Blatt (ihr Versatz zur neuen Lage wird angepasst).
pub fn skaliere(k: &mut Kind, anchor: P, sx: f32, sy: f32) {
    let alt = match k {
        Kind::Symbol { pos, .. } | Kind::Text { pos, .. } => *pos,
        _ => P::default(),
    };
    k.scale_about(anchor, sx, sy);
    let f = sx.min(sy);
    match k {
        Kind::Symbol { pos, scale, title, ext, .. } => {
            let neu = (*scale * f).clamp(MIN_SCALE, MAX_SCALE);
            let f = neu / *scale;
            *scale = neu;
            // Titel/Wert: gleiche Stelle auf dem Blatt = alter Versatz + alte Lage − neue Lage
            let d = alt.sub(*pos);
            if let Some((t, _)) = title {
                *t = t.add(d);
            }
            if let Some(v) = &mut ext.value_off {
                *v = v.add(d);
            }
            let mul = |p: &mut P| *p = P::new(p.x * f, p.y * f);
            if let Some((a, b)) = &mut ext.bounds {
                mul(a);
                mul(b);
            }
        }
        Kind::Text { pos, size, style, .. } => {
            let neu = (*size * f).clamp(0.3, 200.0);
            let f = neu / *size;
            *size = neu;
            // die Textbox wächst mit der Schrift um den Bezugspunkt (sonst bliebe die Markierung klein)
            let p = *pos;
            if let Some((a, b)) = &mut style.rect {
                for c in [a, b] {
                    *c = P::new(p.x + (c.x - p.x) * f, p.y + (c.y - p.y) * f);
                }
            }
        }
        _ => {}
    }
}

/// Umriss einer Beschriftung (mm), wie `canvas::draw_object` sie zeichnet (Schriftmaße der Times, textmass.rs).
fn text_rect(at: P, text: &str, h: f32, align: u8, vert: bool) -> (P, P) {
    let w = crate::textmass::breite(text, h, false, false);
    canvas::title_rect(at, w, crate::textmass::zeilenhoehe(h), align, vert)
}

/// Umrisse (mm) von Titel und Wert eines Symbols (None = nicht sichtbar oder ohne gespeicherte Titellage), mit
/// den Texten, wie sie im Symbol stehen (BMK-Vorlage ungelöst).
pub fn beschriftungen(k: &Kind) -> [Option<(P, P)>; 2] {
    let Kind::Symbol { bmk, value, .. } = k else { return [None, None] };
    beschriftungen_mit(k, bmk, value)
}

/// Wie `beschriftungen`, mit den angezeigten Texten (aufgelöste BMK, Wert mit ersetzten Platzhaltern).
pub fn beschriftungen_mit(k: &Kind, bmk: &str, value: &str) -> [Option<(P, P)>; 2] {
    let Kind::Symbol { pos, title: Some((off, vert)), ext, .. } = k else { return [None, None] };
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
    let Kind::Symbol { bmk, value, .. } = k else { return None };
    beschriftung_bei_mit(k, bmk, value, m, tol)
}

/// Wie `beschriftung_bei`, mit den angezeigten Texten.
pub fn beschriftung_bei_mit(k: &Kind, bmk: &str, value: &str, m: P, tol: f32) -> Option<(bool, P)> {
    let Kind::Symbol { title: Some((off, _)), .. } = k else { return None };
    // Wert zuerst (liegt meist unter dem Titel und ist kleiner)
    for (i, r) in beschriftungen_mit(k, bmk, value).iter().enumerate().rev() {
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
    /// Angezeigte Texte (BMK aufgelöst, Wert mit ersetzten Platzhaltern) eines Symbols auf dem aktuellen Blatt.
    pub fn beschriftung_texte(&self, k: &Kind) -> (String, String) {
        let Kind::Symbol { bmk, value, ext, .. } = k else { return (String::new(), String::new()) };
        let ctx = self.cur().and_then(|r| Some(canvas::bmk_ctx(self.project.anlagen.get(r.anlage)?, self.project.sheet(r)?))).unwrap_or_default();
        (crate::symbols::display_bmk(bmk, ext.counter, &ctx), self.project.expand_placeholders(value))
    }

    /// Beschriftung (Titel/Wert) eines Symbols unter `m` auf dem aktuellen Blatt – wie im Original auch ohne
    /// vorheriges Markieren, das oberste Symbol zuerst: (Symbol, wert, Lage relativ zum Symbol, Umriss).
    pub fn beschriftung_unter(&self, m: P, tol: f32) -> Option<(u64, bool, P, (P, P))> {
        if self.sym_edit.is_some() {
            return None;
        }
        let s = self.sheet()?;
        s.objects.iter().rev().filter(|o| matches!(o.kind, Kind::Symbol { .. }) && self.selectable(o)).find_map(|o| {
            let (b, v) = self.beschriftung_texte(&o.kind);
            let (wert, off) = beschriftung_bei_mit(&o.kind, &b, &v, m, tol)?;
            let r = beschriftungen_mit(&o.kind, &b, &v)[wert as usize]?;
            Some((o.id, wert, off, r))
        })
    }

    /// Druck auf die Beschriftung eines Symbols: Ziehen der Beschriftung beginnen; das Symbol wird markiert, falls
    /// es das noch nicht ist. true = behandelt.
    pub fn beschriftung_greifen(&mut self, m: P, tol: f32) -> bool {
        let Some((id, wert, orig, _)) = self.beschriftung_unter(m, tol * 0.4) else { return false };
        self.checkpoint();
        if !self.selection.contains(&id) {
            self.selection = vec![id];
        }
        self.drag = Drag::Beschriftung { id, wert, start: m, orig };
        true
    }

    /// Beschriftung beim Ziehen nachführen (Verschiebung seit dem Drücken, auf 0,1 mm gerundet).
    pub fn beschriftung_ziehen(&mut self, m: P) {
        let Drag::Beschriftung { id, wert, start, orig } = self.drag.clone() else { return };
        if m.dist(start) < 1e-4 {
            // bloßer Klick: nichts verändern
            return;
        }
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
        // Symbol um die Ecke (0,0) verdoppeln: Lage und Maßstab wachsen mit, Titel und Wert bleiben auf dem Blatt
        // an ihrer Stelle (Titel bei (4, 18), Wert 2,8 mm darunter)
        let mut k = sym(P::new(10.0, 20.0));
        let vorher = beschriftungen(&k);
        skaliere(&mut k, P::new(0.0, 0.0), 2.0, 2.0);
        let Kind::Symbol { pos, scale, title, .. } = &k else { unreachable!() };
        assert_eq!((*pos, *scale), (P::new(20.0, 40.0), 2.0));
        assert_eq!(pos.add(title.unwrap().0), P::new(4.0, 18.0));
        assert_eq!(beschriftungen(&k), vorher);
        // Text mit Textbox: Schrift und Box wachsen gemeinsam, der Umriss deckt den gezeichneten Text
        let st = TextStyle { rect: Some((P::new(10.0, 18.0), P::new(20.0, 22.0))), ..Default::default() };
        let mut t = Kind::Text { pos: P::new(10.0, 20.0), text: "Raum 2".into(), size: 3.5, align: Align::Left, vertical: false, style: st };
        skaliere(&mut t, P::new(10.0, 20.0), 3.0, 3.0);
        let Kind::Text { pos, size, style, .. } = &t else { unreachable!() };
        assert_eq!((*pos, *size), (P::new(10.0, 20.0), 10.5));
        assert_eq!(style.rect, Some((P::new(10.0, 14.0), P::new(40.0, 26.0))));
        let (a, b) = canvas::obj_bounds(&obj(t.clone()), &[]);
        let w = crate::textmass::breite("Raum 2", 10.5, false, false);
        assert!(a.x <= 10.0 && b.x >= 10.0 + w - 1e-3 && a.y <= 20.0 - 10.5 * 1.15 / 2.0 + 1e-3 && b.y >= 20.0 + 10.5 * 1.15 / 2.0 - 1e-3);
        // Rahmen und Griffe (sel_frame) beruhen auf demselben Umriss
        let (ra, rb) = rahmen(&[obj(t)], &[]).unwrap();
        assert_eq!((ra, rb), (P::new(a.x - RAND, a.y - RAND), P::new(b.x + RAND, b.y + RAND)));
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
