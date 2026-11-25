//! Auswahlbefehle und Auswahlfilter (todo NR13, Nutzer 25.11.2026; Nachbau-Erweiterung, im Original nur Strg+A).
//!
//! - **Befehle** (Bearbeiten › Auswahl, Kontextmenü der freien Fläche, Tastenkürzel): alle Objekte einer Art auf dem
//!   Blatt, „Gleiche wie markiert“, „Auswahl umkehren“, „Verbundene Leitungen dazu“. Sie achten auf Fixierung und
//!   Sichtbarkeit wie Klick und Rahmen, aber nicht auf den Auswahlfilter: ein ausdrücklicher Befehl geht vor.
//! - **Filter** (nur für die laufende Sitzung, Nutzerentscheidung): Objektarten, die Klick, Rahmen und Strg+A nicht
//!   treffen. Die Statuszeile nennt einen aktiven Filter; die Optionen grauen Einstellungen aus, die er wirkungslos
//!   macht (Mehrfachklick auf Leitungen bei gefilterten Leitungen).

use crate::app::App;
use crate::model::*;

/// Objektart für Auswahl und Filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Art {
    Leitung,
    Symbol,
    Klemme,
    Schiene,
    Zeichnung,
    Text,
    Verweis,
    TabelleBild,
    Spiegel,
}

/// Alle Arten mit Anzeigenamen (Mehrzahl) für Menüs und Filter.
pub const ARTEN: &[(Art, &str)] = &[
    (Art::Leitung, "Leitungen"),
    (Art::Symbol, "Symbole"),
    (Art::Klemme, "Klemmstellen"),
    (Art::Schiene, "Sammelschienen"),
    (Art::Zeichnung, "Zeichenobjekte"),
    (Art::Text, "Texte und Textfelder"),
    (Art::Verweis, "Verweise"),
    (Art::TabelleBild, "Tabellen und Bilder"),
    (Art::Spiegel, "Kontaktspiegel"),
];

/// Art eines Objekts. Zeichenobjekte: Linie, Kurve, Rechteck, Dreieck, Kreis, Polygon.
pub fn art(k: &Kind) -> Art {
    match k {
        Kind::Wire { .. } => Art::Leitung,
        Kind::Symbol { .. } => Art::Symbol,
        Kind::Terminal { .. } => Art::Klemme,
        Kind::Busbar { .. } => Art::Schiene,
        Kind::Line { .. } | Kind::Curve { .. } | Kind::Rect { .. } | Kind::Triangle { .. } | Kind::Circle { .. } | Kind::Poly { .. } => Art::Zeichnung,
        Kind::Text { .. } => Art::Text,
        Kind::Verweis { .. } => Art::Verweis,
        Kind::Table { .. } | Kind::Image { .. } => Art::TabelleBild,
        Kind::Spiegel { .. } => Art::Spiegel,
    }
}

/// Name einer Art in Einzahl/Mehrzahl für die Statuszeile.
fn name(a: Art, n: usize) -> &'static str {
    let (eins, viele) = match a {
        Art::Leitung => ("Leitung", "Leitungen"),
        Art::Symbol => ("Symbol", "Symbole"),
        Art::Klemme => ("Klemmstelle", "Klemmstellen"),
        Art::Schiene => ("Sammelschiene", "Sammelschienen"),
        Art::Zeichnung => ("Zeichenobjekt", "Zeichenobjekte"),
        Art::Text => ("Text", "Texte"),
        Art::Verweis => ("Verweis", "Verweise"),
        Art::TabelleBild => ("Tabelle/Bild", "Tabellen/Bilder"),
        Art::Spiegel => ("Kontaktspiegel", "Kontaktspiegel"),
    };
    if n == 1 {
        eins
    } else {
        viele
    }
}

/// Zählung je Art: „12 Leitungen, 3 Symbole markiert“ (leer bei keiner Markierung).
pub fn zaehlung(s: &Sheet, ids: &[u64]) -> String {
    let mut n: std::collections::BTreeMap<Art, usize> = Default::default();
    for o in s.objects.iter().filter(|o| ids.contains(&o.id)) {
        *n.entry(art(&o.kind)).or_default() += 1;
    }
    if n.is_empty() {
        return String::new();
    }
    let teile: Vec<String> = n.iter().map(|(a, k)| format!("{k} {}", name(*a, *k))).collect();
    format!("{} markiert", teile.join(", "))
}

/// Auswahlfilter der Sitzung: gesperrte Arten.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    pub aus: Vec<Art>,
}

