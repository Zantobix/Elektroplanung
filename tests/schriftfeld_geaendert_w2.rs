//! Schriftfeld %Geändert gegen W2.epr: Blätter ohne Änderungsdatum (BLT @120 = 0, u. a. die Klemmpläne) zeigen
//! „---------“ wie im Original, nicht das Projektdatum. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{canvas, epr};
use std::path::PathBuf;

#[test]
fn klemmplan_ohne_aenderungsdatum() {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    if !p.exists() {
        return;
    }
    let project = epr::import(&p).unwrap();
    let a = project.anlagen.iter().find(|a| a.name == "Unterverteilung 1").unwrap();
    let wert = |name: &str| canvas::macro_values(&project, a, a.sheets.iter().find(|s| s.name == name).unwrap()).geaendert;
    assert_eq!(wert("Klemmplan -X1"), "---------");
    assert_eq!(wert("Klemmplan -X2"), "---------");
    assert_eq!(wert("Sicherungsbezeichnung"), "11.03.2025");
    // Klemmplan mit gespeichertem Datum (Heizungssteuerung) zeigt dieses
    let h = project.anlagen.iter().find(|a| a.name.starts_with("Heizungssteuerung")).unwrap();
    let k = h.sheets.iter().find(|s| s.art == 4).unwrap();
    assert_eq!(canvas::macro_values(&project, h, k).geaendert, "11.03.2025");
}
