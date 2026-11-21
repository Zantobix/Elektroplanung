//! Vormarkierung und Mauszeiger auf der Zeichenfläche wie im Original: Schon beim Überfahren mit der Maus bekommt
//! das Objekt unter dem Zeiger ein gestricheltes Rechteck um seinen Umriss, und der Zeiger zeigt, was ein Druck
//! bewirken würde – über Objekten/Symbolen ein Pfeil mit Verschiebekreuz, über Texten (auch Titel und Wert eines
//! Symbols) ein Pfeil mit „A“, über Verweisen und den Querverweisen der Sammelschienen die Hand mit dem Hinweis
//! auf Strg+Klick, über den Griffen des Rahmens die Größenpfeile.
//!
//! egui kennt nur die Standardzeiger des Systems. Pfeil + Zusatz gibt es dort nicht; deshalb bleibt der normale
//! Pfeil und der Zusatz (Kreuz bzw. „A“) wird wie im Original rechts unter der Pfeilspitze dazugezeichnet.
//! Die Reihenfolge der Prüfung ist dieselbe wie beim Drücken der Maustaste (view.rs), damit die Vormarkierung
//! genau das Objekt zeigt, das ein Klick greift.

use crate::app::{frame_handles, App, FRAME_HANDLE};
use crate::canvas::{self, Xf};
use crate::model::*;
use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Shape, Stroke, Vec2};

/// Was unter der Maus liegt (Umrisse in mm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ziel {
    /// Griff 0–7 des Rahmens zur Größenänderung (links oben im Uhrzeigersinn)
    Griff(usize),
    /// Eckpunkt eines Zwei-Punkt-Objekts (Größe ändern)
    Ecke,
    /// Anschluss: ein Druck beginnt eine Leitung (grüner Kreis, keine Vormarkierung)
    Anschluss,
    /// Titel (`wert` = false) bzw. Wert eines Symbols: Text gegenüber dem Symbol verschieben
    Beschriftung { id: u64, wert: bool, mn: P, mx: P },
    /// Verweis bzw. Querverweistext einer Sammelschiene: Strg+Klick springt zum Ziel
    Verweis { id: u64, mn: P, mx: P },
    /// Text auf dem Blatt
    Text { id: u64, mn: P, mx: P },
    /// jedes andere Objekt (Symbol, Leitung, Linie …)
    Objekt { id: u64, mn: P, mx: P },
}

impl Ziel {
    /// Umriss der Vormarkierung (None = keine).
    pub fn umriss(&self) -> Option<(P, P)> {
        match *self {
            Ziel::Beschriftung { mn, mx, .. } | Ziel::Verweis { mn, mx, .. } | Ziel::Text { mn, mx, .. } | Ziel::Objekt { mn, mx, .. } => Some((mn, mx)),
            _ => None,
        }
    }

    /// Mauszeiger des Systems für dieses Ziel.
    pub fn zeiger(&self) -> CursorIcon {
        match *self {
            Ziel::Griff(i) => match i % 8 {
                0 | 4 => CursorIcon::ResizeNwSe,
                2 | 6 => CursorIcon::ResizeNeSw,
                1 | 5 => CursorIcon::ResizeVertical,
                _ => CursorIcon::ResizeHorizontal,
            },
            Ziel::Ecke => CursorIcon::Crosshair,
            Ziel::Verweis { .. } => CursorIcon::PointingHand,
            _ => CursorIcon::Default,
        }
    }

    /// Zusatz am Pfeil: Some(true) = „A“ (Text verschieben), Some(false) = Verschiebekreuz, None = keiner.
    pub fn zusatz(&self) -> Option<bool> {
        match self {
            Ziel::Beschriftung { .. } | Ziel::Text { .. } => Some(true),
            Ziel::Objekt { .. } => Some(false),
            _ => None,
        }
    }
}

impl App {
    /// Ziel unter dem Mauspunkt `m` (mm) im Auswahlwerkzeug – Reihenfolge wie beim Drücken in view.rs: Griffe des
    /// Rahmens, Eckpunkte, Titel/Wert eines Symbols, Anschlüsse, Querverweistexte der Sammelschienen, oberstes Objekt.
    pub fn vormarkierung(&self, m: P, tol: f32, z: f32) -> Option<Ziel> {
        if let Some((mn, mx)) = self.sel_frame() {
            let r = (FRAME_HANDLE / 2.0).max(2.5 / z) + 0.5 / z;
            let nur_ecken = crate::groesse::proportional(&self.selected_objs());
            if let Some(i) = frame_handles(mn, mx).iter().enumerate().position(|(i, q)| !(nur_ecken && i % 2 == 1) && (q.x - m.x).abs() <= r && (q.y - m.y).abs() <= r) {
                return Some(Ziel::Griff(i));
            }
        }
        if self.size_frame && self.selection.len() == 1 {
            if let Some((a, b)) = self.selected_objs().first().and_then(|o| o.kind.frame_points().filter(|_| !(o.fixed && self.fix_active))) {
                if [a, b, P::new(a.x, b.y), P::new(b.x, a.y)].iter().any(|c| c.dist(m) <= tol * 1.2) {
                    return Some(Ziel::Ecke);
                }
            }
        }
        if let Some((id, wert, _, (mn, mx))) = self.beschriftung_unter(m, tol * 0.4) {
            return Some(Ziel::Beschriftung { id, wert, mn, mx });
        }
        if self.pin_near(m, tol, &[]).is_some() && !self.selected_objs().iter().any(|o| canvas::hit(o, m, tol, &self.lib)) {
            return Some(Ziel::Anschluss);
        }
        if let Some(f) = self.cur().filter(|_| self.sym_edit.is_none()).and_then(|r| crate::verweise::busbar_text_at(&self.project, r, m, tol)) {
            return Some(Ziel::Verweis { id: f.quelle, mn: f.mn, mx: f.mx });
        }
        let id = self.hit_top(m, tol)?;
        let o = self.sheet()?.objects.iter().find(|o| o.id == id)?;
        let (mn, mx) = canvas::obj_bounds(o, &self.lib);
        Some(match o.kind {
            Kind::Verweis { .. } if self.sym_edit.is_none() => Ziel::Verweis { id, mn, mx },
            Kind::Text { .. } => Ziel::Text { id, mn, mx },
            _ => Ziel::Objekt { id, mn, mx },
        })
    }
}

