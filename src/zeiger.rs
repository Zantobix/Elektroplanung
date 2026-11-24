//! Mauszeiger der Zeichenfläche wie im Original: Pfeil mit einem Zusatz, der zeigt, was ein Druck bewirkt.
//!
//! Das Original lädt dafür eigene Zeiger (Pfeil + Zeichen) und wählt sie am Ende seiner MouseMove-Routine aus der
//! Vormarkierung, dem Werkzeug und dem Ziehzustand. Übernommen sind nur Form und Bedeutung; die Zusätze sind eigene
//! Vektorzeichnungen rechts unter der Pfeilspitze (egui kennt nur die Systemzeiger). Unterschiede des Originals:
//! - Auswahlwerkzeug über einem Objekt (Symbol, Leitung, Text, Linie …): Pfeil mit Pfeilkreuz (verschieben);
//!   über einem Punkt eines Zeichenobjekts (Ende einer Linie, Ecke eines Rechtecks, Kurvenpunkt): Pfeil mit vier
//!   Pfeilspitzen (Original Zeiger 5 = Ressource 2, 0x415f30 Treffer 6 mit Punktindex; zeichen_punkt.rs)
//! - über Titel/Wert eines Symbols und den Texten von Verbindungen (Querverweis einer Sammelschiene): Pfeil mit „A“
//! - über einem Anschluss, einer Sammelschiene oder einem Verweis (Leitung beginnt hier): Pfeil mit Leitungszeichen;
//!   mit Strg zählt ein Anschluss nicht (Pfeilkreuz, das Symbol wird verschoben)
//! - beim Ziehen einer Leitung nach mehr als 5 Bildpunkten: Pfeil mit Schrägpfeil nach rechts oben bzw. – wenn die
//!   Maus nach links oder oben gezogen wird – nach rechts unten
//! - über einem Eckpunkt (Größe ändern): Pfeil mit vier einzelnen Pfeilspitzen; über dem Griff des Rahmens zur
//!   Größenänderung: Pfeil mit Doppelpfeil
//! - Klemmstellen wie Sammelschienen und Verweise (Sonderobjekt Art 0–2, 0x415e9e): Leitungszeichen;
//!   Kontaktspiegel: einfacher Pfeil; fixiertes Objekt bei aktiver Fixierung: Pfeil mit Sperrkreis
//! - rechte Maustaste auf einem Objekt gedrückt (Kopie ziehen, rechts_ziehen.rs): Pfeil mit Plus
//! - Werkzeug Klemmstelle: Pfeil mit Klemme (über einer Leitung: Klemme in der Leitung); Zeichenwerkzeuge: Fadenkreuz
//! - Auswahl eines Symbols (Eigentümer setzen, Referenz): Hand über gültigem Ziel, sonst Sperrzeiger; Lupe: Lupe
//! - Verweis mit gedrückter Strg-Taste (Nachbau: Strg+Klick springt zum Ziel): Hand

use crate::app::{App, Drag, Tool};
use crate::canvas;
use crate::hover::{Objektart, Ziel};
use crate::model::*;
use eframe::egui::{self, Color32, CursorIcon, Pos2, Shape, Stroke, Vec2};

/// Richtung eines Doppelpfeils (Größe ändern).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Richtung {
    Waagerecht,
    Senkrecht,
    /// links oben – rechts unten
    NwSo,
    /// rechts oben – links unten
    NoSw,
}

/// Zusatz am Pfeil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zusatz {
    /// Pfeilkreuz: Objekt verschieben
    Verschieben,
    /// „A“ mit Pfeilen: Text verschieben
    Text,
    /// Leitungszeichen: hier beginnt eine Leitung
    Anschluss,
    /// Leitung wird gezogen: Schrägpfeil nach rechts oben bzw. (`nach_unten`) rechts unten
    LeitungZiehen { nach_unten: bool },
    /// vier einzelne Pfeilspitzen: Punkt verschieben
    Punkt,
    /// Doppelpfeil: Größe ändern
    Groesse(Richtung),
    /// Sperrkreis: fixiert
    Gesperrt,
    /// Klemmstellen-Werkzeug
    Klemme,
    /// Klemmstellen-Werkzeug über einer Leitung
    KlemmeInLeitung,
    /// Plus: Kopie ziehen (rechte Maustaste auf einem Objekt, rechts_ziehen.rs)
    Kopieren,
}

