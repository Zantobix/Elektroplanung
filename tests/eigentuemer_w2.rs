//! Eigentümer (VSY › OWR) und Lage der Kontaktspiegel gegen das Beispielprojekt (W2.epr) und den Original-Ausdruck
//! "Elektroplanung Wohnung 2.pdf" (Seiten 10, 18, 29: Schließer mit „-K1“ und „/5“, „/4“, „/5“ darunter).
//! Ohne EP_SAMPLE werden die Tests übersprungen.

use elektroplanung::{berechnet, canvas::bmk_ctx, eigentuemer, epr, epr_write, eprfile::EprFile, klemmplan, kontaktspiegel, model::*, suche, symbols};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

/// Alle Symbole mit Eigentümer: (Anlage, eigene Vorlage, gezeigte Kennzeichnung, Querverweis auf dem eigenen Blatt).
fn mit_eigentuemer(p: &Project) -> Vec<(String, String, String, String)> {
    let mut v = vec![];
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            for o in &s.objects {
                if let Kind::Symbol { bmk, ext, .. } = &o.kind {
                    if ext.eigentuemer != 0 {
                        let hier = SheetRef { anlage: ai, sheet: si };
                        v.push((a.name.clone(), bmk.clone(), eigentuemer::anzeige_bmk(p, o, &bmk_ctx(a, s)), eigentuemer::querverweis(p, o, Some(hier)).unwrap_or_default()));
                    }
                }
            }
        }
    }
    v
}

#[test]
fn kontakte_mit_eigentuemer_wie_im_ausdruck() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let v = mit_eigentuemer(&p);
    assert_eq!(v.len(), 3, "{v:#?}");
    // gezeigt wird die Kennzeichnung der Spule, darunter ihr Pfad auf demselben Blatt (Ausdruck: /5, /4, /5)
    let mut qv: Vec<&str> = v.iter().map(|x| x.3.as_str()).collect();
    qv.sort();
    assert_eq!(qv, ["/4", "/5", "/5"]);
    // Kennzeichnung der Spule statt der eigenen Vorlage „-Q1+%P“ (UV3, Seite 29: „-K1-X17.100“)
    let mut k: Vec<&str> = v.iter().map(|x| x.2.as_str()).collect();
    k.sort();
    assert_eq!(k, ["-K1", "-K1", "-K1-X17.100"]);
    assert!(v.iter().all(|x| x.1 == "-Q1+%P"), "{v:#?}");
}

#[test]
fn owr_rundreise() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let ai = p.anlagen.iter().position(|a| a.name == "Unterverteilung 1").unwrap();
    // Spule -K1 und ihr Kontakt mit OWR (Blatt „Sicherung -F7“)
    let (si, kontakt, spule) = p.anlagen[ai]
        .sheets
        .iter()
        .enumerate()
        .find_map(|(si, s)| s.objects.iter().find_map(|o| if let Kind::Symbol { ext, .. } = &o.kind { (ext.eigentuemer != 0).then_some((si, o.id, ext.eigentuemer)) } else { None }))
        .unwrap();
    // ein anderes Symbol desselben Blattes bekommt die Spule als Eigentümer, der vorhandene Kontakt verliert ihn
    let anderes = p.anlagen[ai].sheets[si].objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { ext, .. } if ext.eigentuemer == 0) && o.id != spule).map(|o| o.id).unwrap();
    assert!(kontaktspiegel::eigentuemer_setzen(&mut p, spule, &[anderes]));
    for o in p.anlagen[ai].sheets[si].objects.iter_mut().filter(|o| o.id == kontakt) {
        if let Kind::Symbol { ext, .. } = &mut o.kind {
            ext.eigentuemer = 0;
        }
    }
    let tmp = std::env::temp_dir().join("ep_owr_rundreise.epr");
    epr_write::save(&p, &tmp).unwrap();
    let back = epr::import(&tmp).unwrap();
    let v = mit_eigentuemer(&back);
    assert_eq!(v.len(), 3, "{v:#?}");
    assert!(v.iter().all(|x| x.2.starts_with("-K1")), "{v:#?}");
    // der neue Kontakt liegt auf demselben Blatt wie die Spule: „/Pfad“
    let s_back = &back.anlagen[ai].sheets[si];
    let neu: Vec<&Obj> = s_back.objects.iter().filter(|o| matches!(&o.kind, Kind::Symbol { ext, .. } if ext.eigentuemer != 0)).collect();
    assert_eq!(neu.len(), 1);
    // OWR wie im Original: Kennung der Spule, ihres Blattes und ihrer Anlage
    let f = EprFile::read(&tmp).unwrap();
    let mut owr = vec![];
    fn walk<'a>(b: &'a elektroplanung::eprfile::Block, v: &mut Vec<&'a elektroplanung::eprfile::Block>) {
        if b.is("VSY") && b.kid("OWR").is_some() {
            v.push(b);
        }
        for k in &b.kids {
            walk(k, v);
        }
    }
    for r in &f.roots {
        walk(r, &mut owr);
    }
    assert_eq!(owr.len(), 3);
    let raw_neu = neu[0].raw.as_deref().unwrap();
    let w = raw_neu.block.kid("OWR").unwrap();
    assert_eq!(w.data.len(), eigentuemer::OWR_LEN);
    assert!(raw_neu.block.kids.last().unwrap().is("OWR"), "OWR ist der letzte Unterblock");
    let (rs, so) = back.find_obj(if let Kind::Symbol { ext, .. } = &neu[0].kind { ext.eigentuemer } else { 0 }).unwrap();
    let id = |d: &[u8], at: usize| u64::from_le_bytes(d[at..at + 8].try_into().unwrap());
    assert_eq!(id(&w.data, 0), id(&so.raw.as_deref().unwrap().block.data, 96));
    assert_eq!(id(&w.data, 8), id(&back.sheet(rs).unwrap().raw.as_deref().unwrap().block.data, epr::BLT_ID_AT));
    assert_eq!(id(&w.data, 16), id(&back.anlagen[rs.anlage].raw.as_deref().unwrap().0.data, epr::ANL_ID_AT));
    // unverändert gelesen und geschrieben: byteidentisch (OWR bleibt verknüpft, aber unverändert)
    let tmp2 = std::env::temp_dir().join("ep_owr_rundreise2.epr");
    epr_write::save(&back, &tmp2).unwrap();
    assert_eq!(std::fs::read(&tmp).unwrap(), std::fs::read(&tmp2).unwrap());
}

