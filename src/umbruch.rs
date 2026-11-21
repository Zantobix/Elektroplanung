//! Zeilenumbruch in Textboxen (Textbox-Dialog, Schalter „Umbruch“; .epr: ZCN Byte 0x19). Umbrochen wird an
//! Leerzeichen; ein Wort, das allein breiter als die Box ist, bleibt ganz (wie Windows-DrawText mit Wortumbruch).

/// Eine Zeile (ohne Zeilenwechsel) in Stücke zerlegen, die höchstens `breite` breit sind; `mass` liefert die
/// Breite eines Textes in derselben Einheit.
pub fn umbrechen(zeile: &str, breite: f32, mass: &dyn Fn(&str) -> f32) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    for wort in zeile.split(' ') {
        let probe = if cur.is_empty() { wort.to_string() } else { format!("{cur} {wort}") };
        if cur.is_empty() || mass(&probe) <= breite {
            cur = probe;
        } else {
            out.push(std::mem::replace(&mut cur, wort.to_string()));
        }
    }
    out.push(cur);
    out
}

/// Ganzen Text umbrechen: vorhandene Zeilenwechsel bleiben, jede Zeile wird für sich umbrochen.
pub fn text_umbrechen(text: &str, breite: f32, mass: &dyn Fn(&str) -> f32) -> Vec<String> {
    text.split('\n').flat_map(|z| umbrechen(z.trim_end_matches('\r'), breite, mass)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// jedes Zeichen 1 breit
    fn m(s: &str) -> f32 {
        s.chars().count() as f32
    }

    #[test]
    fn an_leerzeichen() {
        assert_eq!(umbrechen("eins zwei drei", 9.0, &m), ["eins zwei", "drei"]);
        assert_eq!(umbrechen("eins zwei drei", 100.0, &m), ["eins zwei drei"]);
        assert_eq!(umbrechen("eins zwei drei", 4.0, &m), ["eins", "zwei", "drei"]);
        // zu langes Wort bleibt ganz
        assert_eq!(umbrechen("Sicherungsautomat B16", 5.0, &m), ["Sicherungsautomat", "B16"]);
        assert_eq!(umbrechen("", 5.0, &m), [""]);
    }

    #[test]
    fn zeilenwechsel_bleiben() {
        assert_eq!(text_umbrechen("a b c\r\n\nd e", 3.0, &m), ["a b", "c", "", "d e"]);
    }
}
