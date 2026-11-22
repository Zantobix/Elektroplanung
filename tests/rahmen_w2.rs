//! Blattrahmen aus den Rändern der Anlage (siehe `src/rahmen.rs`) am Beispielprojekt W2.epr: dort 50/50/100/50, also
//! quer oben 10 mm, sonst 5 mm; hoch links 10 mm. Geänderte Ränder werden geschrieben, wieder gelesen und ändern den
//! Rahmen. Fehlt die Datei (EP_SAMPLE), werden die Tests übersprungen.

use elektroplanung::{epr, epr_write, rahmen};
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
fn w2_rahmen_unveraendert() {
    let Some(p) = sample() else { return };
    let prj = epr::import(&p).unwrap();
    let mut n = 0;
    for a in &prj.anlagen {
        assert_eq!(a.set.raender, rahmen::RAENDER_W2, "{}", a.name);
        for s in &a.sheets {
            let soll = if s.portrait { (10.0, 5.0, 205.0, 292.0) } else { (5.0, 10.0, 292.0, 205.0) };
            assert_eq!(s.frame(), soll, "{} / {}", a.name, s.name);
            n += 1;
        }
    }
    assert!(n > 10);
    // unverändert gespeichert: byteidentisch
    assert_eq!(epr_write::to_bytes(&prj).unwrap(), std::fs::read(&p).unwrap());
}

#[test]
fn geaenderte_raender_rundreise() {
    let Some(p) = sample() else { return };
    let mut prj = epr::import(&p).unwrap();
    prj.anlagen[0].set.raender = [70, 60, 250, 80];
    rahmen::abgleichen(&mut prj);
    // quer: links = Unten 6, oben = Links 25, rechts = Oben 7, unten = Rechts 8; hoch wie im Dialog
    let soll = |hoch: bool| if hoch { (25.0, 7.0, 202.0, 291.0) } else { (6.0, 25.0, 290.0, 202.0) };
    for s in &prj.anlagen[0].sheets {
        assert_eq!(s.frame(), soll(s.portrait), "{}", s.name);
    }
    let bytes = epr_write::to_bytes(&prj).unwrap();
    let tmp = std::env::temp_dir().join(format!("ep_rahmen_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let back = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(back.anlagen[0].set.raender, [70, 60, 250, 80]);
    for s in &back.anlagen[0].sheets {
        assert_eq!(s.frame(), soll(s.portrait), "{}", s.name);
    }
    // die übrigen Anlagen behalten ihren Rahmen
    for a in &back.anlagen[1..] {
        assert_eq!(a.set.raender, rahmen::RAENDER_W2);
    }
}
