//! Verbindungsknoten (Leiter an Leiter, Leiter an Sammelschiene) im .epr nachführen (todo.md Punkt 1, 22.11.2026).
//!
//! Das Original speichert Knoten **nicht** als eigene Objekte. Jedes Leitungsende (VBG) trägt seine Anschlussart:
//! Byte 0 (Anfang) bzw. Byte 0x14 (Ende) = 1 Symbol, 2 Sammelschiene, 3 andere Leitung, 5 Klemmstelle; dazu die
//! Kennung des Ziels (Feld 2/3 bzw. 6/7) und die Anschlussnummer (Feld 4 bzw. 8, −1 bei Art 3). Endet eine Leitung
//! beim Zeichnen auf einer anderen (MouseUp, Dekompilat bei 0x41d371), setzt das Original Art 3, Anschluss −1 und die
//! Kennung der getroffenen Leitung (deren VBG Feld 10/11). Die Zeichenroutine der Leitung (0x46422c) setzt dann
//! einen gefüllten Punkt (Ellipse) an das Ende der Art 3 (bzw. Art 2 mit Anschluss ≠ 1).
//!
//! W2 (fixtures/W2.epr): 453 Enden der Art 3, alle am Ende (Feld 5), alle geometrisch im Innern einer anderen
//! Leitung; kein Anfang der Art 3. Der Nachbau zeichnet die Punkte weiter aus der Geometrie (`canvas::junctions`),
//! damit sie beim Verschieben (Teilstück ziehen, Symbol/Klemmstelle mitziehen) sofort entstehen und verschwinden.
//! Beim Speichern schreibt `vbg_enden` Anschlussart, Ziel und Anschluss aller Enden aus der Geometrie, damit das
//! Original dieselben Punkte zeigt; unveränderte Blöcke bleiben byteidentisch, solange ihre Enden noch stimmen.
//!
//! **Knotenregel** (`punkte`, neu geschrieben nach KiCad Eeschema `JUNCTION_HELPERS::AnalyzePoint`, GPL-3.0; kein
//! Code übernommen): An einem Ort entsteht ein Punkt, wenn dort mindestens ein Leitungsende liegt und mindestens
//! drei **verschiedene** Leitungsrichtungen zusammentreffen. Eine Leitung, die durch den Ort läuft, zählt mit zwei
//! Richtungen (auch über einen Knick: dann die beiden Schenkel), ein Ende mit einer. Gleiche Richtungen
//! (übereinander laufende Stücke) zählen einmal. Damit gilt wie bisher: Ende im Innern einer anderen Leitung =
//! Punkt (T-Abzweig, auch auf einem Knick), reine Kreuzung = kein Punkt; neu: zwei Enden stumpf aufeinander oder in
//! gerader Flucht = kein Punkt, drei Enden, von denen zwei übereinander liegen = kein Punkt, ein Ende auf der
//! Kreuzung zweier Leitungen = Punkt. Was das Original zeigt, bleibt: Ein Ende im Innern einer anderen Leitung
//! (Art 3) bekommt immer einen Punkt, auch wenn das Endstück auf ihr zurückläuft und nur zwei Richtungen zählen
//! (W2, Blatt „Sicherung -F9“: Leitung endet auf dem Knick einer anderen, ihr letztes Stück liegt auf deren Stück).
//! Abgriffe an Sammelschienen bekommen wie im Original immer einen Punkt.

use crate::model::{seg_dist, Kind, Obj, Sheet, P};

/// Anschlussart einer Leitung auf einer anderen Leitung.
pub const ART_LEITUNG: u8 = 3;
/// Anschlussart einer Leitung an einer Sammelschiene.
pub const ART_SCHIENE: u8 = 2;
const ART_KLEMME: u8 = 5;
const T: f32 = 0.05;

/// Liegt `p` im Innern (nicht an einem Ende) der Leitung `pts`?
pub fn im_innern(pts: &[P], p: P) -> bool {
    pts.len() >= 2 && pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) < T) && pts[0].dist(p) > T && pts[pts.len() - 1].dist(p) > T
}

/// Richtung von `a` nach `b` als ganze Gradzahl 0…359 (gleiche Richtungen fallen zusammen).
fn winkel(a: P, b: P) -> i32 {
    ((b.y - a.y).atan2(b.x - a.x).to_degrees().round() as i32).rem_euclid(360)
}

