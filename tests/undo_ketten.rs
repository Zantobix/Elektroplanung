//! Maustest-Reihe 3, MT3 (todo N2a): Rückgängig/Wiederholen-Ketten. Auf jedem W2-Blatt zehn gemischte
//! Bearbeitungen (Verschieben, Teilstück ziehen, Drehen, Löschen, Kopieren/Einfügen) – nach jedem Schritt wird das
//! Projekt gemerkt; danach zehnmal Rückgängig und zehnmal Wiederholen: jeder Zwischenstand muss genau wiederkommen.
use elektroplanung::blatt_tasten::test_app;
use elektroplanung::model::*;

struct Zufall(u64);
impl Zufall {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / ((1u64 << 31) as f32)
    }
}

fn unterschied(a: &[Obj], b: &[Obj]) -> String {
    let mut t = String::new();
    for o in a {
        match b.iter().find(|x| x.id == o.id) {
            None => t.push_str(&format!("\n    nur jetzt: {} {:?}", o.id, o.kind)),
            Some(x) if x != o => t.push_str(&format!("\n    anders {}: jetzt {:?}\n      erwartet {:?}", o.id, o.kind, x.kind)),
            _ => {}
        }
    }
    for o in b.iter().filter(|o| !a.iter().any(|x| x.id == o.id)) {
        t.push_str(&format!("\n    fehlt: {} {:?}", o.id, o.kind));
    }
    if t.is_empty() && a.len() == b.len() {
        t.push_str("\n    (nur Reihenfolge)");
    }
    t
}

fn stand(app: &elektroplanung::app::App) -> Vec<Obj> {
    app.sheet().map(|s| s.objects.clone()).unwrap_or_default()
}

#[test]
fn w2_rueckgaengig_wiederholen_ketten() {
    let Some(f) = std::env::var_os("EP_SAMPLE") else { return };
    let mut app = test_app();
    app.open_path(std::path::PathBuf::from(f), false);
    let mut z = Zufall(31337);
    let mut abweichungen = vec![];
    let mut schritte_gesamt = 0;
    for b in 0..app.tabs.len() {
        app.active = b;
        app.undo.clear();
        app.redo.clear();
        let mut staende = vec![stand(&app)];
        let mut arten = vec![];
        for _ in 0..10 {
            let Some(s) = app.sheet().cloned() else { break };
            let kand: Vec<&Obj> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. } | Kind::Wire { .. } | Kind::Text { .. })).collect();
            if kand.is_empty() {
                break;
            }
            let o = kand[(z.next() * kand.len() as f32) as usize % kand.len()].clone();
            let r = app.raster();
            let d = P::new(((z.next() * 6.0) as i32 - 3) as f32 * r, ((z.next() * 6.0) as i32 - 2) as f32 * r);
            let undo_vorher = app.undo.len();
            let art = z.next();
            let was = match &o.kind {
                Kind::Wire { pts, .. } if art < 0.4 && pts.len() >= 2 => {
                    let m = P::new((pts[0].x + pts[1].x) / 2.0, (pts[0].y + pts[1].y) / 2.0);
                    app.selection = vec![o.id];
                    if !app.teilstueck_greifen(o.id, m, 0.3) {
                        continue;
                    }
                    app.teilstueck_ziehen(m.add(d));
                    // wie beim Loslassen in der Oberfläche: aufräumen nur nach echter Bewegung
                    app.nach_ziehen_vereinigen();
                    app.drag = elektroplanung::app::Drag::None;
                    "Teilstück"
                }
                Kind::Symbol { .. } if art < 0.3 => {
                    app.selection = vec![o.id];
                    app.transform_selection(1);
                    "Drehen"
                }
                _ if art < 0.45 => {
                    app.selection = vec![o.id];
                    app.delete_selection();
                    "Löschen"
                }
                _ if art < 0.55 => {
                    app.selection = vec![o.id];
                    app.copy();
                    app.paste();
                    "Kopieren/Einfügen"
                }
                _ => {
                    app.checkpoint();
                    app.selection = vec![o.id];
                    app.stoesse_an_markierung_zusammenfassen();
                    app.netz_merken();
                    app.netz_verschieben(&[o.id], d);
                    app.aufraeumen();
                    // wie beim Loslassen: ein ganz abgewiesenes Ziehen hinterlässt keinen leeren Schritt
                    app.leeren_schritt_entfernen();
                    "Verschieben"
                }
            };
            // ein Schritt ohne Wirkung (z. B. vom Netz-Wächter abgewiesen) darf keinen Rückgängig-Schritt hinterlassen,
            // der nichts zurücknimmt – hier zählt nur, was einen Schritt angelegt hat
            if app.undo.len() == undo_vorher {
                continue;
            }
            if app.undo.len() != undo_vorher + 1 {
                abweichungen.push(format!("Blatt {b}: {was} legte {} Rückgängig-Schritte an", app.undo.len() - undo_vorher));
            }
            if stand(&app) == *staende.last().unwrap() {
                abweichungen.push(format!("Blatt {b}: „{was}“ hinterließ einen leeren Rückgängig-Schritt"));
            }
            staende.push(stand(&app));
            arten.push(was);
        }
        let n = arten.len();
        schritte_gesamt += n;
        // zurück
        for k in (0..n).rev() {
            app.do_undo();
            if stand(&app) != staende[k] {
                abweichungen.push(format!("Blatt {b}: Rückgängig nach „{}“ (Schritt {k}) ergibt nicht den Stand davor{}", arten[k], unterschied(&stand(&app), &staende[k])));
            }
        }
        // vor
        for k in 0..n {
            app.do_redo();
            if stand(&app) != staende[k + 1] {
                abweichungen.push(format!("Blatt {b}: Wiederholen von „{}“ (Schritt {k}) ergibt nicht den Stand danach", arten[k]));
            }
        }
        // für das nächste Blatt wieder zurück auf den Ausgangsstand
        for _ in 0..n {
            app.do_undo();
        }
    }
    for a in abweichungen.iter().take(30) {
        println!("{a}");
    }
    println!("{schritte_gesamt} Schritte, {} Abweichungen", abweichungen.len());
    assert!(abweichungen.is_empty());
}
