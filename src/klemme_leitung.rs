//! Werkzeug „Neue Klemmstelle“ wie im Original (MouseDown 0x409fb1, Werkzeug 9): Klick auf eine Leitung setzt die
//! Klemmstelle IN die Leitung (Routine 0x47bf14 mit Art 5 = Klemmstelle, dieselbe, die Symbole in Leitungen
//! einfügt): die Verbindung wird an der Klemme in zwei Verbindungen geteilt, die erste endet an der Klemme, die
//! zweite beginnt dort; beide übernehmen Leitung und Eigenschaften der alten, ihre Enden verweisen auf die Klemmstelle
//! (VBG Feld 2/3 bzw. 6/7 = OBJ-Kennung der Klemme, Anschluss 0). Abseits einer Leitung entsteht eine freie
//! Klemmstelle. Nummer = nächste freie der gewählten Klemmleiste (-X2 mit 14 Klemmen → „-X2.15“).
//!
//! Lage der Enden: In W2 enden alle Leitungen an Klemmstellen 0,8 mm vor dem Mittelpunkt (am Kreis, siehe auch
//! klemmplan.rs `KLEMME_R`); so teilt auch der Nachbau. Die Klemme liegt auf dem Rasterpunkt unter der Maus, auf die
//! Leitung gelotet (waagerechte Leitung: ihre Höhe, senkrechte: ihre x-Lage).
//!
//! Datei: Der erste Teil behält Objekt und Originalblock der alten Leitung (nur Punkte und das Ende an der Klemme
//! ändern sich), der zweite bekommt eine Kopie des Originalblocks mit neuer Kennung – so bleiben Leitungsart,
//! Querschnitt, Stift und der Verweis des fernen Endes (z. B. auf einen Symbolanschluss) erhalten.

use crate::app::App;
use crate::canvas;
use crate::model::*;
use std::cell::Cell;

/// Abstand der Leitungsenden vom Mittelpunkt der Klemmstelle (mm).
pub const KLEMME_R: f32 = 0.8;

/// Teilung einer Leitung an der Stelle `q` auf Abschnitt `i`: (erster Teil, zweiter Teil).
pub fn teilen(pts: &[P], i: usize, q: P) -> (Vec<P>, Vec<P>) {
    let (a, b) = (pts[i], pts[i + 1]);
    let len = a.dist(b).max(1e-6);
    let u = P::new((b.x - a.x) / len, (b.y - a.y) / len);
    let r = KLEMME_R.min(q.dist(a)).min(q.dist(b));
    let mut p1: Vec<P> = pts[..=i].to_vec();
    p1.push(P::new(q.x - u.x * r, q.y - u.y * r));
    let mut p2 = vec![P::new(q.x + u.x * r, q.y + u.y * r)];
    p2.extend_from_slice(&pts[i + 1..]);
    p1.dedup_by(|x, y| x.dist(*y) < 0.01);
    p2.dedup_by(|x, y| x.dist(*y) < 0.01);
    (p1, p2)
}

/// Lage der Klemme auf Abschnitt `a`–`b` für die Maus `m` (`raster` = Rasterpunkt der Maus): auf den Abschnitt
/// gelotet, mindestens `KLEMME_R` von seinen Enden entfernt.
pub fn lage(a: P, b: P, m: P, raster: P) -> P {
    let len = a.dist(b);
    if len < 1e-4 {
        return a;
    }
    let u = P::new((b.x - a.x) / len, (b.y - a.y) / len);
    // waagerecht/senkrecht: Rasterpunkt auf die Leitung, sonst Lot der Maus
    let p = if (a.y - b.y).abs() < 1e-3 {
        P::new(raster.x, a.y)
    } else if (a.x - b.x).abs() < 1e-3 {
        P::new(a.x, raster.y)
    } else {
        m
    };
    let t = ((p.x - a.x) * u.x + (p.y - a.y) * u.y).clamp(KLEMME_R.min(len / 2.0), (len - KLEMME_R).max(len / 2.0));
    P::new(a.x + u.x * t, a.y + u.y * t)
}

thread_local! {
    static ZAEHLER: Cell<u32> = const { Cell::new(0) };
}

/// Neue VBG-Kennung (Delphi-Zeitstempel wie die Kennungen des Originals, je Aufruf eine Millisekunde weiter).
fn neue_kennung() -> u64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let jetzt = (chrono::Local::now().naive_local() - base).num_milliseconds() as f64 / 86_400_000.0;
    let n = ZAEHLER.with(|z| {
        z.set(z.get() + 1);
        z.get()
    });
    (jetzt + (1000.0 + n as f64) / 86_400_000.0).to_bits()
}

impl App {
    /// Nummer der nächsten Klemmstelle der Leiste `leiste` (aktuelle Anlage) wie im Original: der Zähler der
    /// Klemmleiste (KLM Feld 18, „nächste automatische Nummer“) – in W2 steht er bei -X2 (Unterverteilung 1) auf 15,
    /// obwohl dort schon eine -X2.15 liegt; das Original vergibt dann „-X2.15“. In dieser Sitzung neu gesetzte
    /// Klemmstellen zählen weiter. Leisten ohne Originalblock: höchste Nummer + 1.
    pub fn klemmen_zaehler(&self, leiste: &str) -> u32 {
        let Some(a) = self.cur().and_then(|r| self.project.anlagen.get(r.anlage)) else { return 1 };
        let id = a.klemmen.iter().find(|k| k.name == leiste).map(|k| k.id);
        let zaehler = a.raw.as_deref().and_then(|(b, _)| b.kids_of("KLM").find(|k| Some(k.i32(13)) == id)).map(|k| k.i32(18).max(1) as u32);
        let Some(z) = zaehler else { return self.next_terminal(leiste) };
        let neu = a.sheets.iter().flat_map(|s| &s.objects).filter(|o| o.raw.is_none()).filter_map(|o| match &o.kind {
            Kind::Terminal { leiste: l, nr, .. } if l == leiste => Some(*nr + 1),
            _ => None,
        });
        neu.fold(z, u32::max)
    }

