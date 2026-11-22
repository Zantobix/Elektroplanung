//! Leitung beim Verschieben eines Endes in ihrer Form nachführen („Profil“).
//!
//! Vorbild: QElectroTech (GPL-2.0-or-later), `Conductor::updateConductorPath`, `ConductorProfile` und
//! `shareOffsetBetweenSegments`. Der Algorithmus ist hier in Rust neu geschrieben (kein übernommener Code):
//!
//! - Das Profil wird zur Laufzeit aus dem Polygonzug abgeleitet (Folge von Teilstücken: Achse waagerecht/senkrecht
//!   und vorzeichenbehaftete Länge); die Datei (.epr) bleibt unverändert.
//! - Bewegt sich der Anfang, ändern sich Breite und Höhe der Leitung. Die Differenz je Achse wird gleichmäßig auf
//!   alle Teilstücke dieser Achse mit Länge ≠ 0 verteilt. Ein Teilstück, das dabei das Vorzeichen wechseln würde,
//!   wird 0 (entfällt) und der Rest auf die übrigen verteilt. Knicke und Rechtwinkligkeit bleiben.
//! - Neu entstehende Lagen freier Teilstücke (nicht am Anfang oder Ende) kommen auf das Raster.
//! - Passt das Profil nicht mehr – Ziel in einem anderen Quadranten (Breite oder Höhe wechselt das Vorzeichen) –,
//!   meldet die Routine `Neufuehren`; der Aufrufer legt die Leitung dann neu (leitung_route.rs, Stich + Z). Schräge
//!   Teilstücke oder eine Achse ohne Teilstück (gerade Leitung quer verschoben) ergeben `Unpassend`.

use crate::model::P;

const EPS: f32 = 1e-3;

/// Achse eines Teilstücks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Achse {
    Waagerecht,
    Senkrecht,
}

/// Teilstück des Profils: Achse und vorzeichenbehaftete Länge (mm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stueck {
    pub achse: Achse,
    pub len: f32,
}

/// Ergebnis des Nachführens.
#[derive(Clone, Debug, PartialEq)]
pub enum Ergebnis {
    /// Form erhalten: neue Punkte
    Form(Vec<P>),
    /// Ziel in einem anderen Quadranten: Leitung neu führen
    Neufuehren,
    /// Profil nicht anwendbar (schräges Teilstück, Achse ohne Teilstück)
    Unpassend,
}

/// Doppelte aufeinanderfolgende Punkte entfernen.
fn ohne_doppelte(pts: &[P]) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_none_or(|q| q.dist(*p) >= EPS) {
            out.push(*p);
        }
    }
    out
}

/// Profil eines rechtwinkligen Polygonzugs (ohne doppelte Punkte); `None` bei einem schrägen Teilstück.
pub fn profil(pts: &[P]) -> Option<Vec<Stueck>> {
    pts.windows(2)
        .map(|w| {
            let (dx, dy) = (w[1].x - w[0].x, w[1].y - w[0].y);
            if dy.abs() < EPS {
                Some(Stueck { achse: Achse::Waagerecht, len: dx })
            } else if dx.abs() < EPS {
                Some(Stueck { achse: Achse::Senkrecht, len: dy })
            } else {
                None
            }
        })
        .collect()
}

/// Differenz `diff` auf die Teilstücke `idx` von `st` verteilen (gleichmäßig auf die mit Länge ≠ 0; ein Teilstück,
/// das das Vorzeichen wechseln würde, wird 0 und der Rest neu verteilt). Rückgabe: nicht aufgenommener Rest.
pub fn verteilen(st: &mut [Stueck], idx: &[usize], diff: f32) -> f32 {
    let mut aktiv: Vec<usize> = idx.iter().copied().filter(|&i| st[i].len.abs() >= EPS).collect();
    let mut rest = diff;
    // jede Runde nimmt entweder alles auf oder legt mindestens ein Teilstück still
    for _ in 0..=idx.len() + 1 {
        if rest.abs() < EPS || aktiv.is_empty() {
            break;
        }
        let teil = rest / aktiv.len() as f32;
        let mut weiter = Vec::with_capacity(aktiv.len());
        for &i in &aktiv {
            let alt = st[i].len;
            let neu = alt + teil;
            if neu * alt <= 0.0 || neu.abs() < EPS {
                // Vorzeichenwechsel: Teilstück entfällt, es nimmt nur seine eigene Länge auf
                st[i].len = 0.0;
                rest += alt;
            } else {
                st[i].len = neu;
                rest -= teil;
                weiter.push(i);
            }
        }
        aktiv = weiter;
    }
    rest
}