/// Gewählter Zeiger: Systemzeiger und Zusatz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zeigerwahl {
    pub system: CursorIcon,
    pub zusatz: Option<Zusatz>,
}

impl Zeigerwahl {
    fn pfeil(z: Zusatz) -> Self {
        Zeigerwahl { system: CursorIcon::Default, zusatz: Some(z) }
    }
    fn nur(system: CursorIcon) -> Self {
        Zeigerwahl { system, zusatz: None }
    }
}

/// Werkzeug für die Zeigerwahl.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Werkzeug {
    Auswahl,
    Klemmstelle,
    /// Linie, Kurve, Rechteck, Dreieck, Kreis, Text, Textbox, Tabelle, Bild
    Zeichnen,
    Lupe,
    /// Symbol aus der Bibliothek absetzen
    Platzieren,
}

/// Laufender Ziehvorgang.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ziehen {
    Nichts,
    Verschieben,
    Beschriftung,
    Griff(usize),
    Ecke,
    /// Leitung ziehen: Abstand der Maus vom Anfang in Bildpunkten
    Leitung { dx: f32, dy: f32 },
    Aufziehen,
    Schieben,
    /// rechte Maustaste auf einem Objekt gedrückt: Kopie ziehen
    Kopieren,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zustand {
    pub werkzeug: Werkzeug,
    pub ziehen: Ziehen,
    pub ziel: Option<Ziel>,
    /// Auswahlmodus (Eigentümer setzen, Kontaktspiegel-Referenz): Some(gültiges Ziel unter der Maus)
    pub waehlen: Option<bool>,
    pub strg: bool,
    /// fixiertes Objekt unter der Maus bei aktiver Fixierung
    pub gesperrt: bool,
    /// Leitung unter der Maus (Werkzeug Klemmstelle)
    pub ueber_leitung: bool,
    /// Auswahlwerkzeug über einem inneren Knick der Leitung unter der Maus (0x415f10: Treffer 2 mit F+0x954)
    pub knick: bool,
}

/// Richtung des Doppelpfeils für Griff 0–7 des Rahmens (links oben im Uhrzeigersinn).
pub fn griff_richtung(i: usize) -> Richtung {
    match i % 8 {
        0 | 4 => Richtung::NwSo,
        2 | 6 => Richtung::NoSw,
        1 | 5 => Richtung::Senkrecht,
        _ => Richtung::Waagerecht,
    }
}

