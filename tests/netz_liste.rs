//! Werkzeug (kein Prüfling): elektrische Netze eines Blattes auflisten – je Netz Schienen, Klemmstellen und
//! Symbolanschlüsse (BMK:Anschlussnummer). `NETZ_DATEI=… NETZ_BLATT=<Reiter> cargo test --release --test netz_liste
//! -- --nocapture`. Zum Prüfen nachgezeichneter Blätter (todo N0: Netz-Invariante je Blatt).
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

#[test]
fn netz_liste() {
    let (Ok(f), Ok(b)) = (std::env::var("NETZ_DATEI"), std::env::var("NETZ_BLATT")) else { return };
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    app.active = b.parse().unwrap();
    let s = app.sheet().unwrap().clone();
    println!("Blatt {}", s.name);
    if std::env::var("NETZ_PINS").is_ok() {
        for o in &s.objects {
            if let Kind::Symbol { bmk, .. } = &o.kind {
                let v: Vec<String> = elektroplanung::canvas::obj_pins(o, &app.lib).iter().map(|q| format!("({:.2}|{:.2})", q.x, q.y)).collect();
                println!("  Anschlüsse {bmk}: {}", v.join(" "));
            }
        }
    }
    let knoten = elektroplanung::netz_waechter::knoten(&s, &app.lib, &[]);
    let mut netze: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
    for ((id, k), n) in &knoten {
        let o = s.objects.iter().find(|o| o.id == *id).unwrap();
        let name = match &o.kind {
            Kind::Symbol { bmk, .. } => format!("{}:{}", bmk.replace("+%P", ""), k + 1),
            Kind::Busbar { name, .. } => format!("Schiene {name}"),
            Kind::Terminal { leiste, nr, .. } => format!("Klemme {leiste}.{nr}"),
            Kind::Verweis { text, .. } => format!("Verweis {text}"),
            _ => format!("{id}"),
        };
        netze.entry(*n).or_default().push(name);
    }
    let mut zeilen: Vec<String> = netze.into_values().map(|mut v| {
        v.sort();
        v.join(", ")
    }).collect();
    zeilen.sort();
    for z in zeilen {
        println!("  {z}");
    }
    // Vergleich mit einer zweiten Datei (MT2 Rückkehrtest): abweichende Objekte des Blattes
    if let Ok(f2) = std::env::var("NETZ_DATEI2") {
        let mut b = test_app();
        b.open_path(std::path::PathBuf::from(f2), false);
        b.active = app.active;
        let t = b.sheet().unwrap();
        let mut n = 0;
        for o in &s.objects {
            match t.objects.iter().find(|x| x.id == o.id) {
                Some(x) if x.kind == o.kind => {}
                Some(x) => {
                    n += 1;
                    println!("  anders {}: {:?}\n        {:?}", o.id, o.kind, x.kind);
                }
                None => {
                    n += 1;
                    println!("  nur in Datei 1: {} {:?}", o.id, o.kind);
                }
            }
        }
        for x in t.objects.iter().filter(|x| !s.objects.iter().any(|o| o.id == x.id)) {
            n += 1;
            println!("  nur in Datei 2: {} {:?}", x.id, x.kind);
        }
        println!("  {n} abweichende Objekte");
    }
}
