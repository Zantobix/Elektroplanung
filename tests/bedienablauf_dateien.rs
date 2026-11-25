//! NR2 (Datei-Sicherheit): Dateien aus längeren Bedienabläufen über die Funktionen der Oberfläche erzeugen,
//! speichern, wieder laden und prüfen. Mit `EP_RUNDREISE_AUS=<ordner>` werden sie abgelegt, um sie im Original zu
//! öffnen (`werkzeuge/linux/original_pruefen.sh`, Cloud-Gegenprobe 25.11.2026).
//! Abläufe: neue Anlage mit Blättern, Sicherungsübersicht und Klemmplan als Blattart, Klemmstelle in eine Leitung,
//! DXF-Import (Linie, Kreis, Bogen, Polylinie, Text), Sammelschiene über den Werkzeugknopf, Symbol löschen mit
//! Leitungen, Teilstück löschen – jeweils im neuen Projekt und in einer W2-Kopie.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use std::path::PathBuf;

const DXF: &str = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\nWand\n10\n0\n20\n0\n11\n100\n21\n0\n\
0\nCIRCLE\n8\nWand\n10\n50\n20\n50\n40\n10\n\
0\nARC\n8\nTuer\n10\n80\n20\n20\n40\n15\n50\n0\n51\n90\n\
0\nLWPOLYLINE\n8\nWand\n90\n3\n70\n0\n10\n0\n20\n60\n10\n40\n20\n60\n10\n40\n20\n90\n\
0\nTEXT\n8\nBeschriftung\n10\n10\n20\n70\n40\n5\n1\nKüche\n\
0\nENDSEC\n0\nEOF\n";

fn speichern(app: &mut elektroplanung::app::App, name: &str) -> PathBuf {
    let dir = std::env::var("EP_RUNDREISE_AUS").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("ep_bedienablauf"));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{name}.epr"));
    app.path = Some(p.clone());
    app.save();
    p
}

fn laden(p: &PathBuf) -> Project {
    let f = elektroplanung::eprfile::EprFile::read(p).unwrap();
    assert_eq!(f.bad_checksums, 0, "{}: Prüfsummen", p.display());
    elektroplanung::epr::import(p).unwrap()
}

fn ablauf(app: &mut elektroplanung::app::App, name: &str) {
    // neue Anlage mit zwei weiteren Blättern: Sicherungsübersicht und Klemmplan
    app.new_anlage();
    let a = app.project.anlagen.len() - 1;
    app.new_sheet();
    app.new_sheet();
    {
        let an = &mut app.project.anlagen[a];
        let n = an.sheets.len();
        an.sheets[n - 2].art = 5;
        an.sheets[n - 2].art_fest = true;
        an.sheets[n - 2].name = "Sicherungsübersicht".into();
        an.sheets[n - 1].art = 4;
        an.sheets[n - 1].art_fest = true;
        an.sheets[n - 1].name = "Klemmplan".into();
    }
    // auf dem ersten Blatt der neuen Anlage: Schiene, Leitung, Klemmstelle in der Leitung, DXF
    app.open_sheet(SheetRef { anlage: a, sheet: 0 });
    app.schiene_objekt_neu();
    app.dlg3_ok();
    let y = app.sheet().unwrap().objects.iter().find_map(|o| if let Kind::Busbar { a, .. } = &o.kind { Some(a.y) } else { None }).unwrap();
    app.add_obj(Kind::Wire { pts: vec![P::new(60.0, y), P::new(60.0, 120.0), P::new(120.0, 120.0)], leitung: 0, bez: vec![(P::new(61.0, 80.0), "sw".into())], opts: 0 });
    app.klemmstelle_setzen(P::new(60.0, 90.0), 1.0);
    let datei = std::env::temp_dir().join(format!("ep_{name}.dxf"));
    std::fs::write(&datei, DXF).unwrap();
    let z = elektroplanung::dxf::lesen(DXF.as_bytes()).unwrap();
    let d = elektroplanung::dxf_dialog::DxfDialog::neu(datei, z);
    app.dxf_uebernehmen(&d);
}

fn pruefen(p: &PathBuf, anlagen_vorher: usize) {
    let q = laden(p);
    assert_eq!(q.anlagen.len(), anlagen_vorher + 1, "neue Anlage");
    let an = q.anlagen.last().unwrap();
    assert_eq!(an.sheets.len(), 3);
    assert_eq!((an.sheets[1].blattart(), an.sheets[2].blattart()), (5, 4), "Blattarten");
    let s = &an.sheets[0];
    let zahl = |f: &dyn Fn(&Kind) -> bool| s.objects.iter().filter(|o| f(&o.kind)).count();
    assert_eq!(zahl(&|k| matches!(k, Kind::Busbar { .. })), 1);
    assert_eq!(zahl(&|k| matches!(k, Kind::Terminal { .. })), 1);
    assert_eq!(zahl(&|k| matches!(k, Kind::Wire { .. })), 2, "Leitung an der Klemme geteilt");
    assert!(zahl(&|k| matches!(k, Kind::Line { .. } | Kind::Circle { .. } | Kind::Curve { .. } | Kind::Poly { .. })) >= 3, "DXF-Elemente: {:?}", s.objects.iter().map(|o| std::mem::discriminant(&o.kind)).collect::<Vec<_>>());
    assert!(s.objects.iter().any(|o| matches!(&o.kind, Kind::Text { text, .. } if text == "Küche")), "DXF-Text");
}

#[test]
fn bedienablauf_neues_projekt() {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    let n = app.project.anlagen.len();
    ablauf(&mut app, "ablauf_neu");
    let p = speichern(&mut app, "ablauf_neu");
    pruefen(&p, n);
}

#[test]
fn bedienablauf_w2_kopie() {
    let Some(w2) = std::env::var("EP_SAMPLE").ok().filter(|p| std::path::Path::new(p).exists()) else { return };
    let mut app = test_app();
    app.open_path(w2.into(), false);
    let n = app.project.anlagen.len();
    // zusätzlich in W2: Symbol -F2 löschen (mit Leitungen) und ein Teilstück löschen
    let si = app.project.anlagen[1].sheets.iter().position(|s| s.name.starts_with("Sicherungen -F10")).unwrap();
    app.open_sheet(SheetRef { anlage: 1, sheet: si });
    let f2 = app.sheet().unwrap().objects.iter().find(|o| matches!(&o.kind, Kind::Symbol { pos, .. } if pos.dist(P::new(97.5, 85.0)) < 0.01)).unwrap().id;
    app.selection = vec![f2];
    app.delete_selection();
    app.mehrfachklick(426, P::new(28.0, 155.0), 0.5, 1);
    app.delete_selection();
    ablauf(&mut app, "ablauf_w2");
    let p = speichern(&mut app, "ablauf_w2");
    pruefen(&p, n);
}
