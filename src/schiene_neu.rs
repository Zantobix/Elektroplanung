//! Neue Sammelschiene aus der Schienenauswahl eines Sammelschienen-Objekts wie im Original: Die Liste
//! „S-Schiene“ (Objekt-Eigenschaften 0x4427d4, Fall Sammelschiene) beginnt mit dem Eintrag `Neue Schiene "L<n>"`
//! (Wine-Gegenprobe Runde 12: erster Eintrag, dann „Schiene 1 (L1)“ …)
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

/// Lage einer neuen Schiene des Werkzeugknopfs „Neue Sammelschiene“ (ToolButton18Click 0x46d9b0): Anfang 8 mm
/// rechts vom linken Rahmen, Ende Papierbreite − 13 mm (im Original gespeichert und nachgemessen, Cloud 25.11.2026:
/// Hochformat Rand 10 → 18…197, Querformat Rand 5 → 13…284; im .epr steht die Länge 179 bzw. 271). Die Höhe setzt
/// der Auto-Stapel.
pub fn werkzeug_lage(s: &Sheet) -> (f32, f32) {
    let (x0, ..) = s.frame();
    let (w, _) = s.dims();
    (x0 + 8.0, w - 13.0)
}

impl crate::app::App {
    /// Werkzeugknopf „Neue Sammelschiene“ wie im Original (ToolButton18Click 0x46d9b0, Cloud-Gegenprobe 25.11.2026):
    /// nur auf Schaltplänen (Blattart 3) ein Sammelschienen-Objekt am Ende des Blattes anlegen und gleich seinen
    /// Eigenschaften-Dialog öffnen, Auswahl „S-Schiene“ auf `Neue Schiene "L<n>"`. OK legt die Schiene an und
    /// stapelt das Objekt (Auto), Abbruch nimmt alles zurück. „Hinzufügen › Sammelschiene“ legt dagegen nur die
    /// Schiene an (`new_busbar`).
    pub fn schiene_objekt_neu(&mut self) {
        let Some(r) = self.cur() else { return };
        if self.project.sheet(r).is_none_or(|s| s.blattart() != 3) {
            return;
        }
        self.checkpoint();
        self.neu_merken();
        let id = self.project.alloc_id();
        let Some(anl) = self.project.anlagen.get(r.anlage) else { return };
        let neu = eintrag(anl);
        let mut sliste = vec![neu.clone()];
        sliste.extend(anl.sammelschienen.iter().cloned());
        // vorläufig unter dem künftigen Kürzel und gleich an der Auto-Lage, damit hinter dem Dialog schon das
        // Ergebnis steht; bei OK übernimmt die Auswahl „S-Schiene“ den Namen (neue Schiene wird dann angelegt)
        let kurz = format!("L{}", naechste(anl));
        let abstand = self.opt.sms_abstand;
        let schriftfeld = self.project.title_h();
        let pfad = anl.set.pfad_hoehe as f32 / 10.0;
        let Some(s) = self.project.sheet_mut(r) else { return };
        let (x1, x2) = werkzeug_lage(s);
        let (_, y0, ..) = s.frame();
        let kind = Kind::Busbar { a: P::new(x1, y0), x2, name: kurz, label: P::new(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 };
        s.objects.push(Obj { id, kind, pen: 0, fixed: false, group: None, raw: None });
        let (oben, unten) = crate::sammelschiene_lage::stapel_anfang(s, pfad, schriftfeld);
        let lagen = crate::sammelschiene_lage::stapeln(s, abstand, oben, unten);
        crate::sammelschiene_lage::anwenden(s, &lagen);
        let y0 = s.objects.last().and_then(|o| if let Kind::Busbar { a, .. } = &o.kind { Some(a.y) } else { None }).unwrap_or(y0);
        let f1 = |v: f32| format!("{v:.1}").replace('.', ",");
        self.dlg3 = Some(crate::dialoge3::Dlg3::BusbarObj {
            id,
            sliste,
            sel: 0,
            einsp: false,
            auto: true,
            unten: false,
            offset: f1(0.0),
            tx: f1(0.0),
            ty: f1(-4.0),
            px: f1(x1),
            py: f1(y0),
            laenge: f1(x2 - x1),
        });
    }
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

    #[test]
    fn werkzeug_lage_wie_im_original() {
        // Ränder wie W2 (Vorgabe `RAENDER_W2`)
        let mut hoch = Sheet::new(1);
        hoch.portrait = true;
        let quer = Sheet::new(2);
        // Werte aus vom Original gespeicherten Dateien (W2-Kopie, Neubau EG bzw. Sicherungen -F10 …)
        assert_eq!(hoch.frame().0, 10.0);
        assert_eq!(werkzeug_lage(&hoch), (18.0, 197.0));
        assert_eq!(quer.frame().0, 5.0);
        assert_eq!(werkzeug_lage(&quer), (13.0, 284.0));
    }
}
