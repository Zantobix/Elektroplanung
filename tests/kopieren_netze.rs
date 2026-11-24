//! Maustest-Reihe 3, MT5 (todo N2a): verdrahtete Gruppen kopieren und einfügen – auf dasselbe Blatt (versetzt), auf
//! ein anderes Blatt derselben Anlage und in eine andere Anlage. Die Kopie hat dieselbe Netzstruktur wie das Original
//! (Netze nach Zahl der Knoten), und die vorhandenen Netze des Zielblatts bleiben unverändert (keine Verbindung
//! zwischen Kopie und Bestand).
use elektroplanung::app::App;
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;
use elektroplanung::netz_waechter::{knoten, verbunden};

/// Netzstruktur: je Netz die Zahl seiner Knoten (Anschlüsse, Klemmstellen, Schienen, Verweise) außer Schienen-
/// und Verweisknoten, die über den Namen blattweit verbinden; sortiert.
fn struktur(s: &Sheet, app: &App, ids: &[u64]) -> Vec<usize> {
    let k = knoten(s, &app.lib, &[]);
    let mut je: std::collections::HashMap<usize, usize> = Default::default();
    for ((id, _), n) in &k {
        let ist_name = s.objects.iter().find(|o| o.id == *id).is_some_and(|o| matches!(o.kind, Kind::Busbar { .. } | Kind::Verweis { .. }));
        if ids.contains(id) && !ist_name {
            *je.entry(*n).or_default() += 1;
        }
    }
    let mut v: Vec<usize> = je.into_values().collect();
    v.sort();
    v
}

/// Symbole, Klemmstellen und Leitungen eines W2-Blattes (ohne Schienen und Verweise, deren Name blattweit verbindet).
fn gruppe(s: &Sheet) -> Vec<u64> {
    s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Wire { .. })).map(|o| o.id).collect()
}

fn w2() -> Option<App> {
    let f = std::env::var_os("EP_SAMPLE")?;
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    Some(app)
}

#[test]
fn w2_kopie_hat_dieselben_netze() {
    let Some(mut app) = w2() else { return };
    let mut befunde = vec![];
    let mut abgewiesen = 0;
    let n = app.tabs.len();
    for quelle in 0..n {
        app.active = quelle;
        let s = app.sheet().unwrap().clone();
        let ids = gruppe(&s);
        if ids.iter().filter(|id| s.objects.iter().any(|o| o.id == **id && matches!(o.kind, Kind::Wire { .. }))).count() < 3 {
            continue;
        }
        // Soll: dasselbe Blatt nur mit der Gruppe (ohne Schienen/Verweise, die nicht mitkopiert werden)
        let nur = Sheet { objects: s.objects.iter().filter(|o| ids.contains(&o.id)).cloned().collect(), ..s.clone() };
        let soll = struktur(&nur, &app, &ids);
        app.selection = ids.clone();
        app.copy();
        // Ziele: dasselbe Blatt, das nächste Blatt (zyklisch)
        for ziel in [quelle, (quelle + 1) % n] {
            app.active = ziel;
            let vorher = app.sheet().unwrap().clone();
            let alt: Vec<u64> = vorher.objects.iter().map(|o| o.id).collect();
            app.paste();
            let nachher = app.sheet().unwrap().clone();
            let neu: Vec<u64> = nachher.objects.iter().map(|o| o.id).filter(|id| !alt.contains(id)).collect();
            if neu.is_empty() {
                assert!(app.info.starts_with("Nicht eingefügt"), "{}", app.info);
                abgewiesen += 1;
                continue;
            }
            // die Kopie für sich hat dieselbe Netzstruktur wie das Original
            let kopie = Sheet { objects: nachher.objects.iter().filter(|o| neu.contains(&o.id)).cloned().collect(), ..nachher.clone() };
            let ist = struktur(&kopie, &app, &neu);
            if ist != soll {
                befunde.push(format!("Blatt {quelle} → {ziel}: Kopie {ist:?} statt {soll:?}"));
            }
            // Bestand des Zielblatts: keine Netze verbunden oder getrennt
            let (a, b) = (knoten(&vorher, &app.lib, &[]), knoten(&nachher, &app.lib, &[]));
            let (v, g) = (verbunden(&a, &b, &neu), verbunden(&b, &a, &neu));
            if v + g > 0 {
                befunde.push(format!("Blatt {quelle} → {ziel}: Bestand {v} verbunden, {g} getrennt"));
            }
            app.do_undo();
        }
    }
    for b in befunde.iter().take(20) {
        println!("{b}");
    }
    println!("{} Befunde, {abgewiesen} Einfügungen abgewiesen", befunde.len());
    assert!(befunde.is_empty());
}