impl Filter {
    pub fn erlaubt(&self, k: &Kind) -> bool {
        !self.aus.contains(&art(k))
    }
    pub fn aktiv(&self) -> bool {
        !self.aus.is_empty()
    }
    pub fn umschalten(&mut self, a: Art) {
        if let Some(i) = self.aus.iter().position(|x| *x == a) {
            self.aus.remove(i);
        } else {
            self.aus.push(a);
        }
    }
    /// Hinweis für die Statuszeile, z. B. „Auswahlfilter: ohne Leitungen, Texte und Textfelder“.
    pub fn hinweis(&self) -> String {
        let namen: Vec<&str> = ARTEN.iter().filter(|(a, _)| self.aus.contains(a)).map(|(_, n)| *n).collect();
        if namen.is_empty() {
            String::new()
        } else {
            format!("Auswahlfilter: ohne {}", namen.join(", "))
        }
    }
}

/// Merkmal für „Gleiche wie markiert“: gleiche Art und bei Symbolen dasselbe Symbol, bei Leitungen dieselbe
/// Leitungsart, bei Zeichenobjekten derselbe Stift, bei Klemmstellen dieselbe Klemmleiste, bei Schienen derselbe Name.
fn merkmal(o: &Obj) -> (Art, String) {
    let a = art(&o.kind);
    let m = match &o.kind {
        Kind::Symbol { key, .. } => key.clone(),
        Kind::Wire { leitung, .. } => leitung.to_string(),
        Kind::Terminal { leiste, .. } => leiste.clone(),
        Kind::Busbar { name, .. } => name.clone(),
        _ if a == Art::Zeichnung => o.pen.to_string(),
        _ => String::new(),
    };
    (a, m)
}

/// Befehle; `id` wie in `kuerzel.rs` (Tastenkürzel) und in den Menüs.
pub const BEFEHLE: &[(&str, &str)] = &[
    ("ausw_leitungen", "Alle Leitungen"),
    ("ausw_symbole", "Alle Symbole"),
    ("ausw_klemmen", "Alle Klemmstellen"),
    ("ausw_schienen", "Alle Sammelschienen"),
    ("ausw_zeichnung", "Alle Zeichenobjekte"),
    ("ausw_texte", "Alle Texte und Textfelder"),
    ("ausw_verweise", "Alle Verweise"),
    ("ausw_tabellen", "Alle Tabellen und Bilder"),
    ("ausw_gleiche", "Gleiche wie markiert"),
    ("ausw_umkehren", "Auswahl umkehren"),
    ("ausw_verbunden", "Verbundene Leitungen dazu"),
];

fn befehl_art(id: &str) -> Option<Art> {
    Some(match id {
        "ausw_leitungen" => Art::Leitung,
        "ausw_symbole" => Art::Symbol,
        "ausw_klemmen" => Art::Klemme,
        "ausw_schienen" => Art::Schiene,
        "ausw_zeichnung" => Art::Zeichnung,
        "ausw_texte" => Art::Text,
        "ausw_verweise" => Art::Verweis,
        "ausw_tabellen" => Art::TabelleBild,
        _ => return None,
    })
}

/// Reine Auswahlrechnung (für Tests ohne Oberfläche): Ergebnis des Befehls `id` auf dem Blatt, `frei` = markierbar.
pub fn rechnen(s: &Sheet, lib: &[crate::symbols::SymDef], sel: &[u64], id: &str, frei: &dyn Fn(&Obj) -> bool) -> Option<Vec<u64>> {
    let frei_ids = || s.objects.iter().filter(|o| frei(o));
    if let Some(a) = befehl_art(id) {
        return Some(frei_ids().filter(|o| art(&o.kind) == a).map(|o| o.id).collect());
    }
    match id {
        "ausw_gleiche" => {
            let m: Vec<(Art, String)> = s.objects.iter().filter(|o| sel.contains(&o.id)).map(merkmal).collect();
            Some(frei_ids().filter(|o| m.contains(&merkmal(o))).map(|o| o.id).collect())
        }
        "ausw_umkehren" => Some(frei_ids().filter(|o| !sel.contains(&o.id)).map(|o| o.id).collect()),
        "ausw_verbunden" => {
            let mut neu: Vec<u64> = sel.to_vec();
            for id in sel {
                for n in crate::netz::netz_von(s, lib, *id) {
                    let ok = s.objects.iter().any(|o| o.id == n && matches!(o.kind, Kind::Wire { .. }) && frei(o));
                    if ok && !neu.contains(&n) {
                        neu.push(n);
                    }
                }
            }
            Some(neu)
        }
        _ => None,
    }
}

