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
//! Beim Speichern bringt [`enden_nachfuehren`] die Anschlussart geänderter Leitungen in Übereinstimmung, damit das
//! Original dieselben Punkte zeigt; unveränderte Blöcke bleiben byteidentisch, solange ihr Knoten noch stimmt.

use crate::eprfile::Block;
use crate::model::{seg_dist, Kind, Obj, Project, Sheet, P};

/// Anschlussart einer Leitung auf einer anderen Leitung.
pub const ART_LEITUNG: u8 = 3;
/// Anschlussart einer Leitung an einer Sammelschiene.
pub const ART_SCHIENE: u8 = 2;
const ART_KLEMME: u8 = 5;
const T: f32 = 0.05;

/// Byte der Anschlussart, Feld der Zielkennung, Feld der Anschlussnummer – für Anfang (0) und Ende (1).
const ENDEN: [(usize, usize, usize); 2] = [(0, 2, 4), (0x14, 6, 8)];

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

fn kennung_bei(b: &Block, feld: usize) -> u64 {
    b.data.get(feld * 4..feld * 4 + 8).map(|d| u64::from_le_bytes(d.try_into().unwrap())).unwrap_or(0)
}

/// Stimmt der gespeicherte Knoten (Art 3 bzw. 2) noch mit der Geometrie überein?
fn knoten_stimmt(sheet: &Sheet, art: u8, kennung: u64, p: P) -> bool {
    sheet.objects.iter().any(|o| {
        crate::epr_neu::kennung(o) == kennung
            && match (&o.kind, art) {
                (Kind::Wire { pts, .. }, ART_LEITUNG) => pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) < T),
                (k, ART_SCHIENE) => auf_schiene(k, p),
                _ => false,
            }
    })
}

/// Anschlussart, Zielkennung und Anschlussnummer beider Enden der Leitung `o` im VBG-Block `b` nachführen. `alt` =
/// Punkte beim Laden (None bei neuen Leitungen). Ein Ende wird nur angefasst, wenn es sich bewegt hat oder sein
/// gespeicherter Knoten (Art 3) nicht mehr zur Geometrie passt. Symbolanschlüsse (Art 1 mit Kennung) bleiben,
/// wie sie sind (die Anschlussnummer kennt nur die Symboldefinition).
pub fn enden_nachfuehren(project: &Project, o: &Obj, b: &mut Block, alt: Option<&[P]>) {
    let Kind::Wire { pts, .. } = &o.kind else { return };
    if pts.len() < 2 || b.data.len() < 0x24 {
        return;
    }
    let Some(sheet) = project.find_obj(o.id).and_then(|(r, _)| project.sheet(r)) else { return };
    for (i, (art_at, id_feld, nr_feld)) in ENDEN.into_iter().enumerate() {
        let p = if i == 0 { pts[0] } else { pts[pts.len() - 1] };
        let a = alt.and_then(|a| if i == 0 { a.first() } else { a.last() }).copied();
        let art = b.data[art_at];
        let kennung = kennung_bei(b, id_feld);
        let bewegt = a.is_none_or(|a| a.dist(p) > 1e-4);
        let veraltet = art == ART_LEITUNG && !knoten_stimmt(sheet, art, kennung, p);
        if !(bewegt || veraltet) || (art == 1 && kennung != 0) {
            continue;
        }
        // Enden an der Pfeilspitze eines Verweises führt `epr_neu::verweis_enden` (Art dort ungeklärt)
        if sheet.objects.iter().any(|v| matches!(&v.kind, Kind::Verweis { pos, .. } if pos.dist(p) < T)) {
            continue;
        }
        match ziel(sheet, o.id, p) {
            Some((neu @ (ART_LEITUNG | ART_SCHIENE), z)) => {
                b.data[art_at] = neu;
                b.data[id_feld * 4..id_feld * 4 + 8].copy_from_slice(&crate::epr_neu::kennung(z).to_le_bytes());
                b.set_i32(nr_feld, if neu == ART_LEITUNG { -1 } else { 0 });
            }
            // Kennung der Klemmstelle schreibt `epr_neu::verweis_enden`
            Some((neu, _)) => b.data[art_at] = neu,
            None if matches!(art, ART_LEITUNG | ART_SCHIENE | ART_KLEMME) => {
                b.data[art_at] = 0;
                b.data[id_feld * 4..id_feld * 4 + 8].fill(0);
                b.set_i32(nr_feld, 0);
            }
            None => {}
        }
    }
}

/// Unveränderte Leitung beim Speichern: Block mit nachgeführtem Knoten, falls ihr gespeicherter Knoten nicht mehr
/// stimmt (die Leitung, auf der sie endete, wurde verschoben oder gelöscht), sonst `None` (Originalblock bleibt).
pub fn veralteter_knoten(project: &Project, o: &Obj, b: &Block) -> Option<Block> {
    let Kind::Wire { pts, .. } = &o.kind else { return None };
    let mut neu = b.clone();
    enden_nachfuehren(project, o, &mut neu, Some(pts));
    (neu.data != b.data).then_some(neu)
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
