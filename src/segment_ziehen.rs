//! Teilstück einer Leitung mit der Maus verschieben, ohne sie zu trennen (Maus-Befund M2, 22.11.2026).
//!
//! Bisher verschob das Ziehen an einer Leitung die ganze Leitung; ihre Enden lösten sich von den Anschlüssen.
//! Das Original (Gegenprobe unter Wine) verschiebt nur das gegriffene Teilstück quer zu seiner Richtung; die
//! Nachbarteilstücke werden länger bzw. kürzer, die Enden bleiben an ihren Anschlüssen. Ist das gegriffene
//! Teilstück das erste oder letzte, entsteht am festen Ende ein neues Querstück.
//!
//! Regeln neu geschrieben nach KiCad Eeschema `SCH_MOVE_TOOL::orthoLineDrag` (GPL; kein Code übernommen): Ein
//! Nachbarstück, das quer zum gegriffenen liegt, wird gestreckt bzw. gestaucht; liegt es nicht so (schräg, in
//! Flucht) oder ist dort das feste Leitungsende, wird ein Knick (Querstück über den alten Eckpunkt) eingefügt.
//! Danach verschmelzen Stücke der Länge 0 und Stücke, die auf dem vorigen zurücklaufen
//! (`leitung_fuehrung::bereinigen`), sodass weder doppelte Punkte noch Rückläufer bleiben. Gerechnet wird immer
//! von der Ausgangslage aus; zurück in die Ausgangslage ergibt also wieder die (bereinigte) Ausgangsleitung.

use crate::app::{App, Drag};
use crate::model::{Kind, P};

const EPS: f32 = 1e-3;

/// Abstand von `p` zur Strecke `a`–`b`.
fn abstand(p: P, a: P, b: P) -> f32 {
    let d = b.sub(a);
    let l2 = d.x * d.x + d.y * d.y;
    if l2 < 1e-9 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * d.x + (p.y - a.y) * d.y) / l2).clamp(0.0, 1.0);
    p.dist(P::new(a.x + d.x * t, a.y + d.y * t))
}

/// Index des Teilstücks von `pts`, das `m` (Toleranz `tol`) trifft; `None` auf den Leitungsenden (dort beginnt
/// das Ziehen eine neue Leitung bzw. verschiebt wie bisher).
pub fn teilstueck_bei(pts: &[P], m: P, tol: f32) -> Option<usize> {
    if pts.len() < 2 || m.dist(pts[0]) <= tol || m.dist(pts[pts.len() - 1]) <= tol {
        return None;
    }
    (0..pts.len() - 1).filter(|&i| abstand(m, pts[i], pts[i + 1]) <= tol).min_by(|&i, &j| abstand(m, pts[i], pts[i + 1]).total_cmp(&abstand(m, pts[j], pts[j + 1])))
}

/// Verschiebung `d` quer zum Teilstück `a`–`b` (waagerecht: nur senkrecht, senkrecht: nur waagerecht, schräg: ganz).
pub fn quer(a: P, b: P, d: P) -> P {
    if (a.y - b.y).abs() < EPS {
        P::new(0.0, d.y)
    } else if (a.x - b.x).abs() < EPS {
        P::new(d.x, 0.0)
    } else {
        d
    }
}

fn achsengerade(a: P, b: P) -> bool {
    (a.x - b.x).abs() < EPS || (a.y - b.y).abs() < EPS
}

/// Punkte nach dem Verschieben des Teilstücks `idx` von `orig` um `d` (schon quer). Anfang und Ende bleiben fest;
/// wird ein Nachbarstück schräg (oder ist das Teilstück das erste/letzte), entsteht ein Querstück über den alten
/// Eckpunkt.
pub fn verschieben(orig: &[P], idx: usize, d: P) -> Vec<P> {
    verschieben_frei(orig, idx, d, [false, false])
}

