//! Kontextmenü „Bezeichner setzen“ im Symboleditor: einem Anschluss einen Text des Symbols als Bezeichnung
//! zuordnen (z. B. „13“/„14“ an einem Kontakt; der Klemmplan zeigt sie in Spalte „KL“).
//!
//! Belegt aus dem Original:
//! - Der Eintrag teilt sich den Handler mit „Ziel setzen“ und „Eigentümer › Setzen“ (Zielsetzen1Click 0x480424):
//!   bei Vormarkierung Zeichenobjekt (Anschluss, Element Typ 7) merkt er Anschluss, Anlage, Blatt und Zeit und
//!   schaltet den Wartemodus 3 ein (F+0x7b8).
//! - Der nächste Klick (MouseUp 0x40ad8c): ist das angeklickte Objekt ein Zeichenobjekt mit Element Typ 6 (Text)
//!   und das gemerkte ein Anschluss (Typ 7), setzt das Original rückgängig machbar Anschluss Feld 5 (+0x14) :=
//!   Text Feld 7 (+0x1c, Textnummer). Danach ist der Wartemodus beendet, auch wenn kein Text getroffen wurde.

use crate::app::App;
use crate::model::*;
use std::collections::HashMap;

/// Textnummer des Textobjekts `text` (neue Texte bekommen die nächste freie Nummer, 1 = BMK-Vorlage) und
/// Zuordnung an den Anschluss `anschluss` (Blatt des Editors, Textnummern je Objekt, Zuordnungen). `None`, wenn
/// eines der beiden nicht passt.
pub fn binden(sheet: &Sheet, text_idx: &mut HashMap<u64, u32>, pin_text: &mut HashMap<u64, u32>, anschluss: u64, text: u64) -> Option<u32> {
    let ist = |id: u64, f: fn(&Kind) -> bool| sheet.objects.iter().any(|o| o.id == id && f(&o.kind));
    if !ist(anschluss, |k| matches!(k, Kind::Terminal { .. })) || !ist(text, |k| matches!(k, Kind::Text { .. })) {
        return None;
    }
    if text == crate::extra::EDIT_TITLE_ID || text == crate::extra::EDIT_VALUE_ID {
        return None;
    }
    let nr = match text_idx.get(&text) {
        Some(n) => *n,
        None => {
            let n = text_idx.values().copied().max().unwrap_or(1).max(1) + 1;
            text_idx.insert(text, n);
            n
        }
    };
    pin_text.insert(anschluss, nr);
    Some(nr)
}

impl App {
    /// „Bezeichner setzen“ auf dem Anschluss `anschluss`: auf den Klick auf einen Text warten.
    pub fn bezeichner_start(&mut self, anschluss: u64) {
        if let Some(e) = self.sym_edit.as_mut() {
            e.bezeichner_pick = Some(anschluss);
            self.placing = None;
            self.drag = crate::app::Drag::None;
            self.info = "Bezeichner setzen: Text anklicken (Esc = Abbruch)".into();
        }
    }

    pub fn bezeichner_pick_aktiv(&self) -> bool {
        self.sym_edit.as_ref().is_some_and(|e| e.bezeichner_pick.is_some())
    }

    /// Klick im Wartemodus: `Some(id)` = Objekt unter der Maus. Der Modus endet mit jedem Klick (wie im Original).
    pub fn bezeichner_klick(&mut self, id: Option<u64>) {
        let Some(e) = self.sym_edit.as_mut() else { return };
        let Some(anschluss) = e.bezeichner_pick.take() else { return };
        let Some(id) = id.filter(|i| *i != anschluss) else {
            self.info = "Bezeichner setzen: kein Text angeklickt".into();
            return;
        };
        let text = e.sheet.objects.iter().find(|o| o.id == id).and_then(|o| if let Kind::Text { text, .. } = &o.kind { Some(text.clone()) } else { None });
        match binden(&e.sheet, &mut e.text_idx, &mut e.pin_text, anschluss, id) {
            Some(nr) => {
                self.info = format!("Bezeichner des Anschlusses: „{}“ (Text {nr})", text.unwrap_or_default());
                self.dirty = true;
            }
            None => self.info = "Bezeichner setzen: kein Text angeklickt".into(),
        }
    }

    pub fn bezeichner_abbruch(&mut self) {
        if let Some(e) = self.sym_edit.as_mut() {
            if e.bezeichner_pick.take().is_some() {
                self.info = "Bezeichner setzen abgebrochen".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn text_an_anschluss() {
        let mut s = Sheet::new(0);
        let p = P::default();
        let text = |t: &str| Kind::Text { pos: p, text: t.into(), size: 2.5, align: Align::Left, vertical: false, style: Default::default() };
        s.objects.push(obj(1, Kind::Terminal { pos: p, leiste: "Anschluss".into(), nr: 1, level: 0, text_off: None, tpl: String::new(), opt: Default::default() }));
        s.objects.push(obj(2, text("13")));
        s.objects.push(obj(3, text("14")));
        s.objects.push(obj(4, Kind::Line { a: p, b: p }));
        let mut idx: HashMap<u64, u32> = [(2, 3)].into_iter().collect();
        let mut pins = HashMap::new();
        // vorhandene Textnummer wird übernommen, ein Text ohne Nummer bekommt die nächste freie
        assert_eq!(binden(&s, &mut idx, &mut pins, 1, 2), Some(3));
        assert_eq!(binden(&s, &mut idx, &mut pins, 1, 3), Some(4));
        assert_eq!((pins[&1], idx[&3]), (4, 4));
        // kein Text bzw. kein Anschluss: nichts
        assert_eq!(binden(&s, &mut idx, &mut pins, 1, 4), None);
        assert_eq!(binden(&s, &mut idx, &mut pins, 2, 3), None);
    }
}
