//! Eigenschaften der Zeichenobjekte (Formular TZeichen im Dialog TObjekt_Eigenschaften, Lücke 28 der
//! Verhaltensmetrik). Das Original zeigt nach Doppelklick bzw. „Eigenschaften“ zwei Tabellen (TIconTab):
//! oben die Koordinaten (Spalten „Von“, „X“, „Y“, Werte in mm mit einer Nachkommastelle), darunter die
//! Eigenschaften (Spalte „Wert“). Überschriften und Zeilennamen stehen in der Zeichenkettentabelle der Exe
//! (je Art: Name, Überschrift 1, „Von/X/Y“, Punkte, Überschrift 2, „Wert“, Eigenschaften):
//! - Linie: „Linienkoordinaten“ (Punkt 1, 2), „Linieneigenschaften“ (Farbe, Stift, Fixiert)
//! - Viereck: „Koordinaten vom Viereck“ (Punkt 1–4), „Eigenschaft vom Viereck“ (Füllfarbe, Stift, Fixiert)
//! - Dreieck: „Koordinaten vom Dreieck“ (Punkt 1–3), „Eigenschaft vom Dreieck“ (Füllfarbe, Stift, Fixiert)
//! - Kreis: „Kreiskoordinaten“ (Position), „Kreiseigenschaften“ (Breite, Höhe, Füllfarbe, Stift, Fixiert)
//! - Bezier: „Koordinaten vom Bezier“ (Anfangspunkt, Kontrollpunkt 1, 2, Endpunkt), „Eigenschaft vom Bezier“
//!   (Farbe, Stift, Fixiert)
//! Die Zeile „Seitenoffset“ der Tabelle zeigt das Original nicht an (Aufnahmen des Originals).
//! Farbe bzw. Füllfarbe: FormShow 0x4a27f8 setzt als Vorgabe „Schwarz als Vorgabe“ (Linie, Bezier) bzw.
//! „Transparent“ (Arten 3–5). Der Nachbau speichert keine eigene Farbe der Zeichenobjekte; die Zeile zeigt die
//! Vorgabe und ist gesperrt.

use crate::model::*;
use eframe::egui::{self, Color32, RichText, Vec2};

/// Art des Zeichenobjekts (Reihenfolge wie die Typbytes 1–5 der ZCN-Blöcke).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    Linie,
    Bezier,
    Viereck,
    Dreieck,
    Kreis,
}

impl Art {
    /// Überschriften der beiden Tabellen (Zeichenkettentabelle der Exe).
    pub fn ueberschriften(self) -> (&'static str, &'static str) {
        match self {
            Art::Linie => ("Linienkoordinaten", "Linieneigenschaften"),
            Art::Bezier => ("Koordinaten vom Bezier", "Eigenschaft vom Bezier"),
            Art::Viereck => ("Koordinaten vom Viereck", "Eigenschaft vom Viereck"),
            Art::Dreieck => ("Koordinaten vom Dreieck", "Eigenschaft vom Dreieck"),
            Art::Kreis => ("Kreiskoordinaten", "Kreiseigenschaften"),
        }
    }

    /// Zeilennamen der Koordinatentabelle.
    pub fn punkte(self) -> &'static [&'static str] {
        match self {
            Art::Linie => &["Punkt 1", "Punkt 2"],
            Art::Bezier => &["Anfangspunkt", "Kontrollpunkt 1", "Kontrollpunkt 2", "Endpunkt"],
            Art::Viereck => &["Punkt 1", "Punkt 2", "Punkt 3", "Punkt 4"],
            Art::Dreieck => &["Punkt 1", "Punkt 2", "Punkt 3"],
            Art::Kreis => &["Position"],
        }
    }

    /// Flächen haben „Füllfarbe“ (Vorgabe „Transparent“), Linien „Farbe“ (Vorgabe „Schwarz als Vorgabe“).
    pub fn flaeche(self) -> bool {
        matches!(self, Art::Viereck | Art::Dreieck | Art::Kreis)
    }
}

/// Zustand des Dialogs.
#[derive(Clone, Debug, PartialEq)]
pub struct ZeichenProps {
    pub id: u64,
    pub art: Art,
    /// je Zeile X und Y als Text in mm (Komma)
    pub punkte: Vec<[String; 2]>,
    /// nur Kreis: Breite und Höhe in mm
    pub breite: String,
    pub hoehe: String,
    pub stift: usize,
    pub fixiert: bool,
    /// Punkte beim Öffnen (unverändert → Objektart bleibt, z. B. Dreieck aus dem Werkzeug)
    start: Vec<[String; 2]>,
}

