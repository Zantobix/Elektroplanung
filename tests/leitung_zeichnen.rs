//! Runde 13: Leitung mit der Maus ziehen – Zufallsbahnen wie in der Oberfläche (App::leitung_zug_bewegen,
//! App::leitung_vorschau). Gesucht werden Sprünge der Vorschau („das Routing springt plötzlich um“).
use elektroplanung::app::{App, Drag};
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::leitung_fuehrung::Fuehrung;
use elektroplanung::model::*;
use elektroplanung::symbols;

fn p(x: f32, y: f32) -> P {
    P::new(x, y)
}

fn sym(id: u64, key: &str, pos: P, rot: u8) -> Obj {
    Obj { id, kind: Kind::Symbol { key: key.into(), pos, rot, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
}

fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
    Obj { id, kind: Kind::Wire { pts: pts.iter().map(|&(x, y)| P::new(x, y)).collect(), leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
}

/// Blatt: Schließer bei (105,105) (Anschlüsse oben/unten), Schließer gedreht bei (185,105), Leitung waagerecht bei
/// y = 65 von x 45 bis 245, senkrechte Leitung bei x = 225.
pub fn blatt() -> App {
    let mut app = test_app();
    app.project = Project::new_default();
    app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
    app.active = 0;
    app.lib = symbols::builtin();
    app.with_sheet(|s, _| {
        s.objects.push(sym(1, "builtin:schliesser", p(105.0, 105.0), 0));
        s.objects.push(sym(2, "builtin:schliesser", p(185.0, 105.0), 1));
        s.objects.push(wire(3, &[(45.0, 65.0), (245.0, 65.0)]));
        s.objects.push(wire(4, &[(225.0, 65.0), (225.0, 165.0)]));
    });
    app
}

fn skalar(a: P, b: P) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Größter Abstand der beiden Linienzüge (in beide Richtungen).
pub fn abweichung(u: &[P], v: &[P]) -> f32 {
    fn zum_zug(q: P, v: &[P]) -> f32 {
        if v.len() == 1 {
            return q.dist(v[0]);
        }
        v.windows(2)
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let d = b.sub(a);
                let l2 = skalar(d, d);
                let t = if l2 < 1e-9 { 0.0 } else { (skalar(q.sub(a), d) / l2).clamp(0.0, 1.0) };
                q.dist(P::new(a.x + d.x * t, a.y + d.y * t))
            })
            .fold(f32::MAX, f32::min)
    }
    fn proben(u: &[P]) -> Vec<P> {
        let mut out = vec![u[0]];
        for w in u.windows(2) {
            for i in 1..=20 {
                let t = i as f32 / 20.0;
                out.push(P::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
            }
        }
        out
    }
    let a = proben(u).into_iter().map(|q| zum_zug(q, v)).fold(0.0, f32::max);
    let b = proben(v).into_iter().map(|q| zum_zug(q, u)).fold(0.0, f32::max);
    a.max(b)
}

struct Zufall(u64);
impl Zufall {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / ((1u64 << 31) as f32)
    }
}

/// Bahn abfahren (Schritte 0,4 mm), je Bild die Vorschau; liefert die Sprünge (Bild, Abweichung, vorher, nachher).
pub fn fahren(app: &mut App, start: P, ecken: &[P], z: f32) -> (Vec<(usize, f32, Vec<P>, Vec<P>)>, Vec<P>) {
    app.drag = Drag::Wire { f: Fuehrung::neu(start) };
    let mut alt: Option<Vec<P>> = None;
    let mut spruenge = vec![];
    let mut vorher = start;
    let mut bild = 0;
    let mut letzte = vec![];
    for e in ecken {
        let n = (vorher.dist(*e) / 0.4).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let m = P::new(vorher.x + (e.x - vorher.x) * t, vorher.y + (e.y - vorher.y) * t);
            app.leitung_zug_bewegen(m, z);
            let w = app.leitung_vorschau(m, z).unwrap();
            if let Some(a) = alt.as_ref().filter(|a| a.len() >= 2 && w.len() >= 2) {
                let d = abweichung(a, &w);
                // erlaubt: das gefangene Ende springt um ein Raster, dazu der Stich am Anschluss
                if d > app.raster() * 1.01 + app.raster() / 2.0 + 0.5 {
                    spruenge.push((bild, d, a.clone(), w.clone()));
                }
            }
            alt = Some(w.clone());
            letzte = w;
            bild += 1;
        }
        vorher = *e;
    }
    (spruenge, letzte)
}

#[test]
fn zufallsbahnen_ohne_spruenge() {
    let mut z = Zufall(12345);
    let mut faelle = vec![];
    for fall in 0..300 {
        let mut app = blatt();
        let pins = elektroplanung::canvas::obj_pins(&app.sheet().unwrap().objects[(fall % 2) as usize].clone(), &app.lib);
        let start = pins[(fall / 2) % pins.len()];
        let mut ecken = vec![];
        let mut q = start;
        let k = 2 + (z.next() * 3.0) as usize;
        for _ in 0..k {
            // überwiegend achsparallele Züge, manchmal schräg
            let r = z.next();
            let l = 10.0 + z.next() * 60.0;
            let s = if z.next() < 0.5 { -1.0 } else { 1.0 };
            q = if r < 0.4 { P::new(q.x + s * l, q.y) } else if r < 0.8 { P::new(q.x, q.y + s * l) } else { P::new(q.x + s * l, q.y + (z.next() - 0.5) * l) };
            q = P::new(q.x.clamp(30.0, 270.0), q.y.clamp(30.0, 180.0));
            ecken.push(q);
        }
        let (sp, _) = fahren(&mut app, start, &ecken, 3.0);
        if !sp.is_empty() {
            faelle.push((fall, start, ecken.clone(), sp.len(), sp[0].clone()));
        }
    }
    for (fall, start, ecken, n, (bild, d, a, b)) in faelle.iter().take(12) {
        println!("Fall {fall}: Start {start:?} Bahn {ecken:?}: {n} Sprünge, erster Bild {bild} um {d:.1} mm\n  vorher {a:?}\n  nachher {b:?}");
    }
    println!("{} von 300 Bahnen mit Sprüngen", faelle.len());
    // Stand Runde 13: 14 von 300 (vorher 72) – übrig ist der Seitenwechsel des Umwegs um das eigene Symbol, wenn die
    // Maus quer durch das Symbol auf die andere Seite fährt; er kommt erst, wenn der gehaltene Umweg mehr als
    // 2,5 Raster länger wäre (Formhalt)
    // 24.11.2026 (Nutzer, Frage F8): das Querstück liegt auf der Höhe, auf der die Maus abgebogen ist; fährt die Maus
    // hinter dieses Knie zurück, wird es vergessen und die Form wechselt einmal (Linux 16 statt 15, Windows lag bisher
    // eins höher) – daher Grenze 17
    assert!(faelle.len() <= 17, "{} Bahnen mit Sprüngen", faelle.len());
}