    /// Werkzeug Klemmstelle: Klick bei `m` (mm), Fangweite `tol`. Liefert die Id der neuen Klemmstelle.
    pub fn klemmstelle_setzen(&mut self, m: P, tol: f32) -> u64 {
        let ks = self.klemmleisten();
        let leiste = ks.get(self.klemme).cloned().unwrap_or_else(|| crate::klemmen::vorgabe_leiste(1).name);
        let nr = self.klemmen_zaehler(&leiste);
        // Leitung unter der Maus (oberste) und ihr nächster Abschnitt
        let treffer = self.sheet().and_then(|s| {
            s.objects.iter().rev().filter(|o| matches!(o.kind, Kind::Wire { .. }) && canvas::hit(o, m, tol, &self.lib)).find_map(|o| {
                let Kind::Wire { pts, .. } = &o.kind else { return None };
                let i = (0..pts.len().saturating_sub(1)).min_by(|x, y| seg_dist(m, pts[*x], pts[*x + 1]).total_cmp(&seg_dist(m, pts[*y], pts[*y + 1])))?;
                Some((o.id, i, pts[i], pts[i + 1]))
            })
        });
        let im_editor = self.sym_edit.is_some();
        let pos = match treffer {
            Some((_, _, a, b)) if !im_editor => lage(a, b, m, self.snap(m)),
            _ => self.snap(m),
        };
        // Bezeichnung wie beim Original links über der Klemme (Titelposition −1/−2 mm, rechtsbündig)
        let text_off = (!im_editor).then_some(P::new(-1.0, -2.0));
        let id = self.add_obj(Kind::Terminal { pos, leiste, nr, level: 0, text_off, tpl: String::new(), opt: Default::default() });
        if let Some((wid, i, _, _)) = treffer.filter(|_| !im_editor) {
            self.leitung_teilen(wid, i, pos);
        }
        id
    }

    /// Leitung `wid` auf Abschnitt `i` an der Klemme bei `q` teilen (ohne eigenen Rückgängig-Schritt).
    pub fn leitung_teilen(&mut self, wid: u64, i: usize, q: P) {
        let neu_id = self.project.alloc_id();
        self.with_sheet(|s, _| {
            let Some(o) = s.objects.iter_mut().find(|o| o.id == wid) else { return };
            let Kind::Wire { pts, leitung, bez, opts } = &mut o.kind else { return };
            if i + 1 >= pts.len() {
                return;
            }
            let (p1, p2) = teilen(pts, i, q);
            // Bezeichnungen zum näheren Teil
            let naehe = |p: P, z: &[P]| z.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f32::MAX, f32::min);
            let (b1, b2): (Vec<_>, Vec<_>) = bez.iter().cloned().partition(|(p, _)| naehe(*p, &p1) <= naehe(*p, &p2));
            let zweite = Kind::Wire { pts: p2, leitung: *leitung, bez: b2, opts: *opts };
            *pts = p1;
            *bez = b1;
            let mut raw = o.raw.clone();
            if let Some(r) = raw.as_mut() {
                // Kopie des Originalblocks mit eigener Kennung (VBG Byte 40); `kind` der alten Leitung, damit der
                // Schreiber Punkte, Enden und Bezeichnungen nachträgt
                if r.block.data.len() >= 48 {
                    r.block.data[40..48].copy_from_slice(&neue_kennung().to_le_bytes());
                }
            }
            let neu = Obj { id: neu_id, kind: zweite, pen: o.pen, fixed: o.fixed, group: o.group, raw };
            s.objects.push(neu);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teilen_waagerecht() {
        let pts = vec![P::new(10.0, 50.0), P::new(60.0, 50.0), P::new(60.0, 80.0)];
        let q = lage(pts[0], pts[1], P::new(30.4, 50.6), P::new(30.0, 50.0));
        assert_eq!(q, P::new(30.0, 50.0));
        let (a, b) = teilen(&pts, 0, q);
        assert_eq!(a, vec![P::new(10.0, 50.0), P::new(29.2, 50.0)]);
        assert_eq!(b, vec![P::new(30.8, 50.0), P::new(60.0, 50.0), P::new(60.0, 80.0)]);
        // senkrechter Abschnitt
        let q = lage(pts[1], pts[2], P::new(60.3, 70.2), P::new(60.0, 70.0));
        assert_eq!(q, P::new(60.0, 70.0));
        let (a, b) = teilen(&pts, 1, q);
        assert_eq!(a.last().copied(), Some(P::new(60.0, 69.2)));
        assert_eq!(b, vec![P::new(60.0, 70.8), P::new(60.0, 80.0)]);
        // nahe am Ende: mindestens 0,8 mm Abstand
        let q = lage(pts[0], pts[1], P::new(10.1, 50.0), P::new(10.0, 50.0));
        assert_eq!(q, P::new(10.8, 50.0));
    }
}
