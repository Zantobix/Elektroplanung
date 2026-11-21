//! Verweise anklicken: Sprung zum Ziel eines Verweises mit Strg+Klick (Hand als Mauszeiger), per Doppelklick wie
//! im Original je nach Option "Doppelklick auf Verweise = Ziel anzeigen / Eigenschaften" in TForm4, und zu den
//! Querverweisen der Sammelschienen ("Blatt/Pfad"). Ein einfacher Klick markiert und verschiebt wie bei anderen Objekten.
//! Dieselben Flächen werden im PDF als Verknüpfungen auf die Zielseite ausgegeben (pdf.rs).

use crate::app::App;
use crate::canvas;
use crate::model::*;
use crate::symbols::SymDef;

/// Ziel eines Sprungs: Blatt, Objekt (zum Markieren) und Lage (zum Zentrieren).
#[derive(Clone, Debug, PartialEq)]
pub struct Sprung {
    pub blatt: SheetRef,
    pub obj: Option<u64>,
    pub at: Option<P>,
}

/// Anklickbare Fläche (mm) eines Blattes mit ihrem Ziel; `ziel` = Err(Meldung), wenn es sich nicht auflösen lässt.
#[derive(Clone, Debug)]
pub struct Flaeche {
    pub mn: P,
    pub mx: P,
    pub quelle: u64,
    pub ziel: Result<Sprung, String>,
}

/// Schrifthöhe der Querverweise an Sammelschienen (wie canvas.rs) und grob geschätzte Zeichenbreite.
const XREF_H: f32 = 1.8;
const XREF_CHAR_W: f32 = 0.95;

/// Flächen der Querverweistexte einer Sammelschiene: (Anfang "vom vorigen Blatt", Ende "zum nächsten Blatt").
fn busbar_text_rects(a: P, x2: f32, xin: &str, xout: &str) -> [Option<(P, P)>; 2] {
    let w = |s: &str| s.chars().count() as f32 * XREF_CHAR_W + 0.4;
    let h = XREF_H * 0.7;
    let rin = (!xin.is_empty()).then(|| (P::new(a.x - 2.2 - w(xin), a.y - h), P::new(a.x - 0.2, a.y + h)));
    let rout = (!xout.is_empty()).then(|| (P::new(x2 + 0.2, a.y - h), P::new(x2 + 2.4 + w(xout), a.y + h)));
    [rin, rout]
}

/// Voriges (`ende = false`, Querverweis am Anfang) bzw. nächstes (`ende = true`) Vorkommen derselben Sammelschiene
/// auf einem anderen Blatt der Anlage – dieselbe Reihenfolge wie `crate::epr::update_busbar_xrefs`.
pub fn busbar_ziel(project: &Project, r: SheetRef, id: u64, ende: bool) -> Option<Sprung> {
    let a = project.anlagen.get(r.anlage)?;
    let mut occ: Vec<(usize, u64, u64, P, P)> = vec![];
    for (si, sh) in a.sheets.iter().enumerate() {
        for o in &sh.objects {
            if let Kind::Busbar { a: s, x2, sms, .. } = &o.kind {
                occ.push((si, o.id, *sms, *s, P::new(*x2, s.y)));
            }
        }
    }
    let i = occ.iter().position(|o| o.0 == r.sheet && o.1 == id)?;
    let sms = occ[i].2;
    if sms == 0 {
        return None;
    }
    let hit = if ende { occ[i + 1..].iter().find(|o| o.2 == sms && o.0 != r.sheet) } else { occ[..i].iter().rev().find(|o| o.2 == sms && o.0 != r.sheet) };
    hit.map(|o| Sprung { blatt: SheetRef { anlage: r.anlage, sheet: o.0 }, obj: Some(o.1), at: Some(if ende { o.3 } else { o.4 }) })
}

/// Ziel eines Verweises (Gegenverweis bzw. Blatt).
pub fn verweis_ziel(project: &Project, ziel: Option<&VerweisZiel>) -> Result<Sprung, String> {
    let z = ziel.ok_or("Für diesen Verweis ist kein Ziel gesetzt (Kontextmenü › Ziel setzen)")?;
    match project.resolve_ziel(z) {
        Some((blatt, obj)) => Ok(Sprung { blatt, obj, at: None }),
        None => Err(format!("Ziel des Verweises nicht gefunden: Anlage \"{}\", Blatt {}", z.anlage, z.blatt)),
    }
}

