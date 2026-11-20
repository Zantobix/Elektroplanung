//! Lage von Symboltexten und BMK in gedrehten/gespiegelten Symbolen, verglichen mit dem Original-Ausdruck
//! "Elektroplanung Wohnung 2.pdf" (Seite 1 "Motorsymbole" gedreht, Seite 7 "Motor" gedreht und gespiegelt).

use elektroplanung::{canvas, epr, model::*, symbols};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn text_of<'a>(def: &'a symbols::SymDef, s: &str) -> Option<(usize, &'a symbols::Prim)> {
    def.prims.iter().enumerate().find(|(_, p)| matches!(p, symbols::Prim::TextBox(_, _, t, _) if t.starts_with(s)))
}

#[test]
fn text_orientation_field_decides_rotation() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let ms = p.symdefs.iter().find(|d| d.name == "Motorsymbole").unwrap();
    let (i, _) = text_of(ms, "M").expect("M in Motorsymbole");
    assert_eq!(ms.text_mode[i], 2, "Motorsymbole: M dreht mit (Seite 1)");
    let m = p.symdefs.iter().find(|d| d.name == "Motor").unwrap();
    let (i, pr) = text_of(m, "M").expect("M in Motor");
    assert_eq!(m.text_mode[i], 0, "Motor: M bleibt waagerecht (Seite 7)");
    // "M" und "~" sind zwei Zeilen eines Textfeldes (6 bzw. 8 mm hoch) → je Zeile ≥ 3 mm, Schrift ≥ 2,2 mm
    // (im Ausdruck ≈ 3 mm; vorher war die Schrift auf 2,6 mm für beide Zeilen zusammen begrenzt)
    if let symbols::Prim::TextBox(a, b, t, _) = pr {
        assert_eq!(t.lines().count(), 2, "{t:?}");
        assert!((b.y - a.y).abs() / 2.0 >= 3.0, "{a:?} {b:?}");
    }
}

#[test]
fn motor_rotated_and_mirrored_keeps_m_inside_circle() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // Seite 7, "Schaltplan -X1": Motor -M1.1+W2 bei (152,5 / 126,9), Drehung 270° + Spiegelung
    let a = p.anlagen.iter().find(|a| a.name == "Unterverteilung 1").unwrap();
    let s = a.sheets.iter().find(|s| s.name == "Schaltplan -X1").unwrap();
    let (pos, rot, mh, mv) = s
        .objects
        .iter()
        .find_map(|o| match &o.kind {
            Kind::Symbol { pos, rot, mirror_h, mirror_v, bmk, .. } if bmk.starts_with("-M1.1") => Some((*pos, *rot, *mirror_h, *mirror_v)),
            _ => None,
        })
        .unwrap();
    assert_eq!((pos.x, pos.y, rot), (152.5, 126.9, 3));
    let m = p.symdefs.iter().find(|d| d.name == "Motor").unwrap();
    let (_, pr) = text_of(m, "M").unwrap();
    let symbols::Prim::TextBox(ta, tb, ..) = pr else { unreachable!() };
    let c = sym_xf(P::new((ta.x + tb.x) / 2.0, (ta.y + tb.y) / 2.0), pos, rot, mh, mv, 1.0);
    // Kreismitte im Ausdruck: (152,6 / 127,0)
    assert!(c.dist(P::new(152.6, 127.0)) < 0.6, "Textmitte {c:?}");
}

#[test]
fn vertical_title_starts_at_anchor_and_reads_upwards() {
    // Seite 1: "-M2.1" (Motorsymbole bei 202,0 / 145,0; Titel X/Y −1,5 / 12,5; hochkant, Links)
    let at = P::new(202.0 - 1.5, 145.0 + 12.5);
    let (mn, mx) = canvas::title_rect(at, 9.0, 2.6, 0, true);
    assert!((mx.y - 157.5).abs() < 0.01, "unten am Anker");
    assert!(mn.y < mx.y && (mx.y - mn.y - 9.0).abs() < 0.01, "nach oben");
    // gemessen im Ausdruck: Text von y = 147,6 bis 156,5 mm
    assert!((mn.y - 147.6).abs() < 1.5 && (mx.y - 156.5).abs() < 1.5);
    // Mittig/Rechts verschieben den Text nach unten
    assert!((canvas::title_rect(at, 9.0, 2.6, 1, true).1.y - 162.0).abs() < 0.01);
    assert!((canvas::title_rect(at, 9.0, 2.6, 2, true).1.y - 166.5).abs() < 0.01);
    // waagerecht: Rechts endet am Anker
    assert!((canvas::title_rect(at, 9.0, 2.6, 2, false).1.x - at.x).abs() < 0.01);
}

#[test]
fn mirror_rule_rotate_then_mirror() {
    // Original: erst drehen, dann spiegeln. Bei 90°/270° vertauschen sich die Spiegelachsen.
    for rot in 0..4u8 {
        for (b16, b8) in [(false, false), (true, false), (false, true), (true, true)] {
            let (mh, mv) = mirror_from_bits(rot, b16, b8);
            assert_eq!(mirror_to_bits(rot, mh, mv), (b16, b8));
            // Punkt (1, 0): erst drehen, dann am Original spiegeln
            let q = P::new(1.0, 2.0);
            let r = sym_xf(q, P::default(), rot, false, false, 1.0);
            let want = P::new(if b16 { -r.x } else { r.x }, if b8 { -r.y } else { r.y });
            let got = sym_xf(q, P::default(), rot, mh, mv, 1.0);
            assert!(got.dist(want) < 1e-4, "rot {rot} b16 {b16} b8 {b8}: {got:?} != {want:?}");
        }
    }
}
