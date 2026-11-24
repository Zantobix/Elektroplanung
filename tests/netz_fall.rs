//! Diagnose (N00, nicht Teil der Prüfung): einen Fall des Netz-Zufallsprüfers einzeln nachstellen.
//! `NETZ_FALL="blatt art id dx dy [i]"` mit art = ganz | teil | dreh | obj.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

#[test]
fn netz_fall() {
    let (Some(f), Ok(fall)) = (std::env::var_os("EP_SAMPLE"), std::env::var("NETZ_FALL")) else { return };
    let v: Vec<&str> = fall.split_whitespace().collect();
    let b: usize = v[0].parse().unwrap();
    let id: u64 = v[2].parse().unwrap();
    let d = P::new(v[3].parse().unwrap(), v[4].parse().unwrap());
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    app.active = b;
    if std::env::var("OHNE_WAECHTER").is_ok() {
        app.opt.netz_waechter = false;
    }
    let s = app.sheet().unwrap().clone();
    let lib = app.lib.clone();
    match v[1] {
        "ganz" | "obj" => {
            app.checkpoint();
            app.selection = vec![id];
            app.netz_merken();
            app.netz_verschieben(&[id], d);
        }
        "dreh" => {
            app.selection = vec![id];
            app.transform_selection(1);
        }
        _ => {
            let i: usize = v[5].parse().unwrap();
            let Some(Kind::Wire { pts, .. }) = s.objects.iter().find(|o| o.id == id).map(|o| &o.kind) else { panic!() };
            let m = P::new((pts[i].x + pts[i + 1].x) / 2.0, (pts[i].y + pts[i + 1].y) / 2.0);
            app.selection = vec![id];
            assert!(app.teilstueck_greifen(id, m, 0.3));
            app.teilstueck_ziehen(m.add(d));
            app.drag = elektroplanung::app::Drag::None;
        }
    }
    println!("info: {}", app.info);
    let roh = app.sheet().unwrap().clone();
    if std::env::var("OHNE_AUFR").is_err() {
        app.aufraeumen();
    }
    let n = app.sheet().unwrap().clone();
    let zeige = |t: &str, sh: &Sheet| {
        println!("== {t}");
        for o in &sh.objects {
            let alt = s.objects.iter().find(|x| x.id == o.id);
            if alt.is_some_and(|a| a.kind == o.kind) && t != "vorher" {
                continue;
            }
            if t == "vorher" && !n.objects.iter().find(|x| x.id == o.id).is_none_or(|x| x.kind != o.kind) {
                continue;
            }
            match &o.kind {
                Kind::Wire { pts, .. } => println!("  Leitung {} {pts:?}", o.id),
                k => println!("  {} {:?}", o.id, k),
            }
        }
        for o in s.objects.iter().filter(|o| !sh.objects.iter().any(|x| x.id == o.id)) {
            println!("  entfallen {}", o.id);
        }
    };
    zeige("vorher", &s);
    zeige("roh (vor Aufräumen)", &roh);
    zeige("nachher", &n);
    // Nachbarleitungen: berühren eine geänderte Leitung (vorher oder nachher)
    let geaendert: Vec<Vec<P>> = n.objects.iter().chain(s.objects.iter()).filter(|x| {
        let a = s.objects.iter().find(|y| y.id == x.id);
        let b = n.objects.iter().find(|y| y.id == x.id);
        a.map(|a| &a.kind) != b.map(|b| &b.kind)
    }).filter_map(|x| if let Kind::Wire { pts, .. } = &x.kind { Some(pts.clone()) } else { None }).collect();
    for o in &s.objects {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        let ber = geaendert.iter().any(|g| g.windows(2).any(|w| pts.iter().any(|p| seg_dist(*p, w[0], w[1]) < 0.06)) || pts.windows(2).any(|w| g.iter().any(|p| seg_dist(*p, w[0], w[1]) < 0.06)));
        if ber {
            println!("  Nachbar {} {pts:?}", o.id);
        }
    }
    // Pins der Symbole in der Nähe der geänderten Leitungen
    for o in &s.objects {
        if matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Busbar { .. }) {
            let pins = elektroplanung::canvas::obj_pins(o, &lib);
            let nah = n.objects.iter().filter(|x| s.objects.iter().find(|y| y.id == x.id).is_none_or(|y| y.kind != x.kind)).any(|x| match &x.kind {
                Kind::Wire { pts, .. } => pins.iter().any(|q| pts.iter().any(|p| p.dist(*q) < 15.0)),
                _ => false,
            });
            if nah {
                println!("  Objekt {} Anschlüsse {pins:?}", o.id);
            }
        }
    }
}
