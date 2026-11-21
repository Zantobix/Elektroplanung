//! Testdatei für das Original (Gegenprobe plaene2/darstellung): eine Kopie von W2.epr mit Kontaktspiegel unter einer
//! vorhandenen Spule, neuem Querverweis-Blatt (2 Tabellen), drei Artikellisten (Summenliste, Summe+BMK,
//! Einzelliste), neuem Inhaltsverzeichnis, je einer Tabelle „Querverweise“ und „Inhaltsverzeichnis“, geänderten
//! Freigaben eines Projektsymbols, einem Projektsymbol mit eigenem Stift und grafisch geändertem Schriftfeld.
//!
//! `cargo run --release --example gegenprobe_plaene [-- <W2.epr> <ziel.epr>]`; Vorgabe: EP_SAMPLE →
//! %TEMP%\ep_orig_test\gegenprobe_plaene.epr. Die Quelle wird nur gelesen: zuerst nach
//! %TEMP%\ep_orig_test\gegenprobe_plaene_quelle.epr kopiert, gearbeitet wird mit der Kopie. Ist Gruppen.tfi der
//! Datenbank gesperrt, wird die Datenbank nach %TEMP%\ep_orig_test\Datenbank kopiert und EP_DATENBANK dorthin gesetzt.
use elektroplanung::schriftfeld_editor::{ziehen, Griff};
use elektroplanung::{berechnet, eigentuemer, epr, epr_projekt, epr_write, kontaktspiegel, model::*, symbol_stifte, symbols, titleblock::TbOwned};
use std::path::{Path, PathBuf};

/// Ordner rekursiv kopieren.
fn kopiere_ordner(von: &Path, nach: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(nach)?;
    for e in std::fs::read_dir(von)? {
        let e = e?;
        let ziel = nach.join(e.file_name());
        if e.file_type()?.is_dir() {
            kopiere_ordner(&e.path(), &ziel)?;
        } else {
            std::fs::copy(e.path(), ziel)?;
        }
    }
    Ok(())
}

/// Ist Gruppen.tfi gesperrt (das Original läuft), mit einer Kopie der Datenbank arbeiten.
fn datenbank_pruefen(ordner: &Path) {
    let Ok(db) = std::env::var("EP_DATENBANK").map(PathBuf::from) else { return };
    let gruppen = db.join("Gruppen.tfi");
    if !gruppen.exists() || std::fs::File::open(&gruppen).is_ok() {
        return;
    }
    let kopie = ordner.join("Datenbank");
    match kopiere_ordner(&db, &kopie) {
        Ok(()) => {
            std::env::set_var("EP_DATENBANK", &kopie);
            println!("Gruppen.tfi gesperrt – Datenbank kopiert nach {}", kopie.display());
        }
        Err(e) => println!("Gruppen.tfi gesperrt, Kopie der Datenbank fehlgeschlagen: {e}"),
    }
}