/// Vorzeichen mit Totzone.
fn signum(x: f32) -> i8 {
    if x > EPS {
        1
    } else if x < -EPS {
        -1
    } else {
        0
    }
}

/// Leitung `pts`, deren Anfang nach `neu` wandert (Ende bleibt), in ihrer Form nachführen. `rund` legt neu
/// entstehende Lagen freier Teilstücke auf das Raster.
pub fn nachfuehren(pts: &[P], neu: P, rund: &dyn Fn(P) -> P) -> Ergebnis {
    let pts = ohne_doppelte(pts);
    if pts.len() < 2 {
        return Ergebnis::Unpassend;
    }
    let Some(mut st) = profil(&pts) else { return Ergebnis::Unpassend };
    let (a, e) = (pts[0], pts[pts.len() - 1]);
    let alt = (e.x - a.x, e.y - a.y);
    let soll = (e.x - neu.x, e.y - neu.y);
    // anderer Quadrant: das Profil passt nicht mehr
    if signum(alt.0) * signum(soll.0) < 0 || signum(alt.1) * signum(soll.1) < 0 {
        return Ergebnis::Neufuehren;
    }
    for (achse, diff) in [(Achse::Waagerecht, soll.0 - alt.0), (Achse::Senkrecht, soll.1 - alt.1)] {
        if diff.abs() < EPS {
            continue;
        }
        let idx: Vec<usize> = (0..st.len()).filter(|&i| st[i].achse == achse).collect();
        if !idx.iter().any(|&i| st[i].len.abs() >= EPS) {
            return Ergebnis::Unpassend;
        }
        if verteilen(&mut st, &idx, diff).abs() >= EPS {
            return Ergebnis::Unpassend;
        }
    }
    // Punkte vom neuen Anfang aus aufbauen
    let mut out = vec![neu];
    let mut p = neu;
    for s in &st {
        p = match s.achse {
            Achse::Waagerecht => P::new(p.x + s.len, p.y),
            Achse::Senkrecht => P::new(p.x, p.y + s.len),
        };
        out.push(p);
    }
    // Läufe gleicher Achse: Lage am Anfang und Ende genau festlegen, freie Lagen (falls verändert) auf das Raster
    let n = st.len();
    let mut k = 0;
    while k < n {
        let mut j = k;
        while j + 1 < n && st[j + 1].achse == st[k].achse {
            j += 1;
        }
        let waag = st[k].achse == Achse::Waagerecht;
        let lage = |q: P| if waag { q.y } else { q.x };
        let neu_lage = if k == 0 {
            Some(lage(neu))
        } else if j == n - 1 {
            Some(lage(e))
        } else {
            let (vorher, jetzt) = (lage(pts[k]), lage(out[k]));
            ((jetzt - vorher).abs() >= EPS).then(|| lage(rund(P::new(jetzt, jetzt))))
        };
        if let Some(c) = neu_lage {
            for q in &mut out[k..=j + 1] {
                if waag {
                    q.y = c;
                } else {
                    q.x = c;
                }
            }
        }
        k = j + 1;
    }
    out[0] = neu;
    *out.last_mut().unwrap() = e;
    let out = ohne_doppelte(&out);
    if out.len() < 2 {
        return Ergebnis::Form(vec![neu, e]);
    }
    Ergebnis::Form(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(p: P) -> P {
        p
    }

    fn eq(a: &[P], b: &[P]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.dist(*y) < 1e-4)
    }

    fn form(e: Ergebnis) -> Vec<P> {
        match e {
            Ergebnis::Form(p) => p,
            x => panic!("{x:?}"),
        }
    }

    fn rechtwinklig(p: &[P]) -> bool {
        profil(p).is_some()
    }

    #[test]
    fn verteilen_gleichmaessig_und_mit_vorzeichenwechsel() {
        let h = |len| Stueck { achse: Achse::Waagerecht, len };
        let mut st = vec![h(10.0), h(30.0)];
        assert!(verteilen(&mut st, &[0, 1], 10.0).abs() < 1e-4);
        assert_eq!((st[0].len, st[1].len), (15.0, 35.0));
        // −30: das kurze Stück fiele unter 0 → entfällt, der Rest geht auf das lange
        let mut st = vec![h(10.0), h(30.0)];
        assert!(verteilen(&mut st, &[0, 1], -30.0).abs() < 1e-4);
        assert_eq!((st[0].len, st[1].len), (0.0, 10.0));
        // mehr als vorhanden: Rest bleibt
        let mut st = vec![h(10.0)];
        assert!((verteilen(&mut st, &[0], -15.0) + 5.0).abs() < 1e-4);
    }

    #[test]
    fn z_form_waagerecht_verschoben_laengen_verteilt() {
        // Z: waagerecht 20, senkrecht 40, waagerecht 30; Anfang 10 nach rechts → beide waagerechten Stücke −5
        let pts = [P::new(0.0, 0.0), P::new(20.0, 0.0), P::new(20.0, 40.0), P::new(50.0, 40.0)];
        let r = form(nachfuehren(&pts, P::new(10.0, 0.0), &id));
        assert!(eq(&r, &[P::new(10.0, 0.0), P::new(25.0, 0.0), P::new(25.0, 40.0), P::new(50.0, 40.0)]), "{r:?}");
        assert!(rechtwinklig(&r));
    }

    #[test]
    fn z_form_senkrecht_verschoben_knicke_bleiben() {
        let pts = [P::new(0.0, 0.0), P::new(20.0, 0.0), P::new(20.0, 40.0), P::new(50.0, 40.0)];
        // 10 nach unten: nur das senkrechte Stück wird kürzer, Punktzahl bleibt
        let r = form(nachfuehren(&pts, P::new(0.0, 10.0), &id));
        assert!(eq(&r, &[P::new(0.0, 10.0), P::new(20.0, 10.0), P::new(20.0, 40.0), P::new(50.0, 40.0)]), "{r:?}");
        // U-Form (senkrecht hin und zurück): Differenz auf beide senkrechten Stücke
        let u = [P::new(0.0, 0.0), P::new(0.0, 20.0), P::new(30.0, 20.0), P::new(30.0, 10.0)];
        let r = form(nachfuehren(&u, P::new(0.0, 4.0), &id));
        assert!(eq(&r, &[P::new(0.0, 4.0), P::new(0.0, 22.0), P::new(30.0, 22.0), P::new(30.0, 10.0)]), "{r:?}");
        assert!(rechtwinklig(&r));
    }

    #[test]
    fn freie_lagen_auf_das_raster() {
        let raster = |p: P| P::new((p.x / 5.0).round() * 5.0, (p.y / 5.0).round() * 5.0);
        let pts = [P::new(0.0, 0.0), P::new(20.0, 0.0), P::new(20.0, 40.0), P::new(50.0, 40.0)];
        // −5 je Stück → senkrechtes Stück bei 22,5 → Raster 25 (bzw. 20); Enden bleiben genau
        let r = form(nachfuehren(&pts, P::new(5.0, 0.0), &raster));
        assert_eq!(r.len(), 4);
        assert!((r[1].x - 20.0).abs() < 1e-4 || (r[1].x - 25.0).abs() < 1e-4, "{r:?}");
        assert_eq!(r[0], P::new(5.0, 0.0));
        assert_eq!(r[3], P::new(50.0, 40.0));
        assert!(rechtwinklig(&r));
    }

    #[test]
    fn quadrantenwechsel_und_unpassend() {
        let l = [P::new(0.0, 0.0), P::new(30.0, 0.0), P::new(30.0, 40.0)];
        assert_eq!(nachfuehren(&l, P::new(40.0, 0.0), &id), Ergebnis::Neufuehren);
        assert_eq!(nachfuehren(&l, P::new(0.0, 50.0), &id), Ergebnis::Neufuehren);
        // gerade Leitung quer verschoben: keine senkrechten Stücke
        let g = [P::new(0.0, 0.0), P::new(50.0, 0.0)];
        assert_eq!(nachfuehren(&g, P::new(0.0, 10.0), &id), Ergebnis::Unpassend);
        // schräg
        let s = [P::new(0.0, 0.0), P::new(30.0, 30.0)];
        assert_eq!(nachfuehren(&s, P::new(0.0, 10.0), &id), Ergebnis::Unpassend);
        // Stück läuft auf 0: entfällt
        let r = form(nachfuehren(&l, P::new(0.0, 40.0), &id));
        assert!(eq(&r, &[P::new(0.0, 40.0), P::new(30.0, 40.0)]), "{r:?}");
    }
}
