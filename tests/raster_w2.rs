//! Rasterursprung der Blätter des Beispielprojekts (W2.epr) wie im Original: Rand der Anlage (quer links/oben +
//! Strompfadleiste, hoch umgedreht). Das Raster selbst steht nicht im .epr (Rundreise: tests/epr_w2.rs).
//! Fehlt die Datei (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::{epr, model::*, raster};
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
fn ursprung_aller_blaetter() {
    let Some(p) = sample() else { return };
    let prj = epr::import(&p).unwrap();
    let mut quer = 0;
    let mut hoch = 0;
    for a in &prj.anlagen {
        assert_eq!(a.set.raender, [50, 50, 100, 50], "{}", a.name);
        assert_eq!(a.set.pfad_hoehe, 50, "{}", a.name);
        for s in &a.sheets {
            let o = raster::ursprung(s, &a.set, true);
            let soll = if s.portrait { P::new(10.0, 10.0) } else { P::new(5.0, 15.0) };
            assert_eq!(o, soll, "{} / {}", a.name, s.name);
            assert_eq!(raster::ursprung(s, &a.set, false), P::new(0.0, 0.0));
            if s.portrait {
                hoch += 1
            } else {
                quer += 1
            }
        }
    }
    assert!(quer > 0 && hoch > 0, "W2 hat Quer- und Hochformatblätter");
}

