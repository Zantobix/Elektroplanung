//! Anmerkungen aus den Maustests (Runde 1) gegen das Beispielprojekt (W2.epr) und ein neues Projekt:
//! Text vergrößern (Umriss passt zur Schrift), Symbol vergrößern (Titel/Wert bleiben stehen), Textversatz und
//! Leitungen an Verweisen überstehen Speichern im Originalformat und Laden.
//! Fehlt W2.epr (EP_SAMPLE), werden die W2-Teile übersprungen.

use elektroplanung::{canvas, epr, epr_write, eprfile::EprFile, groesse, model::*, textmass};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if p.exists() {
        Some(p)
    } else {
        eprintln!("W2.epr nicht gefunden – Test übersprungen");
        None
    }
}

fn rundreise(p: &Project, name: &str) -> Project {
    let tmp = std::env::temp_dir().join(format!("ep_bedienung3_{name}_{}.epr", std::process::id()));
    epr_write::save(p, &tmp).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    q
}

fn alle(p: &Project) -> impl Iterator<Item = (usize, usize, &Obj)> {
    p.anlagen.iter().enumerate().flat_map(|(ai, a)| a.sheets.iter().enumerate().flat_map(move |(si, s)| s.objects.iter().map(move |o| (ai, si, o))))
}

/// Lage (Anlage, Blatt, Index) des ersten Symbols mit BMK und gespeicherter Titellage.
fn stelle(p: &Project) -> (usize, usize, usize) {
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            if let Some(oi) = s.objects.iter().position(|o| matches!(&o.kind, Kind::Symbol { bmk, title: Some(_), .. } if !bmk.is_empty())) {
                return (ai, si, oi);
            }
        }
    }
    panic!("kein Symbol mit BMK");
}

fn u64_at(d: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(d[at..at + 8].try_into().unwrap())
}

#[test]
fn text_vergroessern_umriss_passt() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, si, id) = alle(&p).find(|(_, _, o)| matches!(&o.kind, Kind::Text { text, .. } if text == "Raum 2")).map(|(a, s, o)| (a, s, o.id)).expect("Text Raum 2");
    let o = p.anlagen[ai].sheets[si].objects.iter_mut().find(|o| o.id == id).unwrap();
    let Kind::Text { pos, size: s0, .. } = o.kind.clone() else { unreachable!() };
    // dreimal so groß ziehen (Festpunkt = Bezugspunkt des Textes)
    groesse::skaliere(&mut o.kind, pos, 3.0, 3.0);
    let Kind::Text { size, style, text, .. } = &o.kind else { unreachable!() };
    assert!((size - 3.0 * s0).abs() < 1e-4);
    let (rmn, rmx) = style.rect.expect("Textbox aus W2");
    // Umriss (Markierung, Treffer, Rahmen mit Griffen) umfasst Textbox und gezeichnete Schrift
    let (mn, mx) = canvas::obj_bounds(o, &[]);
    let (w, h) = textmass::groesse(text, *size, false, false);
    assert!(mx.x - mn.x >= w - 1e-3 && mx.y - mn.y >= h - 1e-3, "Umriss {mn:?}–{mx:?} kleiner als Schrift {w}×{h}");
    assert!(mn.x <= rmn.x && mn.y <= rmn.y && mx.x >= rmx.x && mx.y >= rmx.y);
    assert!(canvas::hit(o, P::new(mn.x + w * 0.95, (mn.y + mx.y) / 2.0), 0.0, &[]), "Treffer am Ende der vergrößerten Schrift");
    let erwartet = o.kind.clone();
    // Schriftgrad (pt) und Textbox überstehen Speichern + Laden
    let q = rundreise(&p, "text");
    let g = alle(&q).map(|(_, _, o)| o).find(|o| matches!(&o.kind, Kind::Text { text, .. } if text == "Raum 2")).unwrap();
    let (Kind::Text { size: s2, style: st2, .. }, Kind::Text { size: s1, style: st1, .. }) = (&g.kind, &erwartet) else { unreachable!() };
    assert!((s2 - s1).abs() < 0.2, "Schrift {s2} statt {s1}");
    let ((a2, b2), (a1, b1)) = (st2.rect.unwrap(), st1.rect.unwrap());
    assert!(a2.dist(a1) < 0.06 && b2.dist(b1) < 0.06);
}

#[test]
fn symbol_vergroessern_texte_bleiben_stehen() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, si, oi) = stelle(&p);
    let o = &mut p.anlagen[ai].sheets[si].objects[oi];
    let vorher = groesse::beschriftungen(&o.kind);
    let (a, _) = canvas::obj_bounds(o, &[]);
    let Kind::Symbol { scale: s0, pos: p0, .. } = o.kind.clone() else { unreachable!() };
    // um einen Punkt links oben auf das Doppelte: das Symbol wandert und wächst, Titel und Wert bleiben stehen
    groesse::skaliere(&mut o.kind, P::new(a.x - 5.0, a.y - 5.0), 2.0, 2.0);
    let nachher = groesse::beschriftungen(&o.kind);
    for (v, n) in vorher.iter().zip(&nachher) {
        let ((a0, b0), (a1, b1)) = (v.unwrap_or_default(), n.unwrap_or_default());
        assert!(a0.dist(a1) < 1e-3 && b0.dist(b1) < 1e-3, "Beschriftung {v:?} → {n:?}");
    }
    let Kind::Symbol { pos, title, .. } = o.kind.clone() else { unreachable!() };
    assert!(pos.dist(p0) > 1.0);
    let titel = pos.add(title.unwrap().0);
    let q = rundreise(&p, "symbol");
    let Kind::Symbol { pos: p2, title: t2, scale, .. } = q.anlagen[ai].sheets[si].objects[oi].kind.clone() else { unreachable!() };
    assert!((scale - 2.0 * s0).abs() < 1e-4, "Maßstab {scale}");
    assert!(p2.add(t2.unwrap().0).dist(titel) < 0.1, "Titel auf dem Blatt verschoben");
}