/// Zeiger für den Zustand – Reihenfolge der Prüfungen wie im Original.
pub fn waehlen(z: &Zustand) -> Zeigerwahl {
    use Zusatz::*;
    if z.werkzeug == Werkzeug::Lupe {
        return Zeigerwahl::nur(CursorIcon::ZoomIn);
    }
    if let Some(ok) = z.waehlen {
        return Zeigerwahl::nur(if ok { CursorIcon::PointingHand } else { CursorIcon::NotAllowed });
    }
    match z.ziehen {
        Ziehen::Schieben => return Zeigerwahl::nur(CursorIcon::Move),
        Ziehen::Verschieben | Ziehen::Beschriftung => return Zeigerwahl::pfeil(Verschieben),
        Ziehen::Griff(i) => return Zeigerwahl::pfeil(Groesse(griff_richtung(i))),
        Ziehen::Ecke => return Zeigerwahl::pfeil(Punkt),
        Ziehen::Leitung { dx, dy } => {
            // Original: erst ab 5 Bildpunkten Abstand; nach links/oben gezogen der Pfeil nach rechts unten
            return Zeigerwahl::pfeil(if dx.abs() > 5.0 || dy.abs() > 5.0 { LeitungZiehen { nach_unten: dx < -5.0 || dy < -5.0 } } else { Anschluss });
        }
        Ziehen::Aufziehen => return Zeigerwahl::nur(CursorIcon::Default),
        // Original: Zeiger 3 (Ressource 5, Pfeil mit Plus), solange die rechte Taste auf einem Objekt gedrückt ist
        Ziehen::Kopieren => return Zeigerwahl::pfeil(Kopieren),
        Ziehen::Nichts => {}
    }
    match z.werkzeug {
        Werkzeug::Klemmstelle => return Zeigerwahl::pfeil(if z.ueber_leitung { KlemmeInLeitung } else { Klemme }),
        Werkzeug::Zeichnen => return Zeigerwahl::nur(CursorIcon::Crosshair),
        Werkzeug::Platzieren | Werkzeug::Lupe => return Zeigerwahl::nur(CursorIcon::Default),
        Werkzeug::Auswahl => {}
    }
    let Some(ziel) = z.ziel else {
        return if z.gesperrt { Zeigerwahl::pfeil(Gesperrt) } else { Zeigerwahl::nur(CursorIcon::Default) };
    };
    match ziel {
        Ziel::Griff(i) => Zeigerwahl::pfeil(Groesse(griff_richtung(i))),
        Ziel::Ecke => Zeigerwahl::pfeil(Punkt),
        Ziel::Anschluss if z.strg => Zeigerwahl::pfeil(Verschieben),
        Ziel::Anschluss => Zeigerwahl::pfeil(Anschluss),
        Ziel::Beschriftung { .. } => Zeigerwahl::pfeil(Text),
        // eigenständige Texte sind im Original Zeichenobjekte: Pfeilkreuz wie Linie oder Rechteck
        Ziel::Text { .. } => Zeigerwahl::pfeil(Verschieben),
        Ziel::Verweis { .. } if z.strg => Zeigerwahl::nur(CursorIcon::PointingHand),
        // Querverweistext einer Schiene: kein Verschiebe-/Textzeiger (Nutzer 24.11.2026), Hinweis Strg+Klick bleibt
        Ziel::Verweis { text: true, .. } => Zeigerwahl::nur(CursorIcon::Default),
        Ziel::Verweis { .. } => Zeigerwahl::pfeil(Anschluss),
        // Knick einer Leitung: Zeiger 5 wie an den Punkten der Zeichenobjekte (knick_ziehen.rs)
        Ziel::Leitung { .. } if z.knick => Zeigerwahl::pfeil(Punkt),
        Ziel::Objekt { art: Objektart::Verschiebbar, .. } | Ziel::Leitung { .. } => Zeigerwahl::pfeil(Verschieben),
        Ziel::Objekt { art: Objektart::Anschluss, .. } => Zeigerwahl::pfeil(Anschluss),
        Ziel::Objekt { art: Objektart::Schlicht, .. } => Zeigerwahl::nur(CursorIcon::Default),
    }
}

