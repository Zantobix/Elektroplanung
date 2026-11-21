//! Bauteile, Platzhalter, Verbindungsoptionen, Schienen-Schalter, Rackposition, Projektstatus, Schriftfeld und
//! Projektgrafiken im Originalformat (src/epr_zusatz.rs): an W2.epr lesen, ändern, speichern, wieder lesen.

use elektroplanung::{epr, epr_projekt, epr_write, eprfile::EprFile, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn rundreise(p: &Project, name: &str) -> (Project, Vec<u8>) {
    let out = std::env::temp_dir().join(name);
    epr_write::save(p, &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    (epr::import(&out).unwrap(), bytes)
}

fn objekte(p: &Project) -> impl Iterator<Item = &Obj> {
    p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects)
}

#[test]
fn liest_w2_wie_vom_original_gespeichert() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // Platzhaltergruppe "SPS" ohne Einträge (ERS), keine Bauteile
    assert_eq!(p.platzhalter, vec![Platzhalter { name: "SPS".into(), werte: vec![] }]);
    assert!(p.bauteile.is_empty() && p.part_of.is_empty());
    // Projekt in Bearbeitung, neuer Klemmenplan, Schriftfeld relativ
    assert!(!p.abgeschlossen && !p.alter_klemmenplan && !p.tb_absolut);
    // 35 Verbindungen sind Brücken (VBG Feld 14 Bit 0x100), keine Rackpositionen
    let bruecken = objekte(&p).filter(|o| matches!(o.kind, Kind::Wire { opts, .. } if opts & WIRE_BRUECKE != 0)).count();
    assert_eq!(bruecken, 35);
    assert!(objekte(&p).all(|o| !matches!(&o.kind, Kind::Symbol { ext, .. } if ext.rack != -1)));
    // Schienen: alle mit Querverweisen, ein Teil mit Pfeilen
    let s: Vec<&SchieneSet> = p.anlagen.iter().flat_map(|a| a.schienen.values()).collect();
    assert!(s.iter().all(|s| s.quer));
    assert!(s.iter().any(|s| s.pfeile) && s.iter().any(|s| !s.pfeile));
    assert!(epr_projekt::nicht_gespeichert(&p).is_empty(), "{:?}", epr_projekt::nicht_gespeichert(&p));
    // unverändert gespeichert: byteidentisch
    assert_eq!(epr_write::to_bytes(&p).unwrap(), std::fs::read(&path).unwrap());
}

