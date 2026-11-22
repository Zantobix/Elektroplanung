//! Stetige Leitungsführung beim Ziehen von einem Anschluss (todo 1 „Führung folgt von Anfang an der Maus“).
//!
//! Vorher sprang die Vorschau beim Aufziehen zwischen Formen: Nahe am Anschluss wechselte der Knick mit jeder
//! Mausbewegung über die Diagonale, beim Zurückkehren zum Anschluss wurde die Richtung verworfen, und sobald der
//! einfache Knick in das eigene Symbol lief, kam schlagartig die Z-Führung über die Mitte.
//!
//! Hier gilt:
//! - **Hysterese** für die Richtung des ersten Teilstücks (`hysterese`): einmal bestimmt, wechselt sie nur noch in
//!   der Nähe des Anfangs (`ZONE` × Schwelle) und nur, wenn die andere Richtung deutlich überwiegt
//!   (`VERHAELTNIS`). Weiter draußen bleibt sie (die erste Zugrichtung bestimmt den Knick wie bisher).
//! - **Stetige Form** (`zugweg`): Verläuft das erste Teilstück in Anschlussrichtung, geht es mindestens einen Stich
//!   nach außen und biegt dann zum Ziel ab (J-Form). Liegt das Ziel jenseits des Stichs, ist das der einfache Knick;
//!   liegt es dahinter, wächst das Rückstück stetig aus dem Stich heraus. Damit ändert sich die Vorschau bei
//!   kleinen Mausbewegungen nur wenig.
//! - Die Regeln aus `leitung_route.rs` bleiben: läuft der Weg trotzdem an einem Ende in das Symbol (Ziel genau
//!   hinter dem Anschluss, Zielanschluss von der falschen Seite), wird er mit Stichen und Z über die Mitte gelegt.
//!
//! Vorschau und Ergebnis beim Loslassen kommen aus derselben Funktion; der Klickbetrieb (Regel des Originals) und
//! der Beginn auf einer Sammelschiene sind unverändert.

use crate::leitung_fuehrung::{bereinigen, knick, vorherrschend, Fuehrung, Richtung};
use crate::leitung_route::{laeuft_hinein, mit_stichen};
use crate::model::P;

/// Bereich um den Anfang (Vielfache der Schwelle), in dem die Richtung noch wechseln kann.
pub const ZONE: f32 = 3.0;
/// So viel stärker muss die andere Richtung überwiegen, damit die Richtung wechselt.
pub const VERHAELTNIS: f32 = 2.0;
/// Erste Entscheidung schon nach einem Drittel der Schwelle (2 Bildpunkte), damit die Vorschau nicht vorher mit
/// jeder Bewegung über die Diagonale den Knick wechselt.
pub const ENTSCHEIDUNG: f32 = 1.0 / 3.0;

fn andere(r: Richtung) -> Richtung {
    match r {
        Richtung::Waagerecht => Richtung::Senkrecht,
        Richtung::Senkrecht => Richtung::Waagerecht,
    }
}

/// Richtung des ersten Teilstücks nach der Mausbewegung `d` (vom Anfang aus, mm) mit Hysterese.
/// Ohne bisherige Richtung entscheidet die erste Bewegung über `ENTSCHEIDUNG` × `schwelle` hinaus (vorherrschende
/// Richtung); danach wechselt sie nur nahe am Anfang und nur bei deutlich überwiegender anderer Richtung.
pub fn hysterese(bisher: Option<Richtung>, d: P, schwelle: f32) -> Option<Richtung> {
    let (ax, ay) = (d.x.abs(), d.y.abs());
    let Some(r) = bisher else {
        return (ax.max(ay) > ENTSCHEIDUNG * schwelle).then(|| vorherrschend(d));
    };
    if ax.max(ay) > ZONE * schwelle {
        return Some(r);
    }
    let (eigen, fremd) = match r {
        Richtung::Waagerecht => (ax, ay),
        Richtung::Senkrecht => (ay, ax),
    };
    if fremd > schwelle && fremd > VERHAELTNIS * eigen {
        Some(andere(r))
    } else {
        Some(r)
    }
}

fn skalar(a: P, b: P) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Weg beim Ziehen von `a` (Richtung nach außen `da`) nach `b` (Zielanschluss mit `db`), erstes Teilstück in
/// Richtung `erst`, Stichlänge `s`.
pub fn zugweg(a: P, da: Option<P>, b: P, db: Option<P>, erst: Richtung, s: f32) -> Vec<P> {
    let entlang = |d: P| match erst {
        Richtung::Waagerecht => d.x.abs() > 0.5,
        Richtung::Senkrecht => d.y.abs() > 0.5,
    };
    let w = match da {
        Some(d) if entlang(d) => {
            // mindestens einen Stich nach außen, dann quer bis in die Flucht des Ziels und zum Ziel
            let t = skalar(b.sub(a), d).max(s);
            let a1 = P::new(a.x + d.x * t, a.y + d.y * t);
            let c = P::new(b.x + d.x * skalar(a1.sub(b), d), b.y + d.y * skalar(a1.sub(b), d));
            bereinigen(vec![a, a1, c, b])
        }
        _ => bereinigen(knick(a, b, Some(erst))),
    };
    if laeuft_hinein(&w, da, db) {
        return mit_stichen(a, da, b, db, s);
    }
    w
}