impl App {
    /// Zustand für die Zeigerwahl an der Maus `m` (mm); `ziel` = Vormarkierung des Auswahlwerkzeugs.
    pub fn zeiger_zustand(&self, m: P, tol: f32, z: f32, ziel: Option<Ziel>, strg: bool) -> Zustand {
        let symbol_unter = || self.sheet().is_some_and(|s| s.objects.iter().rev().any(|o| matches!(o.kind, Kind::Symbol { .. }) && canvas::hit(o, m, tol, &self.lib)));
        let waehlen = (self.owner_pick.is_some() || self.spiegel_pick_aktiv()).then(symbol_unter);
        let werkzeug = if self.zoom_tool {
            Werkzeug::Lupe
        } else if self.placing.is_some() || self.ring.is_some() {
            Werkzeug::Platzieren
        } else {
            match self.tool {
                Tool::Select => Werkzeug::Auswahl,
                Tool::Klemmstelle => Werkzeug::Klemmstelle,
                _ => Werkzeug::Zeichnen,
            }
        };
        let ziehen = match &self.drag {
            Drag::None => Ziehen::Nichts,
            Drag::Teilstueck { knick: true, .. } => Ziehen::Ecke,
            Drag::Move { .. } | Drag::Teilstueck { .. } => Ziehen::Verschieben,
            Drag::Beschriftung { .. } => Ziehen::Beschriftung,
            Drag::Scale { handle, .. } => Ziehen::Griff(*handle),
            Drag::Resize { .. } | Drag::Zeichenpunkt { .. } => Ziehen::Ecke,
            Drag::Wire { f } => { let start = f.start(); Ziehen::Leitung { dx: (m.x - start.x) * z, dy: (m.y - start.y) * z } },
            Drag::Rubber { .. } | Drag::Create { .. } | Drag::Kurve(_) => Ziehen::Aufziehen,
            Drag::KurvenGriff { .. } => Ziehen::Ecke,
            Drag::Pan { .. } => Ziehen::Schieben,
            Drag::Rechts { kopie: true, .. } => Ziehen::Kopieren,
            Drag::Rechts { .. } => Ziehen::Nichts,
        };
        let auswahl_frei = werkzeug == Werkzeug::Auswahl && ziehen == Ziehen::Nichts && ziel.is_none();
        let gesperrt = auswahl_frei && self.fix_active && self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.fixed && canvas::hit(o, m, tol, &self.lib)));
        let ueber_leitung = werkzeug == Werkzeug::Klemmstelle && self.sheet().is_some_and(|s| s.objects.iter().any(|o| matches!(o.kind, Kind::Wire { .. }) && canvas::hit(o, m, tol, &self.lib)));
        let knick = match ziel {
            Some(Ziel::Leitung { id }) if werkzeug == Werkzeug::Auswahl && !strg => self.knick_unter(id, m, tol).is_some(),
            _ => false,
        };
        Zustand { werkzeug, ziehen, ziel, waehlen, strg, gesperrt, ueber_leitung, knick }
    }
}

fn speicher_id() -> egui::Id {
    egui::Id::unique("ep_zeiger")
}

/// Zeiger dieses Bildes setzen (None = keiner, z. B. Maus außerhalb der Zeichenfläche).
pub fn setzen(ctx: &egui::Context, wahl: Option<Zeigerwahl>) {
    ctx.data_mut(|d| d.insert_temp(speicher_id(), wahl));
    if let Some(w) = wahl.filter(|w| w.system != CursorIcon::Default) {
        ctx.set_cursor_icon(w.system);
    }
}

/// Zusatz des gemerkten Zeigers rechts unter der Pfeilspitze zeichnen.
pub fn zeichnen(p: &egui::Painter, zeiger: Pos2, ctx: &egui::Context) {
    let Some(z) = ctx.data(|d| d.get_temp::<Option<Zeigerwahl>>(speicher_id())).flatten().and_then(|w| w.zusatz) else { return };
    zusatz(p, zeiger + Vec2::new(15.0, 16.0), z);
}

/// Linienzug weiß umrandet (wie die Systemzeiger) und schwarz.
fn zug(p: &egui::Painter, pts: &[Pos2]) {
    for st in [Stroke::new(3.0, Color32::WHITE), Stroke::new(1.0, Color32::BLACK)] {
        p.add(Shape::line(pts.to_vec(), st));
    }
}

/// Gefüllte Pfeilspitze bei `tip` in Richtung `d` (Einheitsvektor).
fn spitze(p: &egui::Painter, tip: Pos2, d: Vec2, s: f32) {
    let q = Vec2::new(-d.y, d.x);
    let pts = vec![tip, tip - d * s * 1.2 + q * s, tip - d * s * 1.2 - q * s];
    p.add(Shape::convex_polygon(pts, Color32::BLACK, Stroke::new(1.0, Color32::WHITE)));
}

/// Doppelpfeil durch `c` in Richtung `d` mit halber Länge `l`.
fn doppelpfeil(p: &egui::Painter, c: Pos2, d: Vec2, l: f32) {
    zug(p, &[c - d * (l - 1.5), c + d * (l - 1.5)]);
    spitze(p, c + d * l, d, 2.5);
    spitze(p, c - d * l, -d, 2.5);
}

