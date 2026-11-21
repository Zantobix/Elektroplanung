//! Tabelle (ZCN Typ 13) wie im Original: neue Tabelle mit den Vorgaben des Originals, Änderungen über den
//! Dialog TSTab (Liste laden, Zeile löschen, Spalten/Zeilen, Überschrift, Autobreite, Ab) und die Rundreise
//! durch das .epr – im neuen Projekt und in einer Kopie von W2. Dazu die Aufteilung der W2-Tabellen.

use elektroplanung::{epr, epr_write, eprfile::EprFile, model::*, tabelle};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn reload(p: &Project) -> (Project, EprFile) {
    let bytes = epr_write::to_bytes(p).unwrap();
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    (epr::import_file(&f).unwrap(), f)
}

/// Tabelle mit auf rows × cols ergänzten Zellen (die Datei lässt leere Zellen am Ende weg) und auf 0,1 mm
/// gerundetem Rechteck.
fn norm(k: &Kind) -> Kind {
    let mut k = k.clone();
    if let Kind::Table { a, b, rows, cols, cells, .. } = &mut k {
        let q = |p: &mut P| *p = P::new((p.x * 10.0).round() / 10.0, (p.y * 10.0).round() / 10.0);
        q(a);
        q(b);
        cells.resize(*rows * *cols, String::new());
    }
    k
}

fn tabellen(p: &Project) -> Vec<(usize, usize, u64)> {
    let mut v = vec![];
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                if matches!(o.kind, Kind::Table { .. }) {
                    v.push((ai, si, o.id));
                }
            }
        }
    }
    v
}

fn obj_mut(p: &mut Project, (a, s, id): (usize, usize, u64)) -> &mut Obj {
    p.anlagen[a].sheets[s].objects.iter_mut().find(|o| o.id == id).unwrap()
}

/// Dialog wie bei „OK“ auf das Objekt anwenden (dialoge3.rs, Dlg3::Tab).
fn anwenden(o: &mut Obj, d: &tabelle::TabDlg) {
    let (nr, nc, zellen, kopf, fest, daten, ab_wert) = d.werte();
    o.fixed = d.fixiert;
    if let Kind::Table { rows, cols, cells, header, fixed_w, art, ab, .. } = &mut o.kind {
        (*rows, *cols, *cells, *header, *fixed_w, *art, *ab) = (nr, nc, zellen, kopf, fest, daten, ab_wert);
    }
}

fn dialog(o: &Obj) -> tabelle::TabDlg {
    let Kind::Table { rows, cols, cells, header, fixed_w, art, ab, .. } = &o.kind else { panic!() };
    tabelle::TabDlg::neu(o.id, *rows, *cols, cells, *header, *fixed_w, o.fixed, *art, *ab)
}

/// Dialogänderungen: Liste mit mehr Zeilen laden (erweitern), Zeile löschen, Überschrift an, Autobreite aus, Ab.
fn bearbeiten(o: &mut Obj) {
    let mut d = dialog(o);
    d.spalten = "3".into();
    d.zeilen = "2".into();
    d.groesse_uebernehmen();
    let liste = tabelle::liste_lesen(&tabelle::text_aus_bytes(b"Nr\tName\tOrt\r\n1\tLampe \xc4\tK\xfcche\r\n2\tDose\tBad\r\n3\tSchalter\tFlur\r\n"));
    d.laden(&liste, || true);
    d.sel_row = Some(2);
    tabelle::zeile_loeschen(&mut d.cells, d.sel_row);
    d.header = true;
    d.auto = false;
    d.fixiert = true;
    d.ab = "2".into();
    anwenden(o, &d);
}

fn pruefe_bearbeitet(k: &Kind) {
    let Kind::Table { rows, cols, cells, header, fixed_w, art, ab, .. } = k else { panic!() };
    assert_eq!((*rows, *cols, *header, *fixed_w, *art, *ab), (4, 3, true, true, 0, 2));
    let mut c = cells.clone();
    c.resize(12, String::new());
    assert_eq!(c, ["Nr", "Name", "Ort", "1", "Lampe Ä", "Küche", "3", "Schalter", "Flur", "", "", ""]);
}