/// Das Original setzt BLT Feld 24 beim Zeichnen auf Unterkante der Zeichenfläche − größte Kontaktspiegel-Höhe:
/// in W2 1900 bzw. hoch 2770, auf dem Blatt mit dem leeren Spiegel 1870 = 1900 − 30.
#[test]
fn spiegel_lage_wie_blt_feld_24() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let mut mit_spiegel = 0;
    for s in p.anlagen.iter().flat_map(|a| &a.sheets) {
        let f24 = s.raw.as_deref().unwrap().block.i32(24) as f32 / 10.0;
        assert_eq!(kontaktspiegel::start_y(&p, s), f24, "Blatt {}", s.name);
        for o in &s.objects {
            if let Kind::Spiegel { pos, .. } = &o.kind {
                assert_eq!(pos.y, 187.0);
                mit_spiegel += 1;
            }
        }
    }
    assert_eq!(mit_spiegel, 1);
}

/// Querverweisliste wie im Original (Aufnahme „Gegenprobe Querverweise“ der Unterverteilung 1: „-K1“ mit „6/5 6/6“):
/// Kontakte mit Eigentümer stehen mit ihrer eigenen Fundstelle unter der Kennzeichnung der Spule, ihre eigene
/// Vorlage („-Q1+%P“) erscheint nicht.
#[test]
fn querverweisliste_unter_der_spule() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    for a in &p.anlagen {
        let kontakte: Vec<(String, String, String)> = a
            .sheets
            .iter()
            .filter(|s| s.blattart() == 3)
            .flat_map(|s| s.objects.iter().map(move |o| (s, o)))
            .filter_map(|(s, o)| match &o.kind {
                Kind::Symbol { bmk, ext, pos, .. } if ext.eigentuemer != 0 => {
                    Some((eigentuemer::kennzeichnung(&p, o).unwrap(), symbols::display_bmk(bmk, ext.counter, &bmk_ctx(a, s)), berechnet::blatt_pfad(s, pos.x)))
                }
                _ => None,
            })
            .collect();
        if kontakte.is_empty() {
            continue;
        }
        let q = berechnet::querverweise(&p, a);
        for (spule, eigene, lage) in &kontakte {
            let zeile = q.iter().find(|r| r[0] == *spule).unwrap_or_else(|| panic!("{}: {spule} fehlt in {q:#?}", a.name));
            assert!(zeile[1].split(' ').any(|x| x == lage), "{}: {zeile:?} ohne {lage}", a.name);
            assert!(!q.iter().any(|r| r[0] == *eigene), "{}: eigene Kennzeichnung {eigene} gelistet", a.name);
        }
        if a.name == "Unterverteilung 1" {
            assert_eq!(q.iter().find(|r| r[0] == "-K1").map(|r| r[1].as_str()), Some("6/5 6/6"));
        }
    }
}

