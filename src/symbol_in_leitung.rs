//! Symbol auf eine Leitung gesetzt, die sich nicht einbinden lässt – Meldung wie im Original.
//!
//! Beim Ablegen eines Symbols (PaintBox1DragDrop 0x4407d0) ruft das Original 0x47bf14 mit Art 1:
//! - 0x47c998 sucht waagerechte und senkrechte Leitungsstücke, die den Umriss des Symbols ganz durchqueren
//!   (waagerecht: Höhe innerhalb des Umrisses, Stück reicht links und rechts darüber hinaus; senkrecht ebenso);
//! - 0x47cc14 ordnet ihnen Anschlüsse zu: jeder Anschluss bekommt Seitenmerker nach dem nächsten Rand des Umrisses
//!   (1 oben, 4 unten, 8 links, 2 rechts; bei gleichem Abstand beide). Ein senkrechtes Stück braucht einen
//!   Anschluss oben oder unten, ein waagerechtes einen links oder rechts.
//! - Gibt es durchquerende Stücke, aber für keines einen passenden Anschluss, meldet das Original (Überschrift
//!   „Achtung!“) „Das Symbol kann hier nicht automatisch in die Verbindung integriert werden. Bitte dies manuell
//!   durchführen!“; das Symbol bleibt liegen, die Leitung unverändert.
//!
//! Der Nachbau trennt passende Leitungen vorher selbst auf (`App::insert_into_wire`); geprüft wird danach, ob noch
//! ein Stück den Umriss durchquert und ob sich dafür ein Anschluss findet.

use crate::model::*;
use crate::symbols::SymDef;

pub const MELDUNG: &str = "Das Symbol kann hier nicht automatisch in die Verbindung\nintegriert werden. Bitte dies manuell durchführen!";

const EPS: f32 = 1e-3;

/// Seitenmerker eines Anschlusses `p` im Umriss `(mn, mx)` wie 0x47cc14 (Punktanschluss).
pub fn seiten(p: P, mn: P, mx: P) -> u8 {
    let (oben, unten) = (p.y - mn.y, mx.y - p.y);
    let (mut f, senk) = if (oben - unten).abs() < EPS {
        (5u8, oben)
    } else if oben < unten {
        (1, oben)
    } else {
        (4, unten)
    };
    let (links, rechts) = (p.x - mn.x, mx.x - p.x);
    if rechts <= senk + EPS || links <= senk + EPS {
        if rechts < senk - EPS || links < senk - EPS {
            f = 0;
        }
        if (rechts - links).abs() < EPS {
            f |= 10;
        } else if links < rechts {
            f |= 8;
        } else {
            f = 2;
        }
    }
    f
}

/// Durchquerende Leitungsstücke des Umrisses: (waagerecht?) je Stück.
pub fn durchquerungen(sheet: &Sheet, mn: P, mx: P) -> Vec<bool> {
    let mut out = vec![];
    for o in &sheet.objects {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if (a.y - b.y).abs() < EPS && mn.y <= a.y && a.y <= mx.y && a.x.min(b.x) <= mn.x && a.x.max(b.x) >= mx.x {
                out.push(true);
            } else if (a.x - b.x).abs() < EPS && mn.x <= a.x && a.x <= mx.x && a.y.min(b.y) <= mn.y && a.y.max(b.y) >= mx.y {
                out.push(false);
            }
        }
    }
    out
}

/// Meldung nötig? Das Symbol `id` liegt auf durchquerenden Leitungsstücken, für die keiner seiner Anschlüsse passt.
pub fn nicht_integrierbar(sheet: &Sheet, lib: &[SymDef], id: u64) -> bool {
    let Some(o) = sheet.objects.iter().find(|o| o.id == id) else { return false };
    if !matches!(o.kind, Kind::Symbol { .. }) {
        return false;
    }
    let (mn, mx) = crate::canvas::obj_bounds(o, lib);
    let quer = durchquerungen(sheet, mn, mx);
    if quer.is_empty() {
        return false;
    }
    let merker: Vec<u8> = crate::canvas::obj_pins(o, lib).into_iter().map(|p| seiten(p, mn, mx)).collect();
    !quer.iter().any(|waagerecht| merker.iter().any(|f| f & if *waagerecht { 8 | 2 } else { 1 | 4 } != 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seitenmerker_wie_original() {
        let (mn, mx) = (P::new(0.0, 0.0), P::new(4.0, 10.0));
        assert_eq!(seiten(P::new(2.0, 0.0), mn, mx), 1);
        assert_eq!(seiten(P::new(2.0, 10.0), mn, mx), 4);
        assert_eq!(seiten(P::new(0.0, 5.0), mn, mx), 8);
        assert_eq!(seiten(P::new(4.0, 5.0), mn, mx), 2);
        // Mitte eines Quadrats: alle Seiten
        assert_eq!(seiten(P::new(2.0, 2.0), mn, P::new(4.0, 4.0)), 15);
    }

    fn blatt(pts: Vec<P>) -> Sheet {
        let mut s = Sheet::new(1);
        s.objects.push(Obj { id: 1, kind: Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        s
    }

    #[test]
    fn durchquerende_stuecke() {
        let (mn, mx) = (P::new(10.0, 10.0), P::new(14.0, 20.0));
        assert_eq!(durchquerungen(&blatt(vec![P::new(12.0, 0.0), P::new(12.0, 40.0)]), mn, mx), vec![false]);
        assert_eq!(durchquerungen(&blatt(vec![P::new(0.0, 15.0), P::new(30.0, 15.0)]), mn, mx), vec![true]);
        // endet im Umriss (z. B. schon am Anschluss aufgetrennt): kein Durchqueren
        assert!(durchquerungen(&blatt(vec![P::new(12.0, 0.0), P::new(12.0, 10.0)]), mn, mx).is_empty());
    }
}