fn mm(v: f32) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn zahl(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse().ok()
}

fn pt(p: P) -> [String; 2] {
    [mm(p.x), mm(p.y)]
}

/// Ecken eines achsparallelen Vierecks in der Reihenfolge, in der das Original sie speichert (epr_write).
fn ecken(a: P, b: P) -> [P; 4] {
    [a, P::new(b.x, a.y), b, P::new(a.x, b.y)]
}

/// Dialogzustand aus einem Zeichenobjekt; None für Objekte mit eigenem Dialog (Text, Symbol, Tabelle …).
pub fn aus_objekt(o: &Obj) -> Option<ZeichenProps> {
    let (art, p, gr): (Art, Vec<P>, (f32, f32)) = match &o.kind {
        Kind::Line { a, b } => (Art::Linie, vec![*a, *b], (0.0, 0.0)),
        Kind::Curve { a, c1, c2, b } => (Art::Bezier, vec![*a, *c1, *c2, *b], (0.0, 0.0)),
        Kind::Rect { a, b } => (Art::Viereck, ecken(*a, *b).to_vec(), (0.0, 0.0)),
        Kind::Poly { pts } if pts.len() == 4 => (Art::Viereck, pts.clone(), (0.0, 0.0)),
        Kind::Triangle { a, b } => (Art::Dreieck, crate::canvas::triangle_pts(*a, *b).to_vec(), (0.0, 0.0)),
        Kind::Poly { pts } if pts.len() == 3 => (Art::Dreieck, pts.clone(), (0.0, 0.0)),
        Kind::Circle { a, b } => (Art::Kreis, vec![*a], (b.x - a.x, b.y - a.y)),
        _ => return None,
    };
    let punkte: Vec<[String; 2]> = p.into_iter().map(pt).collect();
    Some(ZeichenProps {
        id: o.id,
        art,
        start: punkte.clone(),
        punkte,
        breite: if art == Art::Kreis { mm(gr.0) } else { String::new() },
        hoehe: if art == Art::Kreis { mm(gr.1) } else { String::new() },
        stift: o.pen,
        fixiert: o.fixed,
    })
}

/// Eingaben auf das Objekt übertragen; ungültige Zahlen lassen den alten Wert stehen.
pub fn anwenden(z: &ZeichenProps, o: &mut Obj) {
    o.pen = z.stift;
    o.fixed = z.fixiert;
    let alt: Vec<P> = match aus_objekt(o) {
        Some(a) => a.start.iter().map(|[x, y]| P::new(zahl(x).unwrap_or(0.0), zahl(y).unwrap_or(0.0))).collect(),
        None => return,
    };
    let neu: Vec<P> = z
        .punkte
        .iter()
        .zip(&alt)
        .map(|([x, y], a)| P::new(zahl(x).unwrap_or(a.x), zahl(y).unwrap_or(a.y)))
        .collect();
    let geaendert = z.punkte != z.start;
    match (&mut o.kind, z.art) {
        (Kind::Line { a, b }, _) if geaendert => {
            *a = neu[0];
            *b = neu[1];
        }
        (Kind::Curve { a, c1, c2, b }, _) if geaendert => {
            *a = neu[0];
            *c1 = neu[1];
            *c2 = neu[2];
            *b = neu[3];
        }
        (Kind::Circle { a, b }, _) => {
            let (w0, h0) = (b.x - a.x, b.y - a.y);
            let p = neu[0];
            let w = zahl(&z.breite).unwrap_or(w0);
            let h = zahl(&z.hoehe).unwrap_or(h0);
            *a = p;
            *b = P::new(p.x + w, p.y + h);
        }
        (k, Art::Viereck) if geaendert => {
            let q = [neu[0], neu[1], neu[2], neu[3]];
            let achsparallel = (q[0].y - q[1].y).abs() < 0.01
                && (q[1].x - q[2].x).abs() < 0.01
                && (q[2].y - q[3].y).abs() < 0.01
                && (q[3].x - q[0].x).abs() < 0.01;
            *k = if achsparallel { Kind::Rect { a: q[0], b: q[2] } } else { Kind::Poly { pts: q.to_vec() } };
        }
        (k, Art::Dreieck) if geaendert => *k = Kind::Poly { pts: neu },
        _ => {}
    }
}

