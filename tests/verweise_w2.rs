//! Verweise im Beispielprojekt (W2.epr): Ziele aus OBJ Feld 8–13, Drehen mit Zurückschreiben ins Originalformat,
//! Verknüpfungen im PDF. Fehlt die Datei (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, model::*, pdf, verweise};
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

/// Verweis mit Text `t` auf dem Blatt `blatt` der ersten Anlage.
fn verweis<'a>(p: &'a Project, blatt: &str, t: &str) -> &'a Obj {
    let s = p.anlagen[0].sheets.iter().find(|s| s.name == blatt).unwrap();
    s.objects.iter().find(|o| matches!(&o.kind, Kind::Verweis { text, .. } if text == t)).unwrap()
}

fn ziel(o: &Obj) -> Option<VerweisZiel> {
    match &o.kind {
        Kind::Verweis { ziel, .. } => ziel.clone(),
        _ => None,
    }
}

#[test]
fn ziele_aus_epr() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // "=S3+W2" (Neubau EG) → Anlage "Heizungssteuerung =S3+W2", Blatt 2 (Klemmenplan)
    let z = ziel(verweis(&p, "Neubau EG", "=S3+W2")).expect("Ziel gesetzt");
    assert_eq!((z.anlage.as_str(), z.blatt), ("Heizungssteuerung =S3+W2", 2));
    let s = verweise::verweis_ziel(&p, Some(&z)).unwrap();
    assert_eq!(p.sheet(s.blatt).unwrap().name, "Klemmenplan");
    // "Raum 10" (Neubau EG) ↔ "Raum 7" (Neubeu DG): gegenseitig verknüpft
    let a = verweis(&p, "Neubau EG", "Raum 10");
    let b = verweis(&p, "Neubeu DG", "Raum 7");
    assert_eq!(ziel(a).unwrap().obj, b.id);
    assert_eq!(ziel(b).unwrap().obj, a.id);
    assert_eq!(verweise::verweis_ziel(&p, ziel(a).as_ref()).unwrap().obj, Some(b.id));
    // alle Verweise mit gesetztem Ziel lassen sich auflösen
    let alle: Vec<&Obj> = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| ziel(o).is_some()).collect();
    assert!(alle.len() >= 15, "{}", alle.len());
    for o in alle {
        assert!(verweise::verweis_ziel(&p, ziel(o).as_ref()).is_ok(), "{:?}", o.kind);
    }
}

#[test]
fn drehen_und_ziel_ins_originalformat() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    // je ein Verweis mit Text, ein Querverweis auf eine andere Anlage (Feld 4 = 0x0101FFFF) und einer ohne Text
    let mut ids: Vec<u64> = vec![verweis(&p, "Neubau EG", "Raum 12").id, verweis(&p, "Altbau EG", "=S1+W2").id, verweis(&p, "Altbau EG", "=UV3+W2").id];
    let ohne = p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).find(|o| matches!(&o.kind, Kind::Verweis { text, .. } if text.is_empty())).unwrap().id;
    ids.push(ohne);
    let neu_ziel = VerweisZiel { anlage: "Unterverteilung 2".into(), blatt: 3, obj: 0 };
    for schritt in 1..=4 {
        let mut erwartet = vec![];
        for o in p.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).filter(|o| ids.contains(&o.id)) {
            let vorher = matches!(o.kind, Kind::Verweis { vertical: true, .. });
            verweise::drehen(&mut o.kind, 1);
            // der Text dreht nicht mit (bleibt waagerecht bzw. hochkant wie zuvor)
            assert_eq!(matches!(o.kind, Kind::Verweis { vertical: true, .. }), vorher);
            if o.id == ids[0] {
                if let Kind::Verweis { ziel, .. } = &mut o.kind {
                    *ziel = Some(neu_ziel.clone());
                }
            }
            erwartet.push(o.kind.clone());
        }
        let tmp = std::env::temp_dir().join(format!("ep_verweis_drehen_{}_{schritt}.epr", std::process::id()));
        epr_write::save(&p, &tmp).unwrap();
        let q = epr::import(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        let geladen: Vec<Kind> = q.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects).filter(|o| matches!(o.kind, Kind::Verweis { .. })).map(|o| o.kind.clone()).collect();
        for k in &erwartet {
            // Lage, Richtung, Hochkant und Ziel überstehen Speichern + Laden (Objekt-Ids werden neu vergeben)
            let gleich = |a: &Kind, b: &Kind| match (a, b) {
                (Kind::Verweis { pos, text_pos, text, dir, vertical, ziel }, Kind::Verweis { pos: p2, text_pos: t2, text: x2, dir: d2, vertical: v2, ziel: z2 }) => {
                    pos.dist(*p2) < 0.01 && text_pos.dist(*t2) < 0.06 && text == x2 && dir == d2 && vertical == v2 && ziel.as_ref().map(|z| (&z.anlage, z.blatt)) == z2.as_ref().map(|z| (&z.anlage, z.blatt))
                }
                _ => false,
            };
            assert!(geladen.iter().any(|g| gleich(k, g)), "Schritt {schritt}: {k:?} nicht wiedergefunden");
        }
    }
    // nach vier Drehungen wieder die Ausgangslage
    let orig = epr::import(&path).unwrap();
    for id in &ids[1..] {
        let a = orig.find_obj(*id).unwrap().1;
        let b = p.find_obj(*id).unwrap().1;
        assert_eq!(a.kind, b.kind);
    }
}

#[test]
fn pdf_enthaelt_verknuepfungen() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let mut lib = p.symdefs.clone();
    lib.dedup_by(|a, b| a.key == b.key);
    let (bytes, n) = pdf::render_headless(&p, &lib);
    let dir = std::env::temp_dir().join(format!("ep_pdf_links_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("W2.pdf");
    std::fs::write(&file, &bytes).unwrap();
    let text = String::from_utf8_lossy(&std::fs::read(&file).unwrap()).to_string();
    let _ = std::fs::remove_dir_all(&dir);
    let links: Vec<&str> = text.match_indices("/Subtype /Link").map(|(i, _)| &text[i..]).collect();
    assert!(links.len() >= 15, "nur {} Verknüpfungen", links.len());
    // jedes Ziel ist ein Seitenobjekt (4, 6, 8, … bei n Seiten)
    for l in links {
        let d = &l[l.find("/Dest [").unwrap() + 7..];
        let nr: usize = d.split_whitespace().next().unwrap().parse().unwrap();
        assert!(nr >= 4 && nr < 4 + 2 * n && nr % 2 == 0, "Ziel {nr}");
    }
    assert!(text.contains("/Annots ["));
}
