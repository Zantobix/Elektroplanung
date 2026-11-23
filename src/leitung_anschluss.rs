//! Neue Leitung an einer anderen Leitung beginnen bzw. enden lassen (Prüfliste Runde 12, A5–A8).
//!
//! - Ende im Innern einer Leitung (A5): `leitungsende` fängt auf das getroffene Teilstück (leitung_route.rs,
//!   `ende_auf_leitung`); es entsteht ein Abzweig, den Punkt setzt die Knotenregel (knoten.rs: Ende im Innern
//!   einer anderen Leitung).
//! - Ragt die getroffene Leitung mit einem **freien** Ende (an keinem Anschluss, keiner Klemmstelle, keiner
//!   Schiene, keiner anderen Leitung) über den Verbindungspunkt hinaus und trägt der überstehende Teil nichts
//!   (kein fremder Anschluss, kein Abzweig darauf), wird sie bis zum Verbindungspunkt gekürzt (A6, Nutzer-Befund
//!   Bild 4): aus dem T wird eine Ecke bzw. Fortsetzung ohne Knoten. Ist das andere Ende auch frei (lose Leitung),
//!   bleibt sie, wie sie ist (Abzweig), weil nicht zu entscheiden ist, welcher Teil übersteht.
//! - Ende genau auf dem Ende einer anderen Leitung (A7) ist ein Fangpunkt (`pin_near`) und eine Fortsetzung.
//! - Anfang im Innern einer Leitung (A8): Umschalt + Ziehen auf einer Leitung beginnt dort eine neue Leitung
//!   (ohne Umschalt greift das Ziehen wie bisher die Leitung bzw. ihr Teilstück).
//!
//! Das Kürzen gehört zum Rückgängig-Schritt des Anlegens (`add_obj`).

use crate::app::{App, Drag};
use crate::canvas;
use crate::mitziehen::PIN_TOL;
use crate::model::*;
use crate::symbols::SymDef;

/// Ist das Ende `p` der Leitung `wid` frei (kein Anschluss, keine Klemmstelle, keine Schiene, keine Leitung)?
pub fn freies_ende(sheet: &Sheet, lib: &[SymDef], wid: u64, p: P) -> bool {
    freies_ende_in(&sheet.objects, lib, wid, p)
}

/// Wie `freies_ende` für die Objekte `objs`.
pub fn freies_ende_in(objs: &[Obj], lib: &[SymDef], wid: u64, p: P) -> bool {
    !objs.iter().filter(|o| o.id != wid).any(|o| match &o.kind {
        Kind::Wire { pts, .. } => pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) <= PIN_TOL),
        Kind::Busbar { a, x2, .. } => (p.y - a.y).abs() <= PIN_TOL && p.x >= a.x.min(*x2) - PIN_TOL && p.x <= a.x.max(*x2) + PIN_TOL,
        Kind::Terminal { pos, .. } => pos.dist(p) <= crate::klemme_leitung::KLEMME_R + PIN_TOL,
        _ => canvas::obj_pins(o, lib).iter().any(|q| q.dist(p) <= PIN_TOL),
    })
}

/// Liegt auf dem Zug `teil` (ohne den Punkt `x`) ein Anschluss bzw. Leitungsende eines anderen Objekts als `wid`?
fn traegt(sheet: &Sheet, lib: &[SymDef], wid: u64, teil: &[P], x: P) -> bool {
    sheet
        .objects
        .iter()
        .filter(|o| o.id != wid)
        .flat_map(|o| canvas::obj_pins(o, lib))
        .any(|q| q.dist(x) > PIN_TOL && teil.windows(2).any(|s| seg_dist(q, s[0], s[1]) <= PIN_TOL))
}

