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
}
