//! Platzhalteranzeige der Textvariablen wie im Original: Die Textroutine 0x46e9f4 ersetzt „%“ + Buchstabe
//! (Sprungtabelle 0x46eb18/0x46eb47, Buchstaben A–o). Mit Schalter Bit 0 (Aufruf aus TTextchange.Edit1Change
//! 0x4abe54) schreibt sie statt der Werte die Namen der Variablen in Klammern; so zeigt der Dialog „Symboltitel“
//! (TTextchange Art 1, FormShow 0x4abc10: Höhe 120, Label sichtbar) unter dem Eingabefeld, was der Titel enthält.
//!
//! - %A [Anlage], %B [Bedingt], %C [Anzahl], %F [Strompfad], %O/%o [Eigentümer], %P [Ort], %R [Rack-Nr],
//!   %S [Blatt-Nr], %Z [Neue Zeile] (ohne Schalter ein Zeilenumbruch);
//! - jeder andere Buchstabe (und „%“ am Ende): das „%“ bleibt stehen, der Buchstabe wird normal übernommen
//!   (0x46ffd3);
//! - Leerzeichen am Anfang entfallen, solange das Ergebnis noch leer ist (0x470020).

/// Text mit den Platzhaltern des Originals statt der Variablenwerte.
pub fn platzhalter(text: &str) -> String {
    let mut out = String::new();
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if c == '%' {
            let name = match it.peek() {
                Some('A') => Some("[Anlage]"),
                Some('B') => Some("[Bedingt]"),
                Some('C') => Some("[Anzahl]"),
                Some('F') => Some("[Strompfad]"),
                Some('O') | Some('o') => Some("[Eigentümer]"),
                Some('P') => Some("[Ort]"),
                Some('R') => Some("[Rack-Nr]"),
                Some('S') => Some("[Blatt-Nr]"),
                Some('Z') => Some("[Neue Zeile]"),
                _ => None,
            };
            match name {
                Some(n) => {
                    out.push_str(n);
                    it.next();
                }
                None => out.push('%'),
            }
            continue;
        }
        if c == ' ' && out.is_empty() {
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::platzhalter;

    #[test]
    fn variablen_als_platzhalter() {
        assert_eq!(platzhalter("-%SK%C"), "-[Blatt-Nr]K[Anzahl]");
        assert_eq!(platzhalter("%A%Z%P %R"), "[Anlage][Neue Zeile][Ort] [Rack-Nr]");
        assert_eq!(platzhalter("%B%F%O%o"), "[Bedingt][Strompfad][Eigentümer][Eigentümer]");
        // unbekannter Buchstabe und „%“ am Ende bleiben stehen
        assert_eq!(platzhalter("%X1%"), "%X1%");
        // Leerzeichen am Anfang entfallen
        assert_eq!(platzhalter("  -K1"), "-K1");
    }
}