/// Liegt der Punkt `x` (Ende der neuen Leitung `neu`) im Innern einer anderen Leitung mit freiem, unbelastetem
/// Überstand auf genau einer Seite: Kennung und gekürzte Punkte dieser Leitung.
pub fn gekuerzt(sheet: &Sheet, lib: &[SymDef], neu: u64, x: P) -> Option<(u64, Vec<P>)> {
    for o in sheet.objects.iter().filter(|o| o.id != neu) {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        if !crate::knoten::im_innern(pts, x) {
            continue;
        }
        let Some(i) = (0..pts.len() - 1).find(|&i| seg_dist(x, pts[i], pts[i + 1]) <= PIN_TOL) else { continue };
        let mut a = pts[..=i].to_vec();
        a.push(x);
        let mut b = vec![x];
        b.extend_from_slice(&pts[i + 1..]);
        let (a, b) = (crate::leitung_fuehrung::bereinigen(a), crate::leitung_fuehrung::bereinigen(b));
        let n = pts.len();
        let frei = (freies_ende(sheet, lib, o.id, pts[0]), freies_ende(sheet, lib, o.id, pts[n - 1]));
        match frei {
            (true, false) if !traegt(sheet, lib, o.id, &a, x) => return Some((o.id, b)),
            (false, true) if !traegt(sheet, lib, o.id, &b, x) => return Some((o.id, a)),
            _ => {}
        }
    }
    None
}

