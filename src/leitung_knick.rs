//! Knickrichtung beim Ziehen einer Leitung: „Form halten bis Länge 0“ (todo 1 „Führung folgt von Anfang an der Maus“).
//!
//! Herkunft: Idee nach KiCad Eeschema, `SCH_LINE_WIRE_BUS_TOOL::computeBreakPoint`
//! (eeschema/tools/sch_line_wire_bus_tool.cpp, GPL-3.0-or-later) und dessen Taste „posture“
//! (`switchSegmentPosture`). Hier in Rust neu geschrieben, kein Code übernommen; Recherche in den privaten Notizen
//! (Vergleich QElectroTech/KiCad, Abschnitt a).
//!
//! Regel:
//! - Das erste Teilstück (vom Anschluss aus) bekommt seine Richtung aus der ersten deutlichen Bewegung des
//!   Leitungsendes (vorherrschende Richtung; bei Gleichstand die Anschlussrichtung, sonst waagerecht wie im
//!   Original).
//! - Danach **behält** es diese Richtung, solange es eine Länge hat. Erst wenn es Länge 0 erreicht (das Ende liegt in
//!   der Flucht des Anfangs quer dazu) oder über den Anfang hinweg auf die andere Seite springt (Vorzeichenwechsel
//!   zwischen zwei Bildern, schnelle Maus), wird neu nach der vorherrschenden Richtung entschieden.
//! - Damit gibt es keine Schwellwerte und keine Hysterese mehr: kleine Mausbewegungen ändern die Führung nie, nur das
//!   Zurückführen auf die Höhe (bzw. Flucht) des Anfangs kippt den Knick.
//! - Umschalttaste (Leertaste bzw. „/“ wie KiCad, bewusste Erweiterung): kippt die Knickrichtung sofort und bleibt
//!   für diese Leitung gekippt – auch jede spätere Neuentscheidung fällt dann umgekehrt aus.
//!
//! Gerechnet wird mit dem Leitungsende, wie es gezeichnet wird (auf das Raster bzw. einen Anschluss gefangen), so
//! dass „Länge 0“ genau dann gilt, wenn die Vorschau gerade ist.

use crate::leitung_fuehrung::{vorherrschend, Richtung};
use crate::model::P;

/// Unterhalb dieser Länge (mm) gilt ein Teilstück als Länge 0.
pub const NULL: f32 = 0.01;

/// Die andere Richtung.
pub fn andere(r: Richtung) -> Richtung {
    match r {
        Richtung::Waagerecht => Richtung::Senkrecht,
        Richtung::Senkrecht => Richtung::Waagerecht,
    }
}

/// Anteil von `d` in Richtung `r` (vorzeichenbehaftet): Länge des ersten Teilstücks beim einfachen Knick.
pub fn laenge(d: P, r: Richtung) -> f32 {
    match r {
        Richtung::Waagerecht => d.x,
        Richtung::Senkrecht => d.y,
    }
}

/// Neue Entscheidung für den Weg `d` (Ende − Anfang): vorherrschende Richtung, bei Gleichstand die Achse der
/// Anschlussrichtung `da`; mit `gekippt` umgekehrt.
pub fn entscheiden(d: P, da: Option<P>, gekippt: bool) -> Richtung {
    let r = if (d.x.abs() - d.y.abs()).abs() < NULL {
        match da {
            Some(a) if a.y.abs() > a.x.abs() => Richtung::Senkrecht,
            _ => Richtung::Waagerecht,
        }
    } else {
        vorherrschend(d)
    };
    if gekippt {
        andere(r)
    } else {
        r
    }
}