/// Alle Ziele aller Klemmpläne des Projekts (Anlage, Leiste, Klemme, Kennung).
fn klemmziele(p: &Project) -> Vec<(String, String, String, String)> {
    let mut v = vec![];
    for a in &p.anlagen {
        for k in &a.klemmen {
            for z in klemmplan::plan_projekt(p, a, &k.name, &p.symdefs).zeilen {
                for ziel in [&z.von, &z.nach].into_iter().flatten() {
                    v.push((a.name.clone(), k.name.clone(), z.label.clone(), ziel.kennung.clone()));
                }
            }
        }
    }
    v
}

/// Klemmplan, Querverweisliste und Suche folgen derselben Quelle (`SymExt::eigentuemer`): auch ein im Nachbau
/// gesetzter bzw. entfernter Eigentümer wirkt, nicht nur das aus der Datei gelesene OWR.
#[test]
fn neu_gesetzter_eigentuemer_ueberall() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let vorher = klemmziele(&p);
    // ein Symbol, das im Klemmplan als Ziel erscheint und keinen Eigentümer hat
    let (ai, ziel_id, alt) = p
        .anlagen
        .iter()
        .enumerate()
        .find_map(|(ai, a)| {
            a.sheets.iter().filter(|s| s.blattart() == 3).find_map(|s| {
                s.objects.iter().find_map(|o| match &o.kind {
                    Kind::Symbol { bmk, ext, .. } if ext.eigentuemer == 0 && !bmk.is_empty() => {
                        let b = symbols::display_bmk(bmk, ext.counter, &bmk_ctx(a, s));
                        let n = a.sheets.iter().flat_map(|s2| s2.objects.iter().map(move |o2| (s2, o2))).filter(|(s2, o2)| matches!(&o2.kind, Kind::Symbol { bmk: b2, ext: e2, .. } if symbols::display_bmk(b2, e2.counter, &bmk_ctx(a, s2)) == b)).count();
                        (n == 1 && vorher.iter().any(|x| x.0 == a.name && x.3 == b)).then_some((ai, o.id, b))
                    }
                    _ => None,
                })
            })
        })
        .expect("Symbol am Klemmplan");
    // Spule -K1 der Anlage (Eigentümer eines Kontakts aus W2)
    let spule = p.anlagen[ai].sheets.iter().flat_map(|s| &s.objects).find_map(|o| match &o.kind {
        Kind::Symbol { ext, .. } if ext.eigentuemer != 0 => Some(ext.eigentuemer),
        _ => None,
    });
    let spule = spule.unwrap_or_else(|| p.anlagen[ai].sheets.iter().flat_map(|s| &s.objects).find(|o| matches!(o.kind, Kind::Symbol { .. }) && o.id != ziel_id).unwrap().id);
    let spule_bmk = {
        let (r, o) = p.find_obj(spule).unwrap();
        let a = &p.anlagen[r.anlage];
        eigentuemer::anzeige_bmk(&p, o, &bmk_ctx(a, &a.sheets[r.sheet]))
    };
    assert!(eigentuemer::setzen(&mut p, spule, &[ziel_id]));
    let nachher = klemmziele(&p);
    assert_eq!(vorher.len(), nachher.len());
    let name = p.anlagen[ai].name.clone();
    let geaendert: Vec<_> = vorher.iter().zip(&nachher).filter(|(x, y)| x != y).collect();
    assert!(!geaendert.is_empty(), "{alt}: Klemmplan unverändert");
    for (x, y) in &geaendert {
        assert_eq!((x.0.as_str(), x.3.as_str(), y.3.as_str()), (name.as_str(), alt.as_str(), spule_bmk.as_str()));
    }
    assert!(!nachher.iter().any(|x| x.0 == name && x.3 == alt), "{alt} noch im Klemmplan");
    // Querverweisliste und Suche zeigen die Kennzeichnung der Spule
    let q = berechnet::querverweise(&p, &p.anlagen[ai]);
    assert!(!q.iter().any(|r| r[0] == alt), "{alt} noch in der Querverweisliste");
    let (r, o) = p.find_obj(ziel_id).unwrap();
    let a = &p.anlagen[r.anlage];
    let texte = suche::objekt_texte(&p, a, &a.sheets[r.sheet], o);
    assert!(texte.iter().any(|(k, t)| *k == "BMK" && *t == spule_bmk), "{texte:?}");
    // Eigentümer wieder entfernt: alles wie vorher, auch ohne Änderung am gelesenen OWR
    for o in p.anlagen.iter_mut().flat_map(|a| &mut a.sheets).flat_map(|s| &mut s.objects).filter(|o| o.id == ziel_id) {
        if let Kind::Symbol { ext, .. } = &mut o.kind {
            ext.eigentuemer = 0;
        }
    }
    assert_eq!(klemmziele(&p), vorher);
}