#[test]
fn aenderungen_kommen_an() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    // Bauteil samt Zuordnung zum ersten Symbol
    let sym = objekte(&p).find(|o| matches!(o.kind, Kind::Symbol { .. })).unwrap().id;
    p.bauteile.push(Bauteil { gruppe: "Projekt".into(), name: "Schütz K1".into(), typ: "DILM12".into(), artikel: "276830".into(), breite: "45".into(), hoehe: "68".into(), tiefe: "75".into(), nummer: "4711".into(), ..Default::default() });
    p.part_of.insert(sym, 0);
    // Platzhalter
    p.platzhalter[0].werte = vec![("E1".into(), "Eingang 1".into()), ("A1".into(), "Ausgang 1".into())];
    p.platzhalter.push(Platzhalter { name: "Raum".into(), werte: vec![("R".into(), "Küche".into())] });
    // Verbindungsoptionen der ersten Verbindung mit Bezeichnung, Brücke der ersten Brücke weg
    let mut wire_ids = (0, 0);
    let mut rack_id = 0;
    for o in p.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()) {
        match &mut o.kind {
            Kind::Wire { opts, bez, .. } if wire_ids.0 == 0 && !bez.is_empty() && *opts == 0 => {
                *opts = WIRE_ZIEL_A | WIRE_HOCHKANT | WIRE_MARKIERUNG | WIRE_DURCH;
                wire_ids.0 = o.id;
            }
            Kind::Wire { opts, .. } if wire_ids.1 == 0 && *opts & WIRE_BRUECKE != 0 => {
                *opts &= !WIRE_BRUECKE;
                wire_ids.1 = o.id;
            }
            Kind::Symbol { ext, .. } if rack_id == 0 => {
                ext.rack = 7;
                rack_id = o.id;
            }
            _ => {}
        }
    }
    assert!(wire_ids.0 != 0 && wire_ids.1 != 0);
    // Schienen-Schalter
    let a = p.anlagen.iter().position(|a| a.schienen.contains_key("PE")).unwrap();
    {
        let s = p.anlagen[a].schienen.get_mut("PE").unwrap();
        s.quer = false;
        s.pfeile = !s.pfeile;
        s.klemmen = true;
        s.schleife = true;
        s.autoende = true;
    }
    let pe = p.anlagen[a].schienen["PE"].clone();
    // Projektebene, Schriftfeld, Projektgrafik
    p.abgeschlossen = true;
    p.alter_klemmenplan = true;
    p.tb_absolut = true;
    p.bilder[0].name = "Umbenannt".into();
    // Gruppe „Projekt“ gibt es im Original nicht (gespeichert wird 0 = „Keine Zuordnung“)
    let w = epr_projekt::nicht_gespeichert(&p);
    assert!(w.len() == 1 && w[0].contains("Keine Zuordnung"), "{w:?}");

    let (b, bytes) = rundreise(&p, "ep_zusatz_w2.epr");
    assert_eq!(b.bauteile, p.bauteile);
    let i = b.part_of.values().copied().collect::<Vec<_>>();
    assert_eq!(i, vec![0]);
    assert_eq!(b.platzhalter, p.platzhalter);
    // Objekte behalten ihre Reihenfolge: Zuordnung über die Lage in der Objektliste
    let pos = |p: &Project, id: u64| objekte(p).position(|o| o.id == id).unwrap();
    let back_obj = |n: usize| objekte(&b).nth(n).unwrap();
    assert_eq!(b.part_of.keys().map(|k| pos(&b, *k)).collect::<Vec<_>>(), vec![pos(&p, sym)]);
    assert!(matches!(back_obj(pos(&p, wire_ids.0)).kind, Kind::Wire { opts, .. } if opts == WIRE_ZIEL_A | WIRE_HOCHKANT | WIRE_MARKIERUNG | WIRE_DURCH));
    assert!(matches!(back_obj(pos(&p, wire_ids.1)).kind, Kind::Wire { opts, .. } if opts & WIRE_BRUECKE == 0));
    assert!(matches!(&back_obj(pos(&p, rack_id)).kind, Kind::Symbol { ext, .. } if ext.rack == 7));
    let s = &b.anlagen[a].schienen["PE"];
    assert_eq!((s.quer, s.pfeile, s.klemmen, s.schleife, s.autoende), (pe.quer, pe.pfeile, pe.klemmen, pe.schleife, pe.autoende));
    assert!(b.abgeschlossen && b.alter_klemmenplan && b.tb_absolut);
    assert_eq!(b.bilder[0].name, "Umbenannt");
    assert_eq!(b.bilder[0].jpeg, p.bilder[0].jpeg);

    // Ablage wie im Original: BTL 232 Byte vor ERS, IDL im VSY mit der Kennung des Bauteils
    let f = EprFile::parse(&bytes).unwrap();
    let prj = f.root("PRJ").unwrap();
    let btl = prj.kid("BTL").unwrap();
    assert_eq!(btl.data.len(), 0xe8);
    let at = |t: &str| prj.kids.iter().position(|k| k.is(t)).unwrap();
    assert_eq!(at("BTL") + 1, at("ERS"));
    let idl = prj.kids_of("ANL").flat_map(|a| a.kids_of("BLT")).flat_map(|s| s.kids_of("VSY")).find_map(|v| v.kid("IDL")).unwrap();
    assert_eq!(idl.data, btl.data[..8].to_vec());
    assert_eq!(prj.data[0x90], 1);
    assert_eq!(prj.data[0x70] & 1, 0);
    assert_eq!(prj.kid("FUS").unwrap().i32(1), 0);

    // erneut unverändert gespeichert: byteidentisch
    assert_eq!(epr_write::to_bytes(&b).unwrap(), bytes);

    // zurück auf die Ausgangswerte: wieder die Originaldatei bis auf Bauteil/Platzhalter
    let mut c = b.clone();
    c.bauteile.clear();
    c.part_of.clear();
    c.platzhalter.truncate(1);
    c.platzhalter[0].werte.clear();
    c.abgeschlossen = false;
    c.alter_klemmenplan = false;
    c.tb_absolut = false;
    c.bilder[0].name = epr::import(&path).unwrap().bilder[0].name.clone();
    let orig = epr::import(&path).unwrap();
    for (x, y) in c.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).zip(objekte(&orig)) {
        x.kind = y.kind.clone();
    }
    // relativ → absolut → relativ rundet (wie im Original): Schriftfeld der Ausgangsdatei
    c.titleblock = orig.titleblock.clone();
    for (x, y) in c.anlagen.iter_mut().zip(&orig.anlagen) {
        x.schienen = y.schienen.clone();
    }
    let (_, back) = rundreise(&c, "ep_zusatz_w2_zurueck.epr");
    let o = EprFile::parse(&std::fs::read(&path).unwrap()).unwrap();
    let n = EprFile::parse(&back).unwrap();
    // wie im Original bis auf das Änderungsdatum der zwischendurch geänderten Blätter (BLT Byte 120–128)
    let strip = |f: &EprFile| {
        let mut f = f.clone();
        for s in f.roots[0].kids.iter_mut().filter(|k| k.is("ANL")).flat_map(|a| a.kids.iter_mut()).filter(|k| k.is("BLT")) {
            s.data[120..128].fill(0);
        }
        f.to_bytes()
    };
    assert_eq!(strip(&n), strip(&o));
}