/// Wie `verschieben`; ein **freies** Leitungsende (`frei`: Anfang, Ende – an nichts angeschlossen) ist kein
/// Festpunkt: gehört es zum gezogenen Teilstück, wandert es mit, statt ein Querstück zu bekommen (Nutzer
/// 23.11.2026: „ein Leitungsende wird immer als dedizierter Endpunkt interpretiert, bitte nicht“).
pub fn verschieben_frei(orig: &[P], idx: usize, d: P, frei: [bool; 2]) -> Vec<P> {
    let n = orig.len();
    if n < 2 || idx + 1 >= n {
        return orig.to_vec();
    }
    let a = P::new(orig[idx].x + d.x, orig[idx].y + d.y);
    let b = P::new(orig[idx + 1].x + d.x, orig[idx + 1].y + d.y);
    let mut out: Vec<P> = orig[..idx].to_vec();
    // vorne: am festen Anfang bzw. bei schrägem Nachbarstück den alten Eckpunkt behalten
    if (idx == 0 && !frei[0]) || (idx > 0 && !achsengerade(orig[idx - 1], a)) {
        out.push(orig[idx]);
    }
    out.push(a);
    out.push(b);
    if (idx + 1 == n - 1 && !frei[1]) || (idx + 1 < n - 1 && !achsengerade(b, orig[idx + 2])) {
        out.push(orig[idx + 1]);
    }
    out.extend_from_slice(&orig[(idx + 2).min(n)..]);
    crate::leitung_fuehrung::bereinigen(out)
}

/// Wie `verschieben_frei` für das erste bzw. letzte Teilstück mit festem Ende: das Teilstück behält am festen Ende
/// einen Stich der Länge `l` in seiner Richtung, erst danach folgt das Querstück (die Leitung verlässt den Anschluss
/// in seiner Richtung). `None`, wenn kein festes Ende betroffen oder das Teilstück zu kurz ist.
pub fn mit_stich(orig: &[P], idx: usize, d: P, frei: [bool; 2], l: f32) -> Option<Vec<P>> {
    let n = orig.len();
    if n < 2 || idx + 1 >= n {
        return None;
    }
    let (a, b) = (orig[idx], orig[idx + 1]);
    let len = a.dist(b);
    if len <= l + EPS || !achsengerade(a, b) {
        return None;
    }
    let u = P::new((b.x - a.x) / len, (b.y - a.y) / len);
    let mut v = orig.to_vec();
    let mut i = idx;
    let mut geaendert = false;
    if idx == 0 && !frei[0] {
        v.insert(1, P::new(a.x + u.x * l, a.y + u.y * l));
        i += 1;
        geaendert = true;
    }
    if idx + 1 == n - 1 && !frei[1] && v[i].dist(b) > l + EPS {
        let k = v.len() - 1;
        v.insert(k, P::new(b.x - u.x * l, b.y - u.y * l));
        geaendert = true;
    }
    if !geaendert {
        return None;
    }
    let f = [frei[0], frei[1]];
    Some(verschieben_frei(&v, i, d, f))
}

/// Fluchten, an die ein waagerechtes (`waagerecht`) bzw. senkrechtes Teilstück beim Ziehen gefangen wird: Höhen bzw.
/// x-Lagen gleichgerichteter Teilstücke anderer Leitungen, Sammelschienen und Anschlüsse (ohne Leitung `ohne`).
pub fn fluchten(sheet: &crate::model::Sheet, lib: &[crate::symbols::SymDef], ohne: u64, waagerecht: bool) -> Vec<f32> {
    let mut v = vec![];
    for o in &sheet.objects {
        match &o.kind {
            Kind::Wire { pts, .. } if o.id != ohne => {
                for w in pts.windows(2) {
                    if waagerecht && (w[0].y - w[1].y).abs() < EPS {
                        v.push(w[0].y);
                    } else if !waagerecht && (w[0].x - w[1].x).abs() < EPS {
                        v.push(w[0].x);
                    }
                }
            }
            Kind::Busbar { a, .. } if waagerecht => v.push(a.y),
            Kind::Symbol { .. } | Kind::Terminal { .. } => {
                v.extend(crate::canvas::obj_pins(o, lib).iter().map(|p| if waagerecht { p.y } else { p.x }));
            }
            _ => {}
        }
    }
    v
}