#[test]
fn neue_tabelle_rundreise() {
    let mut p = Project::new_default();
    let id = p.alloc_id();
    let k = tabelle::neu(P::new(20.0, 30.0), P::new(110.0, 60.0));
    // Vorgaben des Originals
    assert!(matches!(&k, Kind::Table { rows: 2, cols: 2, header: false, fixed_w: false, art: 0, ab: 0, .. }));
    p.anlagen[0].sheets[0].objects.push(Obj { id, kind: k.clone(), pen: 0, fixed: false, group: None, raw: None });
    let (back, f) = reload(&p);
    let o = back.anlagen[0].sheets[0].objects.iter().find(|o| matches!(o.kind, Kind::Table { .. })).unwrap();
    assert_eq!(norm(&o.kind), norm(&k));
    // ZCN: 2 Spalten, 2 Zeilen, Überschrift 0, Autobreite 1, Art 0
    let zcn = f.root("PRJ").unwrap().kids_of("ANL").next().unwrap().kids_of("BLT").next().unwrap().kids.iter().rfind(|b| b.is("ZCN")).unwrap();
    assert_eq!((zcn.i32(4), zcn.i32(5), zcn.data[0x24], zcn.data[0x25], zcn.data[0x26]), (2, 2, 0, 1, 0));

    // im Dialog ändern, schreiben, lesen
    let mut p2 = back.clone();
    let t = tabellen(&p2)[0];
    bearbeiten(obj_mut(&mut p2, t));
    let (back2, f2) = reload(&p2);
    let o2 = back2.anlagen[0].sheets[0].objects.iter().find(|o| matches!(o.kind, Kind::Table { .. })).unwrap();
    pruefe_bearbeitet(&o2.kind);
    assert!(o2.fixed);
    let zcn = f2.root("PRJ").unwrap().kids_of("ANL").next().unwrap().kids_of("BLT").next().unwrap().kids.iter().rfind(|b| b.is("ZCN")).unwrap();
    assert_eq!((zcn.i32(4), zcn.i32(5), zcn.i32(6), zcn.data[0x24], zcn.data[0x25]), (3, 4, 2, 1, 0));
    // nochmals schreiben: unverändert
    let (back3, _) = reload(&back2);
    assert_eq!(back3.anlagen[0].sheets[0].objects.iter().map(|o| norm(&o.kind)).collect::<Vec<_>>(), back2.anlagen[0].sheets[0].objects.iter().map(|o| norm(&o.kind)).collect::<Vec<_>>());
}

/// W2: jede Tabelle bearbeiten bzw. eine neue anlegen, Rundreise; die übrigen Objekte bleiben unverändert.
#[test]
fn w2_tabellen_bearbeiten_rundreise() {
    let Some(path) = sample() else { return };
    let orig = epr::import(&path).unwrap();
    let mut p = orig.clone();
    let mut ziele = tabellen(&p);
    if ziele.is_empty() {
        let id = p.alloc_id();
        p.anlagen[0].sheets[0].objects.push(Obj { id, kind: tabelle::neu(P::new(30.0, 40.0), P::new(120.0, 70.0)), pen: 0, fixed: false, group: None, raw: None });
        ziele = tabellen(&p);
    }
    let t = ziele[0];
    bearbeiten(obj_mut(&mut p, t));
    let (back, _) = reload(&p);
    let o = back.anlagen[t.0].sheets[t.1].objects.iter().find(|o| o.id == t.2).expect("Tabelle wieder da");
    pruefe_bearbeitet(&o.kind);
    // alle anderen Tabellen wie vorher
    for (a, s, id) in tabellen(&orig).into_iter().filter(|x| *x != t) {
        let vor = orig.anlagen[a].sheets[s].objects.iter().find(|o| o.id == id).unwrap();
        let nach = back.anlagen[a].sheets[s].objects.iter().find(|o| o.id == id).unwrap();
        assert_eq!(norm(&vor.kind), norm(&nach.kind));
    }
}

/// Aufteilung aller W2-Tabellen: Spalten füllen genau die Breite, keine Spalte schmaler als 0, Schrift > 0.
#[test]
fn w2_tabellen_aufteilung() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    for (a, s, id) in tabellen(&p) {
        let o = p.anlagen[a].sheets[s].objects.iter().find(|o| o.id == id).unwrap();
        let Kind::Table { a: p0, b: p1, rows, cols, cells, header, fixed_w, ab, .. } = &o.kind else { continue };
        let (mn, mx) = elektroplanung::canvas::norm_rect(*p0, *p1);
        let t = tabelle::aufteilung(cells, *cols, *rows, *header, !*fixed_w, *ab as usize, mx.x - mn.x, mx.y - mn.y, &|t, f, s| tabelle::textbreite("", t, f, s));
        assert!((t.breiten.iter().sum::<f32>() - (mx.x - mn.x)).abs() < 0.01, "{:?}", t.breiten);
        assert!(t.breiten.iter().all(|w| *w > 0.0), "{:?}", t.breiten);
        assert!(t.schrift > 0.0 && t.schrift <= t.zeilenhoehe);
    }
}