#[test]
fn projektgrafik_ersetzen_und_loeschen() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    assert!(!p.bilder.is_empty());
    let img = image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([x as u8 * 4, y as u8 * 5, 90]));
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85).encode_image(&img).unwrap();
    p.bilder[0].jpeg = jpeg.clone();
    let (b, _) = rundreise(&p, "ep_zusatz_bild.epr");
    assert_eq!(b.bilder[0].jpeg, jpeg);
    assert_eq!(b.bilder[0].nr, p.bilder[0].nr);
    let mut c = b.clone();
    c.bilder.clear();
    let (d, bytes) = rundreise(&c, "ep_zusatz_bild2.epr");
    assert!(d.bilder.is_empty());
    assert!(EprFile::parse(&bytes).unwrap().root("PRJ").unwrap().kid("BLD").is_none());
}

#[test]
fn neues_projekt_mit_bauteil_und_platzhalter() {
    let mut p = Project::new_default();
    p.bauteile.push(Bauteil { gruppe: "Projekt".into(), name: "LS B16".into(), breite: "18".into(), hoehe: "90".into(), tiefe: "70".into(), ..Default::default() });
    p.platzhalter.push(Platzhalter { name: "Haus".into(), werte: vec![("S".into(), "Straße".into())] });
    p.abgeschlossen = true;
    let (b, _) = rundreise(&p, "ep_zusatz_neu.epr");
    assert_eq!(b.bauteile, p.bauteile);
    assert_eq!(b.platzhalter, p.platzhalter);
    assert!(b.abgeschlossen);
}

fn fixture(n: &str) -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("EP_FIXTURES").ok()?).join(n);
    p.exists().then_some(p)
}

/// Koordinaten der Schriftfeld-Elemente (GRS, je 0x50 Byte: Feld 0–3) und FUS Feld 1.
fn schriftfeld_roh(bytes: &[u8]) -> (i32, Vec<[i32; 4]>) {
    let f = EprFile::parse(bytes).unwrap();
    let fus = f.root("PRJ").unwrap().kid("FUS").unwrap().clone();
    let v = fus.kids_of("GRS").flat_map(|g| g.data.chunks_exact(0x50).map(|e| std::array::from_fn(|i| elektroplanung::eprfile::i32_at(e, i * 4))).collect::<Vec<_>>()).collect();
    (fus.i32(1), v)
}

#[test]
fn schriftfeld_absolut_wie_im_original() {
    // x_schriftfeld_absolut.epr: W2, im Original auf Maßangaben „Absolut“ umgeschaltet und gespeichert
    let (Some(path), Some(fx)) = (sample(), fixture("x_schriftfeld_absolut.epr")) else { return };
    let orig = std::fs::read(&fx).unwrap();
    let soll = schriftfeld_roh(&orig);
    assert_eq!(soll.0, 0);
    // Umschalten im Nachbau: dieselben Koordinaten wie das Original
    let mut p = epr::import(&path).unwrap();
    p.tb_absolut = true;
    let (_, bytes) = rundreise(&p, "ep_zusatz_sf_abs.epr");
    assert_eq!(schriftfeld_roh(&bytes), soll);
    // die Datei des Originals: absolut gelesen, als dasselbe relative Schriftfeld wie W2, unverändert byteidentisch
    let q = epr::import(&fx).unwrap();
    assert!(q.tb_absolut);
    assert_eq!(q.titleblock.len(), p.titleblock.len());
    for (a, b) in q.titleblock.iter().zip(&p.titleblock) {
        let k = |e: &elektroplanung::titleblock::TbOwned| match e {
            elektroplanung::titleblock::TbOwned::Line(a, b, c, d) | elektroplanung::titleblock::TbOwned::Logo(a, b, c, d) | elektroplanung::titleblock::TbOwned::Text(a, b, c, d, ..) => [*a, *b, *c, *d],
        };
        // Rundung der Umrechnung: höchstens 0,05 mm in x (17 Einheiten), 0,05 mm in y (34 Einheiten)
        assert!(k(a).iter().zip(k(b)).all(|(x, y)| (x - y).abs() <= 34), "{a:?} {b:?}");
    }
    assert_eq!(epr_write::to_bytes(&q).unwrap(), orig);
    // zurück auf relativ: Koordinaten wie W2 bis auf die Rundung, FUS Feld 1 = 2
    let mut r = q.clone();
    r.tb_absolut = false;
    let (_, back) = rundreise(&r, "ep_zusatz_sf_rel.epr");
    let w2 = schriftfeld_roh(&std::fs::read(&path).unwrap());
    let rel = schriftfeld_roh(&back);
    assert_eq!(rel.0, 2);
    assert_eq!(rel.1.len(), w2.1.len());
    assert!(rel.1.iter().zip(&w2.1).all(|(a, b)| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 34)));
}
