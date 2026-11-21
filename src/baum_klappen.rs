//! Projektexplorer: Auf- und Zuklappen wie im Windows-TreeView des Originals. Ein Klick auf das Plus/Minus-Kästchen
//! und ein Doppelklick auf den Text klappen um, ein einfacher Klick auf den Text markiert nur. Pfeil rechts klappt
//! auf (ist der Knoten offen: zum ersten Kind), Pfeil links klappt zu (ist er zu oder ein Blatt: zum übergeordneten
//! Knoten).

use crate::app::TNode;
use eframe::egui::{self, Color32, Id, Pos2, Rect, Stroke};
use std::collections::HashSet;

/// Wo eine Baumzeile getroffen wurde.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Klick {
    /// Plus/Minus-Kästchen
    Kaestchen,
    /// einfacher Klick auf Symbol/Text
    Text,
    /// Doppelklick auf Symbol/Text
    Doppelt,
}

/// Klappt dieser Klick den Knoten um? Nur Knoten mit Kindern lassen sich auf- und zuklappen.
pub fn klappt_um(hat_kinder: bool, k: Klick) -> bool {
    hat_kinder && matches!(k, Klick::Kaestchen | Klick::Doppelt)
}

/// Auf-/Zu-Zustand eines Knotens umkehren.
pub fn umschalten(offen: &mut HashSet<String>, id: &str) {
    if !offen.remove(id) {
        offen.insert(id.to_string());
    }
}

/// Knoten samt Elternknoten suchen.
fn finden<'a>(nodes: &'a [TNode], id: &str, eltern: Option<&'a TNode>) -> Option<(&'a TNode, Option<&'a TNode>)> {
    for n in nodes {
        if n.id == id {
            return Some((n, eltern));
        }
        if let Some(t) = finden(&n.children, id, Some(n)) {
            return Some(t);
        }
    }
    None
}

/// Pfeiltaste links/rechts auf den markierten Knoten `sel`: klappt um und liefert die neue Markierung.
pub fn pfeil(nodes: &[TNode], offen: &mut HashSet<String>, sel: &str, rechts: bool) -> Option<String> {
    let (n, eltern) = finden(nodes, sel, None)?;
    let kinder = !n.children.is_empty();
    let auf = offen.contains(&n.id);
    if rechts {
        if !kinder {
            return None;
        }
        if auf {
            return Some(n.children[0].id.clone());
        }
        offen.insert(n.id.clone());
        None
    } else if kinder && auf {
        offen.remove(&n.id);
        None
    } else {
        eltern.map(|e| e.id.clone())
    }
}

/// Plus/Minus-Kästchen (9 × 9, grauer Rahmen, weiß gefüllt) mittig auf `c` (Pixelmitte, also x,5/y,5).
pub fn kaestchen(p: &egui::Painter, c: Pos2, offen: bool) {
    let r = Rect::from_min_max(Pos2::new(c.x - 4.0, c.y - 4.0), Pos2::new(c.x + 4.0, c.y + 4.0));
    p.rect_filled(r, 0.0, Color32::WHITE);
    p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(128)), egui::StrokeKind::Middle);
    let s = Stroke::new(1.0, Color32::BLACK);
    p.line_segment([Pos2::new(c.x - 2.5, c.y), Pos2::new(c.x + 2.5, c.y)], s);
    if !offen {
        p.line_segment([Pos2::new(c.x, c.y - 2.5), Pos2::new(c.x, c.y + 2.5)], s);
    }
}

/// Merker „der Baum hat die Tastatur“ (gesetzt durch einen Klick in den Baum, gelöscht durch einen Klick
/// außerhalb); solange gilt, bewegen die Pfeiltasten nicht die markierten Objekte auf dem Blatt.
fn fokus_id() -> Id {
    Id::unique("baum_tastaturfokus")
}

pub fn hat_fokus(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(fokus_id()).unwrap_or(false))
}

pub fn setze_fokus(ctx: &egui::Context, an: bool) {
    ctx.data_mut(|d| d.insert_temp(fokus_id(), an));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::TAction;

    fn k(id: &str, children: Vec<TNode>) -> TNode {
        TNode { id: id.into(), label: id.into(), bold: !children.is_empty(), icon: 0, action: TAction::None, children }
    }

    fn baum() -> Vec<TNode> {
        vec![k("a", vec![k("a/1", vec![]), k("a/2", vec![k("a/2/x", vec![])])]), k("b", vec![])]
    }

    #[test]
    fn klick_auf_text_klappt_nicht() {
        assert!(!klappt_um(true, Klick::Text));
        assert!(klappt_um(true, Klick::Kaestchen));
        assert!(klappt_um(true, Klick::Doppelt));
        // Blätter (ohne Kinder) klappen nie
        assert!(!klappt_um(false, Klick::Kaestchen));
        assert!(!klappt_um(false, Klick::Doppelt));
        // Doppelklick = zwei einfache Klicks + Doppelklick: genau einmal umgeschaltet
        let mut offen = HashSet::new();
        for kl in [Klick::Text, Klick::Text, Klick::Doppelt] {
            if klappt_um(true, kl) {
                umschalten(&mut offen, "a");
            }
        }
        assert!(offen.contains("a"));
    }

    #[test]
    fn pfeiltasten() {
        let b = baum();
        let mut offen = HashSet::new();
        // rechts: zu → auf, Markierung bleibt
        assert_eq!(pfeil(&b, &mut offen, "a", true), None);
        assert!(offen.contains("a"));
        // rechts auf offenem Knoten: zum ersten Kind
        assert_eq!(pfeil(&b, &mut offen, "a", true).as_deref(), Some("a/1"));
        // links auf Blatt: zum Elternknoten
        assert_eq!(pfeil(&b, &mut offen, "a/1", false).as_deref(), Some("a"));
        // links auf offenem Knoten: zu
        assert_eq!(pfeil(&b, &mut offen, "a", false), None);
        assert!(!offen.contains("a"));
        // links auf geschlossener Wurzel: nichts
        assert_eq!(pfeil(&b, &mut offen, "a", false), None);
        // rechts auf Blatt ohne Kinder: nichts
        assert_eq!(pfeil(&b, &mut offen, "b", true), None);
        assert!(offen.is_empty());
        // tiefer Knoten
        assert_eq!(pfeil(&b, &mut offen, "a/2/x", false).as_deref(), Some("a/2"));
        assert_eq!(pfeil(&b, &mut offen, "unbekannt", true), None);
    }
}
