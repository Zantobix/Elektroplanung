//! Tastenkürzel: feste Liste der Befehle mit Vorgabe-Belegung (die des Originals, wo es eine hat: Strg+Z,
//! Strg+W, Strg+C/X/V, Entf, S = Strompfadausrichtung, A = Automatische Verbindungen), in Optionen › Tastenkürzel
//! änderbar. Schreibweise: "Strg+Umschalt+S", "Alt+Enter", "F2", "Delete", "PageDown" (Tastennamen wie egui).

use eframe::egui::{self, Key, Modifiers};
use std::collections::BTreeMap;

pub struct Befehl {
    pub id: &'static str,
    pub name: &'static str,
    pub vorgabe: &'static str,
}

pub const BEFEHLE: &[Befehl] = &[
    Befehl { id: "neu", name: "Neues Projekt", vorgabe: "Strg+N" },
    Befehl { id: "oeffnen", name: "Projekt öffnen", vorgabe: "Strg+O" },
    Befehl { id: "speichern", name: "Speichern", vorgabe: "Strg+S" },
    Befehl { id: "speichern_als", name: "Projekt speichern als", vorgabe: "Strg+Umschalt+S" },
    Befehl { id: "drucken", name: "Drucken", vorgabe: "Strg+P" },
    Befehl { id: "rueck", name: "Rückgängig", vorgabe: "Strg+Z" },
    Befehl { id: "vor", name: "Forwärts (Wiederholen)", vorgabe: "Strg+W" },
    Befehl { id: "vor2", name: "Wiederholen (zweite Belegung)", vorgabe: "Strg+Y" },
    Befehl { id: "loeschen", name: "Löschen", vorgabe: "Delete" },
    Befehl { id: "alles", name: "Alles markieren", vorgabe: "Strg+A" },
    Befehl { id: "eigenschaften", name: "Eigenschaften", vorgabe: "Alt+Enter" },
    Befehl { id: "suchen", name: "Suchen (alle Blätter)", vorgabe: "Strg+F" },
    Befehl { id: "drehen", name: "Drehen", vorgabe: "R" },
    Befehl { id: "spiegel_v", name: "Vertikal spiegeln", vorgabe: "Umschalt+V" },
    Befehl { id: "spiegel_h", name: "Horizontal spiegeln", vorgabe: "Umschalt+H" },
    Befehl { id: "gruppieren", name: "Gruppieren", vorgabe: "Strg+G" },
    Befehl { id: "aufloesen", name: "Gruppe auflösen", vorgabe: "Strg+Umschalt+G" },
    Befehl { id: "symbol_bearbeiten", name: "Symbol bearbeiten", vorgabe: "E" },
    Befehl { id: "einpassen", name: "Ganzes Blatt", vorgabe: "Strg+Num0" },
    Befehl { id: "zoom_in", name: "Vergrößern", vorgabe: "Strg+Plus" },
    Befehl { id: "zoom_out", name: "Verkleinern", vorgabe: "Strg+Minus" },
    Befehl { id: "blatt_neu", name: "Neues Blatt", vorgabe: "Strg+Umschalt+N" },
    Befehl { id: "blatt_vor", name: "Nächstes Blatt", vorgabe: "Strg+PageDown" },
    Befehl { id: "blatt_zurueck", name: "Voriges Blatt", vorgabe: "Strg+PageUp" },
    Befehl { id: "pfadfang", name: "Strompfadausrichtung ein/aus", vorgabe: "S" },
    Befehl { id: "autoverb", name: "Automatische Verbindungen ein/aus", vorgabe: "A" },
    Befehl { id: "gitter", name: "Gitter ein/aus", vorgabe: "G" },
    Befehl { id: "w_auswahl", name: "Werkzeug: Auswahl", vorgabe: "Escape" },
    Befehl { id: "w_linie", name: "Werkzeug: Linie", vorgabe: "L" },
    Befehl { id: "w_rechteck", name: "Werkzeug: Rechteck", vorgabe: "Umschalt+R" },
    Befehl { id: "w_kreis", name: "Werkzeug: Kreis", vorgabe: "K" },
    Befehl { id: "w_text", name: "Werkzeug: Text", vorgabe: "T" },
    Befehl { id: "w_klemme", name: "Werkzeug: Klemmstelle", vorgabe: "X" },
    Befehl { id: "fehlerbericht", name: "Fehlerbericht speichern", vorgabe: "F12" },
    // Auswahlbefehle (auswahl.rs, NR13): ohne Vorgabe außer „Auswahl umkehren“
    Befehl { id: "ausw_leitungen", name: "Auswahl: alle Leitungen", vorgabe: "" },
    Befehl { id: "ausw_symbole", name: "Auswahl: alle Symbole", vorgabe: "" },
    Befehl { id: "ausw_klemmen", name: "Auswahl: alle Klemmstellen", vorgabe: "" },
    Befehl { id: "ausw_schienen", name: "Auswahl: alle Sammelschienen", vorgabe: "" },
    Befehl { id: "ausw_zeichnung", name: "Auswahl: alle Zeichenobjekte", vorgabe: "" },
    Befehl { id: "ausw_texte", name: "Auswahl: alle Texte und Textfelder", vorgabe: "" },
    Befehl { id: "ausw_verweise", name: "Auswahl: alle Verweise", vorgabe: "" },
    Befehl { id: "ausw_tabellen", name: "Auswahl: alle Tabellen und Bilder", vorgabe: "" },
    Befehl { id: "ausw_gleiche", name: "Auswahl: gleiche wie markiert", vorgabe: "" },
    Befehl { id: "ausw_umkehren", name: "Auswahl umkehren", vorgabe: "Strg+I" },
    Befehl { id: "ausw_verbunden", name: "Auswahl: verbundene Leitungen dazu", vorgabe: "" },
];