impl App {
    /// Untermenü „Auswahl“ (Bearbeiten-Menü und Kontextmenü der freien Fläche): Befehle mit Tastenkürzel, dazu der
    /// Auswahlfilter der Sitzung als Häkchen je Art.
    pub fn auswahl_menue(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;
        let hat_blatt = self.sheet().is_some();
        let hat_sel = !self.selection.is_empty();
        ui.menu_button("Auswahl", |ui| {
            for (id, n) in BEFEHLE {
                let braucht_sel = matches!(*id, "ausw_gleiche" | "ausw_verbunden");
                if *id == "ausw_gleiche" {
                    ui.separator();
                }
                let knopf = egui::Button::new(*n).shortcut_text(crate::kuerzel::belegung(&self.opt.tasten, id));
                if ui.add_enabled(hat_blatt && (hat_sel || !braucht_sel), knopf).clicked() {
                    self.auswahl_befehl(id);
                    ui.close();
                }
            }
            ui.separator();
            ui.menu_button("Filter (diese Sitzung)", |ui| {
                ui.label("Klick, Rahmen und Strg+A treffen:");
                for (a, n) in ARTEN {
                    let mut an = !self.zusatz.filter.aus.contains(a);
                    if ui.checkbox(&mut an, *n).changed() {
                        self.zusatz.filter.umschalten(*a);
                        // Markiertes, das der Filter jetzt ausschließt, bleibt nicht markiert
                        let f = self.zusatz.filter.clone();
                        if let Some(s) = self.sheet().cloned() {
                            self.selection.retain(|id| s.objects.iter().find(|o| o.id == *id).is_none_or(|o| f.erlaubt(&o.kind)));
                        }
                    }
                }
                ui.separator();
                if ui.add_enabled(self.zusatz.filter.aktiv(), egui::Button::new("Alle Arten")).clicked() {
                    self.zusatz.filter = Filter::default();
                    ui.close();
                }
            });
        });
    }

    /// Markierbar ohne Rücksicht auf den Auswahlfilter (für ausdrückliche Auswahlbefehle).
    pub fn markierbar_ohne_filter(&self, o: &Obj) -> bool {
        self.edit_visible(o.id) && !(o.fixed && self.fix_active)
    }

    /// Auswahlbefehl ausführen; false = kein Auswahlbefehl.
    pub fn auswahl_befehl(&mut self, id: &str) -> bool {
        let Some(s) = self.sheet() else { return befehl_art(id).is_some() || BEFEHLE.iter().any(|b| b.0 == id) };
        let frei = |o: &Obj| self.markierbar_ohne_filter(o);
        let Some(ids) = rechnen(s, &self.lib, &self.selection, id, &frei) else { return false };
        self.selection = ids;
        self.zusatz.teil = None;
        self.expand_groups();
        let z = self.sheet().map(|s| zaehlung(s, &self.selection)).unwrap_or_default();
        self.info = if z.is_empty() { "Nichts markiert".into() } else { z };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    fn blatt() -> Sheet {
        let mut s = Sheet::new(1);
        let w = |a: f32, b: f32, leitung: usize| Kind::Wire { pts: vec![P::new(a, 10.0), P::new(b, 10.0)], leitung, bez: vec![], opts: Default::default() };
        s.objects.push(obj(1, w(0.0, 10.0, 0)));
        s.objects.push(obj(2, w(10.0, 20.0, 0)));
        s.objects.push(obj(3, w(50.0, 60.0, 1)));
        s.objects.push(obj(4, Kind::Line { a: P::new(0.0, 50.0), b: P::new(9.0, 50.0) }));
        s.objects.push(obj(5, Kind::Rect { a: P::new(0.0, 60.0), b: P::new(9.0, 70.0) }));
        s
    }

    #[test]
    fn arten_gleiche_umkehren_verbunden() {
        let s = blatt();
        let alle = |_: &Obj| true;
        assert_eq!(rechnen(&s, &[], &[], "ausw_leitungen", &alle), Some(vec![1, 2, 3]));
        assert_eq!(rechnen(&s, &[], &[], "ausw_zeichnung", &alle), Some(vec![4, 5]));
        assert_eq!(rechnen(&s, &[], &[1], "ausw_gleiche", &alle), Some(vec![1, 2]));
        assert_eq!(rechnen(&s, &[], &[1, 4], "ausw_umkehren", &alle), Some(vec![2, 3, 5]));
        assert_eq!(rechnen(&s, &[], &[1], "ausw_verbunden", &alle), Some(vec![1, 2]));
        // fixierte/unsichtbare Objekte bleiben außen vor
        let ohne3 = |o: &Obj| o.id != 3;
        assert_eq!(rechnen(&s, &[], &[], "ausw_leitungen", &ohne3), Some(vec![1, 2]));
        assert_eq!(rechnen(&s, &[], &[], "unbekannt", &alle), None);
        assert_eq!(zaehlung(&s, &[1, 2, 4]), "2 Leitungen, 1 Zeichenobjekt markiert");
    }

    #[test]
    fn filter() {
        let mut f = Filter::default();
        assert!(!f.aktiv());
        f.umschalten(Art::Leitung);
        f.umschalten(Art::Text);
        assert!(!f.erlaubt(&blatt().objects[0].kind));
        assert!(f.erlaubt(&blatt().objects[3].kind));
        assert_eq!(f.hinweis(), "Auswahlfilter: ohne Leitungen, Texte und Textfelder");
        f.umschalten(Art::Leitung);
        assert_eq!(f.aus, vec![Art::Text]);
    }
}
