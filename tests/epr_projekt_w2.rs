//! Projekt-, Anlagen- und Blattebene im Originalformat (src/epr_projekt.rs): Werte aus W2.epr lesen, im Modell
//! ändern, speichern, wieder lesen – die Änderung muss ankommen, alles andere unverändert bleiben.

use elektroplanung::{epr, epr_projekt, epr_write, eprfile::EprFile, model::*, titleblock::TbOwned};
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

#[test]
fn liest_werte_wie_vom_original_gespeichert() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // eigene Stifte (SIT): "Rot" 0,2 mm rot, "Buss" 0,6 mm grau – Nummer 4 und 5
    assert_eq!(p.stifte.len(), 6);
    assert_eq!((p.stifte[4].name.as_str(), p.stifte[4].color, (p.stifte[4].width * 10.0).round()), ("Rot", [255, 0, 0], 2.0));
    assert_eq!((p.stifte[5].name.as_str(), p.stifte[5].color, (p.stifte[5].width * 10.0).round()), ("Buss", [128, 128, 128], 6.0));
    // Leitung "Leitung 1" = Vorgabe des Originals (1,00 mm², 1 Ader); Busleitung 2,5 mm², 3 Adern, Zahlen, Stift "Buss"
    assert_eq!((p.leitungen[0].quer, p.leitungen[0].adern), (1.0, 1));
    let bus = p.leitungen.iter().find(|l| l.style == 5).expect("Leitung mit Stift Buss");
    assert_eq!((bus.quer, bus.adern, bus.kennz, bus.color), (2.5, 3, 1, [0, 0, 255]));
    // Potentialwerte (POT Feld 5/6 in mV/mA)
    assert!(p.pot_werte.values().any(|(u, i)| u == "230" && i == "63"), "{:?}", p.pot_werte);
    // Sammelschiene PE: Potential PE, Stift Strich/Punkt
    let uv = p.anlagen.iter().find(|a| a.schienen.contains_key("PE")).expect("Anlage mit Schiene PE");
    assert_eq!((uv.schienen["PE"].potential.as_str(), uv.schienen["PE"].stift), ("PE", 2));
    // Blätter: Strompfade von der Anlage, keine Pfadbeschreibungen, Schriftfeld 15 mm
    assert!(p.anlagen.iter().flat_map(|a| &a.sheets).all(|s| s.col_auto && s.col_labels.is_empty() && !s.pfad_linien));
    assert_eq!(p.title_h(), 15.0);
    assert!(epr_projekt::nicht_gespeichert(&p).is_empty(), "{:?}", epr_projekt::nicht_gespeichert(&p));
}