/// Verschiedene Leitungsrichtungen, die am Ort `p` zusammentreffen (KiCad `AnalyzePoint`): je Teilstück, das in
/// `p` beginnt oder endet, eine Richtung; läuft es durch `p`, beide. Nullstücke zählen nicht.
pub fn richtungen(leitungen: &[&[P]], p: P) -> Vec<i32> {
    let mut r: Vec<i32> = vec![];
    for pts in leitungen {
        for s in pts.windows(2) {
            let (a, b) = (s[0], s[1]);
            if a.dist(b) < T {
                continue;
            }
            if p.dist(a) < T {
                r.push(winkel(a, b));
            } else if p.dist(b) < T {
                r.push(winkel(b, a));
            } else if seg_dist(p, a, b) < T {
                r.push(winkel(a, b));
                r.push(winkel(b, a));
            }
        }
    }
    r.sort_unstable();
    r.dedup();
    r
}

/// Verbindungspunkte eines Blattes (Anzeige, Druck, Export): Leitungsenden mit mindestens drei Richtungen
/// (`richtungen`), Leitungsenden im Innern einer anderen Leitung (Art 3 des Originals) und Leitungsenden auf
/// Sammelschienen.
pub fn punkte(sheet: &Sheet) -> Vec<P> {
    let leitungen: Vec<&[P]> = sheet.objects.iter().filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.as_slice()) } else { None }).collect();
    let enden: Vec<(usize, P)> = leitungen.iter().enumerate().filter(|(_, w)| w.len() >= 2).flat_map(|(i, w)| [(i, w[0]), (i, w[w.len() - 1])]).collect();
    let mut out: Vec<P> = vec![];
    for (i, e) in &enden {
        if out.iter().any(|q| q.dist(*e) < T) {
            continue;
        }
        // Abgriffe an Sammelschienen: das Original zeichnet beim Zeichnen der Schiene an jedem angeschlossenen
        // Leiter einen Punkt; Leiter, die eine Schiene nur kreuzen, bekommen keinen.
        let art3 = leitungen.iter().enumerate().any(|(j, w)| j != *i && im_innern(w, *e));
        if art3 || sheet.objects.iter().any(|o| auf_schiene(&o.kind, *e)) || richtungen(&leitungen, *e).len() >= 3 {
            out.push(*e);
        }
    }
    out
}

/// Liegt `p` auf der Sammelschiene?
fn auf_schiene(k: &Kind, p: P) -> bool {
    matches!(k, Kind::Busbar { a, x2, .. } if (p.y - a.y).abs() < T && p.x >= a.x.min(*x2) - T && p.x <= a.x.max(*x2) + T)
}

