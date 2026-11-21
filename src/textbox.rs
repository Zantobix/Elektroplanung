//! Textboxen: Im Original sind Text und Textbox dasselbe Blattelement (ZCN Typ 11) mit demselben Eigenschaftendialog
//! (TObjekt_Eigenschaften, Seite TTextbox: Textboxabmessung, Rahmen, Umbruch, Fixiert, Ausrichtung, Drehung, Schrift,
//! Text). Der Nachbau legt eine neue Textbox deshalb als `Kind::Text` mit `style.rect` an – wie die aus einer .epr-Datei
//! gelesenen – und Doppelklick bzw. Kontextmenü › Eigenschaften öffnen den TTextbox-Dialog.
//!
//! Vorgaben einer neuen Textbox aus dem Anlegen der Elemente im Original (0x406d70, Typ 0xb): Byte 0x1a Rahmen = 1,
//! Byte 0x19 Umbruch = 0, Feld 4 Schriftgrad = 8 pt, Feld 5 Drehung = 0, Byte 0x18 Ausrichtung = 0 (links),
//! Fett/Kursiv/Unterstrichen = 0, Farbe (Feld 8) = 0 (schwarz).
//!
//! Ältere .epj-Dateien des Nachbaus enthalten noch die frühere eigene Art `TextBox` (immer mit Rahmen und Umbruch);
//! `kind_lesen` wandelt sie beim Laden in diese Form um.

use crate::model::{Align, Kind, TextStyle, P};
use serde::{Deserialize, Deserializer};

/// Schriftgrad einer neuen Textbox im Original (pt).
pub const VORGABE_PT: f32 = 8.0;

/// Neue Textbox zwischen den Ecken `a` und `b` mit den Vorgaben des Originals (Rahmen an, Umbruch aus, 8 pt, links).
pub fn neu(a: P, b: P, text: &str) -> Kind {
    mit(a, b, text, VORGABE_PT * 0.3528, TextStyle { frame: true, wrap: false, ..Default::default() })
}

/// Textbox zwischen `a` und `b` mit Schrifthöhe `size` (mm) und den übrigen Einstellungen aus `style`
/// (links ausgerichtet, waagerecht; `style.rect` wird gesetzt).
pub fn mit(a: P, b: P, text: &str, size: f32, mut style: TextStyle) -> Kind {
    let (mn, mx) = crate::canvas::norm_rect(a, b);
    style.rect = Some((mn, mx));
    Kind::Text { pos: crate::epr::text_anchor(mn, mx, Align::Left, false), text: text.into(), size, align: Align::Left, vertical: false, style }
}

/// Frühere Art `TextBox` des Nachbaus (nur noch in .epj-Dateien), so wie serde sie geschrieben hat.
#[derive(Deserialize)]
struct AltTextBox {
    a: P,
    b: P,
    text: String,
    size: f32,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct AltForm {
    TextBox: AltTextBox,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum KindOderAlt {
    Kind(Kind),
    Alt(AltForm),
}

/// `Obj::kind` lesen; die frühere Art `TextBox` wird zur Textbox (Text mit Box, Rahmen und Umbruch wie bisher gezeichnet).
pub fn kind_lesen<'de, D: Deserializer<'de>>(d: D) -> Result<Kind, D::Error> {
    Ok(match KindOderAlt::deserialize(d)? {
        KindOderAlt::Kind(k) => k,
        KindOderAlt::Alt(AltForm { TextBox: t }) => mit(t.a, t.b, &t.text, t.size, TextStyle { frame: true, wrap: true, ..Default::default() }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Obj;

    #[test]
    fn neue_textbox_wie_im_original() {
        let Kind::Text { pos, size, align, vertical, style, .. } = neu(P::new(60.0, 40.0), P::new(20.0, 30.0), "Text") else { panic!() };
        assert_eq!(style.rect, Some((P::new(20.0, 30.0), P::new(60.0, 40.0))));
        assert!(style.frame && !style.wrap && !style.bold);
        assert_eq!((align, vertical, style.rotation), (Align::Left, false, 0));
        assert_eq!(((size / 0.3528).round(), pos), (8.0, P::new(20.0, 35.0)));
    }

    #[test]
    fn alte_textbox_aus_epj() {
        let j = r#"{"id": 7, "kind": {"TextBox": {"a": {"x": 10.0, "y": 20.0}, "b": {"x": 50.0, "y": 30.0}, "text": "Hallo", "size": 3.5}}}"#;
        let o: Obj = serde_json::from_str(j).unwrap();
        let Kind::Text { text, size, style, .. } = &o.kind else { panic!("{:?}", o.kind) };
        assert_eq!((text.as_str(), *size), ("Hallo", 3.5));
        assert!(style.frame && style.wrap);
        assert_eq!(style.rect, Some((P::new(10.0, 20.0), P::new(50.0, 30.0))));
        // andere Arten unverändert, auch nach Schreiben und Lesen
        let r: Obj = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(r.kind, o.kind);
        let l: Obj = serde_json::from_str(r#"{"id": 1, "kind": {"Line": {"a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 1.0}}}}"#).unwrap();
        assert!(matches!(l.kind, Kind::Line { .. }));
    }
}
