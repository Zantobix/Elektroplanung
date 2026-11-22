//! Vormarkierung auf der Zeichenfläche wie im Original: Schon beim Überfahren mit der Maus bekommt das Objekt unter
//! dem Zeiger ein gestricheltes Rechteck um seinen Umriss; über Verweisen und den Querverweisen der Sammelschienen
//! erscheint der Hinweis auf Strg+Klick. Welchen Zeiger das Ziel bekommt, entscheidet `zeiger.rs`.
//!
//! Die Reihenfolge der Prüfung ist dieselbe wie beim Drücken der Maustaste (view.rs), damit die Vormarkierung
//! genau das Objekt zeigt, das ein Klick greift.

use crate::app::{frame_handles, App, FRAME_HANDLE};
use crate::canvas::{self, Xf};
use crate::model::*;
use eframe::egui::{self, Color32, Rect, Shape, Stroke};

/// Art eines Objekts für den Mauszeiger (wie im Original unterschieden).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objektart {
    /// Symbol, Leitung, Text, Zeichenobjekt: Pfeilkreuz
    Verschiebbar,
    /// Sammelschiene (Leitungen beginnen dort): Leitungszeichen
    Anschluss,
    /// Klemmstelle, Kontaktspiegel: einfacher Pfeil
    Schlicht,
}

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
    /// Verweis bzw. (`text`) Querverweistext einer Sammelschiene: Strg+Klick springt zum Ziel
    Verweis { id: u64, mn: P, mx: P, text: bool },
    /// Text auf dem Blatt
    Text { id: u64, mn: P, mx: P },
    /// Leitung: Pfeilkreuz, aber ohne gestricheltes Rechteck (Nutzerwunsch 22.11.2026: Vormarkierung nur bei
    /// „wirklichen“ Objekten – bewusste Abweichung)
    Leitung { id: u64 },
    /// jedes andere Objekt (Symbol, Linie …)
    Objekt { id: u64, mn: P, mx: P, art: Objektart },
}

impl Ziel {
    /// Umriss der Vormarkierung (None = keine).
    pub fn umriss(&self) -> Option<(P, P)> {
        match *self {
            Ziel::Beschriftung { mn, mx, .. } | Ziel::Verweis { mn, mx, .. } | Ziel::Text { mn, mx, .. } | Ziel::Objekt { mn, mx, .. } => Some((mn, mx)),
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
            return Some(Ziel::Verweis { id: f.quelle, mn: f.mn, mx: f.mx, text: true });
        }
        // Punkt eines Zeichenobjekts: Pfeil mit vier Pfeilspitzen wie beim Eckpunkt (Original Zeiger 5, zeichen_punkt.rs)
        if self.zeichenpunkt_unter(m, tol).is_some() {
            return Some(Ziel::Ecke);
        }
        let id = self.hit_top(m, tol)?;
        let o = self.sheet()?.objects.iter().find(|o| o.id == id)?;
        let (mn, mx) = canvas::obj_bounds(o, &self.lib);
        Some(match o.kind {
            Kind::Verweis { .. } if self.sym_edit.is_none() => Ziel::Verweis { id, mn, mx, text: false },
            Kind::Text { .. } => Ziel::Text { id, mn, mx },
            // im Symboleditor sind alle Objekte Zeichenelemente
            _ if self.sym_edit.is_some() => Ziel::Objekt { id, mn, mx, art: Objektart::Verschiebbar },
            // Sonderobjekte Art 0–2 (Klemmstelle, Sammelschiene, Verweis): Leitungszeiger (Original 0x415e9e)
            Kind::Busbar { .. } | Kind::Terminal { .. } => Ziel::Objekt { id, mn, mx, art: Objektart::Anschluss },
            Kind::Wire { .. } => Ziel::Leitung { id },
            Kind::Spiegel { .. } => Ziel::Objekt { id, mn, mx, art: Objektart::Schlicht },
            _ => Ziel::Objekt { id, mn, mx, art: Objektart::Verschiebbar },
        })
    }
}

fn speicher_id() -> egui::Id {
    egui::Id::unique("ep_vormarkierung")
}

/// Ziel dieses Bildes merken (Eingabe → Zeichnen); über Verweisen erscheint der Hinweis.
pub fn setzen(ctx: &egui::Context, resp: &egui::Response, ziel: Option<Ziel>) {
    ctx.data_mut(|d| d.insert_temp(speicher_id(), ziel));
    let Some(z) = ziel else { return };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umriss() {
        let r = (P::default(), P::new(1.0, 1.0));
        let objekt = Ziel::Objekt { id: 1, mn: r.0, mx: r.1, art: Objektart::Verschiebbar };
        assert!(Ziel::Anschluss.umriss().is_none() && Ziel::Griff(1).umriss().is_none() && objekt.umriss() == Some(r));
        assert!(Ziel::Leitung { id: 1 }.umriss().is_none());
        assert_eq!(Ziel::Verweis { id: 1, mn: r.0, mx: r.1, text: true }.umriss(), Some(r));
    }
}
