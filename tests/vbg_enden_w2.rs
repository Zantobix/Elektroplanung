//! Leitungsenden (VBG Art/Ziel/Anschluss) aus der Geometrie gegen alle vom Original geschriebenen Enden in W2
//! (vbg_enden.rs, Maus-Befund M21), Kennungen eindeutig (kennungen.rs) und Klemmplan-Datei aus dem Nachbau.
//! Ohne EP_SAMPLE bzw. EP_FIXTURES werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, eprfile::{Block, EprFile}, model::*, vbg_enden::{self, Ende}};
use std::collections::HashMap;
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn fixture(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("EP_FIXTURES").ok()?).join(name);
    p.exists().then_some(p)
}

/// Alle VBG-Blöcke (rekursiv).
fn vbgs<'a>(b: &'a Block, out: &mut Vec<&'a Block>) {
    if b.is("VBG") {
        out.push(b);
    }
    for k in &b.kids {
        vbgs(k, out);
    }
}

fn alle_vbg(f: &EprFile) -> Vec<&Block> {
    let mut v = vec![];
    for r in &f.roots {
        vbgs(r, &mut v);
    }
    v
}

/// Alle Kennungen (OBJ, VBG, VSY, SMS) mit Anzahl.
fn kennungen(f: &EprFile) -> HashMap<u64, Vec<String>> {
    fn rek(b: &Block, m: &mut HashMap<u64, Vec<String>>) {
        if matches!(b.tag_str(), "OBJ" | "VBG" | "VSY" | "SMS") {
            let k = elektroplanung::kennungen::kennung_im_block(b);
            m.entry(k).or_default().push(b.tag_str().to_string());
        }
        for k in &b.kids {
            rek(k, m);
        }
    }
    let mut m = HashMap::new();
    for r in &f.roots {
        rek(r, &mut m);
    }
    m
}

fn doppelte(f: &EprFile) -> Vec<(u64, Vec<String>)> {
    kennungen(f).into_iter().filter(|(k, v)| *k != 0 && v.len() > 1).collect()
}

/// Prüfmaßstab: jedes Ende jeder Verbindung in W2 aus der Geometrie neu bestimmt (wie der Nachbau beim Speichern
/// neuer bzw. geänderter Leitungen) = vom Original gespeicherte Werte. Ergebnis: 1751 von 1754 gleich; die drei
/// Abweichungen sind einzeln begründet (siehe `BEGRUENDET`).
#[test]
fn w2_alle_enden_aus_der_geometrie() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let (mut gleich, mut gesamt) = (0, 0);
    let mut abweichungen: Vec<(u32, P, usize, Ende, Ende)> = vec![];
    let mut vergleich = |s: &Sheet, eigen: u64, p: P, i: usize, soll: Ende| {
        let ist = vbg_enden::bestimmen(&project, s, eigen, p, i == 1).unwrap_or_default();
        gesamt += 1;
        if ist == soll {
            gleich += 1;
        } else {
            abweichungen.push((s.number, p, i, soll, ist));
        }
    };
    for a in &project.anlagen {
        for s in &a.sheets {
            for o in &s.objects {
                let (Kind::Wire { pts, .. }, Some(r)) = (&o.kind, o.raw.as_deref()) else { continue };
                vergleich(s, o.id, pts[0], 0, Ende::lesen(&r.block, 0));
                vergleich(s, o.id, *pts.last().unwrap(), 1, Ende::lesen(&r.block, 1));
            }
            // Verbindungen mit nur einem Punkt liest der Nachbau nicht als Leitung (sie bleiben unverändert)
            let Some(r) = s.raw.as_ref() else { continue };
            for (i, k) in r.block.kids.iter().enumerate() {
                if k.is("VBG") && r.map.get(i).copied().flatten().is_none() {
                    let v = k.kid("VBP").expect("VBP");
                    let p = P::new(v.i32(0) as f32 / 10.0, v.i32(1) as f32 / 10.0);
                    vergleich(s, 0, p, 0, Ende::lesen(k, 0));
                    vergleich(s, 0, p, 1, Ende::lesen(k, 1));
                }
            }
        }
    }
    println!("{gleich} von {gesamt} Enden gleich");
    for x in &abweichungen {
        println!("Abweichung: {x:?}");
    }
    assert_eq!(gesamt, 2 * 877, "alle VBG-Blöcke in W2");
    // Begründete Abweichungen (Blatt, Lage, Ende):
    // - Blatt 2 (280,0/185,5): die Leitung endet 9 mm hinter dem Ende der Schiene (x2 = 271,0), gespeichert ist
    //   die Schiene – im Original wurde sie offenbar nach dem Anschließen gekürzt, die Verbindung prüft es nicht
    //   nach. Beim Speichern bleibt das Ende (Ziel unverändert, `vbg_enden::schreiben`).
    // - Blatt 2 (64,3/74,5) und (66,3/79,5): Verbindungen aus einem einzigen Punkt im Innern einer Schiene, Anfang
    //   Anschluss 0, Ende Anschluss 1 (sonst steht 1 nur an Enden auf dem Ende einer Schiene); vermutlich ebenfalls
    //   später geänderte Schienen. Der Nachbau liest sie nicht als Leitung, sie bleiben byteidentisch.
    const BEGRUENDET: [(u32, f32, f32, usize); 3] = [(2, 280.0, 185.5, 1), (2, 64.3, 74.5, 1), (2, 66.3, 79.5, 1)];
    let rest: Vec<_> = abweichungen.iter().filter(|(n, p, i, _, _)| !BEGRUENDET.iter().any(|b| b.0 == *n && P::new(b.1, b.2).dist(*p) < 0.01 && b.3 == *i)).collect();
    assert!(rest.is_empty(), "{gleich} von {gesamt} gleich, unbegründet: {rest:?}");
    assert_eq!(gleich, gesamt - BEGRUENDET.len());
}