/// Schlüssel in `Optionen::tasten` für die Taste zum Erweitern der Markierung („Umschalt“, sonst Strg wie im
/// Original). Kein Befehl mit Taste, daher nicht in `BEFEHLE`.
pub const AUSWAHL_DAZU: &str = "auswahl_dazu";

/// Erweitert Umschalt (statt Strg) die Markierung?
pub fn auswahl_dazu_umschalt(eigene: &BTreeMap<String, String>) -> bool {
    eigene.get(AUSWAHL_DAZU).is_some_and(|v| v.eq_ignore_ascii_case("umschalt") || v.eq_ignore_ascii_case("shift"))
}

/// Belegung eines Befehls (eigene aus den Optionen, sonst Vorgabe; leer = keine).
pub fn belegung(eigene: &BTreeMap<String, String>, id: &str) -> String {
    eigene.get(id).cloned().unwrap_or_else(|| BEFEHLE.iter().find(|b| b.id == id).map(|b| b.vorgabe.to_string()).unwrap_or_default())
}

/// "Strg+Umschalt+S" → (Modifikatoren, Taste).
pub fn parse(s: &str) -> Option<(Modifiers, Key)> {
    let mut m = Modifiers::NONE;
    let mut key = None;
    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_lowercase().as_str() {
            "strg" | "ctrl" | "steuerung" => m.ctrl = true,
            "umschalt" | "shift" => m.shift = true,
            "alt" => m.alt = true,
            _ => key = key_from(part),
        }
    }
    // "Strg++" (Plus-Taste)
    if key.is_none() && s.ends_with("++") {
        key = Some(Key::Plus);
    }
    key.map(|k| (m, k))
}

fn key_from(name: &str) -> Option<Key> {
    let n = match name {
        "Entf" => "Delete",
        "Einfg" => "Insert",
        "Pos1" => "Home",
        "Ende" => "End",
        "BildAuf" | "Bild↑" => "PageUp",
        "BildAb" | "Bild↓" => "PageDown",
        "Leertaste" => "Space",
        "Num0" => "0",
        other => other,
    };
    Key::from_name(n).or_else(|| Key::from_name(&n.to_uppercase()))
}

/// Schreibweise einer Tastenkombination für Anzeige und Speicherung.
pub fn text(m: Modifiers, k: Key) -> String {
    let mut s = String::new();
    if m.ctrl || m.command && !m.mac_cmd {
        s.push_str("Strg+");
    }
    if m.shift {
        s.push_str("Umschalt+");
    }
    if m.alt {
        s.push_str("Alt+");
    }
    s.push_str(k.name());
    s
}

/// Wurde der Befehl in diesem Bild ausgelöst? Modifikatoren müssen genau stimmen.
pub fn gedrueckt(i: &egui::InputState, eigene: &BTreeMap<String, String>, id: &str) -> bool {
    let Some((m, k)) = parse(&belegung(eigene, id)) else { return false };
    let im = i.modifiers;
    let ctrl = im.ctrl || (im.command && !im.mac_cmd);
    i.key_pressed(k) && ctrl == m.ctrl && im.shift == m.shift && im.alt == m.alt
}

/// Nächste gedrückte Taste (mit Modifikatoren) zum Aufnehmen einer neuen Belegung.
pub fn aufnehmen(i: &egui::InputState) -> Option<String> {
    i.events.iter().find_map(|e| match e {
        egui::Event::Key { key, pressed: true, modifiers, .. } => Some(text(*modifiers, *key)),
        _ => None,
    })
}

/// Doppelt belegte Kombinationen (für den Hinweis in den Optionen).
pub fn doppelt(eigene: &BTreeMap<String, String>) -> Vec<(String, Vec<&'static str>)> {
    let mut m: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    for b in BEFEHLE {
        let s = belegung(eigene, b.id);
        if !s.is_empty() {
            m.entry(s).or_default().push(b.name);
        }
    }
    m.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_und_text() {
        let (m, k) = parse("Strg+Umschalt+S").unwrap();
        assert!(m.ctrl && m.shift && !m.alt);
        assert_eq!(k, Key::S);
        assert_eq!(text(m, k), "Strg+Umschalt+S");
        assert_eq!(parse("Delete").unwrap().1, Key::Delete);
        assert_eq!(parse("Entf").unwrap().1, Key::Delete);
        assert_eq!(parse("Strg+PageDown").unwrap().1, Key::PageDown);
        assert_eq!(parse("Strg+Plus").unwrap().1, Key::Plus);
        assert!(parse("").is_none());
    }

    #[test]
    fn alle_vorgaben_gueltig_und_eindeutig() {
        let leer = BTreeMap::new();
        for b in BEFEHLE {
            assert!(b.vorgabe.is_empty() || parse(b.vorgabe).is_some(), "{} = {}", b.id, b.vorgabe);
        }
        assert!(!auswahl_dazu_umschalt(&leer));
        let mut u = BTreeMap::new();
        u.insert(AUSWAHL_DAZU.to_string(), "Umschalt".to_string());
        assert!(auswahl_dazu_umschalt(&u));
        assert!(doppelt(&leer).is_empty(), "{:?}", doppelt(&leer));
        let mut eigene = BTreeMap::new();
        eigene.insert("drehen".to_string(), "Strg+S".to_string());
        assert_eq!(belegung(&eigene, "drehen"), "Strg+S");
        assert_eq!(doppelt(&eigene).len(), 1);
    }
}
