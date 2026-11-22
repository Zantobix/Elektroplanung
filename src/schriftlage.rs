//! Lage und Größe von Texten in Boxen, vermessen am Original-Ausdruck „Elektroplanung Wohnung 2.pdf“
//! (Installationspläne Seite 1–4, Schriftfeld aller Blätter; Textpositionen aus dem PDF-Inhalt gelesen).
//!
//! - Textbox (ZCN Typ 11, z. B. Raumnamen): Der Text steht oben links in der Box, um 0,2 mm nach innen
//!   gerückt (in Schreibrichtung und quer dazu; hochkant entsprechend gedreht: links unten). Die erste
//!   Grundlinie liegt 0,2 mm + Oberlänge unter der Oberkante. Zeilenabstand 1,13 Geviert (gemessen: 14 pt
//!   5,59 mm, 16 pt 6,39 mm, 20 pt 7,96 mm) – kleiner als der Zeilenabstand der egui-Schrift (1,15 Geviert).
//!   Hochkante Zeilen stehen nebeneinander ab dem linken Boxrand (nicht mittig).
//! - Schriftfeld: Schrift = 0,88 × Feldhöhe je Zeile (gemessen 0,877–0,887 an „Neubau EG“, „Wohnung 2“,
//!   „1 / 4“), zu breite Texte auf die Feldbreite verkleinert. Symboltexte folgen dieser Regel nicht (die Option
//!   „2“ der Steckdose passt zu 0,72 × Höhe, „SPD“, „M“, „Dusch/WC“ sind im Ausdruck 7–20 % größer) und bleiben
//!   vorerst bei 0,72 (canvas.rs).
//! - Spaltenleiste: Strompfad-Nummern in 12 pt, unabhängig vom Blattformat.
//! - Kennzeichnung (BMK) und Wert eines Symbols: Schriftgröße aus der Anlage („Titel“ ANL Feld 46, „Wert“ Feld 47;
//!   die Zeichenroutine 0x438b94 reicht ANL +0xb8 bzw. +0xbc an die Textroutine 0x491eec). Diese setzt
//!   lfHeight = −Größe (Geviert) und rechnet die Größe mit 254/96 in 0,1 mm um (0x492572: Größe × 254 × 1/96 ×
//!   Maßstab) – die Größe zählt also in Bildpunkten bei 96 dpi: Titel 10 = 2,646 mm (Ausdruck: 2,62 mm,
//!   gerundet auf Druckerpunkte), Wert 8 = 2,117 mm. Bisher zeichnete der Nachbau fest 2,5 bzw. 2,2 mm.

/// Einzug des Textes in der Textbox in mm (in Schreibrichtung und quer dazu)
pub const TEXTBOX_RAND: f32 = 0.2;

/// Zeilenabstand mehrzeiliger Textboxen in Geviert
pub const TEXTBOX_ZEILE: f32 = 1.13;

/// Schriftgröße (Geviert) je mm Feldhöhe und Zeile im Schriftfeld
pub const FELD_SCHRIFT: f32 = 0.88;

/// Schriftgröße der Strompfad-Nummern in der Spaltenleiste in mm (12 pt, Original-Ausdruck)
pub const SPALTEN_SCHRIFT: f32 = 12.0 * 25.4 / 72.0;

/// Schriftgröße (Geviert, mm) der Symbolbeschriftungen aus einer Anlageneinstellung (Titel/Wert/Verbindungen):
/// Bildpunkte bei 96 dpi (siehe Modulbeschreibung).
pub fn anlage_schrift(groesse: i32) -> f32 {
    groesse.max(1) as f32 * 25.4 / 96.0
}

/// Vorgaben (Titel 10, Wert 8 wie in W2 und neuen Projekten) als Schriftgröße in mm
pub const TITEL_SCHRIFT: f32 = 10.0 * 25.4 / 96.0;
pub const WERT_SCHRIFT: f32 = 8.0 * 25.4 / 96.0;

/// Zeilenabstand einer Textbox in mm bei Schriftgröße `size` (mm).
pub fn textbox_zeile(size: f32) -> f32 {
    size * TEXTBOX_ZEILE
}

/// Schriftgröße (mm) für ein Feld der Höhe `h` (mm) mit `zeilen` Zeilen.
pub fn feld_schrift(h: f32, zeilen: usize) -> f32 {
    FELD_SCHRIFT * h / zeilen.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn werte_aus_dem_ausdruck() {
        // 20 pt = 7,056 mm: Zeilenabstand 7,96 mm im Ausdruck
        assert!((textbox_zeile(7.056) - 7.96).abs() < 0.02);
        assert!((textbox_zeile(5.644) - 6.39).abs() < 0.02);
        // Feld „%Name“ 9 mm hoch: „Neubau EG“ mit 7,92 mm Geviert
        assert!((feld_schrift(9.0, 1) - 7.92).abs() < 0.01);
        assert!((feld_schrift(9.0, 2) - 3.96).abs() < 0.01);
        // Titel 10: im Ausdruck 2,62 mm Geviert
        assert!((anlage_schrift(10) - 2.62).abs() < 0.03);
        assert!((anlage_schrift(10) - TITEL_SCHRIFT).abs() < 1e-6);
        assert!((anlage_schrift(8) - WERT_SCHRIFT).abs() < 1e-6);
    }
}