/// Richtung des ersten Teilstücks nach dem Schritt von `vorher` (bisheriges Ende − Anfang, falls schon eines da war)
/// nach `d` (jetziges Ende − Anfang). `bisher` = Richtung bis jetzt, `min` = Weg, ab dem überhaupt entschieden wird
/// (gegen Zittern beim Aufsetzen ohne Raster).
pub fn naechste(bisher: Option<Richtung>, vorher: Option<P>, d: P, da: Option<P>, gekippt: bool, min: f32) -> Option<Richtung> {
    // noch zu nah am Anfang: nichts entscheiden, die bisherige Richtung bleibt
    if d.x.abs().max(d.y.abs()) <= min {
        return bisher;
    }
    let Some(r) = bisher else {
        return Some(entscheiden(d, da, gekippt));
    };
    let jetzt = laenge(d, r);
    let frueher = vorher.map(|v| laenge(v, r)).unwrap_or(jetzt);
    // Länge 0 erreicht oder über den Anfang hinweg: neu entscheiden, sonst Form halten
    if jetzt.abs() < NULL || frueher * jetzt < 0.0 {
        Some(entscheiden(d, da, gekippt))
    } else {
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leitung_fuehrung::Fuehrung;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }
    const UNTEN: Option<P> = Some(P { x: 0.0, y: 1.0 });
    const RASTER: f32 = 5.0;
    const MIN: f32 = 1.0;
    const STICH: f32 = 2.5;

    /// Mausbahn (mm, ungefangen) in kleinen Schritten durchspielen; das Ende wird wie in der Oberfläche auf das
    /// Raster gefangen. Liefert je Bild die Richtung des ersten Teilstücks und den Weg.
    fn bahn(f: &mut Fuehrung, ecken: &[P], da: Option<P>) -> Vec<(Option<Richtung>, Vec<P>)> {
        let fang = |q: P| P::new((q.x / RASTER).round() * RASTER, (q.y / RASTER).round() * RASTER);
        let mut out = Vec::new();
        for w in ecken.windows(2) {
            let n = (w[0].dist(w[1]) / 0.3).ceil().max(1.0) as usize;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let m = fang(P::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
                f.bewegen_mit(m, da, MIN);
                out.push((f.richtung, f.zugweg(m, da, None, STICH)));
            }
        }
        out
    }

    fn wechsel(v: &[(Option<Richtung>, Vec<P>)]) -> usize {
        v.windows(2).filter(|w| w[0].0.is_some() && w[0].0 != w[1].0).count()
    }

    #[test]
    fn form_halten_bis_laenge_null() {
        // waagerecht entschieden; solange das Ende rechts vom Anfang bleibt, bleibt es waagerecht – auch weit unten
        let r = naechste(None, None, p(5.0, 0.0), None, false, MIN);
        assert_eq!(r, Some(Richtung::Waagerecht));
        let r = naechste(r, Some(p(5.0, 0.0)), p(5.0, 40.0), None, false, MIN);
        assert_eq!(r, Some(Richtung::Waagerecht));
        // zurück auf die Flucht des Anfangs (Länge 0): neu entschieden → senkrecht
        let r = naechste(r, Some(p(5.0, 40.0)), p(0.0, 40.0), None, false, MIN);
        assert_eq!(r, Some(Richtung::Senkrecht));
        // und wieder nach rechts: bleibt senkrecht (erstes Teilstück hat Länge 40)
        assert_eq!(naechste(r, Some(p(0.0, 40.0)), p(60.0, 40.0), None, false, MIN), Some(Richtung::Senkrecht));
    }

    #[test]
    fn sprung_ueber_den_anfang_zaehlt_wie_laenge_null() {
        // schnelle Maus: von unterhalb auf oberhalb des Anfangs ohne Bild genau auf der Höhe
        let r = naechste(Some(Richtung::Senkrecht), Some(p(30.0, 5.0)), p(35.0, -5.0), None, false, MIN);
        assert_eq!(r, Some(Richtung::Waagerecht));
        // in dieselbe Richtung weiter: bleibt
        assert_eq!(naechste(Some(Richtung::Senkrecht), Some(p(30.0, 5.0)), p(35.0, 10.0), None, false, MIN), Some(Richtung::Senkrecht));
    }

    #[test]
    fn gleichstand_nimmt_die_anschlussrichtung() {
        assert_eq!(entscheiden(p(5.0, 5.0), UNTEN, false), Richtung::Senkrecht);
        assert_eq!(entscheiden(p(5.0, 5.0), Some(p(1.0, 0.0)), false), Richtung::Waagerecht);
        // ohne Anschlussrichtung: waagerecht wie die Regel des Originals
        assert_eq!(entscheiden(p(5.0, -5.0), None, false), Richtung::Waagerecht);
        assert_eq!(entscheiden(p(5.0, 5.0), UNTEN, true), Richtung::Waagerecht);
    }

    #[test]
    fn zittern_um_die_diagonale_kippt_nie() {
        // Maus schlängelt sich um die Diagonale nach rechts unten (früher Hysterese-Fall): eine Entscheidung, kein Wechsel
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let mut ecken = vec![p(0.0, 0.0)];
        for i in 4..40 {
            let r = i as f32 * 1.5;
            let z = if i % 2 == 0 { 2.0 } else { -2.0 };
            ecken.push(p(r + z, r - z));
        }
        let v = bahn(&mut f, &ecken, None);
        assert_eq!(wechsel(&v), 0, "{:?}", v.iter().map(|x| x.0).collect::<Vec<_>>());
    }

    #[test]
    fn kreis_um_den_anschluss_kippt_nur_auf_den_achsen() {
        // Maus geht nach rechts, dann im Bogen nach unten und weiter nach links: der Knick kippt erst, wenn das Ende
        // senkrecht unter dem Anfang steht (waagerechte Länge 0)
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let v = bahn(&mut f, &[p(0.0, 0.0), p(30.0, 0.0), p(30.0, 30.0), p(3.0, 30.0)], None);
        assert!(v.iter().filter(|x| x.0.is_some()).all(|x| x.0 == Some(Richtung::Waagerecht)), "vor der Achse gekippt");
        assert_eq!(v.last().unwrap().1, vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 30.0)]);
        let w = bahn(&mut f, &[p(3.0, 30.0), p(-20.0, 30.0)], None);
        // auf der Achse neu entschieden (senkrecht) und danach gehalten
        assert_eq!(wechsel(&w), 1);
        assert_eq!(w.last().unwrap().0, Some(Richtung::Senkrecht));
        assert_eq!(w.last().unwrap().1, vec![p(0.0, 0.0), p(0.0, 30.0), p(-20.0, 30.0)]);
    }

    #[test]
    fn unterer_anschluss_erst_nach_unten() {
        // Anschluss nach unten, Maus nach unten, dann nach rechts: senkrecht bleibt, Knick unten
        let mut f = Fuehrung::neu(p(40.0, 100.0));
        let v = bahn(&mut f, &[p(40.0, 100.0), p(40.0, 120.0), p(80.0, 125.0)], UNTEN);
        assert_eq!(wechsel(&v), 0);
        assert_eq!(v.last().unwrap().1, vec![p(40.0, 100.0), p(40.0, 125.0), p(80.0, 125.0)]);
        // Maus über die Höhe des Anschlusses nach oben: dort kippt der Knick (Stich nach unten, quer, dann hoch) –
        // nicht in das Symbol, kein Z über die Mitte
        let v = bahn(&mut f, &[p(80.0, 125.0), p(80.0, 70.0)], UNTEN);
        assert_eq!(wechsel(&v), 1);
        assert_eq!(v.last().unwrap().1, vec![p(40.0, 100.0), p(40.0, 102.5), p(80.0, 102.5), p(80.0, 70.0)]);
    }

    #[test]
    fn oberer_anschluss_erst_quer_dann_runter_mit_stich() {
        // Maus-Befund M27: oberer Anschluss (zeigt nach oben), Maus erst 4 Raster nach rechts, dann weit nach unten.
        // Knick waagerecht (erste Bewegung), aber die Leitung verlässt den Anschluss zuerst mit dem Stich nach oben.
        let oben = Some(P { x: 0.0, y: -1.0 });
        let mut f = Fuehrung::neu(p(40.0, 100.0));
        let v = bahn(&mut f, &[p(40.0, 100.0), p(60.0, 100.0), p(60.0, 140.0)], oben);
        assert_eq!(f.richtung, Some(Richtung::Waagerecht));
        for (_, w) in v.iter().filter(|x| x.1.len() >= 2 && x.1[0].dist(*x.1.last().unwrap()) > 0.01) {
            assert!(w[1].y < w[0].y, "nicht zuerst nach oben: {w:?}");
        }
        assert_eq!(v.last().unwrap().1, vec![p(40.0, 100.0), p(40.0, 97.5), p(60.0, 97.5), p(60.0, 140.0)]);
        // Umschalten: senkrecht zuerst → Stich nach oben, dann seitlich und von oben herunter (J-Form wie bisher)
        assert!(f.umschalten());
        let w = f.zugweg(p(60.0, 140.0), oben, None, STICH);
        assert!(w[1].y < w[0].y && !crate::leitung_route::laeuft_hinein(&w, oben, None), "{w:?}");
    }

    #[test]
    fn umschalttaste_kippt_und_bleibt() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        let v = bahn(&mut f, &[p(0.0, 0.0), p(30.0, 2.0), p(30.0, 20.0)], None);
        assert_eq!(v.last().unwrap().1, vec![p(0.0, 0.0), p(30.0, 0.0), p(30.0, 20.0)]);
        assert!(f.umschalten());
        assert_eq!(f.richtung, Some(Richtung::Senkrecht));
        // ohne Bewegung bleibt die gekippte Form (Loslassen an derselben Stelle)
        f.bewegen_mit(p(30.0, 20.0), None, MIN);
        assert_eq!(f.zugweg(p(30.0, 20.0), None, None, STICH), vec![p(0.0, 0.0), p(0.0, 20.0), p(30.0, 20.0)]);
        // weiter ziehen hält die gekippte Form, auch über eine Neuentscheidung auf der Achse hinweg
        let v = bahn(&mut f, &[p(30.0, 20.0), p(30.0, 0.0), p(50.0, 0.0), p(50.0, 30.0)], None);
        assert_eq!(v.last().unwrap().1, vec![p(0.0, 0.0), p(0.0, 30.0), p(50.0, 30.0)]);
        // zweimal umschalten = wie vorher
        assert!(f.umschalten());
        assert_eq!(f.zugweg(p(50.0, 30.0), None, None, STICH), vec![p(0.0, 0.0), p(50.0, 0.0), p(50.0, 30.0)]);
        // vor der ersten Bewegung: die erste Entscheidung fällt gekippt aus
        let mut g = Fuehrung::neu(p(0.0, 0.0));
        assert!(g.umschalten());
        let v = bahn(&mut g, &[p(0.0, 0.0), p(30.0, 1.0), p(30.0, 20.0)], None);
        assert_eq!(v.last().unwrap().1, vec![p(0.0, 0.0), p(0.0, 20.0), p(30.0, 20.0)]);
    }

    #[test]
    fn umschalten_nicht_im_klickbetrieb_und_an_schiene() {
        let mut f = Fuehrung::neu(p(0.0, 0.0));
        f.klickbetrieb = true;
        assert!(!f.umschalten());
        let mut s = Fuehrung::von_schiene(p(10.0, 30.0), (30.0, 0.0, 100.0));
        assert!(!s.umschalten());
        assert_eq!(s.richtung, Some(Richtung::Senkrecht));
    }
}
