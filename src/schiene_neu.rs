//! Neue Sammelschiene aus der Schienenauswahl eines Sammelschienen-Objekts wie im Original: Die Liste
//! „S-Schiene“ (Objekt-Eigenschaften 0x4427d4, Fall Sammelschiene) endet mit dem Eintrag `Neue Schiene "L<n>"`
//! (0x442c1a, n = Anzahl der Schienen + 1). Ist er bei OK gewählt, legt das Original eine neue Schiene an
//! (0x405e08 mit 1) und hängt das Objekt daran. Der Nachbau legt sie wie „Neue Sammelschiene“ an
//! (Schlüssel/Kürzel „L<n>“, Name „Schiene n“).

use crate::model::*;

/// Anfang des Listeneintrags (Text der Exe 0x5bab7a).
pub const NEUE_SCHIENE: &str = r#"Neue Schiene "L"#;

/// Nummer der nächsten Schiene der Anlage (erste freie ab Anzahl + 1).
pub fn naechste(anl: &Anlage) -> usize {
    let mut n = anl.sammelschienen.len() + 1;
    while anl.sammelschienen.iter().any(|x| *x == format!("L{n}")) {
        n += 1;
    }
    n
}

/// Listeneintrag für die neue Schiene.
pub fn eintrag(anl: &Anlage) -> String {
    format!("{NEUE_SCHIENE}{}\"", naechste(anl))
}

/// Ist `s` der Eintrag „Neue Schiene …“?
pub fn ist_neu(s: &str) -> bool {
    s.starts_with(NEUE_SCHIENE) && s.ends_with('"')
}

/// Neue Schiene anlegen (Vorgaben wie der Dialog des Originals: Name „Schiene n“, keine Pfeile); Rückgabe: Schlüssel.
pub fn anlegen(anl: &mut Anlage) -> String {
    let n = naechste(anl);
    let key = format!("L{n}");
    anl.sammelschienen.push(key.clone());
    anl.schienen.insert(key.clone(), SchieneSet { name: format!("Schiene {n}"), pfeile: false, ..Default::default() });
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eintrag_und_anlegen() {
        let mut p = Project::new_default();
        let a = &mut p.anlagen[0];
        a.sammelschienen = vec!["L1".into(), "N".into()];
        let e = eintrag(a);
        assert_eq!(e, "Neue Schiene \"L3\"");
        assert!(ist_neu(&e) && !ist_neu("L1"));
        assert_eq!(anlegen(a), "L3");
        assert_eq!(a.schienen["L3"].name, "Schiene 3");
        assert_eq!(a.sammelschienen.len(), 3);
    }
}