/// Große blaue Überschrift wie im Original (fette Schrift, Unterkante bündig über der Tabelle).
fn ueberschrift(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).font(crate::props::bold(22.0)).color(Color32::from_rgb(0, 0, 255)));
}

/// Kopfzelle der Tabellen (grauer Grund).
fn kopf(ui: &mut egui::Ui, t: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 18.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, Color32::from_gray(240));
    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, Color32::from_gray(160)), egui::StrokeKind::Inside);
    ui.painter().text(r.left_center() + Vec2::new(3.0, 0.0), egui::Align2::LEFT_CENTER, t, egui::FontId::proportional(12.0), Color32::BLACK);
}

fn zelle_text(ui: &mut egui::Ui, t: &str, w: f32, gesperrt: bool) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, Color32::from_gray(200)), egui::StrokeKind::Inside);
    let c = if gesperrt { Color32::from_gray(120) } else { Color32::BLACK };
    ui.painter().text(r.left_center() + Vec2::new(3.0, 0.0), egui::Align2::LEFT_CENTER, t, egui::FontId::proportional(12.0), c);
}

fn zelle_eingabe(ui: &mut egui::Ui, s: &mut String, w: f32) {
    ui.add_sized([w, 20.0], egui::TextEdit::singleline(s).font(egui::FontId::proportional(12.0)));
}

