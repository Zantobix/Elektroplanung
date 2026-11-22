//! Variantenrad beim Platzieren eines Bibliothekssymbols wie im Original.
//!
//! Hat das Symbol Bildvarianten (Seiten/Bildindex, GRD › SEI), erscheint beim Absetzen auf dem Blatt ein
//! hellblauer Ring um die Absetzstelle: im Loch das abgesetzte Symbol (Bild 0), auf dem Ring alle Bilder,
//! links beginnend im Uhrzeigersinn (bei genau drei Bildern links, oben, rechts). Die Maus wählt das nächste
//! Bild (vergrößert ×1,3 und fett gezeichnet), im Loch Bild 0; ein Klick setzt das Symbol mit diesem
//! Bildindex. Wird die Maus deutlich aus dem Ring hinaus bewegt, schließt sich das Rad und das Symbol wird mit
//! Bild 0 gesetzt (Original: Fenster versteckt sich, Ergebnis bleibt 0).
//!
//! Original (Ghidra): Anzeige in fcn.004407d0 (PaintBox1DragDrop, Bedingung: Symbol hat eine Seitenliste mit
//! Einträgen), Aufbau fcn.004b2cb8 (höchstens 12 Bilder, Maßstab 70 px / größte Bildabmessung, Auswahl ×1,3,
//! Startwinkel −π, Schritt 2π/n bzw. π/2 bei n = 3), Zeichnen fcn.004b2aa4 (Außenkreis = Breite, Loch =
//! Breite/3), Mausbewegung fcn.004b28e4 (nächstes Bild, Loch = Bild 0, Schließen außerhalb bei
//! Abstand² > Minimum + 5000), Klick fcn.004b2fe4 (Ergebnis = gewähltes Bild → Bildindex des Symbols).
//!
//! Abweichend vom festen Maßstab des Originals werden die Bilder so verkleinert, dass jedes (auch das gewählte,
//! ×1,3) in die Ringbreite passt und seine Nachbarn nicht berührt; jedes Bild ist um die Mitte seines Umrisses
//! (einschließlich der Anschlussnummern) zentriert, nicht um den Ursprung bzw. Pin 0.

use crate::app::App;
use crate::canvas::{self, Xf};
use crate::model::P;
use crate::symbols::{self, SymDef};
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

/// Durchmesser des Rades in Punkten (Bildschirmfoto des Originals: 248 px)
pub const DURCHMESSER: f32 = 248.0;
/// Bilder höchstens (Original: 12)
pub const MAX_BILDER: usize = 12;
/// größte Abmessung eines Bildes auf dem Ring in Punkten (Original: 70)
pub const BILD_PX: f32 = 70.0;
/// Vergrößerung des gewählten Bildes (Original: 1,3)
pub const AUSWAHL_FAKTOR: f32 = 1.3;
/// Ringfarbe (Bildschirmfoto des Originals)
pub const FARBE: Color32 = Color32::from_rgb(173, 204, 238);

/// Offenes Variantenrad.
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    /// Bibliothekssymbol
    pub key: String,
    /// Absetzstelle auf dem Blatt (mm, wie beim Platzieren ohne Rad)
    pub at: P,
    /// Zahl der Bilder auf dem Ring
    pub n: usize,
    /// gewähltes Bild
    pub sel: usize,
    /// kleinster bisheriger Mausabstand² zur Mitte (None = noch keine Bewegung ausgewertet)
    pub min_d2: Option<f32>,
}

/// Ergebnis einer Mausbewegung über dem Rad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bewegung {
    Offen,
    /// Maus hat das Rad verlassen → mit Bild 0 setzen
    Verlassen,
}

pub fn aussen_r() -> f32 {
    DURCHMESSER / 2.0
}

/// Lochradius: Breite − 2·(Breite/3), halbiert
pub fn innen_r() -> f32 {
    (DURCHMESSER - 2.0 * (DURCHMESSER / 3.0)) / 2.0
}

/// Abstand der Bildmitten von der Radmitte (Mitte des Ringes)
pub fn bild_r() -> f32 {
    (aussen_r() + innen_r()) / 2.0
}

