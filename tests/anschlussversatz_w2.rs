//! Versatz je Option (Element Feld 13/14, `symbols::Versatz`): mehrpolige Symbole werden gezeichnet, gefangen und
//! angeschlossen, wie das Original sie in W2 gespeichert hat. Ohne EP_SAMPLE bzw. EP_DATENBANK übersprungen.

use elektroplanung::{canvas, epr, model::*, symbols::{self, Prim}, vbg_enden::{self, Ende}};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn bibliothek() -> Option<Vec<symbols::SymDef>> {
    let p = PathBuf::from(std::env::var("EP_DATENBANK").ok()?);
    symbols::load_db(&p).ok()
}

/// Jedes vom Original gespeicherte Leitungsende der Art 1 an einem Symbol mit Option > 0 liegt auf einem
/// gezeichneten (und gefangenen) Anschluss dieses Symbols (`canvas::obj_pins`) mit der gespeicherten Nummer.
#[test]
fn w2_enden_an_mehrpoligen_symbolen_liegen_auf_anschluessen() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let (mut gesamt, mut versetzt, mut symbole) = (0, 0, std::collections::BTreeSet::new());
    let mut fehler = vec![];
    for a in &project.anlagen {
        for s in &a.sheets {
            for o in &s.objects {
                let (Kind::Wire { pts, .. }, Some(r)) = (&o.kind, o.raw.as_deref()) else { continue };
                for i in 0..2 {
                    let e = Ende::lesen(&r.block, i);
                    if e.art != vbg_enden::ART_SYMBOL {
                        continue;
                    }
                    let Some(sym) = s.objects.iter().find(|x| matches!(x.kind, Kind::Symbol { .. }) && elektroplanung::epr_neu::kennung(x) == e.ziel) else { continue };
                    let Kind::Symbol { option, key, .. } = &sym.kind else { continue };
                    if *option == 0 {
                        continue;
                    }
                    let p = if i == 0 { pts[0] } else { *pts.last().unwrap() };
                    gesamt += 1;
                    symbole.insert(sym.id);
                    let def = symbols::find(&project.symdefs, key).unwrap();
                    let gezeichnet = canvas::obj_pins(sym, &project.symdefs);
                    let mit_nr = vbg_enden::symbol_anschluesse(&project, sym);
                    let auf_pin = gezeichnet.iter().any(|q| q.dist(p) < 0.05);
                    let nr_passt = mit_nr.iter().any(|(q, nr)| q.dist(p) < 0.05 && *nr == e.nr);
                    // ohne Versatz läge das Ende nicht auf einem Anschluss
                    let Kind::Symbol { pos, rot, mirror_h, mirror_v, scale, flags, .. } = &sym.kind else { unreachable!() };
                    let ohne = symbols::anschluss_lagen(def, 0, *flags).into_iter().map(|(k, _)| sym_xf(def.pins[k], *pos, *rot, *mirror_h, *mirror_v, *scale));
                    if !ohne.clone().any(|q| q.dist(p) < 0.05) {
                        versetzt += 1;
                    }
                    if !(auf_pin && nr_passt) {
                        fehler.push((s.number, def.name.clone(), *option, p, e.nr));
                    }
                }
            }
        }
    }
    println!("{gesamt} Enden an {} Symbolen mit Option > 0, davon {versetzt} nur mit Versatz auf einem Anschluss", symbole.len());
    assert!(fehler.is_empty(), "Enden nicht auf einem gezeichneten Anschluss: {fehler:?}");
    assert!(gesamt > 0 && versetzt > 0);
}

/// Der vom Original gespeicherte Umriss (VSY Feld 32–35) mehrpoliger Symbole enthält alle versetzten Anschlüsse,
/// und `symbols::bounds_ext` mit den Zusatzoptionen der Instanz (Umriss neuer Symbole, Treffer) liegt in ihm
/// (Leistungskontakt 3-polig mit Hilfskontakt: 37,0–57,0 in 36,5–58,3; mit allen Zusatzoptionen wären es 66,0).
#[test]
fn w2_umriss_mehrpoliger_symbole() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let mut n = 0;
    for o in project.anlagen.iter().flat_map(|a| a.sheets.iter()).flat_map(|s| s.objects.iter()) {
        let Kind::Symbol { key, pos, option, rot, mirror_h, mirror_v, ext, flags, .. } = &o.kind else { continue };
        let (Some((a, b)), true) = (ext.bounds, *option > 0) else { continue };
        let Some(def) = symbols::find(&project.symdefs, key) else { continue };
        if def.versatz.ist_leer() || *rot != 0 || *mirror_h || *mirror_v {
            continue;
        }
        n += 1;
        let (a, b) = (pos.add(a), pos.add(b));
        for q in canvas::obj_pins(o, &project.symdefs) {
            assert!(q.x >= a.x - 0.05 && q.x <= b.x + 0.05 && q.y >= a.y - 0.05 && q.y <= b.y + 0.05, "{} Option {option}: Anschluss {q:?} außerhalb {a:?}–{b:?}", def.name);
        }
        let (mn, mx) = symbols::bounds_ext(def, *option, Some(*flags));
        let (mn, mx) = (pos.add(mn), pos.add(mx));
        assert!(mn.x >= a.x - 0.6 && mx.x <= b.x + 0.6, "{} Option {option}: {mn:?}–{mx:?} nicht in {a:?}–{b:?}", def.name);
    }
    assert!(n > 0);
}

