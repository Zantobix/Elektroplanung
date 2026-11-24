//! Tools › Projekt prüfen, Hinweisart „Gleiches Potential getrennt“ (todo NR11) auf W2.epr: W2 ist ein fertiger,
//! geprüfter Plan – jeder Hinweis dort wäre ein Fehlalarm.
use elektroplanung::pruefen::{pruefen, HinweisArt};

#[test]
fn keine_fehlalarme_auf_w2() {
    let Ok(pfad) = std::env::var("EP_SAMPLE") else { return };
    if !std::path::Path::new(&pfad).exists() {
        return;
    }
    let mut app = elektroplanung::blatt_tasten::test_app();
    app.open_path(pfad.into(), false);
    let h: Vec<_> = pruefen(&app.project, &app.lib).into_iter().filter(|x| x.art == HinweisArt::GleichesPotential).collect();
    for x in &h {
        println!("{:?} {}", x.sheet, x.text);
    }
    assert!(h.is_empty(), "{} Hinweise auf W2", h.len());
}