/// Ziel eines Leitungsendes `p` der Leitung `eigen` nach der Geometrie: Anschlussart und Objekt.
pub fn ziel<'a>(sheet: &'a Sheet, eigen: u64, p: P) -> Option<(u8, &'a Obj)> {
    // Klemmstelle vor Schiene und Leitung (Leitungsteile enden 0,8 mm vor dem Mittelpunkt)
    if let Some(o) = sheet.objects.iter().find(|o| matches!(&o.kind, Kind::Terminal { pos, .. } if pos.dist(p) <= crate::klemme_leitung::KLEMME_R + T)) {
        return Some((ART_KLEMME, o));
    }
    if let Some(o) = sheet.objects.iter().find(|o| auf_schiene(&o.kind, p)) {
        return Some((ART_SCHIENE, o));
    }
    sheet.objects.iter().find(|o| o.id != eigen && matches!(&o.kind, Kind::Wire { pts, .. } if im_innern(pts, p))).map(|o| (ART_LEITUNG, o))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    fn leitung(id: u64, pts: Vec<P>) -> Obj {
        obj(id, Kind::Wire { pts, leitung: 0, bez: vec![], opts: 0 })
    }

    #[test]
    fn punkte_an_abgriff_und_abzweig_nicht_an_kreuzung() {
        let mut s = Sheet::new(1);
        s.objects.push(obj(1, Kind::Busbar { a: p(20.0, 30.0), x2: 200.0, name: "L1".into(), label: p(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 }));
        // Abgriff an der Schiene, senkrecht nach unten
        s.objects.push(leitung(2, vec![p(50.0, 30.0), p(50.0, 90.0)]));
        // kreuzt Schiene und Leitung 2, ohne anzuschließen
        s.objects.push(leitung(3, vec![p(80.0, 20.0), p(80.0, 60.0), p(40.0, 60.0)]));
        // T-Abzweig auf Leitung 2
        s.objects.push(leitung(4, vec![p(100.0, 70.0), p(50.0, 70.0)]));
        let j = crate::canvas::junctions(&s);
        assert!(j.iter().any(|q| q.dist(p(50.0, 30.0)) < 0.01), "Abgriff");
        assert!(j.iter().any(|q| q.dist(p(50.0, 70.0)) < 0.01), "Abzweig");
        assert_eq!(j.len(), 2, "keine Punkte an Kreuzungen: {j:?}");
        assert_eq!(ziel(&s, 4, p(50.0, 70.0)).map(|(a, o)| (a, o.id)), Some((ART_LEITUNG, 2)));
        assert_eq!(ziel(&s, 2, p(50.0, 30.0)).map(|(a, o)| (a, o.id)), Some((ART_SCHIENE, 1)));
        // Teilstück von Leitung 2 weg: Abzweig hängt frei
        s.objects[1].kind = Kind::Wire { pts: vec![p(50.0, 30.0), p(50.0, 40.0), p(30.0, 40.0), p(30.0, 90.0)], leitung: 0, bez: vec![], opts: 0 };
        assert!(ziel(&s, 4, p(50.0, 70.0)).is_none());
        assert!(!crate::canvas::junctions(&s).iter().any(|q| q.dist(p(50.0, 70.0)) < 0.01));
    }

    fn blatt(ls: Vec<Vec<P>>) -> Sheet {
        let mut s = Sheet::new(1);
        for (i, pts) in ls.into_iter().enumerate() {
            s.objects.push(leitung(i as u64 + 1, pts));
        }
        s
    }

    #[test]
    fn knotenregel_nach_richtungen() {
        // Ende auf dem Knick einer anderen Leitung: 2 Schenkel + 1 = 3 → Punkt
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0)], vec![p(40.0, 0.0), p(20.0, 0.0)]]);
        assert_eq!(punkte(&s), vec![p(20.0, 0.0)]);
        // zwei Enden über Eck (L aus zwei Leitungen) und in gerader Flucht: kein Punkt
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0)], vec![p(20.0, 0.0), p(20.0, 20.0)]]);
        assert!(punkte(&s).is_empty());
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0)], vec![p(20.0, 0.0), p(40.0, 0.0)]]);
        assert!(punkte(&s).is_empty());
        // drei Enden aus drei Richtungen: Punkt; liegen zwei übereinander (gleiche Richtung): kein Punkt
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0)], vec![p(40.0, 0.0), p(20.0, 0.0)], vec![p(20.0, 20.0), p(20.0, 0.0)]]);
        assert_eq!(punkte(&s), vec![p(20.0, 0.0)]);
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0)], vec![p(0.0, 0.0), p(20.0, 0.0)], vec![p(40.0, 0.0), p(20.0, 0.0)]]);
        assert!(punkte(&s).is_empty());
        // Ende, das auf der anderen Leitung zurückläuft (übereinander): nur 2 Richtungen, aber im Innern (Art 3 des
        // Originals, W2 „Sicherung -F9“) → Punkt wie im Original
        let s = blatt(vec![vec![p(0.0, 0.0), p(40.0, 0.0)], vec![p(30.0, 10.0), p(30.0, 0.0), p(20.0, 0.0)]]);
        assert_eq!(richtungen(&[&[p(0.0, 0.0), p(40.0, 0.0)], &[p(30.0, 10.0), p(30.0, 0.0), p(20.0, 0.0)]], p(20.0, 0.0)).len(), 2);
        assert_eq!(punkte(&s), vec![p(20.0, 0.0)]);
        // reine Kreuzung: 4 Richtungen, aber kein Ende → kein Punkt
        let s = blatt(vec![vec![p(0.0, 10.0), p(40.0, 10.0)], vec![p(20.0, 0.0), p(20.0, 30.0)]]);
        assert!(punkte(&s).is_empty());
        // Ende genau auf einer Kreuzung: Punkt
        let s = blatt(vec![vec![p(0.0, 10.0), p(40.0, 10.0)], vec![p(20.0, 0.0), p(20.0, 30.0)], vec![p(20.0, 10.0), p(30.0, 0.0)]]);
        assert_eq!(punkte(&s), vec![p(20.0, 10.0)]);
        // Nullstücke (doppelte Punkte) zählen nicht
        let s = blatt(vec![vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 0.0)], vec![p(20.0, 0.0), p(20.0, 20.0)]]);
        assert!(punkte(&s).is_empty());
    }
}