/// Leistungskontakt 3-polig (Bibliothek, Option 2): drei Pole bei −7/0/+7 mm, je zwei Anschlüsse, drei
/// Schaltstücke; die Pole liegen 7 mm auseinander, die Wirklinie verbindet sie.
#[test]
fn leistungskontakt_dreipolig() {
    let Some(lib) = bibliothek() else { return };
    let def = lib.iter().find(|d| d.name == "Leistungskontakt").expect("Leistungskontakt");
    assert_eq!(def.options, ["1 polig", "2 polig", "3 polig"]);
    let lagen = |opt| {
        let mut v: Vec<(i32, i32)> = symbols::anschluss_lagen(def, opt, 0).into_iter().map(|(_, q)| ((q.x * 10.0).round() as i32, (q.y * 10.0).round() as i32)).collect();
        v.sort();
        v
    };
    assert_eq!(lagen(0), [(0, -68), (0, 60)]);
    assert_eq!(lagen(1), [(-35, -68), (-35, 60), (35, -68), (35, 60)]);
    assert_eq!(lagen(2), [(-70, -68), (-70, 60), (0, -68), (0, 60), (70, -68), (70, 60)]);
    // Schaltstücke (schräge Linien von (−1,5/−3,5) nach (0/2,0) relativ zum Pol)
    let stuecke = |opt: u16| {
        let mut v: Vec<i32> = (0..def.prims.len())
            .filter(|i| symbols::visible(opt, def.vis.get(*i)) && symbols::flag_visible(0, def.flag.get(*i)))
            .filter_map(|i| match &*symbols::prim_lage(def, i, opt) {
                Prim::Line(a, b, _) if (b.x - a.x - 1.5).abs() < 0.01 && (a.y + 3.5).abs() < 0.01 && (b.y - 2.0).abs() < 0.01 => Some((b.x * 10.0).round() as i32),
                _ => None,
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(stuecke(0), [0]);
    assert_eq!(stuecke(1), [-35, 35]);
    assert_eq!(stuecke(2), [-70, 0, 70]);
    // Anschlusstexte „1“…„6“ wandern mit ihren Polen
    let text_x = |opt: u16, t: &str| {
        (0..def.prims.len()).filter(|i| symbols::visible(opt, def.vis.get(*i))).find_map(|i| match &*symbols::prim_lage(def, i, opt) {
            Prim::TextBox(a, _, s, _) if s == t => Some((a.x * 10.0).round() as i32),
            _ => None,
        })
    };
    assert_eq!(text_x(2, "1"), Some(-19 - 70));
    // „5“/„6“ gehören zum dritten Pol (nur Option 2, ohne Versatz)
    assert_eq!(text_x(2, "6"), Some(52));
    // Umriss über alle drei Pole; Titelanker rückt mit
    let (mn, mx) = symbols::bounds_opt(def, 2);
    assert!(mn.x <= -8.5 && mx.x >= 7.0, "{mn:?}–{mx:?}");
    assert_eq!(symbols::title_lage(def, 2).map(|t| (t.x * 10.0).round() as i32), Some(-67 - 70));

    // auf dem Blatt: alle sechs Anschlüsse gefangen und mit den Bibliotheksnummern angeschlossen
    let mut project = Project::new_default();
    project.symdefs.push(def.clone());
    let o = Obj { id: 1, kind: Kind::Symbol { key: def.key.clone(), pos: P::new(100.0, 100.0), rot: 0, mirror_h: false, mirror_v: false, bmk: "-Q1".into(), value: String::new(), scale: 1.0, option: 2, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None };
    let pins = canvas::obj_pins(&o, &project.symdefs);
    assert_eq!(pins.len(), 6);
    assert!(pins.iter().any(|q| q.dist(P::new(93.0, 93.2)) < 0.01) && pins.iter().any(|q| q.dist(P::new(107.0, 106.0)) < 0.01));
    let mut sheet = Sheet::new(1);
    sheet.objects.push(o);
    let mut nrn = vec![];
    for q in &pins {
        let e = vbg_enden::bestimmen(&project, &sheet, 99, *q, true).expect("Anschluss getroffen");
        assert_eq!(e.art, vbg_enden::ART_SYMBOL);
        nrn.push(e.nr);
    }
    nrn.sort();
    // Feld 4 der Anschlusselemente (Hilfskontakt 8/9 nur mit Zusatzoption)
    assert_eq!(nrn, [2, 3, 4, 5, 6, 7]);
}

/// Sicherung 3-polig (Bibliothek): alle Anschlüsse der höchsten Option liegen auf verschiedenen Polen, die Pole
/// stehen gleichmäßig um den Bezugspunkt.
#[test]
fn sicherung_mehrpolig() {
    let Some(lib) = bibliothek() else { return };
    let def = lib.iter().find(|d| d.name == "Sicherung").expect("Sicherung");
    for opt in 0..def.options.len() as u16 {
        let v = symbols::anschluss_lagen(def, opt, 0);
        let mut xs: Vec<i32> = v.iter().map(|(_, q)| (q.x * 10.0).round() as i32).collect();
        xs.sort();
        xs.dedup();
        println!("Sicherung {:?}: Pole bei {xs:?}", def.options.get(opt as usize));
        // eine Polreihe je Option, symmetrisch zum Bezugspunkt
        assert_eq!(xs.len(), opt as usize + 1, "{:?}", def.options);
        assert_eq!(xs.first().map(|x| -x), xs.last().copied());
    }
}

/// Bildexport: Leistungskontakt 3-polig zeigt alle drei Pole (Anschlusslinien bei x = 93/100/107 mm), 1-polig nur
/// den mittleren.
#[test]
fn leistungskontakt_dreipolig_bild() {
    let Some(lib) = bibliothek() else { return };
    let def = lib.iter().find(|d| d.name == "Leistungskontakt").expect("Leistungskontakt").clone();
    let bild = |option: u16| {
        let mut p = Project::new_default();
        p.symdefs.push(def.clone());
        let kind = Kind::Symbol { key: def.key.clone(), pos: P::new(100.0, 100.0), rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option, title: None, flags: 0, ext: Default::default() };
        p.anlagen[0].sheets[0].objects.push(Obj { id: 1, kind, pen: 0, fixed: false, group: None, raw: None });
        elektroplanung::bildexport::render_headless(&p, &[def.clone()], SheetRef { anlage: 0, sheet: 0 }, 150, true).unwrap()
    };
    let k = 150.0 / 25.4;
    // dunkle Pixel auf der oberen Anschlusslinie (y 93,5–96,5 mm) in einer Spalte um x mm
    let pol = |x: f32| ((x - 0.3) * k) as u32..=((x + 0.3) * k) as u32;
    let strich = |img: &image::RgbImage, x: f32| pol(x).any(|px| ((93.5 * k) as u32..(96.5 * k) as u32).all(|py| img.get_pixel(px, py).0[0] < 160));
    let drei = bild(2);
    assert!(strich(&drei, 93.0) && strich(&drei, 100.0) && strich(&drei, 107.0));
    assert!(!strich(&drei, 96.5) && !strich(&drei, 103.5));
    let eins = bild(0);
    assert!(strich(&eins, 100.0) && !strich(&eins, 93.0) && !strich(&eins, 107.0));
}

/// Geänderte Symbole schreibt der Nachbau neu (epr_write): der Versatz je Option bleibt in Feld 13/14 erhalten.
#[test]
fn versatz_beim_neu_schreiben() {
    let Some(lib) = bibliothek() else { return };
    let mut d = lib.iter().find(|d| d.name == "Leistungskontakt").expect("Leistungskontakt").clone();
    assert!(!d.versatz.ist_leer());
    d.raw.clear();
    let (els, texts, _) = elektroplanung::epr_write::sym_elements(&d);
    let el = symbols::parse_elements(&els, &texts);
    assert_eq!(el.versatz, d.versatz);
}

/// Variantenrad (Bildauswahl beim Absetzen): das Bild „3 polig“ zeigt alle drei Pole und wird um seine Mitte
/// zentriert (Umriss über −7…+7 mm statt nur über den ersten Pol).
#[test]
fn variantenrad_mehrpolig() {
    let Some(lib) = bibliothek() else { return };
    let def = lib.iter().find(|d| d.name == "Leistungskontakt").expect("Leistungskontakt");
    let (a1, b1) = elektroplanung::variantenrad::umriss(def, 0);
    let (a3, b3) = elektroplanung::variantenrad::umriss(def, 2);
    assert!(b3.x - a3.x >= b1.x - a1.x + 13.9, "{a1:?}–{b1:?} / {a3:?}–{b3:?}");
    assert!(a3.x <= -8.5 && b3.x >= 7.0);
    // zentriert: ohne den Hilfskontakt (Zusatzoption) rechts
    assert!((a3.x + b3.x).abs() < 3.0, "{a3:?}–{b3:?}");
}
