//! Anschlusskreise (nur Bildschirm) im Beispielprojekt: Jeder Symbolanschluss bekommt einen Kreis, auch die
//! angeschlossenen (Original unter Wine, Blatt „Sicherung -F7“). Ohne EP_SAMPLE bzw. EP_DATENBANK übersprungen.

use elektroplanung::{anschlusskreise, epr, model::*, symbols};
use std::path::PathBuf;

#[test]
fn kreise_an_allen_symbolanschluessen() {
    let (Ok(s), Ok(d)) = (std::env::var("EP_SAMPLE"), std::env::var("EP_DATENBANK")) else { return };
    if !PathBuf::from(&s).exists() {
        return;
    }
    let p = epr::import(&PathBuf::from(s)).unwrap();
    let mut lib = symbols::load_db(&PathBuf::from(d)).unwrap();
    // Projektsymbole wie beim Laden in der Oberfläche
    let fehlt: Vec<_> = p.symdefs.iter().filter(|x| symbols::find(&lib, &x.key).is_none()).cloned().collect();
    lib.extend(fehlt);
    let a = p.anlagen.iter().find(|a| a.name == "Unterverteilung 1").unwrap();
    let blatt = a.sheets.iter().find(|s| s.number == 6).unwrap();
    let k = anschlusskreise::punkte(blatt, &lib);
    // Steckdosen -X7.x mit N, PE, L und Taster -S7.x mit 13/14 tragen Kreise
    assert!(k.len() >= 10, "{}", k.len());
    // mindestens ein Kreis liegt auf einem Leitungsende (angeschlossen)
    let enden: Vec<P> = blatt.objects.iter().filter_map(|o| match &o.kind {
        Kind::Wire { pts, .. } if pts.len() >= 2 => Some([pts[0], pts[pts.len() - 1]]),
        _ => None,
    }).flatten().collect();
    assert!(k.iter().any(|q| enden.iter().any(|e| (e.x - q.x).abs() < 0.3 && (e.y - q.y).abs() < 0.3)));
}
