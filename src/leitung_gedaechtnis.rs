//! Formgedächtnis der Leitungen je Quadrant (nach QElectroTech `ConductorProfilesGroup`, GPL-2.0-or-later; neu
//! geschrieben, kein Code übernommen).
//!
//! QElectroTech merkt sich für jeden Leiter eine Form („Profil“) je Quadrant, in dem das Ende vom Anfang aus liegt.
//! Wandert ein Ende in einen anderen Quadranten und zurück, kehrt die alte Form wieder. Im Nachbau:
//!
//! - Beim Beginn jeder Geste (`netz_merken`) wird die aktuelle Form jeder Leitung für ihren aktuellen Quadranten
//!   gemerkt (überschreibt: die Lage auf dem Blatt ist immer die gültige Form dieses Quadranten – auch nach
//!   Handänderungen).
//! - Benutzt wird eine gemerkte Form nur, wenn eine Leitung zwischen zwei festen Anschlüssen beim Nachführen in einen
//!   **anderen** Quadranten kommt, für den eine Form gemerkt ist; sie wird dann wie beim Profil-Verfahren
//!   (`leitung_profil.rs`) auf die neue Breite und Höhe gebracht.
//!
//! Nur zur Laufzeit, die Datei bleibt unverändert. Wirkung: Symbol hin und zurück ⇒ dieselbe Leitungsform
//! (Kriterium L10).

use crate::model::P;
use std::collections::HashMap;

const T: f32 = 0.05;

/// Quadrant des Endes `b` vom Anfang `a` aus (je Achse −1, 0, 1).
pub fn quadrant(a: P, b: P) -> (i8, i8) {
    let s = |d: f32| if d > T { 1 } else if d < -T { -1 } else { 0 };
    (s(b.x - a.x), s(b.y - a.y))
}

/// Gemerkte Formen: je Leitung und Quadrant die Punkte relativ zum Anfang.
#[derive(Default, Clone, Debug)]
pub struct Gedaechtnis {
    m: HashMap<u64, Vec<((i8, i8), Vec<P>)>>,
    /// zuletzt gemerkte genaue Lagen je Leitung (höchstens `LAGEN`, neueste zuletzt; je Endpunktpaar eine)
    lagen: HashMap<u64, Vec<Vec<P>>>,
}

/// Zahl der gemerkten genauen Lagen je Leitung.
const LAGEN: usize = 6;

impl Gedaechtnis {
    /// Form der Leitung `id` für ihren Quadranten merken (ersetzt eine frühere Form dieses Quadranten).
    pub fn merken(&mut self, id: u64, pts: &[P]) {
        if pts.len() < 2 {
            return;
        }
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let l = self.lagen.entry(id).or_default();
        l.retain(|w| !(w[0].dist(a) < T && w[w.len() - 1].dist(b) < T));
        l.push(pts.to_vec());
        if l.len() > LAGEN {
            l.remove(0);
        }
        let q = quadrant(a, b);
        let rel: Vec<P> = pts.iter().map(|p| p.sub(pts[0])).collect();
        let v = self.m.entry(id).or_default();
        match v.iter_mut().find(|(k, _)| *k == q) {
            Some(e) => e.1 = rel,
            None => v.push((q, rel)),
        }
    }

    /// Gemerkte Form der Leitung `id` von `a` nach `b`, wenn für deren Quadrant eine gemerkt ist und sie sich auf
    /// die neue Breite und Höhe bringen lässt.
    pub fn form(&self, id: u64, a: P, b: P, rund: &dyn Fn(P) -> P) -> Option<Vec<P>> {
        let q = quadrant(a, b);
        let rel = &self.m.get(&id)?.iter().find(|(k, _)| *k == q)?.1;
        let ende = *rel.last()?;
        // gemerkte Form so legen, dass sie in `b` endet, dann den Anfang nach `a` nachführen
        let start = b.sub(ende);
        let pts: Vec<P> = rel.iter().map(|r| start.add(*r)).collect();
        match crate::leitung_profil::nachfuehren(&pts, a, rund) {
            crate::leitung_profil::Ergebnis::Form(p) => Some(p),
            _ => None,
        }
    }

    /// Genau gemerkte Lage der Leitung `id` mit den Enden `a` und `b` (beide Enden wieder dort, wo die Form gemerkt
    /// wurde: Rückkehr, Kriterium L10).
    pub fn genau(&self, id: u64, a: P, b: P) -> Option<Vec<P>> {
        self.lagen.get(&id)?.iter().rev().find(|w| w[0].dist(a) < 0.01 && w[w.len() - 1].dist(b) < 0.01).cloned()
    }

    /// Gemerkte Lage, deren Ende `k` (0 Anfang, 1 Ende) genau bei `q` liegt und deren anderes Ende `passt` (z. B. auf
    /// demselben Wirt wie jetzt: ein Abzweig-Ende gleitet zurück).
    pub fn mit_ende(&self, id: u64, k: usize, q: P, passt: &dyn Fn(P) -> bool) -> Option<Vec<P>> {
        self.lagen.get(&id)?.iter().rev().find(|w| {
            let (e, anders) = if k == 0 { (w[0], w[w.len() - 1]) } else { (w[w.len() - 1], w[0]) };
            e.dist(q) < 0.01 && passt(anders)
        }).cloned()
    }

    /// Alles vergessen (anderes Projekt).
    pub fn leeren(&mut self) {
        self.m.clear();
        self.lagen.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_kehrt_im_alten_quadranten_zurueck() {
        let id = |p: P| p;
        let mut g = Gedaechtnis::default();
        // Stich 10 nach unten, rechts, hoch: Ende rechts oberhalb des Anfangs
        let alt = [P::new(0.0, 0.0), P::new(0.0, 10.0), P::new(20.0, 10.0), P::new(20.0, -90.0)];
        g.merken(7, &alt);
        assert_eq!(quadrant(alt[0], alt[3]), (1, -1));
        // anderer Quadrant: nichts gemerkt
        assert!(g.form(7, P::new(30.0, 0.0), P::new(20.0, -90.0), &id).is_none());
        // derselbe Quadrant, gleiche Lage: dieselbe Form
        assert_eq!(g.form(7, P::new(0.0, 0.0), P::new(20.0, -90.0), &id).unwrap(), alt.to_vec());
        // Anfang 5 nach unten: Form bleibt, die Höhe verteilt sich auf die senkrechten Stücke (Profil-Verfahren)
        let f = g.form(7, P::new(0.0, 5.0), P::new(20.0, -90.0), &id).unwrap();
        assert_eq!(f, vec![P::new(0.0, 5.0), P::new(0.0, 12.5), P::new(20.0, 12.5), P::new(20.0, -90.0)]);
        // genaue Lage nur bei denselben Enden
        assert_eq!(g.genau(7, alt[0], alt[3]).unwrap(), alt.to_vec());
        assert!(g.genau(7, P::new(0.0, 5.0), alt[3]).is_none());
    }
}