impl App {
    /// Versatz des Teilstücks `idx` von `orig` beim Ziehen um `roh` (Maus seit dem Drücken, ungefangen): die neue Lage
    /// kommt auf das Raster **absolut** – auch wenn das Teilstück vorher daneben lag – oder auf die Flucht einer
    /// anderen Leitung, Schiene bzw. eines Anschlusses, wenn diese näher liegt (höchstens ein halbes Raster).
    /// Nutzerwunsch Runde 12: Leitungen neben dem Raster ließen sich nicht auf eine andere legen und vereinfachen.
    pub fn teilstueck_versatz(&self, id: u64, orig: &[P], idx: usize, roh: P) -> P {
        let (a, b) = (orig[idx], orig[idx + 1]);
        let d = quer(a, b, roh);
        let waagerecht = (a.y - b.y).abs() < EPS;
        let senkrecht = (a.x - b.x).abs() < EPS;
        if !(waagerecht || senkrecht) {
            return quer(a, b, self.snap(a.add(roh)).sub(a));
        }
        let alt = if waagerecht { a.y } else { a.x };
        let soll = alt + if waagerecht { d.y } else { d.x };
        let raster = if waagerecht { self.snap(P::new(a.x, soll)).y } else { self.snap(P::new(soll, a.y)).x };
        let fang = self.raster() / 2.0;
        // mit Netz-Wächter nur Fluchten des eigenen Netzes (und Anschlüsse): auf eine fremde Leitung gefangen, würde das
        // Teilstück zwei Netze übereinanderlegen und abgewiesen (netz_waechter.rs)
        let flucht = self
            .sheet()
            .map(|s| {
                if !self.opt.netz_waechter {
                    return fluchten(s, &self.lib, id, waagerecht);
                }
                let netz = crate::netz::netz_von(s, &self.lib, id);
                let mut t = s.clone();
                t.objects.retain(|o| !matches!(o.kind, Kind::Wire { .. } | Kind::Busbar { .. }) || netz.contains(&o.id));
                fluchten(&t, &self.lib, id, waagerecht)
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|f| (f - soll).abs() <= fang && (f - alt).abs() > EPS)
            .min_by(|x, y| (x - soll).abs().total_cmp(&(y - soll).abs()));
        let neu = match flucht {
            Some(f) if (f - soll).abs() <= (raster - soll).abs() => f,
            _ => raster,
        };
        if waagerecht {
            P::new(0.0, neu - alt)
        } else {
            P::new(neu - alt, 0.0)
        }
    }

    /// Beim Drücken auf eine einzeln markierte Leitung: Teilstück-Ziehen beginnen. Liefert, ob es begonnen hat.
    pub fn teilstueck_greifen(&mut self, id: u64, m: P, tol: f32) -> bool {
        if self.selection != [id] {
            return false;
        }
        let Some(o) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)) else { return false };
        if o.fixed && self.fix_active {
            return false;
        }
        let Kind::Wire { pts, .. } = &o.kind else { return false };
        // innerer Knick zuerst wie in der Trefferprüfung des Originals (knick_ziehen.rs)
        let (idx, knick) = match crate::knick_ziehen::knick_bei(pts, m, tol) {
            Some(i) => (i, true),
            None => match teilstueck_bei(pts, m, tol) {
                Some(i) => (i, false),
                None => return false,
            },
        };
        let orig = pts.clone();
        self.netz_merken();
        self.drag = Drag::Teilstueck { id, idx, orig, start: self.snap(m), moved: false, knick };
        true
    }

    /// Maus bewegt während des Teilstück-Ziehens.
    pub fn teilstueck_ziehen(&mut self, m: P) {
        let Drag::Teilstueck { id, idx, orig, start, moved, knick } = self.drag.clone() else { return };
        let d = if knick { self.snap(m).sub(start) } else { self.teilstueck_versatz(id, &orig, idx, m.sub(start)) };
        if !moved && d.x.abs() < 1e-4 && d.y.abs() < 1e-4 {
            return;
        }
        if !moved {
            self.checkpoint();
        }
        // Abgriff an der Sammelschiene wandert mit (keine Leitung auf der Schiene, schiene_halten.rs)
        let sch = self.sheet().map(|s| crate::schiene_halten::schienen(s, &[])).unwrap_or_default();
        // freie Enden (vor dem Ziehen an nichts angeschlossen) wandern mit dem Teilstück
        let n = orig.len();
        let frei = [0, n - 1].map(|i| !self.netz_vorher.is_empty() && crate::leitung_anschluss::freies_ende_in(&self.netz_vorher, &self.lib, id, orig[i]));
        let normal = if knick { crate::knick_ziehen::knick_verschieben_frei(&orig, idx, d, frei) } else { verschieben_frei(&orig, idx, d, frei) };
        // Varianten für den Netz-Wächter (netz_waechter.rs): verbindet das Querstück am festen Ende fremde Netze
        // (z. B. waagerecht aus dem Anschluss durch den Anschluss der Nachbarklemme, W2 Blatt 15), erst einen Stich in
        // Richtung des Endstücks, dann das Querstück
        let mut varianten = vec![normal];
        if !knick && self.opt.netz_waechter {
            let st = self.stich();
            for l in [st, 2.0 * st] {
                if let Some(v) = mit_stich(&orig, idx, d, frei, l) {
                    varianten.push(v);
                }
            }
        }
        let gl = crate::schiene_halten::gleise_aus(&self.netz_vorher);
        let zahl = varianten.len();
        for (k, neu_pts) in varianten.into_iter().enumerate() {
            let neu = crate::schiene_halten::abgreifen(&neu_pts, &sch);
            // ebenso auf anderen Leitungen (Anfang auf N: wandert auf N mit, kein Querstück auf N)
            let neu = crate::schiene_halten::abgreifen_gleise(&neu, &gl, id);
            self.with_sheet(|s, _| {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *pts = neu;
                }
            });
            // Abzweige auf der gezogenen Leitung folgen (netz_folgen.rs)
            self.netz_nachfuehren(&[id], false);
            if k + 1 == zahl || self.sheet().is_none_or(|s| crate::netz_waechter::pruefen(&self.netz_vorher, s, &self.lib, &[id]).is_none()) {
                break;
            }
        }
        self.netz_waechter(&[id]);
        self.drag = Drag::Teilstueck { id, idx, orig, start, moved: true, knick };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn einzelnes_gerades_stueck_bekommt_querstuecke() {
        // Q1:14 (40,100) → Klemme (80,100), nach unten gezogen: Enden bleiben, zwei senkrechte Querstücke
        let neu = verschieben(&[p(40.0, 100.0), p(80.0, 100.0)], 0, p(0.0, 20.0));
        assert_eq!(neu, vec![p(40.0, 100.0), p(40.0, 120.0), p(80.0, 120.0), p(80.0, 100.0)]);
    }

    #[test]
    fn mittelstueck_verlaengert_nachbarn() {
        // Z-Leitung wie im Original-Bild: das untere waagerechte Stück nach unten ziehen
        let alt = vec![p(40.0, 100.0), p(40.0, 105.0), p(70.0, 105.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)];
        let neu = verschieben(&alt, 1, p(0.0, 15.0));
        assert_eq!(neu, vec![p(40.0, 100.0), p(40.0, 120.0), p(70.0, 120.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)]);
    }

    #[test]
    fn quer_nur_senkrecht_zum_stueck() {
        assert_eq!(quer(p(0.0, 0.0), p(10.0, 0.0), p(3.0, 4.0)), p(0.0, 4.0));
        assert_eq!(quer(p(0.0, 0.0), p(0.0, 10.0), p(3.0, 4.0)), p(3.0, 0.0));
    }

    #[test]
    fn enden_treffen_kein_teilstueck() {
        let pts = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)];
        assert_eq!(teilstueck_bei(&pts, p(0.2, 0.0), 0.5), None);
        assert_eq!(teilstueck_bei(&pts, p(5.0, 0.3), 0.5), Some(0));
        assert_eq!(teilstueck_bei(&pts, p(10.2, 6.0), 0.5), Some(1));
    }

    /// Keine doppelten Punkte, keine Nullstücke, keine zurücklaufenden Stücke, alles achsgerade.
    fn sauber(v: &[P]) {
        for w in v.windows(2) {
            assert!(w[0].dist(w[1]) > 0.01, "doppelter Punkt {v:?}");
            assert!(achsengerade(w[0], w[1]), "schräg {v:?}");
        }
        for w in v.windows(3) {
            let (d1, d2) = (w[1].sub(w[0]), w[2].sub(w[1]));
            let parallel = (d1.x * d2.y - d1.y * d2.x).abs() < 1e-3;
            assert!(!parallel, "Stücke in Flucht bzw. zurücklaufend {v:?}");
        }
    }

    #[test]
    fn nullstuecke_verschmelzen() {
        // Z-Leitung: Mittelstück auf die Höhe des Anfangs gezogen → erstes Stück hat Länge 0, gerade Leitung
        let alt = vec![p(0.0, 0.0), p(0.0, 10.0), p(20.0, 10.0), p(20.0, 20.0)];
        let neu = verschieben(&alt, 1, p(0.0, -10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)]);
        sauber(&neu);
        // U-Leitung ganz flach gezogen: eine gerade Leitung von Ende zu Ende
        let alt = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(20.0, 10.0), p(20.0, 0.0), p(30.0, 0.0)];
        let neu = verschieben(&alt, 2, p(0.0, -10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(30.0, 0.0)]);
        // erstes Stück auf die Höhe des Folgestücks: Querstück und Nachbar verschmelzen
        let alt = vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0), p(30.0, 10.0)];
        let neu = verschieben(&alt, 0, p(0.0, 10.0));
        assert_eq!(neu, vec![p(0.0, 0.0), p(0.0, 10.0), p(30.0, 10.0)]);
        sauber(&neu);
    }

    #[test]
    fn hin_und_zurueck_ohne_rueckstaende() {
        let alt = vec![p(0.0, 0.0), p(0.0, 10.0), p(20.0, 10.0), p(20.0, 30.0), p(40.0, 30.0)];
        for idx in 0..alt.len() - 1 {
            for d in [-30.0f32, -20.0, -10.0, -5.0, 5.0, 10.0, 20.0, 30.0] {
                let dd = quer(alt[idx], alt[idx + 1], p(d, d));
                let neu = verschieben(&alt, idx, dd);
                sauber(&neu);
                // Enden bleiben angeschlossen
                assert_eq!((neu[0], *neu.last().unwrap()), (alt[0], alt[4]), "idx {idx} d {d}");
            }
            // zurück in die Ausgangslage: wieder die Ausgangsleitung
            assert_eq!(verschieben(&alt, idx, p(0.0, 0.0)), alt);
        }
    }

    #[test]
    fn bereinigen_laeuft_zurueck_bis_zum_vorgaenger() {
        use crate::leitung_fuehrung::bereinigen;
        // Rückläufer bis genau auf den vorigen Eckpunkt: kein doppelter Punkt
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(10.0, 0.0), p(20.0, 0.0)]), vec![p(0.0, 0.0), p(20.0, 0.0)]);
        // über den Anfang hinaus zurück: Anfang bleibt, Leitung läuft in die Gegenrichtung
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(10.0, 0.0), p(-5.0, 0.0)]), vec![p(0.0, 0.0), p(-5.0, 0.0)]);
        // doppelte Endpunkte
        assert_eq!(bereinigen(vec![p(0.0, 0.0), p(0.0, 0.0), p(10.0, 0.0), p(10.0, 0.0)]), vec![p(0.0, 0.0), p(10.0, 0.0)]);
    }
}
