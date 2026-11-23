//! Runde 12, statische Prüfung des Hauptmenüs: Sammelschienenassistent nur auf Stromlaufplänen (0x4862d4).

use elektroplanung::model::Sheet;

#[test]
fn schienen_assistent_nur_auf_schaltplaenen() {
    let mut s = Sheet::new(1);
    assert!(elektroplanung::extra::schienen_assistent_erlaubt(Some(&s)));
    s.art = 3;
    assert!(elektroplanung::extra::schienen_assistent_erlaubt(Some(&s)));
    for art in [4, 5, 6] {
        s.art = art;
        assert!(!elektroplanung::extra::schienen_assistent_erlaubt(Some(&s)));
    }
    assert!(!elektroplanung::extra::schienen_assistent_erlaubt(None));
}
