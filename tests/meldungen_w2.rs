//! Laufzeittexte gegen das Beispielprojekt (W2.epr): Die Fehlerliste des Originals bleibt bei fehlerfreien Dateien
//! leer, eine verfälschte Prüfsumme wird mit Kontext gemeldet, und die Statuszeile nennt Symbole, Sammelschienen und
//! Verbindungen wie das Original. Fehlt die Datei (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::hover::{Objektart, Ziel};
use elektroplanung::{epr, meldungen, model::*};
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

#[test]
fn fehlerliste_bei_fehlerfreien_dateien_leer() {
    let Some(p) = sample() else { return };
    let d = std::fs::read(&p).unwrap();
    assert!(meldungen::ladefehler_vorab(&Ok(d.clone()), false).is_none());
    assert_eq!(meldungen::ladefehler_liste(&d), Vec::<String>::new());
    // vom Original gespeicherte Varianten ebenso
    if let Ok(dir) = std::env::var("EP_FIXTURES") {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "epr") {
                let d = std::fs::read(e.path()).unwrap();
                assert_eq!(meldungen::ladefehler_liste(&d), Vec::<String>::new(), "{}", e.path().display());
            }
        }
    }
}

#[test]
fn verfaelschte_pruefsumme_mit_kontext() {
    let Some(p) = sample() else { return };
    let mut d = std::fs::read(&p).unwrap();
    // erstes VSY: ein Datenbyte ändern (die Prüfsumme bleibt alt)
    let at = d.windows(4).position(|w| w == b"\x99VSY").unwrap();
    d[at + 8] ^= 0x20;
    let l = meldungen::ladefehler_liste(&d);
    assert_eq!(l.first().map(|s| s.as_str()), Some("In den Projektdaten:"));
    assert!(l.iter().any(|s| s.starts_with(" In den Anlagendaten von \"")), "{l:?}");
    assert!(l.iter().any(|s| s.starts_with("  In den Blattdaten von \"")), "{l:?}");
    assert_eq!(l.last().unwrap(), "    Prüfsumme ist Fehlerhaft. Objekt: VSY");
}

#[test]
fn statuszeile_texte_aus_w2() {
    let Some(path) = sample() else { return };
    let p = epr::import(&path).unwrap();
    let ziel = |id| Ziel::Objekt { id, mn: P::default(), mx: P::default(), art: Objektart::Verschiebbar };
    let (mut symbol, mut schiene, mut leitung) = (None, None, None);
    for (ai, a) in p.anlagen.iter().enumerate() {
        for (si, s) in a.sheets.iter().enumerate() {
            let r = SheetRef { anlage: ai, sheet: si };
            for o in &s.objects {
                let t = meldungen::objekt_text(&p, &[], r, &ziel(o.id));
                match &o.kind {
                    Kind::Symbol { .. } if symbol.is_none() => symbol = t,
                    Kind::Busbar { .. } if schiene.is_none() => schiene = t,
                    Kind::Wire { .. } if leitung.is_none() => leitung = t,
                    _ => {}
                }
            }
        }
    }
    let symbol = symbol.unwrap();
    assert!(symbol.starts_with("Symbol: ") && symbol.contains("  X: ") && symbol.contains("  Y: "), "{symbol}");
    // „Sammelschiene: Schiene L1(L1)“: Name der Schiene (SMS) und ihr Titel
    let schiene = schiene.unwrap();
    assert!(schiene.starts_with("Sammelschiene: Schiene ") && schiene.ends_with(')'), "{schiene}");
    let leitung = leitung.unwrap();
    assert!(leitung.starts_with("manuelle Verbindung mit ") || leitung.starts_with("automatische Verbindung mit "), "{leitung}");
}