impl App {
    /// Gezeichnete Leitung anlegen (`neue_leitung`) und getroffene Leitungen mit freiem Überstand an beiden Enden
    /// der neuen Leitung kürzen (gleicher Rückgängig-Schritt).
    pub fn leitung_anschliessen(&mut self, pts: Vec<P>) -> Option<u64> {
        let enden = [*pts.first()?, *pts.last()?];
        let id = self.neue_leitung(pts)?;
        for x in enden {
            let Some((wid, neu)) = self.sheet().and_then(|s| gekuerzt(s, &self.lib, id, x)) else { continue };
            self.with_sheet(|s, _| {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == wid).map(|o| &mut o.kind) {
                    *pts = neu;
                }
            });
        }
        // stößt sie ohne Abzweig an eine andere Leitung: zu einer zusammenfassen (leitung_zusammen.rs); die
        // gezeichnete kann dabei in der älteren aufgehen
        self.leitungen_zusammenfassen(&[id]);
        if self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.id == id)) {
            return Some(id);
        }
        // aufgegangen: die Leitung, die jetzt an ihrer Stelle liegt
        self.sheet().and_then(|s| s.objects.iter().filter(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts.windows(2).any(|w| seg_dist(enden[0], w[0], w[1]) < 0.01) && pts.windows(2).any(|w| seg_dist(enden[1], w[0], w[1]) < 0.01))).map(|o| o.id).next())
    }

    /// Umschalt + Drücken auf einer Leitung (Auswahlwerkzeug): dort eine neue Leitung beginnen (Abzweig, A8). Der
    /// Anfang liegt auf dem getroffenen Teilstück in der Flucht des Rasterpunkts der Maus.
    pub fn leitung_an_leitung_beginnen(&mut self, m: P, tol: f32) -> bool {
        if self.sym_edit.is_some() {
            return false;
        }
        let Some(p) = self.abzweig_anfang(m, tol) else { return false };
        self.drag = Drag::Wire { f: crate::leitung_fuehrung::Fuehrung::neu(p) };
        true
    }

    /// Anfang eines Abzweigs bei Umschalt + Drücken auf einer Leitung: auf dem getroffenen Teilstück im Raster –
    /// auch mit gehaltener Umschalttaste, die sonst den Rasterfang abschaltet (Nutzer Runde 13: „wenn eine Leitung
    /// von einer Leitung gezeichnet wird, soll diese auch am Raster orientiert werden“).
    pub fn abzweig_anfang(&self, m: P, tol: f32) -> Option<P> {
        let q = if self.fang() > 0.0 { crate::raster::fang(m, self.fang(), self.raster_ursprung()) } else { m };
        self.sheet().and_then(|s| crate::leitung_route::ende_auf_leitung(s, q, m, tol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    fn wire(id: u64, pts: Vec<P>) -> Obj {
        Obj { id, kind: Kind::Wire { pts, leitung: Default::default(), bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn klemme(id: u64, pos: P) -> Obj {
        Obj { id, kind: Kind::Terminal { pos, leiste: "X".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn blatt(obj: Vec<Obj>) -> Sheet {
        let mut s = Sheet::new(1);
        s.objects = obj;
        s
    }

    #[test]
    fn freier_ueberstand_wird_gekuerzt() {
        // Leitung 1 von der Klemme (0,0) nach rechts bis (100,0), Ende frei; neue Leitung 2 von unten auf (60,0)
        let s = blatt(vec![klemme(9, p(-0.8, 0.0)), wire(1, vec![p(0.0, 0.0), p(100.0, 0.0)]), wire(2, vec![p(60.0, 50.0), p(60.0, 0.0)])]);
        assert!(!freies_ende(&s, &[], 1, p(0.0, 0.0)));
        assert!(freies_ende(&s, &[], 1, p(100.0, 0.0)));
        assert_eq!(gekuerzt(&s, &[], 2, p(60.0, 0.0)), Some((1, vec![p(0.0, 0.0), p(60.0, 0.0)])));
        // danach: Ecke, kein Knoten
        let mut s2 = s.clone();
        if let Kind::Wire { pts, .. } = &mut s2.objects[1].kind {
            *pts = vec![p(0.0, 0.0), p(60.0, 0.0)];
        }
        assert!(crate::knoten::punkte(&s2).is_empty());
        // ohne Kürzen wäre es ein Abzweig mit Knoten
        assert_eq!(crate::knoten::punkte(&s), vec![p(60.0, 0.0)]);
    }

    #[test]
    fn ueberstand_ueber_knicke_und_freier_anfang() {
        // freier Anfang links, Ende an der Klemme; Überstand mit Knick wird ganz entfernt
        let s = blatt(vec![wire(1, vec![p(0.0, 30.0), p(0.0, 0.0), p(100.0, 0.0)]), klemme(9, p(100.8, 0.0)), wire(2, vec![p(40.0, 50.0), p(40.0, 0.0)])]);
        assert_eq!(gekuerzt(&s, &[], 2, p(40.0, 0.0)), Some((1, vec![p(40.0, 0.0), p(100.0, 0.0)])));
    }

    #[test]
    fn abzweig_bleibt_mit_knoten() {
        // beide Enden angeschlossen: kein Kürzen, Knoten am Abzweig
        let s = blatt(vec![klemme(8, p(-0.8, 0.0)), wire(1, vec![p(0.0, 0.0), p(100.0, 0.0)]), klemme(9, p(100.8, 0.0)), wire(2, vec![p(60.0, 50.0), p(60.0, 0.0)])]);
        assert_eq!(gekuerzt(&s, &[], 2, p(60.0, 0.0)), None);
        assert_eq!(crate::knoten::punkte(&s), vec![p(60.0, 0.0)]);
        // lose Leitung (beide Enden frei): bleibt
        let s = blatt(vec![wire(1, vec![p(0.0, 0.0), p(100.0, 0.0)]), wire(2, vec![p(60.0, 50.0), p(60.0, 0.0)])]);
        assert_eq!(gekuerzt(&s, &[], 2, p(60.0, 0.0)), None);
        // Überstand trägt einen Abzweig (Leitung 3 endet bei x = 80): bleibt
        let s = blatt(vec![klemme(8, p(-0.8, 0.0)), wire(1, vec![p(0.0, 0.0), p(100.0, 0.0)]), wire(3, vec![p(80.0, -30.0), p(80.0, 0.0)]), wire(2, vec![p(60.0, 50.0), p(60.0, 0.0)])]);
        assert_eq!(gekuerzt(&s, &[], 2, p(60.0, 0.0)), None);
    }

    #[test]
    fn fortsetzung_am_freien_ende_ohne_knoten() {
        // Ende genau auf dem freien Ende: nichts zu kürzen, kein Knoten (zwei Richtungen)
        let s = blatt(vec![klemme(8, p(-0.8, 0.0)), wire(1, vec![p(0.0, 0.0), p(100.0, 0.0)]), wire(2, vec![p(100.0, 50.0), p(100.0, 0.0)])]);
        assert_eq!(gekuerzt(&s, &[], 2, p(100.0, 0.0)), None);
        assert!(crate::knoten::punkte(&s).is_empty());
    }
}
