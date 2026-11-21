//! Schriftfeld grafisch bearbeiten (src/schriftfeld_editor.rs): verschobene, neue und gelöschte Elemente kommen
//! über das Originalformat (FUS › GRS/STR) unverändert zurück. Fehlt W2.epr (EP_SAMPLE), wird übersprungen.

use elektroplanung::schriftfeld_editor::{koord, neu, ziehen, Griff};
use elektroplanung::{epr, epr_write, model::Align, titleblock::TbOwned};
use std::path::PathBuf;

#[test]
fn bearbeitetes_schriftfeld_rundreise() {
    let path = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !path.exists() {
        return;
    }
    let mut p = epr::import(&path).unwrap();
    assert!(!p.titleblock.is_empty());
    // Linie verschieben, Textfeld vergrößern, ein Element löschen, Linie und Text neu
    let li = p.titleblock.iter().position(|e| matches!(e, TbOwned::Line(..))).unwrap();
    p.titleblock[li] = ziehen(&p.titleblock[li], Griff::Ganz, 0, 500, true);
    let ti = p.titleblock.iter().position(|e| matches!(e, TbOwned::Text(..))).unwrap();
    p.titleblock[ti] = ziehen(&p.titleblock[ti], Griff::Punkt(2), 300, 0, true);
    p.titleblock.remove(p.titleblock.len() - 1);
    p.titleblock.push(neu(0));
    let mut t = neu(1);
    if let TbOwned::Text(_, _, _, _, s, al, _) = &mut t {
        *s = "Plan %Seite".into();
        *al = Align::Left;
    }
    p.titleblock.push(t);
    // mehrzeiliger, rechtsbündiger Text mit 3 Zeilen
    p.titleblock.push(TbOwned::Text(7000, 0, 10000, 5000, "Zeile 1
Zeile 2
%Datum".into(), Align::Right, 3));
    p.titleblock.push(neu(2));
    let erwartet = p.titleblock.clone();
    let out = std::env::temp_dir().join(format!("ep_sfedit_{}.epr", std::process::id()));
    epr_write::save(&p, &out).unwrap();
    let q = epr::import(&out).unwrap();
    // nochmals gespeichert: dieselben Bytes (das bearbeitete Schriftfeld ist stabil)
    let out2 = out.with_extension("2.epr");
    epr_write::save(&q, &out2).unwrap();
    let gleich = std::fs::read(&out).unwrap() == std::fs::read(&out2).unwrap();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
    assert!(gleich, "zweites Speichern ändert die Datei");
    assert_eq!(q.titleblock, erwartet);
    assert_eq!(q.titleblock.len(), erwartet.len());
    for (a, b) in q.titleblock.iter().zip(&erwartet) {
        assert_eq!(koord(a), koord(b));
        assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b));
    }
    assert!(q.titleblock.iter().any(|e| matches!(e, TbOwned::Text(_, _, _, _, s, Align::Left, _) if s == "Plan %Seite")));
}