impl Fuehrung {
    /// Weg beim Ziehen bis `ende` (Vorschau und Ergebnis); `da`/`db` = Anschlussrichtungen, `s` = Stichlänge.
    pub fn zugweg(&self, ende: P, da: Option<P>, db: Option<P>, s: f32) -> Vec<P> {
        let a = self.start();
        let erst = self.richtung.unwrap_or_else(|| vorherrschend(ende.sub(a)));
        zugweg(a, da, ende, db, erst, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }
    const UNTEN: Option<P> = Some(P { x: 0.0, y: 1.0 });
    const OBEN: Option<P> = Some(P { x: 0.0, y: -1.0 });
    /// Schwelle (6 px bei Zoom 2 px/mm) und Stich (halbes Raster 2,5 mm)
    const SCHWELLE: f32 = 3.0;
    const STICH: f32 = 2.5;

    /// Größter Abstand eines Punktes von `u` zum Linienzug `v` (in beide Richtungen): wie weit sich die Vorschau
    /// zwischen zwei Bildern sichtbar bewegt.
    fn abweichung(u: &[P], v: &[P]) -> f32 {
        fn zum_zug(q: P, v: &[P]) -> f32 {
            if v.len() == 1 {
                return q.dist(v[0]);
            }
            v.windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let d = b.sub(a);
                    let l2 = skalar(d, d);
                    let t = if l2 < 1e-9 { 0.0 } else { (skalar(q.sub(a), d) / l2).clamp(0.0, 1.0) };
                    q.dist(P::new(a.x + d.x * t, a.y + d.y * t))
                })
                .fold(f32::MAX, f32::min)
        }
        fn proben(u: &[P]) -> Vec<P> {
            let mut out = vec![u[0]];
            for w in u.windows(2) {
                for i in 1..=20 {
                    let t = i as f32 / 20.0;
                    out.push(P::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
                }
            }
            out
        }
        let a = proben(u).into_iter().map(|q| zum_zug(q, v)).fold(0.0, f32::max);
        let b = proben(v).into_iter().map(|q| zum_zug(q, u)).fold(0.0, f32::max);
        a.max(b)
    }