/// Inhalt des Dialogs (Breite 289 wie TObjekt_Eigenschaften).
pub fn ui(ui: &mut egui::Ui, z: &mut ZeichenProps, stifte: &[String]) {
    let (u1, u2) = z.art.ueberschriften();
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
    ueberschrift(ui, u1);
    let (w0, w1) = (92.0, 85.0);
    ui.horizontal(|ui| {
        kopf(ui, "Von", w0);
        kopf(ui, "X", w1);
        kopf(ui, "Y", w1);
    });
    let namen = z.art.punkte();
    for (i, p) in z.punkte.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            zelle_text(ui, namen.get(i).copied().unwrap_or(""), w0, false);
            zelle_eingabe(ui, &mut p[0], w1);
            zelle_eingabe(ui, &mut p[1], w1);
        });
    }
    ui.add_space(4.0);
    ueberschrift(ui, u2);
    let (v0, v1) = (104.0, 158.0);
    ui.horizontal(|ui| {
        kopf(ui, "", v0);
        kopf(ui, "Wert", v1);
    });
    if z.art == Art::Kreis {
        for (n, s) in [("Breite", &mut z.breite), ("Höhe", &mut z.hoehe)] {
            ui.horizontal(|ui| {
                zelle_text(ui, n, v0, false);
                zelle_eingabe(ui, s, v1);
            });
        }
    }
    ui.horizontal(|ui| {
        zelle_text(ui, if z.art.flaeche() { "Füllfarbe" } else { "Farbe" }, v0, false);
        zelle_text(ui, if z.art.flaeche() { "Transparent" } else { "Schwarz als Vorgabe" }, v1, true);
    })
    .response
    .on_hover_text("Eigene Farben der Zeichenobjekte speichert der Nachbau nicht");
    ui.horizontal(|ui| {
        zelle_text(ui, "Stift", v0, false);
        let akt = stifte.get(z.stift).cloned().unwrap_or_else(|| format!("Stift {}", z.stift));
        egui::ComboBox::from_id_salt("zeichen_stift").width(v1 - 8.0).selected_text(akt).show_ui(ui, |ui| {
            for (i, s) in stifte.iter().enumerate() {
                ui.selectable_value(&mut z.stift, i, s);
            }
        });
    });
    ui.horizontal(|ui| {
        zelle_text(ui, "Fixiert", v0, false);
        egui::ComboBox::from_id_salt("zeichen_fix").width(v1 - 8.0).selected_text(if z.fixiert { "Ja" } else { "Nein" }).show_ui(ui, |ui| {
            ui.selectable_value(&mut z.fixiert, true, "Ja");
            ui.selectable_value(&mut z.fixiert, false, "Nein");
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(kind: Kind) -> Obj {
        Obj { id: 1, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn linie_wie_im_original() {
        let o = obj(Kind::Line { a: P::new(65.5, 106.0), b: P::new(65.5, 188.5) });
        let z = aus_objekt(&o).unwrap();
        assert_eq!(z.art.ueberschriften(), ("Linienkoordinaten", "Linieneigenschaften"));
        assert_eq!(z.punkte, vec![["65,5".to_string(), "106,0".to_string()], ["65,5".to_string(), "188,5".to_string()]]);
        assert!(!z.art.flaeche());
    }

    #[test]
    fn bezier_vier_punkte() {
        let o = obj(Kind::Curve { a: P::new(148.0, 61.0), c1: P::new(158.0, 61.0), c2: P::new(163.0, 66.0), b: P::new(163.0, 76.0) });
        let mut z = aus_objekt(&o).unwrap();
        assert_eq!(z.art.punkte(), ["Anfangspunkt", "Kontrollpunkt 1", "Kontrollpunkt 2", "Endpunkt"]);
        assert_eq!(z.art.ueberschriften().0, "Koordinaten vom Bezier");
        z.punkte[3][1] = "80".into();
        z.fixiert = true;
        z.stift = 2;
        let mut o2 = o.clone();
        anwenden(&z, &mut o2);
        assert_eq!(o2.kind, Kind::Curve { a: P::new(148.0, 61.0), c1: P::new(158.0, 61.0), c2: P::new(163.0, 66.0), b: P::new(163.0, 80.0) });
        assert!(o2.fixed);
        assert_eq!(o2.pen, 2);
    }

    #[test]
    fn viereck_bleibt_rechteck_oder_wird_polygon() {
        let o = obj(Kind::Rect { a: P::new(10.0, 10.0), b: P::new(30.0, 20.0) });
        let mut z = aus_objekt(&o).unwrap();
        assert_eq!(z.punkte.len(), 4);
        assert!(z.art.flaeche());
        // nur Stift geändert: Objektart unverändert
        let mut o1 = o.clone();
        z.stift = 1;
        anwenden(&z, &mut o1);
        assert_eq!(o1.kind, o.kind);
        // Ecke 2 und 3 nach rechts: bleibt achsparallel
        z.punkte[1][0] = "40".into();
        z.punkte[2][0] = "40".into();
        anwenden(&z, &mut o1);
        assert_eq!(o1.kind, Kind::Rect { a: P::new(10.0, 10.0), b: P::new(40.0, 20.0) });
        // schräg: Polygon mit vier Punkten
        let mut o2 = o.clone();
        let mut z2 = aus_objekt(&o2).unwrap();
        z2.punkte[2][1] = "25".into();
        anwenden(&z2, &mut o2);
        assert!(matches!(&o2.kind, Kind::Poly { pts } if pts.len() == 4));
    }

    #[test]
    fn dreieck_und_kreis() {
        let o = obj(Kind::Triangle { a: P::new(0.0, 0.0), b: P::new(10.0, 10.0) });
        let mut z = aus_objekt(&o).unwrap();
        assert_eq!(z.art, Art::Dreieck);
        let mut o1 = o.clone();
        anwenden(&z, &mut o1);
        assert_eq!(o1.kind, o.kind, "unverändert bleibt es ein Dreieck des Werkzeugs");
        z.punkte[0][0] = "4".into();
        anwenden(&z, &mut o1);
        assert!(matches!(&o1.kind, Kind::Poly { pts } if pts.len() == 3 && pts[0].x == 4.0));

        let k = obj(Kind::Circle { a: P::new(5.0, 5.0), b: P::new(15.0, 11.0) });
        let mut z = aus_objekt(&k).unwrap();
        assert_eq!((z.breite.as_str(), z.hoehe.as_str()), ("10,0", "6,0"));
        assert_eq!(z.art.ueberschriften(), ("Kreiskoordinaten", "Kreiseigenschaften"));
        z.breite = "20".into();
        z.punkte[0] = ["1".into(), "2".into()];
        let mut k1 = k.clone();
        anwenden(&z, &mut k1);
        assert_eq!(k1.kind, Kind::Circle { a: P::new(1.0, 2.0), b: P::new(21.0, 8.0) });
    }

    #[test]
    fn andere_objekte_ohne_tabelle() {
        let o = obj(Kind::Poly { pts: vec![P::new(0.0, 0.0), P::new(1.0, 1.0)] });
        assert!(aus_objekt(&o).is_none());
    }
}