/// Eigene Vektorzeichnung des Zusatzes um den Mittelpunkt `c`.
pub fn zusatz(p: &egui::Painter, c: Pos2, z: Zusatz) {
    let (x, y) = (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0));
    match z {
        Zusatz::Verschieben => {
            // kräftiger als die übrigen Zusätze (Nutzer 24.11.2026: Pfeile nach rechts/links/oben/unten kaum zu
            // erkennen): breitere Striche, größere Spitzen
            for st in [Stroke::new(5.0, Color32::WHITE), Stroke::new(2.5, Color32::BLACK)] {
                p.add(Shape::line_segment([c - x * 6.0, c + x * 6.0], st));
                p.add(Shape::line_segment([c - y * 6.0, c + y * 6.0], st));
            }
            for d in [x, -x, y, -y] {
                spitze(p, c + d * 9.5, d, 3.4);
            }
        }
        Zusatz::Text => {
            let font = egui::FontId::new(12.0, egui::FontFamily::Name("bold".into()));
            for d in [-x, x, -y, y] {
                p.text(c + d, egui::Align2::CENTER_CENTER, "A", font.clone(), Color32::WHITE);
            }
            p.text(c, egui::Align2::CENTER_CENTER, "A", font, Color32::BLACK);
            // kleine Pfeilspitzen in alle vier Richtungen (Text verschieben)
            for d in [x, -x, y, -y] {
                spitze(p, c + d * 9.5, d, 1.8);
            }
        }
        Zusatz::Anschluss => {
            // Anschlusspunkt mit abgehender Leitung (Knick nach unten) und Pfeilspitze
            let a = c + Vec2::new(-5.0, -4.0);
            zug(p, &[a + x * 2.0, c + Vec2::new(3.0, -4.0), c + Vec2::new(3.0, 3.0)]);
            spitze(p, c + Vec2::new(3.0, 6.5), y, 2.2);
            p.circle(a, 2.2, Color32::WHITE, Stroke::new(1.5, Color32::BLACK));
        }
        Zusatz::LeitungZiehen { nach_unten } => {
            let s = c + Vec2::new(-4.0, 0.0);
            zug(p, &[s - y * 6.0, s + y * 6.0]);
            let d = if nach_unten { Vec2::new(0.707, 0.707) } else { Vec2::new(0.707, -0.707) };
            zug(p, &[s, s + d * 8.0]);
            spitze(p, s + d * 10.5, d, 2.2);
        }
        Zusatz::Punkt => {
            // Raute in der Mitte wie Ressource 2 des Originals (Pfeil, Raute, vier Pfeilspitzen)
            let raute = vec![c - y * 3.0, c + x * 3.0, c + y * 3.0, c - x * 3.0];
            p.add(Shape::convex_polygon(raute, Color32::BLACK, Stroke::new(1.0, Color32::WHITE)));
            for d in [x, -x, y, -y] {
                spitze(p, c + d * 7.5, d, 2.2);
            }
        }
        Zusatz::Groesse(r) => {
            let d = match r {
                Richtung::Waagerecht => x,
                Richtung::Senkrecht => y,
                Richtung::NwSo => Vec2::new(0.707, 0.707),
                Richtung::NoSw => Vec2::new(0.707, -0.707),
            };
            doppelpfeil(p, c, d, 7.0);
        }
        Zusatz::Gesperrt => {
            for st in [Stroke::new(3.5, Color32::WHITE), Stroke::new(1.5, Color32::BLACK)] {
                p.circle_stroke(c, 5.0, st);
                p.line_segment([c + Vec2::new(-3.5, 3.5), c + Vec2::new(3.5, -3.5)], st);
            }
        }
        Zusatz::Kopieren => {
            // Pluszeichen in einem weißen Kästchen
            p.rect(egui::Rect::from_center_size(c, Vec2::splat(11.0)), 0.0, Color32::WHITE, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Middle);
            let st = Stroke::new(2.0, Color32::BLACK);
            p.line_segment([c - x * 3.5, c + x * 3.5], st);
            p.line_segment([c - y * 3.5, c + y * 3.5], st);
        }
        Zusatz::Klemme | Zusatz::KlemmeInLeitung => {
            if z == Zusatz::KlemmeInLeitung {
                zug(p, &[c - x * 8.0, c + x * 8.0]);
            } else {
                zug(p, &[c - y * 7.0, c + y * 7.0]);
            }
            p.circle(c, 3.0, Color32::WHITE, Stroke::new(1.5, Color32::BLACK));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zustand(ziel: Option<Ziel>) -> Zustand {
        Zustand { werkzeug: Werkzeug::Auswahl, ziehen: Ziehen::Nichts, ziel, waehlen: None, strg: false, gesperrt: false, ueber_leitung: false, knick: false }
    }

    fn objekt(art: Objektart) -> Option<Ziel> {
        Some(Ziel::Objekt { id: 1, mn: P::default(), mx: P::new(1.0, 1.0), art })
    }

    #[test]
    fn auswahl_ueber_objekten() {
        let z = |zi| waehlen(&zustand(zi)).zusatz;
        assert_eq!(z(objekt(Objektart::Verschiebbar)), Some(Zusatz::Verschieben));
        assert_eq!(z(objekt(Objektart::Anschluss)), Some(Zusatz::Anschluss));
        assert_eq!(z(objekt(Objektart::Schlicht)), None);
        assert_eq!(z(Some(Ziel::Beschriftung { id: 1, wert: true, mn: P::default(), mx: P::default() })), Some(Zusatz::Text));
        assert_eq!(z(Some(Ziel::Anschluss)), Some(Zusatz::Anschluss));
        assert_eq!(z(Some(Ziel::Ecke)), Some(Zusatz::Punkt));
        assert_eq!(z(Some(Ziel::Griff(2))), Some(Zusatz::Groesse(Richtung::NoSw)));
        assert_eq!(z(Some(Ziel::Griff(7))), Some(Zusatz::Groesse(Richtung::Waagerecht)));
        assert_eq!(waehlen(&zustand(None)), Zeigerwahl { system: CursorIcon::Default, zusatz: None });
        let mut s = zustand(None);
        s.gesperrt = true;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Gesperrt));
    }

    #[test]
    fn klemmstelle_leitungszeiger_zeichenobjekt_kreuz_und_raute() {
        // Original 0x415e9e: Sonderobjekt Art 0 (Klemmstelle) → Zeiger 2 (Leitung); 0x415f30: Zeichenobjekt nur mit
        // Punktindex → Zeiger 5 (Raute), Körper → Zeiger 100 (Verschiebekreuz)
        use crate::model::{Obj, Project, SheetRef};
        let mut app = crate::blatt_tasten::test_app();
        app.project = Project::new_default();
        app.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        app.active = 0;
        let obj = |id, kind| Obj { id, kind, pen: 0, fixed: false, group: None, raw: None };
        let klemme = |pos| Kind::Terminal { pos, leiste: "X1".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() };
        // freie Klemmstelle und eine in eine Leitung eingesetzte
        let (frei, drin) = (P::new(40.0, 40.0), P::new(100.0, 60.0));
        let (l1, l2) = crate::klemme_leitung::teilen(&[P::new(80.0, 60.0), P::new(140.0, 60.0)], 0, drin);
        app.with_sheet(|s, _| {
            s.objects.push(obj(1, klemme(frei)));
            s.objects.push(obj(2, Kind::Wire { pts: l1, leitung: 0, bez: vec![], opts: 0 }));
            s.objects.push(obj(3, Kind::Wire { pts: l2, leitung: 0, bez: vec![], opts: 0 }));
            s.objects.push(obj(4, klemme(drin)));
            s.objects.push(obj(5, Kind::Rect { a: P::new(40.0, 100.0), b: P::new(80.0, 120.0) }));
        });
        let tol = 1.5;
        let zusatz = |m: P| waehlen(&app.zeiger_zustand(m, tol, 2.0, app.vormarkierung(m, tol, 2.0), false)).zusatz;
        for m in [frei, P::new(40.6, 40.4), drin, P::new(100.5, 60.3)] {
            assert_eq!(zusatz(m), Some(Zusatz::Anschluss), "{m:?}");
        }
        // Rechteck: Rand → Verschiebekreuz, Ecke → Raute
        assert_eq!(zusatz(P::new(60.0, 100.3)), Some(Zusatz::Verschieben));
        assert_eq!(zusatz(P::new(80.3, 119.8)), Some(Zusatz::Punkt));
        // Leitung abseits der Klemme: Verschiebekreuz ohne Vormarkierungsrechteck
        assert_eq!(zusatz(P::new(125.0, 60.2)), Some(Zusatz::Verschieben));
        assert!(app.vormarkierung(P::new(125.0, 60.2), tol, 2.0).and_then(|z| z.umriss()).is_none());
    }

    #[test]
    fn strg_und_verweise() {
        let verweis = |text| Some(Ziel::Verweis { id: 1, mn: P::default(), mx: P::default(), text });
        assert_eq!(waehlen(&zustand(verweis(false))).zusatz, Some(Zusatz::Anschluss));
        assert_eq!(waehlen(&zustand(verweis(true))), Zeigerwahl { system: CursorIcon::Default, zusatz: None });
        let mut s = zustand(verweis(false));
        s.strg = true;
        assert_eq!(waehlen(&s), Zeigerwahl { system: CursorIcon::PointingHand, zusatz: None });
        // Strg: ein Anschluss zählt nicht, das Symbol wird verschoben
        let mut s = zustand(Some(Ziel::Anschluss));
        s.strg = true;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Verschieben));
    }

    #[test]
    fn ziehen_und_werkzeuge() {
        let mut s = zustand(None);
        s.ziehen = Ziehen::Leitung { dx: 3.0, dy: 2.0 };
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Anschluss));
        s.ziehen = Ziehen::Leitung { dx: 20.0, dy: 2.0 };
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::LeitungZiehen { nach_unten: false }));
        s.ziehen = Ziehen::Leitung { dx: 20.0, dy: -8.0 };
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::LeitungZiehen { nach_unten: true }));
        s.ziehen = Ziehen::Griff(0);
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Groesse(Richtung::NwSo)));
        s.ziehen = Ziehen::Beschriftung;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Verschieben));
        s.ziehen = Ziehen::Schieben;
        assert_eq!(waehlen(&s).system, CursorIcon::Move);
        s.ziehen = Ziehen::Kopieren;
        assert_eq!(waehlen(&s), Zeigerwahl { system: CursorIcon::Default, zusatz: Some(Zusatz::Kopieren) });
        // Knick einer Leitung: Raute (Zeiger 5), sonst Pfeilkreuz; beim Ziehen am Knick ebenso
        let mut s = zustand(Some(Ziel::Leitung { id: 1 }));
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Verschieben));
        s.knick = true;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Punkt));
        let mut s = zustand(None);
        s.werkzeug = Werkzeug::Klemmstelle;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::Klemme));
        s.ueber_leitung = true;
        assert_eq!(waehlen(&s).zusatz, Some(Zusatz::KlemmeInLeitung));
        s.werkzeug = Werkzeug::Zeichnen;
        assert_eq!(waehlen(&s), Zeigerwahl { system: CursorIcon::Crosshair, zusatz: None });
        s.werkzeug = Werkzeug::Lupe;
        assert_eq!(waehlen(&s).system, CursorIcon::ZoomIn);
        let mut s = zustand(objekt(Objektart::Verschiebbar));
        s.waehlen = Some(true);
        assert_eq!(waehlen(&s).system, CursorIcon::PointingHand);
        s.waehlen = Some(false);
        assert_eq!(waehlen(&s).system, CursorIcon::NotAllowed);
    }
}