/// Lage der Bildmitten relativ zur Radmitte (Bildschirm, y nach unten).
pub fn bild_lagen(n: usize) -> Vec<Vec2> {
    let n = n.min(MAX_BILDER);
    if n == 0 {
        return vec![];
    }
    let step = if n == 3 { std::f32::consts::PI / 2.0 } else { std::f32::consts::TAU / n as f32 };
    let r = bild_r();
    (0..n)
        .map(|i| {
            let a = -std::f32::consts::PI + step * i as f32;
            Vec2::new(r * a.cos(), r * a.sin())
        })
        .collect()
}

/// Zahl der Bilder eines Symbols auf dem Rad (0 = kein Rad).
pub fn bilder(def: &SymDef) -> usize {
    def.options.len().min(MAX_BILDER)
}

impl Ring {
    pub fn new(key: String, at: P, n: usize) -> Self {
        Ring { key, at, n: n.min(MAX_BILDER), sel: 0, min_d2: None }
    }

    /// Mausbewegung: `d` = Maus relativ zur Radmitte.
    pub fn bewegen(&mut self, d: Vec2) -> Bewegung {
        let d2 = d.length_sq();
        let min = match self.min_d2 {
            Some(m) => m.min(d2),
            None => d2,
        };
        self.min_d2 = Some(min);
        if d2 > aussen_r() * aussen_r() {
            if d2 > min + 5000.0 {
                return Bewegung::Verlassen;
            }
            return Bewegung::Offen;
        }
        self.sel = if d2 <= innen_r() * innen_r() {
            0
        } else {
            bild_lagen(self.n)
                .iter()
                .enumerate()
                .min_by(|a, b| (d - *a.1).length_sq().total_cmp(&(d - *b.1).length_sq()))
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        Bewegung::Offen
    }
}

/// Umriss (min, max) eines Bildes einschließlich der Textfelder (Anschlussnummern u. ä.), damit das Bild
/// um seine sichtbare Mitte zentriert wird (nicht um den Ursprung/Pin 0).
pub fn umriss(def: &SymDef, opt: u16) -> (P, P) {
    // wie gezeichnet: ohne Zusatzoptionen (z. B. Hilfskontakt rechts neben den Polen des Leistungskontakts)
    let (mut a, mut b) = symbols::bounds_ext(def, opt, Some(0));
    for i in 0..def.prims.len() {
        if let symbols::Prim::TextBox(p, q, s, _) = &*symbols::prim_lage(def, i, opt) {
            if s.is_empty() || !symbols::visible(opt, def.vis.get(i)) || !symbols::flag_visible(0, def.flag.get(i)) {
                continue;
            }
            for r in [p, q] {
                a = P::new(a.x.min(r.x), a.y.min(r.y));
                b = P::new(b.x.max(r.x), b.y.max(r.y));
            }
        }
    }
    (a, b)
}

/// Halbmesser des Kreises, in den jedes (nicht gewählte) Bild auf dem Ring passen muss: das gewählte Bild
/// (×1,3) bleibt innerhalb der Ringbreite und berührt seine Nachbarn nicht.
pub fn bild_platz(n: usize) -> f32 {
    let n = n.min(MAX_BILDER);
    let breite = aussen_r() - innen_r();
    let mut rho = breite / 2.0 / AUSWAHL_FAKTOR;
    if n >= 2 {
        let step = if n == 3 { std::f32::consts::PI / 2.0 } else { std::f32::consts::TAU / n as f32 };
        let sehne = 2.0 * bild_r() * (step / 2.0).sin();
        rho = rho.min(sehne / (1.0 + AUSWAHL_FAKTOR));
    }
    // etwas Luft zum Rand und zwischen den Bildern
    0.92 * rho
}

/// Maßstab (Punkte je mm) für die Bilder auf dem Ring aus den Umrissen aller Bilder: das Bild mit der
/// größten halben Diagonale füllt den Platz `bild_platz`, höchstens jedoch 70 Punkte größte Abmessung
/// (Original). Alle Bilder erhalten denselben Maßstab.
pub fn massstab_umrisse(umrisse: &[(P, P)]) -> f32 {
    let mut diag = 1e-3f32;
    let mut gr = 1e-3f32;
    for (a, b) in umrisse {
        let (w, h) = (b.x - a.x, b.y - a.y);
        diag = diag.max((w * w + h * h).sqrt() / 2.0);
        gr = gr.max(w).max(h);
    }
    (bild_platz(umrisse.len()) / diag).min(BILD_PX / gr)
}

/// Lage der Bilder auf dem Ring als Bildschirmrechtecke relativ zur Radmitte (Punkte); das gewählte Bild
/// ist ×1,3 vergrößert. Jedes Rechteck ist um seine Lage auf dem Ring zentriert.
pub fn bild_rechtecke(umrisse: &[(P, P)], sel: usize) -> Vec<egui::Rect> {
    let s = massstab_umrisse(umrisse);
    bild_lagen(umrisse.len())
        .into_iter()
        .zip(umrisse)
        .enumerate()
        .map(|(i, (d, (a, b)))| {
            let z = if i == sel { s * AUSWAHL_FAKTOR } else { s };
            egui::Rect::from_center_size(Pos2::new(d.x, d.y), Vec2::new((b.x - a.x) * z, (b.y - a.y) * z))
        })
        .collect()
}

/// Rad zeichnen (Ring, Bilder und im Loch das gewählte Bild, um seine Mitte zentriert).
/// `zoom` = Maßstab des Blattes (Punkte je mm), `vorschau` = Farbe des Bildes im Loch.
pub fn zeichnen(p: &egui::Painter, c: Pos2, def: &SymDef, ring: &Ring, zoom: f32, vorschau: Color32) {
    // Kreisring als Dreiecksnetz (genaue Innen- und Außenkante); das Loch bleibt durchsichtig wie im Original
    let mut mesh = egui::Mesh::default();
    let seg = 128u32;
    for k in 0..=seg {
        let a = std::f32::consts::TAU * k as f32 / seg as f32;
        let e = Vec2::new(a.cos(), a.sin());
        mesh.colored_vertex(c + e * innen_r(), FARBE);
        mesh.colored_vertex(c + e * aussen_r(), FARBE);
        if k > 0 {
            let i = 2 * k;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    let umrisse: Vec<(P, P)> = (0..ring.n).map(|i| umriss(def, i as u16)).collect();
    let s = massstab_umrisse(&umrisse);
    for (i, d) in bild_lagen(ring.n).into_iter().enumerate() {
        let gew = i == ring.sel;
        let z = if gew { s * AUSWAHL_FAKTOR } else { s };
        bild(p, c + d, def, i as u16, umrisse[i], z, Stroke::new(if gew { 2.0 } else { 1.0 }, Color32::BLACK));
    }
    // im Loch: das gewählte Bild, in Originalgröße wie beim Platzieren, bei Bedarf auf das Loch verkleinert
    if let Some(u) = umrisse.get(ring.sel) {
        let (w, h) = (u.1.x - u.0.x, u.1.y - u.0.y);
        let halb = (w * w + h * h).sqrt().max(1e-3) / 2.0;
        let z = (0.9 * innen_r() / halb).min(zoom);
        bild(p, c, def, ring.sel as u16, *u, z, Stroke::new(1.0, vorschau));
    }
}

/// Ein Bild mit Maßstab `z` (Punkte je mm) um die Mitte seines Umrisses bei `ziel` zeichnen.
fn bild(p: &egui::Painter, ziel: Pos2, def: &SymDef, opt: u16, (a, b): (P, P), z: f32, st: Stroke) {
    let mitte = P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let xf = Xf { origin: Pos2::new(ziel.x - mitte.x * z, ziel.y - mitte.y * z), z };
    canvas::draw_symbol(p, xf, def, P::new(0.0, 0.0), 0, false, false, 1.0, opt, st);
}

impl App {
    /// Eingaben bei offenem Variantenrad (modal wie im Original). Liefert true, solange das Rad die Eingabe
    /// verbraucht hat.
    pub fn ring_input(&mut self, ui: &egui::Ui, xf: Xf) -> bool {
        let Some(mut ring) = self.ring.take() else { return false };
        let (hover, press_l, press_r, esc) = ui.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_pressed(egui::PointerButton::Secondary),
                i.key_pressed(egui::Key::Escape),
            )
        });
        // Esc und Rechtsklick brechen das Platzieren ab (im Original nicht vorgesehen)
        if esc || press_r {
            self.placing = None;
            self.lib_drag = false;
            return true;
        }
        let c = xf.s(ring.at);
        let mut setzen = false;
        if let Some(h) = hover {
            if ring.bewegen(h - c) == Bewegung::Verlassen {
                ring.sel = 0;
                setzen = true;
            }
        }
        if press_l {
            setzen = true;
        }
        if setzen {
            self.place_symbol_opt(ring.key.clone(), ring.at, ring.sel as u16);
            self.placing = None;
            self.lib_drag = false;
            self.tool = crate::app::Tool::Select;
            self.drag = crate::app::Drag::None;
        } else {
            self.ring = Some(ring);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lagen_wie_original() {
        // drei Bilder: links, oben, rechts (Bildschirmfoto "Sicherung" 1-, 2-, 3-polig)
        let l = bild_lagen(3);
        let r = bild_r();
        assert!((l[0] - Vec2::new(-r, 0.0)).length() < 1e-3);
        assert!((l[1] - Vec2::new(0.0, -r)).length() < 1e-3);
        assert!((l[2] - Vec2::new(r, 0.0)).length() < 1e-3);
        // vier Bilder: links, oben, rechts, unten
        let l = bild_lagen(4);
        assert!((l[3] - Vec2::new(0.0, r)).length() < 1e-3);
        assert_eq!(bild_lagen(20).len(), MAX_BILDER);
        assert!(bild_lagen(0).is_empty());
        // Maße aus dem Bildschirmfoto: Loch ≈ 83 px, Bildmitten ≈ 82 px von der Mitte
        assert!((innen_r() * 2.0 - 82.7).abs() < 1.0);
        assert!((bild_r() - 82.7).abs() < 1.0);
    }

    #[test]
    fn auswahl_und_verlassen() {
        let mut ring = Ring::new("x".into(), P::new(0.0, 0.0), 3);
        assert_eq!(ring.bewegen(Vec2::ZERO), Bewegung::Offen);
        assert_eq!(ring.sel, 0);
        assert_eq!(ring.bewegen(Vec2::new(0.0, -80.0)), Bewegung::Offen);
        assert_eq!(ring.sel, 1);
        assert_eq!(ring.bewegen(Vec2::new(90.0, 10.0)), Bewegung::Offen);
        assert_eq!(ring.sel, 2);
        // zurück ins Loch: Bild 0
        assert_eq!(ring.bewegen(Vec2::new(5.0, 5.0)), Bewegung::Offen);
        assert_eq!(ring.sel, 0);
        // knapp außerhalb (Abstand² nicht 5000 über dem Minimum) bleibt offen, weit draußen schließt
        let mut ring = Ring::new("x".into(), P::new(0.0, 0.0), 3);
        assert_eq!(ring.bewegen(Vec2::new(125.0, 0.0)), Bewegung::Offen);
        assert_eq!(ring.bewegen(Vec2::new(140.0, 0.0)), Bewegung::Offen);
        assert_eq!(ring.bewegen(Vec2::new(150.0, 0.0)), Bewegung::Verlassen);
    }

    #[test]
    fn bilder_im_ring_ohne_ueberlappung() {
        // Umrisse wie Schließer, Sicherung, breite Klemmleiste, Punkt
        let formen = [(4.0, 10.0), (2.0, 8.0), (30.0, 3.0), (0.1, 0.1), (12.0, 12.0), (6.0, 20.0)];
        for n in 1..=MAX_BILDER {
            let umrisse: Vec<(P, P)> = (0..n)
                .map(|i| {
                    let (w, h) = formen[i % formen.len()];
                    // Ursprung absichtlich außerhalb der Mitte (Pin 0 oben)
                    (P::new(-1.0, 0.0), P::new(w - 1.0, h))
                })
                .collect();
            for sel in 0..n {
                let r = bild_rechtecke(&umrisse, sel);
                assert_eq!(r.len(), n);
                for (i, a) in r.iter().enumerate() {
                    // um die Lage auf dem Ring zentriert
                    assert!((a.center().to_vec2() - bild_lagen(n)[i]).length() < 1e-3);
                    for e in [a.left_top(), a.right_top(), a.left_bottom(), a.right_bottom()] {
                        let d = e.to_vec2().length();
                        assert!(d <= aussen_r() && d >= innen_r(), "n={n} sel={sel} Bild {i}: Ecke {d} außerhalb des Rings");
                    }
                    for b in &r[i + 1..] {
                        assert!(!a.intersects(*b), "n={n} sel={sel}: Bilder überlappen");
                    }
                }
            }
        }
        // wenige Bilder bleiben groß (Original: 70 Punkte größte Abmessung als Obergrenze)
        let u = vec![(P::new(0.0, 0.0), P::new(2.0, 8.0)); 4];
        let r = bild_rechtecke(&u, 1);
        assert!(r[0].height() > 40.0 && r[0].height() <= BILD_PX + 1e-3);
    }
}