/// Unveränderte W2-Verbindungen bleiben beim Speichern byteidentisch (auch mit der Prüfung der Enden).
#[test]
fn w2_verbindungen_unveraendert() {
    let Some(path) = sample() else { return };
    let orig = EprFile::read(&path).unwrap();
    let project = epr::import(&path).unwrap();
    let neu = epr_write::build(&project).unwrap();
    let (a, b) = (alle_vbg(&orig), alle_vbg(&neu));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.data, y.data);
    }
}

/// Neues Projekt mit Schiene, Symbol, Leitung und Klemmstelle in der Leitung: alle Kennungen eindeutig, Enden mit
/// Art 1 (Symbol, Anschlussnummer), 2 (Schiene) und 5 (Klemmstelle).
#[test]
fn neues_projekt_klemmstelle_in_leitung() {
    // ein Symbol der eingebauten Bibliothek mit zwei Anschlüssen senkrecht übereinander
    let def = elektroplanung::symbols::builtin()
        .into_iter()
        .find(|d| d.pins.len() == 2 && (d.pins[0].x - d.pins[1].x).abs() < 1e-3 && d.pin_vis.is_empty() && d.pin_info.iter().all(|i| i.1 == 0))
        .expect("Symbol mit zwei Anschlüssen");
    let mut p = Project::new_default();
    p.symdefs.push(def.clone());
    let pos = P::new(100.0, 100.0);
    let (oben, unten) = if def.pins[0].y < def.pins[1].y { (def.pins[0], def.pins[1]) } else { (def.pins[1], def.pins[0]) };
    let a_oben = pos.add(oben);
    let a_unten = pos.add(unten);
    let leiste = p.anlagen[0].klemmleisten[0].clone();
    let k = P::new(a_unten.x, a_unten.y + 20.0);
    let kinds = vec![
        Kind::Busbar { a: P::new(20.0, 40.0), x2: 200.0, name: "L1".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 },
        Kind::Symbol { key: def.key.clone(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: "-E1".into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
        // Leitung von der Schiene zum oberen Anschluss
        Kind::Wire { pts: vec![P::new(a_oben.x, 40.0), a_oben], leitung: 0, bez: vec![], opts: 0 },
        // Leitung vom unteren Anschluss nach unten, an der Klemmstelle geteilt (wie klemme_leitung::teilen)
        Kind::Wire { pts: vec![a_unten, P::new(k.x, k.y - 0.8)], leitung: 0, bez: vec![], opts: 0 },
        Kind::Terminal { pos: k, leiste, nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() },
        Kind::Wire { pts: vec![P::new(k.x, k.y + 0.8), P::new(k.x, k.y + 30.0)], leitung: 0, bez: vec![], opts: 0 },
    ];
    for kind in kinds {
        let o = Obj { id: p.alloc_id(), kind, pen: 0, fixed: false, group: None, raw: None };
        p.anlagen[0].sheets[0].objects.push(o);
    }
    let f = epr_write::build(&p).unwrap();
    assert!(doppelte(&f).is_empty(), "doppelte Kennungen: {:?}", doppelte(&f));
    let k = kennungen(&f);
    let objs: Vec<&Block> = {
        fn rek<'a>(b: &'a Block, t: &str, out: &mut Vec<&'a Block>) {
            if b.is(t) {
                out.push(b);
            }
            for k in &b.kids {
                rek(k, t, out);
            }
        }
        let mut v = vec![];
        for r in &f.roots {
            rek(r, "OBJ", &mut v);
        }
        v
    };
    let klemme = objs.iter().find(|o| o.data[0x12] == 2).map(|o| elektroplanung::kennungen::kennung_im_block(o)).expect("Klemmstellen-OBJ");
    let schiene = objs.iter().find(|o| o.data[0x12] == 0).map(|o| elektroplanung::kennungen::kennung_im_block(o)).expect("Schienen-OBJ");
    let vbg = alle_vbg(&f);
    assert_eq!(vbg.len(), 3);
    let e: Vec<(Ende, Ende)> = vbg.iter().map(|b| (Ende::lesen(b, 0), Ende::lesen(b, 1))).collect();
    assert_eq!(e[0].0, Ende { art: 2, ziel: schiene, nr: 0 }, "Schiene");
    assert_eq!(e[0].1.art, 1);
    assert_eq!(k.get(&e[0].1.ziel).map(|v| v[0].as_str()), Some("VSY"));
    assert_eq!(e[1].0.art, 1);
    assert_ne!(e[0].1.nr, e[1].0.nr, "zwei verschiedene Anschlüsse");
    assert!(e[0].1.nr > 0 && e[1].0.nr > 0);
    assert_eq!(e[1].1, Ende { art: 5, ziel: klemme, nr: 0 }, "erstes Teilstück endet an der Klemme");
    assert_eq!(e[2].0, Ende { art: 5, ziel: klemme, nr: 0 }, "zweites Teilstück beginnt an der Klemme");
    assert_eq!(e[2].1, Ende::default(), "freies Ende");
}

/// Vom Nachbau gezeichnetes Projekt (Maus-Befund M21): laden, speichern, neu lesen – jedes Ende hat Art ≠ 0 und
/// ein Ziel, die Klemmstelle X1.1 hat zwei Teilstücke mit Art 5 an der Klemme, alle Kennungen eindeutig.
#[test]
fn nachbau_klemmplan_k2_rundreise() {
    let Some(path) = fixture("x_nachbau_klemmplan_k2.epr") else { return };
    let vorher = EprFile::read(&path).unwrap();
    assert!(!doppelte(&vorher).is_empty(), "Datei enthält die Kennungskollision des Befunds");
    let project = epr::import(&path).unwrap();
    let bytes = epr_write::to_bytes(&project).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert!(doppelte(&f).is_empty(), "doppelte Kennungen: {:?}", doppelte(&f));
    let k = kennungen(&f);
    let vbg = alle_vbg(&f);
    let mut an_klemme = 0;
    for b in &vbg {
        for i in 0..2 {
            let e = Ende::lesen(b, i);
            assert!(e.art != 0 && e.ziel != 0, "Ende {i} ohne Ziel: {e:?}");
            let typ = k.get(&e.ziel).map(|v| v[0].as_str());
            match e.art {
                1 => assert_eq!(typ, Some("VSY")),
                2 | 5 => assert_eq!(typ, Some("OBJ")),
                3 => assert_eq!(typ, Some("VBG")),
                a => panic!("Art {a}"),
            }
            if e.art == 5 {
                an_klemme += 1;
            }
        }
    }
    assert_eq!(an_klemme, 2, "zwei Teilstücke an der Klemmstelle");
    // nochmals laden und speichern: unverändert
    let tmp = std::env::temp_dir().join(format!("ep_vbg_enden_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let p2 = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let f2 = epr_write::build(&p2).unwrap();
    let (a, b) = (alle_vbg(&f), alle_vbg(&f2));
    assert_eq!(a.iter().map(|x| &x.data).collect::<Vec<_>>(), b.iter().map(|x| &x.data).collect::<Vec<_>>());
}