#[test]
fn aenderungen_kommen_im_epr_an() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    let orig = p.clone();
    p.owner = "Eigentümer\nZeile 2".into();
    p.kunde = "Kunde K".into();
    p.schrift = "Arial".into();
    p.stifte[4].width = 0.5;
    p.stifte[4].color = [0, 128, 0];
    p.stifte[5].name = "Bus neu".into();
    p.stifte.push(Stift { name: "Dick".into(), style: 1, color: [1, 2, 3], width: 1.0 });
    p.leitungen[0].quer = 2.5;
    p.leitungen[0].adern = 5;
    p.leitungen[0].kennz = 2;
    p.leitungen[0].color = [10, 20, 30];
    p.leitungen[0].style = 6;
    p.leitungen.push(Leitung { name: "NYM 5x1,5".into(), color: [0, 0, 0], width_mm: 0.25, quer: 1.5, adern: 5, kennz: 1, style: 0, id: 0 });
    let neu_ltg = p.leitungen.len() - 1;
    let pot = p.potentiale[0].clone();
    p.pot_werte.insert(pot.clone(), ("400".into(), "16".into()));
    p.potentiale.push("24V".into());
    p.pot_werte.insert("24V".into(), ("24".into(), "0,5".into()));
    p.tb_hoehe = 20.0;
    if let Some(TbOwned::Text(_, _, _, _, t, _, _)) = p.titleblock.iter_mut().find(|e| matches!(e, TbOwned::Text(..))) {
        *t = "Geändertes Feld".into();
    }
    let a = &mut p.anlagen[1];
    a.set.znr = "ZN-4711".into();
    a.sammelschienen.push("L9".into());
    a.schienen.insert("L9".into(), SchieneSet { potential: "PE".into(), stift: 2, ..Default::default() });
    a.klemmleisten.push("-X9".into());
    let s = &mut a.sheets[0];
    s.ort = "Keller".into();
    s.col_auto = false;
    s.columns = 12;
    s.col_labels = vec!["Licht".into(), String::new(), "Steckdosen".into()];
    s.pfad_linien = true;
    let id = 900_000;
    s.objects.push(Obj { id, kind: Kind::Wire { pts: vec![P::new(30.0, 40.0), P::new(90.0, 40.0)], leitung: neu_ltg, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });

    let (b, bytes) = rundreise(&p, "ep_projekt_aenderung.epr");
    assert_eq!((b.owner.as_str(), b.kunde.as_str(), b.schrift.as_str()), ("Eigentümer\nZeile 2", "Kunde K", "Arial"));
    assert_eq!(b.stifte.len(), 7);
    assert_eq!(((b.stifte[4].width * 10.0).round(), b.stifte[4].color), (5.0, [0, 128, 0]));
    assert_eq!(b.stifte[5].name, "Bus neu");
    assert_eq!((b.stifte[6].name.as_str(), b.stifte[6].style, b.stifte[6].color, b.stifte[6].width), ("Dick", 1, [1, 2, 3], 1.0));
    let l = &b.leitungen[0];
    assert_eq!((l.quer, l.adern, l.kennz, l.color, l.style), (2.5, 5, 2, [10, 20, 30], 6));
    let n = &b.leitungen[neu_ltg];
    assert_eq!((n.name.as_str(), n.quer, n.adern, n.kennz), ("NYM 5x1,5", 1.5, 5, 1));
    assert!(n.id != 0 && b.leitungen.iter().filter(|x| x.id == n.id).count() == 1, "eindeutige Kennung der neuen Leitung");
    assert_eq!(b.pot_werte[&pot], ("400".to_string(), "16".to_string()));
    assert_eq!(b.potentiale.last().unwrap(), "24V");
    assert_eq!(b.pot_werte["24V"], ("24".to_string(), "0,5".to_string()));
    assert_eq!(b.title_h(), 20.0);
    assert!(b.titleblock.iter().any(|e| matches!(e, TbOwned::Text(_, _, _, _, t, _, _) if t == "Geändertes Feld")));
    assert_eq!(b.titleblock.len(), p.titleblock.len());
    let a = &b.anlagen[1];
    assert_eq!(a.set.znr, "ZN-4711");
    assert_eq!(a.sammelschienen.last().unwrap(), "L9");
    assert_eq!((a.schienen["L9"].potential.as_str(), a.schienen["L9"].stift), ("PE", 2));
    assert_eq!(a.klemmleisten.last().unwrap(), "-X9");
    let s = &a.sheets[0];
    assert_eq!((s.ort.as_str(), s.col_auto, s.columns, s.pfad_linien), ("Keller", false, 12, true));
    assert_eq!(s.col_labels, vec!["Licht".to_string(), String::new(), "Steckdosen".to_string()]);
    let w = s.objects.iter().find_map(|o| if let Kind::Wire { pts, leitung, .. } = &o.kind { (pts[0] == P::new(30.0, 40.0)).then_some(*leitung) } else { None });
    assert_eq!(w, Some(neu_ltg), "neue Verbindung verweist auf die neue Leitung");
    // Unverändertes bleibt: übrige Anlagen, Symbole, Blätter
    assert_eq!(b.anlagen.len(), orig.anlagen.len());
    assert_eq!(b.anlagen[0].sheets[0].objects.len(), orig.anlagen[0].sheets[0].objects.len());
    assert_eq!(b.symdefs.len(), orig.symdefs.len());
    assert_eq!(b.anlagen[2].sammelschienen, orig.anlagen[2].sammelschienen);
    // Erneutes Speichern des gelesenen Ergebnisses ist stabil
    let (_, again) = rundreise(&b, "ep_projekt_aenderung2.epr");
    assert_eq!(again, bytes, "zweites Speichern muss dieselbe Datei ergeben");
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
}

#[test]
fn entfernte_eintraege_verschwinden() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    p.stifte.truncate(5);
    let weg = p.potentiale.remove(1);
    p.anlagen[1].sammelschienen.remove(0);
    let (b, _) = rundreise(&p, "ep_projekt_entfernt.epr");
    assert_eq!(b.stifte.len(), 5);
    assert!(!b.potentiale.contains(&weg));
    assert_eq!(b.anlagen[1].sammelschienen, p.anlagen[1].sammelschienen);
}

#[test]
fn neues_projekt_mit_projektwerten() {
    let mut p = Project::new_default();
    p.name = "Neu".into();
    p.kunde = "Kunde".into();
    p.stifte.push(Stift { name: "Rot".into(), style: 0, color: [255, 0, 0], width: 0.2 });
    p.pot_werte.insert("L1".into(), ("230".into(), "16".into()));
    p.leitungen[0].quer = 1.5;
    p.leitungen[0].adern = 3;
    p.anlagen[0].sammelschienen.push("N".into());
    p.anlagen[0].sheets[0].ort = "EG".into();
    let (b, _) = rundreise(&p, "ep_projekt_neu.epr");
    assert_eq!(b.kunde, "Kunde");
    assert_eq!(b.potentiale, p.potentiale);
    assert_eq!(b.stifte.len(), 5);
    assert_eq!(b.stifte[4].name, "Rot");
    assert_eq!(b.pot_werte["L1"], ("230".to_string(), "16".to_string()));
    assert_eq!((b.leitungen[0].quer, b.leitungen[0].adern), (1.5, 3));
    assert_eq!(b.anlagen[0].sammelschienen, vec!["N".to_string()]);
    assert_eq!(b.anlagen[0].sheets[0].ort, "EG");
}

#[test]
fn warnt_vor_nicht_speicherbarem() {
    let mut p = Project::new_default();
    assert!(epr_projekt::nicht_gespeichert(&p).is_empty());
    p.bauteile.push(Bauteil { name: "Schütz".into(), ..Default::default() });
    p.platzhalter.push(Platzhalter { name: "SPS".into(), werte: vec![("A".into(), "1".into())] });
    p.abgeschlossen = true;
    p.tb_absolut = true;
    p.stifte[0].color = [255, 0, 0];
    let w = epr_projekt::nicht_gespeichert(&p);
    assert_eq!(w.len(), 5, "{w:?}");
    assert!(w[0].starts_with("Bauteile (1)"));
}
