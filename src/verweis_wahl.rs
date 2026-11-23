//! Auswahllisten im Eigenschaftendialog wie im Original:
//! - Verweis (TSVerweis): „Blatt“ (SVerweis1VW_AnlageChange 0x4a468c) beginnt mit „Keine Zuordnung“, danach je Blatt
//!   Blattnummer (0x472aec) + „ “ + Bezeichnung. „Verweislink“ (SVerweis1VW_BlattChange 0x4a48f4) beginnt ebenfalls
//!   mit „Keine Zuordnung“, danach die Verweise des Zielblattes mit der Gegenrichtung (gleiche Richtung und der
//!   Verweis selbst fehlen): Titel, ohne Titel „(n Ziel)“ bzw. „(n Start)“ (n zählt die Verweise ohne Titel ab 1),
//!   mit „ [Vergeben]“, wenn der Verweis schon mit einem anderen verbunden ist.
//! - Bild (TBild): „Grafik“ zeigt die Namen der Grafiken des Projekts (0x446845, Projekt +0xc0).

use crate::model::*;

pub const KEINE_ZUORDNUNG: &str = "Keine Zuordnung";

/// Einträge der Blattliste einer Anlage (Index 0 = „Keine Zuordnung“, Index i = Blatt i−1).
pub fn blatt_eintraege(p: &Project, anlage: usize) -> Vec<String> {
    std::iter::once(KEINE_ZUORDNUNG.to_string())
        .chain(p.anlagen.get(anlage).into_iter().flat_map(|a| &a.sheets).map(|s| format!("{} {}", crate::klemmen_verwendung::blattnummer(s.number, s.subindex), s.name)))
        .collect()
}

/// Richtung eines Verweises: true = „Start“ (wie das Optionsfeld des Dialogs).
pub fn ist_start(dir: u16) -> bool {
    dir == 0 || dir == 270
}

/// Verweise auf Blatt `r`, die als Gegenverweis für Verweis `id` (Richtung `start`) in Frage kommen:
/// (Objekt-Id, Titel, Listeneintrag).
pub fn link_kandidaten(p: &Project, r: SheetRef, id: u64, start: bool) -> Vec<(u64, String, String)> {
    let mut n = 0;
    let mut v = vec![];
    for o in p.sheet(r).map(|s| s.objects.as_slice()).unwrap_or(&[]) {
        let Kind::Verweis { text, dir, ziel, .. } = &o.kind else { continue };
        if o.id == id || ist_start(*dir) == start {
            continue;
        }
        let mut t = if text.trim().is_empty() || text == "%O" {
            n += 1;
            format!("({n} {})", if ist_start(*dir) { "Start" } else { "Ziel" })
        } else {
            text.clone()
        };
        if ziel.as_ref().is_some_and(|z| z.obj != 0 && z.obj != id) {
            t.push_str(" [Vergeben]");
        }
        v.push((o.id, text.clone(), t));
    }
    v
}

/// Einträge der Liste „Verweislink“ (Index 0 = „Keine Zuordnung“).
pub fn link_eintraege(k: &[(u64, String, String)]) -> Vec<String> {
    std::iter::once(KEINE_ZUORDNUNG.to_string()).chain(k.iter().map(|x| x.2.clone())).collect()
}

/// Anzeigename einer Grafik: Name aus der Grafikliste des Projekts („bld:<Nr>“), sonst der Dateiname.
pub fn grafik_name(p: &Project, key: &str) -> String {
    if let Some(nr) = key.strip_prefix("bld:").and_then(|n| n.parse::<i32>().ok()) {
        if let Some(b) = p.bilder.iter().find(|b| b.nr == nr).filter(|b| !b.name.trim().is_empty()) {
            return b.name.clone();
        }
        return key.to_string();
    }
    std::path::Path::new(key).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| key.to_string())
}

/// Anlagenliste von TForm10 (FormShow 0x4ae254): Name, ohne Namen „[AnlageX ohne Namen]“ mit X = laufende Ziffer
/// nur der unbenannten Anlagen (1–9, danach bleibt „X“).
pub fn anlagen_ohne_namen(p: &Project) -> Vec<String> {
    let mut z = 0;
    p.anlagen
        .iter()
        .map(|a| {
            if !a.name.is_empty() {
                return a.name.clone();
            }
            z += 1;
            let x = if z <= 9 { z.to_string() } else { "X".into() };
            format!("[Anlage{x} ohne Namen]")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anlagen_ohne_namen_zaehlt_nur_unbenannte() {
        let mut p = Project::new_default();
        p.anlagen[0].name = "Haus".into();
        let mut b = p.anlagen[0].clone();
        b.name.clear();
        p.anlagen.push(b);
        assert_eq!(anlagen_ohne_namen(&p), vec!["Haus".to_string(), "[Anlage1 ohne Namen]".into()]);
    }

    fn verweis(id: u64, text: &str, dir: u16, ziel: Option<u64>) -> Obj {
        Obj { id, pen: 0, fixed: false, group: None, raw: None, kind: Kind::Verweis { pos: P::new(0.0, 0.0), text_pos: P::new(0.0, 0.0), text: text.into(), dir, vertical: false, spitze: false, ziel: ziel.map(|o| VerweisZiel { anlage: "A".into(), blatt: 1, obj: o }) } }
    }

    #[test]
    fn listen_wie_original() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].name = "Licht".into();
        assert_eq!(blatt_eintraege(&p, 0), vec!["Keine Zuordnung".to_string(), "1 Licht".into()]);
        let s = &mut p.anlagen[0].sheets[0];
        s.objects.push(verweis(1, "eigener", 0, None));
        s.objects.push(verweis(2, "", 180, None));
        s.objects.push(verweis(3, "Küche", 90, Some(7)));
        s.objects.push(verweis(4, "gleiche Richtung", 270, None));
        s.objects.push(verweis(5, "", 180, Some(1)));
        let k = link_kandidaten(&p, SheetRef { anlage: 0, sheet: 0 }, 1, true);
        let e = link_eintraege(&k);
        assert_eq!(e, vec!["Keine Zuordnung", "(1 Ziel)", "Küche [Vergeben]", "(2 Ziel)"]);
        assert_eq!(k[1].1, "Küche");
    }

    #[test]
    fn grafikname() {
        let mut p = Project::new_default();
        p.bilder.push(Bild { name: "Logo".into(), nr: 3, jpeg: vec![] });
        assert_eq!(grafik_name(&p, "bld:3"), "Logo");
        assert_eq!(grafik_name(&p, "/tmp/a/b.png"), "b.png");
    }
}
