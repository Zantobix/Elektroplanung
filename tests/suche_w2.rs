//! Suchen, Projekt prüfen und Bildexport mit dem Beispielprojekt (EP_SAMPLE); fehlt es, werden die Tests übersprungen.
//! Die Suchtexte werden aus dem Projekt genommen (keine Inhalte im Code).

use elektroplanung::model::*;
use elektroplanung::{bildexport, epr, pruefen, suche, symbols};
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

/// Projektsymbole, ergänzt um die Original-Bibliothek, falls vorhanden.
fn lib(p: &Project) -> Vec<symbols::SymDef> {
    let mut lib = p.symdefs.clone();
    if let Some(dir) = symbols::find_db_dir(None) {
        if let Ok(db) = symbols::load_db(&dir) {
            for d in db {
                if symbols::find(&lib, &d.key).is_none() {
                    lib.push(d);
                }
            }
        }
    }
    lib
}

#[test]
fn suche_findet_text_in_w2() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    // erster Text mit mindestens 4 Buchstaben, gesucht wird ein Teil davon in Großschreibung
    let (r, id, text) = p
        .anlagen
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| a.sheets.iter().enumerate().map(move |(si, s)| (ai, si, s)))
        .flat_map(|(ai, si, s)| s.objects.iter().map(move |o| (SheetRef { anlage: ai, sheet: si }, o)))
        .find_map(|(r, o)| match &o.kind {
            Kind::Text { text, .. } if text.chars().filter(|c| c.is_alphabetic()).count() >= 4 && !text.contains("%") => Some((r, o.id, text.clone())),
            _ => None,
        })
        .expect("W2 enthält Texte");
    let teil: String = text.chars().filter(|c| c.is_alphabetic()).take(4).collect();
    let start = text.find(teil.chars().next().unwrap()).unwrap();
    let query: String = text[start..].chars().take_while(|c| *c != '\n').take(6).map(|c| c.to_ascii_uppercase()).collect::<String>();
    let t = suche::suchen(&p, &query, false);
    assert!(t.iter().any(|x| x.sheet == r && x.id == id), "Treffer fehlt ({} Treffer)", t.len());
    // mit Beachtung der Schreibweise nur, wenn der Text selbst so geschrieben ist
    let exakt = suche::suchen(&p, &query, true);
    assert_eq!(exakt.iter().any(|x| x.id == id), text.contains(&query));
}

#[test]
fn pruefen_und_bild_mit_w2() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let lib = lib(&p);
    let h = pruefen::pruefen(&p, &lib);
    let offen = h.iter().filter(|x| x.art == pruefen::HinweisArt::OffenerAnschluss).count();
    eprintln!("W2: {} Hinweise ({offen} offene Anschlüsse, {} doppelte BMK)", h.len(), h.len() - offen);
    for x in &h {
        assert!(p.sheet(x.sheet).is_some_and(|s| s.objects.iter().any(|o| o.id == x.id)));
    }
    let t0 = std::time::Instant::now();
    let img = bildexport::render_headless(&p, &lib, SheetRef { anlage: 0, sheet: 0 }, 150, true).unwrap();
    eprintln!("Blatt 1 mit 150 dpi: {:?} in {:?}", img.dimensions(), t0.elapsed());
    let dunkel = img.pixels().filter(|px| px.0[0] < 128).count();
    assert!(dunkel > 1000, "Blatt sieht leer aus ({dunkel} dunkle Pixel)");
}
