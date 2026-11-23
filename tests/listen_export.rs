//! Listen Export: Betriebsmittel- und Klemmenliste wie im Original (Wine-Gegenprobe 23.11.2026 mit
//! fixtures/r10_klemmen.epr; die leeren Zeilen, die das Original anhängt, entfallen bewusst).
use elektroplanung::listen;

fn projekt() -> Option<elektroplanung::model::Project> {
    let dir = std::env::var("EP_FIXTURES").ok()?;
    let pfad = std::path::Path::new(&dir).join("r10_klemmen.epr");
    if !pfad.exists() {
        return None;
    }
    let mut app = elektroplanung::blatt_tasten::test_app();
    app.open_path(pfad, false);
    Some(app.project)
}

#[test]
fn betriebsmittel_wie_original() {
    let Some(p) = projekt() else { return };
    let (k, z) = listen::liste(&p, 1);
    assert_eq!(listen::text(&k, &z, 1), "Anlage;Ort;Betriebsmittel\r\nAnlage 1;;-F1\r\nAnlage 1;;-Q1\r\nAnlage 1;;-E1\r\nAnlage 1;;-K1.1\r\nAnlage 1;;-E1\r\n");
}

#[test]
fn klemmen_wie_original() {
    let Some(p) = projekt() else { return };
    let (k, z) = listen::liste(&p, 2);
    assert_eq!(listen::text(&k, &z, 0), "Anlage\tOrt\tKlemmleiste\tKlemmenindex\r\nAnlage 1\t\tX1\t1\r\nAnlage 1\t\tX1\t2\r\nAnlage 1\t\tX1\t3\r\n");
}