/// Alle anklickbaren Flächen eines Blattes: Verweise (ganzes Objekt) und die Querverweistexte der Sammelschienen.
pub fn flaechen(project: &Project, r: SheetRef, lib: &[SymDef]) -> Vec<Flaeche> {
    let Some(sheet) = project.sheet(r) else { return vec![] };
    let anlage = &project.anlagen[r.anlage];
    let mut out = vec![];
    for o in &sheet.objects {
        match &o.kind {
            Kind::Verweis { ziel, .. } => {
                let (mn, mx) = canvas::obj_bounds(o, lib);
                out.push(Flaeche { mn, mx, quelle: o.id, ziel: verweis_ziel(project, ziel.as_ref()) });
            }
            Kind::Busbar { a, x2, name, xin, xout, .. } => {
                // nur sichtbare Querverweise (TSSchiene "Querverweise")
                if !anlage.schienen.get(name).cloned().unwrap_or_default().quer {
                    continue;
                }
                for (ende, rect) in busbar_text_rects(*a, *x2, xin, xout).into_iter().enumerate() {
                    if let Some((mn, mx)) = rect {
                        let ziel = busbar_ziel(project, r, o.id, ende == 1).ok_or_else(|| format!("Sammelschiene {name}: kein weiteres Vorkommen gefunden"));
                        out.push(Flaeche { mn, mx, quelle: o.id, ziel });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Querverweistext einer Sammelschiene unter dem Punkt `m` (mm) auf Blatt `r`.
pub fn busbar_text_at(project: &Project, r: SheetRef, m: P, tol: f32) -> Option<Flaeche> {
    flaechen(project, r, &[])
        .into_iter()
        .filter(|f| project.find_obj(f.quelle).is_some_and(|(_, o)| matches!(o.kind, Kind::Busbar { .. })))
        .find(|f| m.x >= f.mn.x - tol && m.x <= f.mx.x + tol && m.y >= f.mn.y - tol && m.y <= f.mx.y + tol)
}

/// Verweis drehen/spiegeln (op wie `App::transform_selection`: 1 = 90° im Uhrzeigersinn, 2 = vertikal,
/// 4 = horizontal spiegeln) um die Lage (OBJ Feld 6/7, Anschlusspunkt an der Leitung). Beim Drehen dreht wie im
/// Original nur der Pfeil; der Text bleibt an seiner Stelle und in seiner Leserichtung (waagerecht bleibt
/// waagerecht; hochkant ist OBJ Feld 4 Byte 3). Vier Drehungen ergeben wieder die Ausgangslage.
pub fn drehen(kind: &mut Kind, op: u8) {
    let Kind::Verweis { pos, text_pos, dir, .. } = kind else { return };
    let (dx, dy) = (text_pos.x - pos.x, text_pos.y - pos.y);
    match op {
        1 => *dir = (*dir + 270) % 360,
        2 => {
            text_pos.x = pos.x - dx;
            *dir = (540 - *dir % 360) % 360;
        }
        _ => {
            text_pos.y = pos.y - dy;
            *dir = (360 - *dir % 360) % 360;
        }
    }
}

fn ziel_von(project: &Project, id: u64) -> Option<VerweisZiel> {
    match &project.find_obj(id)?.1.kind {
        Kind::Verweis { ziel, .. } => ziel.clone(),
        _ => None,
    }
}

fn setze_ziel(project: &mut Project, id: u64, z: Option<VerweisZiel>) {
    for o in project.anlagen.iter_mut().flat_map(|a| a.sheets.iter_mut()).flat_map(|s| s.objects.iter_mut()).filter(|o| o.id == id) {
        if let Kind::Verweis { ziel, .. } = &mut o.kind {
            *ziel = z.clone();
        }
    }
}

/// Ziel eines Verweises (Blatt, Kennung bei Verweisen auf sich selbst)
fn ziel_auf(project: &Project, r: SheetRef, obj: u64) -> Option<VerweisZiel> {
    let a = project.anlagen.get(r.anlage)?;
    Some(VerweisZiel { anlage: a.name.clone(), blatt: a.sheets.get(r.sheet)?.number, obj })
}

/// Partner `p` von `id` lösen, wenn er auf `id` zeigt.
fn loese_partner(project: &mut Project, id: u64, p: u64) -> bool {
    if p != 0 && p != id && ziel_von(project, p).is_some_and(|z| z.obj == id) {
        setze_ziel(project, p, None);
        return true;
    }
    false
}

/// "Ziel setzen": Verweis `id` auf Blatt `ziel` und optional den Gegenverweis `gegen` richten. Wie im Original
/// (Routine 0x48056c) zeigt der Gegenverweis danach zurück; bisherige Partner beider Seiten werden gelöst
/// (das Original fragt dabei "Der Zielverweis ist schon anderwertig belegt …"). Rückgabe: Hinweis für die Statuszeile.
pub fn verknuepfen(project: &mut Project, id: u64, ziel: SheetRef, gegen: Option<u64>) -> Option<String> {
    let (own_ref, _) = project.find_obj(id)?;
    let gegen = gegen.filter(|g| *g != id);
    let mut msg = None;
    if let Some(alt) = ziel_von(project, id) {
        if Some(alt.obj) != gegen {
            loese_partner(project, id, alt.obj);
        }
    }
    setze_ziel(project, id, ziel_auf(project, ziel, gegen.unwrap_or(0)));
    if let Some(g) = gegen {
        if let Some(alt) = ziel_von(project, g) {
            if alt.obj != id && loese_partner(project, g, alt.obj) {
                msg = Some("Der Zielverweis war anderweitig belegt; die alte Verbindung wurde gelöst".to_string());
            }
        }
        setze_ziel(project, g, ziel_auf(project, own_ref, id));
    }
    msg
}

impl App {
    /// Zum Ziel springen bzw. die Meldung in der Statuszeile zeigen.
    pub fn springe(&mut self, ziel: Result<Sprung, String>) {
        match ziel {
            Ok(s) => {
                let name = self.project.sheet(s.blatt).map(|b| format!("[{}] {}", b.number, b.name)).unwrap_or_default();
                self.springe_zu(s.blatt, s.obj.unwrap_or(0), s.at);
                self.info = format!("Verweis: {} › {name}", self.project.anlagen.get(s.blatt.anlage).map(|a| a.name.as_str()).unwrap_or(""));
            }
            Err(e) => self.info = e,
        }
    }

    /// Ziel des Verweises bzw. Sammelschienen-Querverweistextes unter `m` auf dem aktuellen Blatt (für Strg+Klick
    /// und den Hand-Mauszeiger); None = dort ist nichts anzuspringen.
    pub fn verweis_unter(&self, m: P, tol: f32) -> Option<Result<Sprung, String>> {
        let r = self.cur()?;
        if let Some(id) = self.hit_top(m, tol) {
            if let Some(Kind::Verweis { ziel, .. }) = self.project.sheet(r).and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| &o.kind) {
                return Some(verweis_ziel(&self.project, ziel.as_ref()));
            }
        }
        busbar_text_at(&self.project, r, m, tol).map(|f| f.ziel)
    }

    /// Strg+Klick bei `m`: zum Ziel des Verweises springen. true = behandelt.
    pub fn verweis_strg_klick(&mut self, m: P, tol: f32) -> bool {
        match self.verweis_unter(m, tol) {
            Some(z) => {
                self.springe(z);
                true
            }
            None => false,
        }
    }

    /// Doppelklick bei `m` auf dem aktuellen Blatt: Verweis bzw. Querverweis einer Sammelschiene → Ziel anzeigen
    /// (Option "Doppelklick auf Verweise = Ziel anzeigen"). true = behandelt.
    pub fn verweis_doppelklick(&mut self, hit: Option<u64>, m: P, tol: f32) -> bool {
        if self.opt.doppelklick_verweis != 0 {
            return false;
        }
        let Some(r) = self.cur() else { return false };
        if let Some(id) = hit {
            if let Some(Kind::Verweis { ziel, .. }) = self.project.sheet(r).and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| &o.kind) {
                let z = verweis_ziel(&self.project, ziel.as_ref());
                self.springe(z);
                return true;
            }
        }
        if let Some(f) = busbar_text_at(&self.project, r, m, tol) {
            self.springe(f.ziel);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: u64, kind: Kind) -> Obj {
        Obj { id, kind, pen: 0, fixed: false, group: None, raw: None }
    }

    fn verweis(text: &str, ziel: Option<VerweisZiel>) -> Kind {
        Kind::Verweis { pos: P::new(50.0, 50.0), text_pos: P::new(55.0, 49.0), text: text.into(), dir: 0, vertical: false, spitze: false, ziel }
    }

    fn bus(x: f32, sms: u64, xin: &str, xout: &str) -> Kind {
        Kind::Busbar { a: P::new(x, 40.0), x2: x + 100.0, name: "N".into(), label: P::default(), style: 0, sms, xin: xin.into(), xout: xout.into(), lage: 0, versatz: 0.0 }
    }

    fn projekt() -> Project {
        let mut p = Project::new_default();
        p.anlagen[0].sheets.push(Sheet::new(2));
        p.anlagen[0].sheets.push(Sheet::new(3));
        p.anlagen[0].sheets[0].objects.push(obj(1, verweis("=A", Some(VerweisZiel { anlage: "Anlage 1".into(), blatt: 3, obj: 2 }))));
        p.anlagen[0].sheets[2].objects.push(obj(2, verweis("=B", None)));
        p.anlagen[0].sheets[0].objects.push(obj(3, verweis("=C", Some(VerweisZiel { anlage: "Anlage 1".into(), blatt: 2, obj: 99 }))));
        p.anlagen[0].sheets[0].objects.push(obj(4, verweis("=D", Some(VerweisZiel { anlage: "fehlt".into(), blatt: 1, obj: 0 }))));
        p.anlagen[0].sheets[0].objects.push(obj(10, bus(20.0, 7, "", "2/1")));
        p.anlagen[0].sheets[1].objects.push(obj(11, bus(20.0, 7, "1/8", "")));
        p
    }

    #[test]
    fn verweisziele_aufloesen() {
        let p = projekt();
        let s0 = SheetRef { anlage: 0, sheet: 0 };
        // Gegenverweis vorhanden → Blatt 3 und Objekt 2
        let z = |id| match &p.find_obj(id).unwrap().1.kind {
            Kind::Verweis { ziel, .. } => verweis_ziel(&p, ziel.as_ref()),
            _ => unreachable!(),
        };
        assert_eq!(z(1), Ok(Sprung { blatt: SheetRef { anlage: 0, sheet: 2 }, obj: Some(2), at: None }));
        // Gegenverweis fehlt → nur das Blatt (Nummer 2)
        assert_eq!(z(3), Ok(Sprung { blatt: SheetRef { anlage: 0, sheet: 1 }, obj: None, at: None }));
        // kein Ziel / unbekannte Anlage → Meldung
        assert!(z(2).is_err());
        assert!(z(4).unwrap_err().contains("fehlt"));
        let f = flaechen(&p, s0, &[]);
        assert_eq!(f.iter().filter(|f| f.ziel.is_ok()).count(), 3);
    }

    #[test]
    fn ziel_setzen_verknuepft_beide_seiten() {
        let mut p = projekt();
        let s0 = SheetRef { anlage: 0, sheet: 0 };
        let s2 = SheetRef { anlage: 0, sheet: 2 };
        // 1 ↔ 2 verknüpfen: 2 zeigt danach zurück auf 1 (Blatt 1)
        assert_eq!(verknuepfen(&mut p, 1, s2, Some(2)), None);
        assert_eq!(ziel_von(&p, 2), Some(VerweisZiel { anlage: "Anlage 1".into(), blatt: 1, obj: 1 }));
        // 3 auf 2 richten: 2 war mit 1 verbunden → 1 wird gelöst
        assert!(verknuepfen(&mut p, 3, s2, Some(2)).is_some());
        assert_eq!(ziel_von(&p, 1), None);
        assert_eq!(ziel_von(&p, 2).map(|z| z.obj), Some(3));
        // nur Blatt: 3 löst den Gegenverweis 2
        verknuepfen(&mut p, 3, s0, None);
        assert_eq!(ziel_von(&p, 3), Some(VerweisZiel { anlage: "Anlage 1".into(), blatt: 1, obj: 0 }));
        assert_eq!(ziel_von(&p, 2), None);
    }

    #[test]
    fn sammelschiene_querverweis() {
        let p = projekt();
        let s0 = SheetRef { anlage: 0, sheet: 0 };
        let s1 = SheetRef { anlage: 0, sheet: 1 };
        // Ende auf Blatt 1 → Anfang auf Blatt 2
        assert_eq!(busbar_ziel(&p, s0, 10, true), Some(Sprung { blatt: s1, obj: Some(11), at: Some(P::new(20.0, 40.0)) }));
        // Anfang auf Blatt 2 → Ende auf Blatt 1
        assert_eq!(busbar_ziel(&p, s1, 11, false), Some(Sprung { blatt: s0, obj: Some(10), at: Some(P::new(120.0, 40.0)) }));
        assert_eq!(busbar_ziel(&p, s0, 10, false), None);
        // Klick auf "2/1" hinter dem Ende, nicht aber auf die Schiene selbst
        assert!(busbar_text_at(&p, s0, P::new(124.0, 40.0), 0.2).is_some());
        assert!(busbar_text_at(&p, s0, P::new(60.0, 40.0), 0.2).is_none());
        assert!(busbar_text_at(&p, s1, P::new(17.0, 40.0), 0.2).is_some());
    }

    #[test]
    fn drehen_laesst_text_waagerecht() {
        let mut k = verweis("=A", None);
        for n in 1..=4 {
            drehen(&mut k, 1);
            let Kind::Verweis { pos, text_pos, dir, vertical, .. } = &k else { unreachable!() };
            assert!(!vertical, "Drehung {n}: Text darf nicht hochkant werden");
            assert_eq!(*dir, [270, 180, 90, 0][n - 1]);
            // der Text bleibt beim Drehen an derselben Stelle, nur der Pfeil dreht um seine Spitze
            assert_eq!((*pos, *text_pos), (P::new(50.0, 50.0), P::new(55.0, 49.0)));
        }
        assert_eq!(k, verweis("=A", None));
    }
}