#[test]
fn textversatz_in_der_rundreise() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, si, oi) = stelle(&p);
    let o = &mut p.anlagen[ai].sheets[si].objects[oi];
    // Titel (VSY Feld 10/11) gegenüber dem Symbol versetzen; der Wert bleibt, wo er war (Feld 13/14)
    let wert = groesse::wert_off(&o.kind).unwrap();
    groesse::setze_beschriftung(&mut o.kind, false, P::new(4.2, -7.5));
    assert_eq!(groesse::wert_off(&o.kind), Some(wert));
    let q = rundreise(&p, "versatz");
    let k = &q.anlagen[ai].sheets[si].objects[oi].kind;
    let Kind::Symbol { title, .. } = k else { unreachable!() };
    assert!(title.unwrap().0.dist(P::new(4.2, -7.5)) < 0.01);
    assert!(groesse::wert_off(k).unwrap().dist(wert) < 0.06);
}

/// Leitung von `a` zum Verweis an `v`: nach Speichern + Laden endet sie noch dort, und VBG Feld 6/7 trägt die
/// Kennung des Verweises (OBJ Feld 0/1).
fn pruefe_leitung_an_verweis(q: &Project, v: P, sheet: SheetRef) {
    let s = q.sheet(sheet).unwrap();
    let verweis = s.objects.iter().find(|o| matches!(&o.kind, Kind::Verweis { pos, .. } if pos.dist(v) < 0.01)).expect("Verweis");
    let leitung = s.objects.iter().find(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts.last().is_some_and(|e| e.dist(v) < 0.01))).expect("Leitung am Verweis");
    assert!(canvas::obj_pins(verweis, &[]).iter().any(|q| q.dist(v) < 0.01), "Pfeilspitze ist Anschluss");
    let vbg = &leitung.raw.as_ref().unwrap().block;
    let obj = &verweis.raw.as_ref().unwrap().block;
    assert_ne!(u64_at(&obj.data, 0), 0);
    assert_eq!(u64_at(&vbg.data, 24), u64_at(&obj.data, 0), "VBG Feld 6/7 = Kennung des Verweises");
    assert_eq!(vbg.i32(8), 0, "Anschluss 0");
}

#[test]
fn leitung_an_verweis_neues_projekt() {
    let mut p = Project::new_default();
    let v = P::new(100.0, 60.0);
    let s = &mut p.anlagen[0].sheets[0];
    s.objects.push(Obj { id: 1, kind: Kind::Verweis { pos: v, text_pos: P::new(104.0, 60.0), text: "=A".into(), dir: 0, vertical: false, spitze: false, ziel: None }, pen: 0, fixed: false, group: None, raw: None });
    s.objects.push(Obj { id: 2, kind: Kind::Wire { pts: vec![P::new(60.0, 60.0), v], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
    p.next_id = 3;
    let bytes = epr_write::to_bytes(&p).unwrap();
    let q = epr::import_file(&EprFile::parse(&bytes).unwrap()).unwrap();
    pruefe_leitung_an_verweis(&q, v, SheetRef { anlage: 0, sheet: 0 });
}

#[test]
fn leitung_an_verweis_w2() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let (ai, si, v) = alle(&p).find_map(|(a, s, o)| if let Kind::Verweis { pos, text, .. } = &o.kind { (!text.is_empty()).then_some((a, s, *pos)) } else { None }).unwrap();
    let id = p.alloc_id();
    p.anlagen[ai].sheets[si].objects.push(Obj { id, kind: Kind::Wire { pts: vec![P::new(v.x - 20.0, v.y), v], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
    let r = SheetRef { anlage: ai, sheet: si };
    let q = rundreise(&p, "verweis");
    pruefe_leitung_an_verweis(&q, v, r);
    // Ende vom Verweis wegziehen: die Kennung wird beim nächsten Speichern gelöst
    let mut q = q;
    for o in q.anlagen[ai].sheets[si].objects.iter_mut() {
        if let Kind::Wire { pts, .. } = &mut o.kind {
            if pts.last().is_some_and(|e| e.dist(v) < 0.01) {
                *pts.last_mut().unwrap() = P::new(v.x - 5.0, v.y);
            }
        }
    }
    let z = rundreise(&q, "verweis2");
    let w = z.sheet(r).unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Wire { pts, .. } if pts.last().is_some_and(|e| e.dist(P::new(v.x - 5.0, v.y)) < 0.01))).unwrap();
    assert_eq!(u64_at(&w.raw.as_ref().unwrap().block.data, 24), 0);
}