fn speicher_id() -> egui::Id {
    egui::Id::unique("ep_vormarkierung")
}

/// Ziel dieses Bildes merken (Eingabe → Zeichnen) und den Mauszeiger setzen; über Verweisen erscheint der Hinweis.
pub fn setzen(ctx: &egui::Context, resp: &egui::Response, ziel: Option<Ziel>) {
    ctx.data_mut(|d| d.insert_temp(speicher_id(), ziel));
    let Some(z) = ziel else { return };
    if z.zeiger() != CursorIcon::Default {
        ctx.set_cursor_icon(z.zeiger());
    }
    if matches!(z, Ziel::Verweis { .. }) {
        resp.clone().on_hover_text_at_pointer("Strg+Klick: zum Ziel springen");
    }
}

/// Gemerktes Ziel dieses Bildes.
pub fn gemerkt(ctx: &egui::Context) -> Option<Ziel> {
    ctx.data(|d| d.get_temp::<Option<Ziel>>(speicher_id())).flatten()
}

/// Gestricheltes Rechteck um das Ziel (wie im Original 1 px, kurze Striche, etwas Abstand zum Umriss).
pub fn rechteck(p: &egui::Painter, xf: Xf, ziel: &Ziel) {
    let Some((mn, mx)) = ziel.umriss() else { return };
    let r = Rect::from_min_max(xf.s(mn), xf.s(mx)).expand(2.0);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.add(Shape::dashed_line(&pts, Stroke::new(1.0, Color32::BLACK), 2.0, 2.0));
}

/// Zusatz rechts unter der Pfeilspitze: Verschiebekreuz bzw. „A“ (schwarz mit weißem Rand wie die Systemzeiger).
pub fn zusatz(p: &egui::Painter, zeiger: Pos2, ziel: &Ziel) {
    let Some(text) = ziel.zusatz() else { return };
    let c = zeiger + Vec2::new(15.0, 16.0);
    let weiss = Stroke::new(3.0, Color32::WHITE);
    let schwarz = Stroke::new(1.0, Color32::BLACK);
    if text {
        let font = egui::FontId::new(13.0, egui::FontFamily::Name("bold".into()));
        for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
            p.text(c + d, egui::Align2::CENTER_CENTER, "A", font.clone(), Color32::WHITE);
        }
        p.text(c, egui::Align2::CENTER_CENTER, "A", font, Color32::BLACK);
        return;
    }
    // Kreuz mit vier Pfeilspitzen
    let (l, s) = (6.0, 2.5);
    let spitzen = |st: Stroke| {
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let tip = c + Vec2::new(dx * l, dy * l);
            let back = tip - Vec2::new(dx * s, dy * s);
            let quer = Vec2::new(-dy * s, dx * s);
            p.line_segment([tip, back + quer], st);
            p.line_segment([tip, back - quer], st);
        }
    };
    for st in [weiss, schwarz] {
        p.line_segment([c - Vec2::new(l, 0.0), c + Vec2::new(l, 0.0)], st);
        p.line_segment([c - Vec2::new(0.0, l), c + Vec2::new(0.0, l)], st);
        spitzen(st);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeiger_und_zusatz() {
        let r = (P::default(), P::new(1.0, 1.0));
        let objekt = Ziel::Objekt { id: 1, mn: r.0, mx: r.1 };
        let text = Ziel::Beschriftung { id: 1, wert: false, mn: r.0, mx: r.1 };
        let verweis = Ziel::Verweis { id: 1, mn: r.0, mx: r.1 };
        assert_eq!((objekt.zeiger(), objekt.zusatz()), (CursorIcon::Default, Some(false)));
        assert_eq!((text.zeiger(), text.zusatz()), (CursorIcon::Default, Some(true)));
        assert_eq!((verweis.zeiger(), verweis.zusatz()), (CursorIcon::PointingHand, None));
        assert_eq!(Ziel::Griff(4).zeiger(), CursorIcon::ResizeNwSe);
        assert_eq!(Ziel::Griff(3).zeiger(), CursorIcon::ResizeHorizontal);
        assert!(Ziel::Anschluss.umriss().is_none() && objekt.umriss() == Some(r));
    }
}
