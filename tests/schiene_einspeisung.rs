//! Sammelschiene mit „Einspeisung“: Das Original zeichnet am Anfang kein Quadrat (Blattzeichnung 0x438b94 kennt
//! im Zweig Sammelschiene nur Kreise). Früher setzte der Nachbau dort ein gefülltes Quadrat von 1,2 mm.

use elektroplanung::{bildexport, model::*};

#[test]
fn kein_quadrat_am_anfang_bei_einspeisung() {
    let mut p = Project::new_default();
    let id = p.alloc_id();
    let a = P::new(100.0, 100.0);
    // feste Lage (Bit 1), damit die Schiene genau bei a gezeichnet wird
    let kind = Kind::Busbar { a, x2: 200.0, name: "L1".into(), label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 2, versatz: 0.0 };
    p.anlagen[0].sheets[0].objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None });
    let set = p.anlagen[0].schienen.entry("L1".into()).or_default();
    set.einspeisung = true;
    set.quer = false;
    let dpi = 300;
    let img = bildexport::render_headless(&p, &[], SheetRef { anlage: 0, sheet: 0 }, dpi, false).unwrap();
    let k = dpi as f32 / 25.4;
    let px = |x: f32, y: f32| img.get_pixel((x * k).round() as u32, (y * k).round() as u32).0;
    // die Schiene selbst ist gezeichnet …
    assert!(px(150.0, 100.0).iter().all(|&c| c < 128), "Schiene fehlt: {:?}", px(150.0, 100.0));
    // … aber 0,45 mm über und unter dem Anfang (innerhalb des früheren Quadrats, außerhalb der Linie) ist es weiß
    for dy in [-0.45, 0.45] {
        for dx in [-0.45, 0.0] {
            assert!(px(a.x + dx, a.y + dy).iter().all(|&c| c > 200), "Quadrat bei {dx}/{dy}: {:?}", px(a.x + dx, a.y + dy));
        }
    }
}