    /// Mausweg in Schritten von `schritt` mm entlang der Eckpunkte `ecken` durchspielen; liefert die Vorschauen.
    fn durchspielen(f: &mut Fuehrung, ecken: &[P], schritt: f32, da: Option<P>) -> Vec<Vec<P>> {
        let mut out = Vec::new();
        for w in ecken.windows(2) {
            let n = (w[0].dist(w[1]) / schritt).ceil().max(1.0) as usize;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let m = P::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t);
                f.bewegen(m, SCHWELLE);
                out.push(f.zugweg(m, da, None, STICH));
            }
        }
        out
    }

    /// Anzahl der Wechsel der Form (Zahl der Teilstücke bzw. Richtung des ersten Teilstücks) in einer Folge.
    fn formwechsel(wege: &[Vec<P>]) -> usize {
        let form = |w: &Vec<P>| (w.len(), w.len() >= 2 && (w[0].x - w[1].x).abs() < 0.01);
        wege.windows(2).filter(|w| form(&w[0]) != form(&w[1])).count()
    }

    #[test]
    fn hysterese_haelt_die_richtung_an_der_diagonale() {
        // nach rechts entschieden, dann zittert die Maus um die Diagonale: bleibt waagerecht
        let mut r = hysterese(None, p(4.0, 0.5), SCHWELLE);
        assert_eq!(r, Some(Richtung::Waagerecht));
        for (i, d) in [p(5.0, 5.2), p(5.2, 5.0), p(4.8, 5.3), p(5.3, 4.9)].into_iter().enumerate() {
            r = hysterese(r, d, SCHWELLE);
            assert_eq!(r, Some(Richtung::Waagerecht), "Schritt {i}");
        }
        // deutlich nach unten nahe am Anfang: wechselt
        r = hysterese(r, p(1.0, 6.0), SCHWELLE);
        assert_eq!(r, Some(Richtung::Senkrecht));
        // weit draußen wechselt nichts mehr, auch wenn die andere Richtung stark überwiegt
        assert_eq!(hysterese(r, p(40.0, 1.0), SCHWELLE), Some(Richtung::Senkrecht));
        // kleine Bewegungen entscheiden nicht
        assert_eq!(hysterese(None, p(0.5, -0.8), SCHWELLE), None);
    }

    #[test]
    fn zittern_am_anfang_springt_nicht_hin_und_her() {
        // Maus kreist dicht um die Diagonale rechts unten vom Anschluss weg: die Form wechselt höchstens einmal
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let mut ecken = vec![p(0.0, 0.0)];
        for i in 1..30 {
            let r = i as f32 * 0.6;
            let z = if i % 2 == 0 { 0.4 } else { -0.4 };
            ecken.push(p(r + z, r - z));
        }
        let wege = durchspielen(&mut f, &ecken, 0.2, None);
        assert!(formwechsel(&wege) <= 1, "Formwechsel: {}", formwechsel(&wege));
    }

    #[test]
    fn vorschau_folgt_stetig_ueber_die_anschlusshoehe() {
        // unterer Anschluss (Richtung unten): Maus erst nach unten, dann nach rechts und über die Höhe des Anschlusses
        // nach oben. Früher sprang die Vorschau beim Überqueren vom Knick auf die Z-Führung über die Mitte.
        let mut f = Fuehrung::neu(p(40.0, 100.0));
        let ecken = [p(40.0, 100.0), p(40.0, 112.0), p(70.0, 112.0), p(70.0, 80.0)];
        let wege = durchspielen(&mut f, &ecken, 0.25, UNTEN);
        assert_eq!(f.richtung, Some(Richtung::Senkrecht));
        for (i, w) in wege.windows(2).enumerate() {
            let d = abweichung(&w[0], &w[1]);
            assert!(d <= 0.25 + 1e-3, "Bild {i}: Vorschau springt um {d} mm: {:?} → {:?}", w[0], w[1]);
        }
        // nie in das eigene Symbol: das erste Teilstück geht immer nach unten
        for w in &wege {
            assert!(w[1].y > w[0].y, "läuft ins Symbol: {w:?}");
        }
        // Endform: Stich nach unten, quer, von unten zum Ziel
        assert_eq!(wege.last().unwrap(), &vec![p(40.0, 100.0), p(40.0, 102.5), p(70.0, 102.5), p(70.0, 80.0)]);
    }

    #[test]
    fn umkehr_am_anschluss_ohne_sprung() {
        // nach rechts ziehen, zurück zum Anschluss und wieder nach rechts: die Richtung bleibt, die Vorschau springt nicht
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let ecken = [p(0.0, 0.0), p(20.0, 3.0), p(0.5, 0.4), p(20.0, 6.0)];
        let wege = durchspielen(&mut f, &ecken, 0.2, None);
        assert_eq!(f.richtung, Some(Richtung::Waagerecht));
        assert_eq!(formwechsel(&wege), 0);
        for w in wege.windows(2) {
            assert!(abweichung(&w[0], &w[1]) <= 0.2 + 1e-3);
        }
    }

    #[test]
    fn loslassen_ergibt_die_letzte_vorschau() {
        let mut f = Fuehrung::neu(p(40.0, 100.0));
        let ecken = [p(40.0, 100.0), p(46.0, 101.0), p(90.0, 60.0)];
        let wege = durchspielen(&mut f, &ecken, 0.5, UNTEN);
        // Loslassen an derselben Stelle: bewegen ist dort ohne Wirkung, der Weg gleich der letzten Vorschau
        let mut g = f.clone();
        g.bewegen(p(90.0, 60.0), SCHWELLE);
        assert_eq!(g.zugweg(p(90.0, 60.0), UNTEN, None, STICH), *wege.last().unwrap());
        // erstes Teilstück waagerecht (quer zur Anschlussrichtung ist erlaubt)
        assert_eq!(wege.last().unwrap(), &vec![p(40.0, 100.0), p(90.0, 100.0), p(90.0, 60.0)]);
    }

    #[test]
    fn regeln_aus_leitung_route_bleiben() {
        // M1: unterer Anschluss zu höherem oberen Zielanschluss, senkrecht aufgezogen: Stich, Z über die Mitte, von oben
        let w = zugweg(p(40.0, 100.0), UNTEN, p(100.0, 90.0), OBEN, Richtung::Senkrecht, 5.0);
        assert_eq!(w, vec![p(40.0, 100.0), p(40.0, 105.0), p(70.0, 105.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)]);
        assert!(!laeuft_hinein(&w, UNTEN, OBEN));
        // Ziel genau über dem unteren Anschluss: seitlich ausweichen statt durch das Symbol
        let w = zugweg(p(40.0, 100.0), UNTEN, p(40.0, 80.0), None, Richtung::Senkrecht, 5.0);
        assert!(w.iter().any(|q| q.x > 40.0) && !laeuft_hinein(&w, UNTEN, None));
        // tiefer liegendes Ziel unter einem unteren Anschluss: einfacher Knick
        assert_eq!(zugweg(p(40.0, 100.0), UNTEN, p(60.0, 130.0), OBEN, Richtung::Senkrecht, 5.0), vec![p(40.0, 100.0), p(40.0, 130.0), p(60.0, 130.0)]);
        // ohne Anschlussrichtung: Knick wie bisher
        assert_eq!(zugweg(p(0.0, 0.0), None, p(10.0, 4.0), None, Richtung::Senkrecht, 5.0), vec![p(0.0, 0.0), p(0.0, 4.0), p(10.0, 4.0)]);
    }
}
