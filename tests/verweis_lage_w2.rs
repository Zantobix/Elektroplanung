//! Lage, Richtung und Text der Verweise wie im Original-Ausdruck der Installationspläne von W2 (Seiten 1–4,
//! vermessen): Dreieck zwischen Lage und 2,4 mm in Richtung Feld 14, Bit 0 = Spitze an der Lage, Feld 4 Byte 3 =
//! Text hochkant. Fehlt W2.epr (EP_SAMPLE), wird übersprungen.

use elektroplanung::{canvas, epr, model::*};
use std::path::PathBuf;

fn verweis(p: &Project, blatt: &str, t: &str) -> (P, u16, bool, bool) {
    let s = p.anlagen[0].sheets.iter().find(|s| s.name == blatt).unwrap();
    s.objects
        .iter()
        .find_map(|o| match &o.kind {
            Kind::Verweis { pos, text, dir, vertical, spitze, .. } if text == t => Some((*pos, *dir, *vertical, *spitze)),
            _ => None,
        })
        .unwrap()
}

#[test]
fn verweise_der_installationsplaene() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let p = epr::import(&path).unwrap();
    // (Blatt, Text, Richtung der Spitze, hochkant, Spitze an der Lage) wie im Ausdruck
    let soll = [
        ("Neubau EG", "Raum 12", 0, false, false),
        ("Neubau EG", "=S3+W2", 270, false, false),
        ("Neubeu DG", "Raum 7", 270, false, true),
        ("Altbau EG", "=S1+W2", 0, true, false),
        ("Altbau EG", "=UV3+W2", 180, true, false),
        ("Altbau EG", "Raum 7", 180, true, true),
        ("Dachboden", "Raum 10", 180, false, true),
    ];
    for (blatt, t, dir, vertical, spitze) in soll {
        let (_, d, v, s) = verweis(&p, blatt, t);
        assert_eq!((d, v, s), (dir, vertical, spitze), "{blatt} {t}");
    }
    // ▼ bei "=S3+W2": Grundseite an der Lage, Spitze 2,4 mm darunter
    let (pos, dir, _, spitze) = verweis(&p, "Neubau EG", "=S3+W2");
    let t = canvas::verweis_triangle(pos, dir, spitze);
    assert!((t[0].y - pos.y - 2.4).abs() < 1e-3 && (t[0].x - pos.x).abs() < 1e-3, "{t:?}");
    // ◄ bei "Raum 10" (Dachboden): Spitze an der Lage, Grundseite 2,4 mm rechts
    let (pos, dir, _, spitze) = verweis(&p, "Dachboden", "Raum 10");
    let t = canvas::verweis_triangle(pos, dir, spitze);
    assert!(t[0].dist(pos) < 1e-3 && (t[1].x - pos.x - 2.4).abs() < 1e-3, "{t:?}");
    // Pfeile ohne Text (Schriftfeld) liegen mit der Spitze an der Lage wie bisher
    let ohne = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(&o.kind, Kind::Verweis { text, .. } if text.is_empty()));
    for o in ohne {
        assert!(matches!(o.kind, Kind::Verweis { spitze: true, vertical: false, dir: 0 | 270, .. }), "{:?}", o.kind);
    }
}