/// Symbolkennung im Original (VSY Byte 96) und Eigentümer (VSY › OWR Byte 0–7).
fn kennung(o: &Obj) -> u64 {
    o.raw.as_deref().filter(|r| r.block.is("VSY") && r.block.data.len() >= 104).map(|r| u64::from_le_bytes(r.block.data[96..104].try_into().unwrap())).unwrap_or(0)
}
fn eigentuemer(o: &Obj) -> Option<u64> {
    o.raw.as_deref().and_then(|r| r.block.kid("OWR")).and_then(|w| w.data.get(0..8)).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

fn sym_name(p: &Project, o: &Obj) -> String {
    match &o.kind {
        Kind::Symbol { key, .. } => symbols::find(&p.symdefs, key).map(|d| d.name.clone()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Angezeigte BMK und Lage („Blatt/Pfad“) eines Symbols.
fn bmk_lage(p: &Project, id: u64) -> String {
    let Some((r, o)) = p.find_obj(id) else { return String::new() };
    let Kind::Symbol { bmk, ext, pos, .. } = &o.kind else { return String::new() };
    let a = &p.anlagen[r.anlage];
    let s = &a.sheets[r.sheet];
    let eigen = symbols::display_bmk(bmk, ext.counter, &elektroplanung::canvas::bmk_ctx(a, s));
    let gezeigt = eigentuemer::anzeige_bmk(p, o, &elektroplanung::canvas::bmk_ctx(a, s));
    let zusatz = if gezeigt != eigen { format!(" = Eigentümer, eigene Vorlage {eigen}") } else { String::new() };
    format!("{gezeigt} ({}{zusatz}, Blatt {}/{})", sym_name(p, o), s.number, berechnet::blatt_pfad(s, pos.x).rsplit('/').next().unwrap_or(""))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let quelle = a.get(1).cloned().or_else(|| std::env::var("EP_SAMPLE").ok()).expect("W2.epr angeben oder EP_SAMPLE setzen");
    let ordner = std::env::temp_dir().join("ep_orig_test");
    std::fs::create_dir_all(&ordner).expect("Ordner anlegen");
    let ziel = a.get(2).map(PathBuf::from).unwrap_or_else(|| ordner.join("gegenprobe_plaene.epr"));
    if let Some(d) = ziel.parent() {
        std::fs::create_dir_all(d).expect("Zielordner anlegen");
    }
    datenbank_pruefen(&ordner);
    // nur mit einer Kopie arbeiten
    let kopie = ordner.join("gegenprobe_plaene_quelle.epr");
    std::fs::copy(&quelle, &kopie).expect("Quelle kopieren");
    let mut p = epr::import(&kopie).expect("Lesen");
    let mut wo: Vec<String> = vec![];
    let ai = p.anlagen.iter().position(|a| a.name == "Unterverteilung 1").unwrap_or(1);
    let anl = p.anlagen[ai].name.clone();

    // 1. Kontaktspiegel unter der Spule -K1 (Symbol „Relais“) mit ihren Kontakten: gleiche Kennzeichnung,
    //    Eigentümer (OWR) = Spule, dazu ein Wechsler derselben Anlage
    let (sr, spule) = p.anlagen[ai]
        .sheets
        .iter()
        .enumerate()
        .flat_map(|(si, s)| s.objects.iter().map(move |o| (si, o)))
        .find(|(_, o)| sym_name(&p, o) == "Relais")
        .map(|(si, o)| (SheetRef { anlage: ai, sheet: si }, o.id))
        .expect("Relais in Unterverteilung 1");
    let lib = p.symdefs.clone();
    // nur Kontakte derselben Anlage (-K1 gibt es in mehreren Anlagen), keine weitere Spule
    let mut kontakte: Vec<(u64, u8)> = kontaktspiegel::kontakte(&p, &lib, spule)
        .into_iter()
        .filter(|(id, _)| p.find_obj(*id).map(|(r, o)| r.anlage == ai && sym_name(&p, o) != "Relais").unwrap_or(false))
        .collect();
    let k_spule = kennung(p.find_obj(spule).unwrap().1);
    for o in p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects) {
        if k_spule != 0 && eigentuemer(o) == Some(k_spule) && !kontakte.iter().any(|(i, _)| *i == o.id) {
            kontakte.push((o.id, if sym_name(&p, o).contains("Wechsler") { 2 } else { 0 }));
        }
    }
    if let Some(w) = p.anlagen[ai].sheets.iter().flat_map(|s| &s.objects).find(|o| sym_name(&p, o) == "Wechsler") {
        kontakte.push((w.id, 2));
    }
    assert!(kontakte.len() >= 2, "zu wenige Kontakte: {kontakte:?}");
    let rows: Vec<kontaktspiegel::Zeile> = kontakte
        .iter()
        .enumerate()
        .map(|(i, (id, art))| kontaktspiegel::Zeile { art: *art, referenz: *id, felder: if i == 0 { ["13".into(), "14".into(), String::new()] } else { Default::default() }, ..Default::default() })
        .collect();
    let spule_pos = match &p.find_obj(spule).unwrap().1.kind {
        Kind::Symbol { pos, .. } => *pos,
        _ => unreachable!(),
    };
    let id = p.alloc_id();
    p.sheet_mut(sr).unwrap().objects.push(Obj { id, kind: Kind::Spiegel { pos: P::new(spule_pos.x, 0.0), owner: spule, rows: rows.clone(), auto: false }, pen: 0, fixed: false, group: None, raw: None });
    // Y wie im Original: über dem Schriftfeld (Unterkante der Zeichenfläche − Höhe des Spiegels)
    kontaktspiegel::nachfuehren(&mut p, &lib);
    {
        let s = p.sheet(sr).unwrap();
        let y = kontaktspiegel::start_y(&p, s);
        let mut t = format!(
            "{anl} / Blatt {} \u{201e}{}\u{201c}: Kontaktspiegel unter der Spule {} bei X = {:.1} mm, erste Zeile Y = {:.1} mm (Unterkante der Zeichenfläche {:.1} mm − Höhe), Höhe Feld 14 = {}; Zeilen:",
            s.number,
            s.name,
            bmk_lage(&p, spule),
            spule_pos.x,
            y,
            kontaktspiegel::zeichen_unterkante(&p, s),
            kontaktspiegel::hoehe(&rows)
        );
        for (i, z) in rows.iter().enumerate() {
            t += &format!(
                "\n    {}. {} → {}{}",
                i + 1,
                kontaktspiegel::ARTEN[z.art as usize],
                bmk_lage(&p, z.referenz),
                if i == 0 { ", Anschlussnummern 13/14 eingetragen" } else { ", Nummern leer (Anschlusstexte des Kontakts)" }
            );
        }
        wo.push(t);
    }
    // Übernehmen im Dialog: die Spule wird Eigentümer der Kontakte (VSY › OWR); neu ist das beim Wechsler
    let neu_owr: Vec<u64> = rows.iter().map(|z| z.referenz).filter(|i| p.find_obj(*i).is_some_and(|(_, o)| matches!(&o.kind, Kind::Symbol { ext, .. } if ext.eigentuemer != spule))).collect();
    let vorher_owr: Vec<String> = neu_owr.iter().map(|i| bmk_lage(&p, *i)).collect();
    kontaktspiegel::eigentuemer_setzen(&mut p, spule, &neu_owr);
    for (i, v) in neu_owr.iter().zip(&vorher_owr) {
        let (r, o) = p.find_obj(*i).unwrap();
        let qv = eigentuemer::querverweis(&p, o, Some(r)).unwrap_or_default();
        wo.push(format!("    neuer Eigentümer (OWR) = Spule: {v} zeigt jetzt {} und darunter klein \u{201e}{qv}\u{201c} (Lage der Spule)", bmk_lage(&p, *i)));
    }
    // vorhandene OWR aus W2 (Schließer neben -X2.9): Anzeige wie im Original „-K1“ mit „/Pfad“
    for o in p.anlagen[ai].sheets[sr.sheet].objects.iter().filter(|o| matches!(&o.kind, Kind::Symbol { ext, .. } if ext.eigentuemer == spule) && !neu_owr.contains(&o.id)) {
        let qv = eigentuemer::querverweis(&p, o, Some(sr)).unwrap_or_default();
        wo.push(format!("    Eigentümer schon in W2: {} mit \u{201e}{qv}\u{201c} darunter", bmk_lage(&p, o.id)));
    }

    // 2.–4. neue Listenblätter in Unterverteilung 1, abwechselnd, damit keine Folgeblätter entstehen
    let mut nr = p.anlagen[ai].sheets.iter().map(|s| s.number).max().unwrap_or(0);
    let mut neu = |p: &mut Project, art: u8, zusatz: u8, name: &str, was: &str, wo: &mut Vec<String>| -> SheetRef {
        nr += 1;
        let mut s = Sheet::new(nr);
        s.name = name.into();
        s.art = art;
        s.art_fest = true;
        s.zusatz = zusatz;
        p.anlagen[ai].sheets.push(s);
        wo.push(format!("{anl} / Blatt {nr} \u{201e}{name}\u{201c} (neu): {was} (BLT Feld 22 Byte 0 = {art}, Feld 32 Byte 1 = {zusatz})"));
        SheetRef { anlage: ai, sheet: p.anlagen[ai].sheets.len() - 1 }
    };
    neu(&mut p, 6, 0, "Gegenprobe Summenliste", "Blattart Artikelliste, Zusatz Summenliste", &mut wo);
    neu(&mut p, 1, 0, "Gegenprobe Inhalt", "Blattart Inhaltsverzeichnis", &mut wo);
    neu(&mut p, 6, 1, "Gegenprobe Summe+BMK", "Blattart Artikelliste, Zusatz Summe+BMK", &mut wo);
    neu(&mut p, 2, 2, "Gegenprobe Querverweise", "Blattart Querverweise, Zusatz 2 Tabellen", &mut wo);
    neu(&mut p, 6, 2, "Gegenprobe Einzelliste", "Blattart Artikelliste, Zusatz Einzelliste", &mut wo);

    // 5. Tabellen mit Daten „Querverweise“ und „Inhaltsverzeichnis“ auf einem neuen Schaltplan-Blatt
    let tr = neu(&mut p, 3, 0, "Gegenprobe Tabellen", "Blattart Schaltplan mit zwei Tabellen", &mut wo);
    for (art, x, was) in [(2u8, 20.0f32, "Querverweise"), (1, 160.0, "Inhaltsverzeichnis")] {
        let id = p.alloc_id();
        let k = Kind::Table { a: P::new(x, 30.0), b: P::new(x + 110.0, 100.0), rows: 10, cols: 2, cells: vec![String::new(); 20], header: true, fixed_w: false, art, ab: 1 };
        p.sheet_mut(tr).unwrap().objects.push(Obj { id, kind: k, pen: 0, fixed: false, group: None, raw: None });
        wo.push(format!(
            "    Tabelle {x:.0}/30 bis {:.0}/100 mm, 10 Zeilen × 2 Spalten, Überschrift an: Eigenschaften › Daten \u{201e}{was}\u{201c} (ZCN Feld 9 Byte 2 = {art}), Ab = 1",
            x + 110.0
        ));
    }

    // 6. Freigaben eines Projektsymbols (SYM Byte 112): Querverweise aus, Sicherungsübersicht an
    let benutzt = |p: &Project, key: &str| p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(&o.kind, Kind::Symbol { key: k, .. } if k == key)).count();
    let fi = (0..p.symdefs.len()).find(|i| p.symdefs[*i].name == "Magnetschalter" && benutzt(&p, &p.symdefs[*i].key) > 0).expect("Magnetschalter");
    let vorher = p.symdefs[fi].frg;
    p.symdefs[fi].frg = (false, true);
    let wo_fi: Vec<String> = p
        .anlagen
        .iter()
        .flat_map(|a| a.sheets.iter().map(move |s| (a, s)))
        .flat_map(|(a, s)| s.objects.iter().filter(|o| matches!(&o.kind, Kind::Symbol { key, .. } if *key == p.symdefs[fi].key)).map(move |o| (a, s, o)))
        .map(|(a, s, o)| format!("{} / Blatt {} \u{201e}{}\u{201c} {}", a.name, s.number, s.name, bmk_lage(&p, o.id)))
        .collect();
    wo.push(format!(
        "Projektsymbol \u{201e}Magnetschalter\u{201c} (Projekt › Symbole, Eigenschaften): Querverweise aus, Sicherungsübersicht an (vorher {vorher:?}); verwendet bei {}",
        wo_fi.join("; ")
    ));

    // 7. Projektsymbol mit eigenem Stift: alle Linien des Symbols „Lüfter“ mit Stift „Gegenprobe“ rot 0,5 mm
    let li = (0..p.symdefs.len()).find(|i| p.symdefs[*i].name == "Lüfter" && benutzt(&p, &p.symdefs[*i].key) > 0).expect("Lüfter");
    let nrs = symbol_stifte::neue_nummer(&p.symdefs[li].pens);
    p.symdefs[li].pens.push((nrs, Stift { name: "Gegenprobe".into(), style: 0, color: [255, 0, 0], width: 0.5 }));
    let mut n_el = 0;
    for pr in p.symdefs[li].prims.iter_mut() {
        if let symbols::Prim::Line(.., s) | symbols::Prim::Poly(_, s) | symbols::Prim::Bezier(.., s) | symbols::Prim::Ellipse(.., s) = pr {
            *s = nrs as u8;
            n_el += 1;
        }
    }
    let wo_li: Vec<String> = p
        .anlagen
        .iter()
        .flat_map(|a| a.sheets.iter().map(move |s| (a, s)))
        .flat_map(|(a, s)| s.objects.iter().filter(|o| matches!(&o.kind, Kind::Symbol { key, .. } if *key == p.symdefs[li].key)).map(move |o| (a, s, o)))
        .map(|(a, s, o)| format!("{} / Blatt {} \u{201e}{}\u{201c} {}", a.name, s.number, s.name, bmk_lage(&p, o.id)))
        .collect();
    wo.push(format!(
        "Projektsymbol \u{201e}Lüfter\u{201c}: eigener Stift Nr. {nrs} \u{201e}Gegenprobe\u{201c} (durchgezogen, rot, 0,5 mm) für alle {n_el} Linien/Kreise; Symbol bearbeiten › Stifte / Parameter zeigt ihn; rot gezeichnet bei {}",
        wo_li.join("; ")
    ));

    // 8. Schriftfeld grafisch: waagerechte Linie über dem Status 1000 Einheiten (= 1/10 Höhe) höher, Text ergänzt
    let si = p.titleblock.iter().position(|e| matches!(e, TbOwned::Line(8310, 7333, _, 7333))).expect("Linie über dem Status");
    let alt = p.titleblock[si].clone();
    p.titleblock[si] = ziehen(&alt, Griff::Ganz, 0, -1000, false);
    p.titleblock.push(TbOwned::Text(1350, 4000, 2500, 7000, "Gegenprobe".into(), Align::Left, 1));
    let h = p.title_h();
    wo.push(format!(
        "Schriftfeld (alle Blätter, Höhe {h:.0} mm, relativ): die kurze waagerechte Linie rechts über \u{201e}%Status\u{201c} ({alt:?}) liegt jetzt {:?}, also {:.1} mm höher; neuer linksbündiger Text \u{201e}Gegenprobe\u{201c} unter \u{201e}Von:\u{201c} (1350/4000 bis 2500/7000)",
        p.titleblock[si],
        h * 0.1
    ));

    let fehlt = epr_projekt::nicht_gespeichert(&p);
    assert!(fehlt.is_empty(), "nicht speicherbar: {fehlt:?}");
    epr_write::save(&p, &ziel).expect("Speichern");

    // Gegenprobe im Nachbau
    let back = epr::import(&ziel).expect("Lesen");
    let ba = &back.anlagen[ai];
    let neue: Vec<(u8, u8, &str)> = ba.sheets.iter().filter(|s| s.name.starts_with("Gegenprobe")).map(|s| (s.blattart(), s.zusatz, s.name.as_str())).collect();
    assert_eq!(
        neue,
        [(6, 0, "Gegenprobe Summenliste"), (1, 0, "Gegenprobe Inhalt"), (6, 1, "Gegenprobe Summe+BMK"), (2, 2, "Gegenprobe Querverweise"), (6, 2, "Gegenprobe Einzelliste"), (3, 0, "Gegenprobe Tabellen")]
    );
    let tab: Vec<u8> = ba.sheets.iter().find(|s| s.name == "Gegenprobe Tabellen").unwrap().objects.iter().filter_map(|o| if let Kind::Table { art, .. } = o.kind { Some(art) } else { None }).collect();
    assert_eq!(tab, [2, 1]);
    let sp: Vec<&Obj> = ba.sheets[sr.sheet].objects.iter().filter(|o| matches!(&o.kind, Kind::Spiegel { rows, .. } if !rows.is_empty())).collect();
    assert_eq!(sp.len(), 1);
    if let Kind::Spiegel { owner, rows: r2, pos, .. } = &sp[0].kind {
        assert_eq!(bmk_lage(&back, *owner), bmk_lage(&p, spule));
        assert!((pos.x - spule_pos.x).abs() < 0.06);
        assert_eq!(r2.len(), rows.len());
        for (x, y) in rows.iter().zip(r2) {
            assert_eq!((x.art, &x.felder), (y.art, &y.felder));
            assert_eq!(bmk_lage(&back, y.referenz), bmk_lage(&p, x.referenz));
            // OWR geschrieben: Eigentümer jedes Kontakts ist die Spule
            let e = back.find_obj(y.referenz).and_then(|(_, o)| if let Kind::Symbol { ext, .. } = &o.kind { Some(ext.eigentuemer) } else { None });
            assert_eq!(e, Some(*owner), "Eigentümer von {}", bmk_lage(&back, y.referenz));
        }
    }
    let d = |q: &Project, n: &str| q.symdefs.iter().find(|d| d.name == n && benutzt(q, &d.key) > 0).cloned().unwrap();
    assert_eq!(d(&back, "Magnetschalter").frg, (false, true));
    let l = d(&back, "Lüfter");
    assert!(l.pens.iter().any(|(n, s)| *n == nrs && s.color == [255, 0, 0] && (s.width - 0.5).abs() < 1e-3 && s.name == "Gegenprobe"));
    assert_eq!(back.titleblock, p.titleblock);
    println!("{} geschrieben (Quelle kopiert nach {}), wieder eingelesen und geprüft", ziel.display(), kopie.display());
    for w in wo {
        println!("- {w}");
    }
}
