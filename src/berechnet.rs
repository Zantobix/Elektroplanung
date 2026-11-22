//! Berechnete Listenblätter und -tabellen: Inhaltsverzeichnis (Blattart 1), Querverweise (Blattart 2) und
//! Artikelliste (Blattart 6, im Original „Stückliste“), dazu die Tabellen mit Daten „Inhaltsverzeichnis“ bzw.
//! „Querverweise“ (TSTab). Der Inhalt wird wie im Original nicht gespeichert, sondern bei jedem Zeichnen aus dem
//! Projekt berechnet und folgt damit jeder Änderung.
//!
//! Belegt aus dem Original (Listenaufbau 0x48e530, Blattanlage 0x461960, Stringtabelle und Texte der Exe):
//! - Inhaltsverzeichnis: Spalten „Seite“, „Bezeichnung“ (Stringtabelle 4000/4001); je Blatt der Anlage eine Zeile.
//! - Querverweise: Spalten „Objekt“, „Blatt/Pfad“ (4010/4011). Aufgenommen werden Symbole mit Kennzeichnung, deren
//!   Freigabe „Querverweise“ gesetzt ist (VSY Feld 20 Bit 0 = aus); je Kennzeichnung eine Zeile, natürlich sortiert,
//!   die Fundstellen „Blatt/Pfad“ sortiert und durch Leerzeichen getrennt. Ein Symbol mit Eigentümer (Laufzeit
//!   +0xb4, VSY › OWR) steht unter der Kennzeichnung des Eigentümers (0x46e9f4 mit dessen Blatt und Anlage) mit
//!   seiner eigenen Fundstelle und wird unabhängig von seiner eigenen Freigabe aufgenommen (Bedingung im
//!   Listenaufbau: „Freigabe und Kennzeichnung“ oder „Eigentümer“). Aufnahme des Originals: „-K1“ mit „6/5 6/6“
//!   (Spule und Schließer), die eigene Vorlage „-Q1+W2“ des Schließers fehlt.
//! - Stückliste: Kopf je Zusatz – Summenliste „Bezeichnung, Menge, Typnummer, Hersteller, Bestellnummer“,
//!   Summe+BMK „Betriebsmittelkennzeichen, Menge, Bezeichnung, Typnummer, Hersteller, Bestellnummer“ (die
//!   Kennzeichen durch Leerzeichen getrennt), Einzelliste „Betriebsmittelkennzeichen, Bezeichnung, Typnummer,
//!   Hersteller, Bestellnummer“ (je Verwendung eine Zeile). Reihenfolge nach Bauteil (0x49b014).
//! - Zusatz im Blatt (BLT Feld 32 Byte 1): Querverweise 1–3 Tabellen nebeneinander, Stückliste 0/1/2 wie oben.
//! - Platz je Blatt: Tabellen von höchstens 80 mm Breite, Zeilen 5 mm, je Tabelle eine Kopfzeile; reicht das
//!   Inhaltsverzeichnis nicht, stehen zwei Tabellen nebeneinander. „Zusammenhängende Blätter ergeben eine
//!   zusammenhängende Liste“ (Anleitung 4.2); fehlende Folgeblätter legt `folgeblaetter_anlegen` an.
//! Spaltenbreiten, Ränder und Zeilenhöhe (5 mm) vermessen an Bildschirmaufnahmen des Originals (21.11.2026).
//! Der Rahmen beginnt wie bei allen Querblättern 10 mm unter der Blattkante (oberer Rand der Anlage, `Sheet::frame`).

use crate::canvas::{bmk_ctx, draw_grid, natural_key, times, DrawCtx};
use crate::model::*;
use crate::symbols;
use eframe::egui::{Align2, Color32, Stroke};

/// Zeilenhöhe der Listenblätter in mm (wie die Sicherungsübersicht im Original-Ausdruck)
pub const ZEILE: f32 = 5.0;
/// Abstand der Listentabellen vom oberen Rahmen in mm (Aufnahmen des Originals: 10,1 mm)
const OBEN: f32 = 10.0;

pub const KOPF_INHALT: [&str; 2] = ["Seite", "Bezeichnung"];
pub const KOPF_QUER: [&str; 2] = ["Objekt", "Blatt/Pfad"];

/// Seitenangabe eines Blattes: Nummer, Subindex als „.n“; bei Blattnummer 0 der Subindex als Kleinbuchstabe
/// (Anleitung 4.3: „a, b, c usw.“).
pub fn seite(s: &Sheet) -> String {
    match (s.number, s.subindex) {
        (n, 0) => n.to_string(),
        (0, i) => ((b'a' + (i - 1).min(25)) as char).to_string(),
        (n, i) => format!("{n}.{i}"),
    }
}

/// Lage eines Punktes als „Blatt/Pfad“ (Pfad = Strompfad, gezählt ab 1).
pub fn blatt_pfad(s: &Sheet, x: f32) -> String {
    format!("{}/{}", seite(s), crate::epr::column_at(s, x))
}

/// Artikelliste als Summenliste: je Bauteil Menge und Daten (Bauteile aller Symbole der Anlage) –
/// Spaltenfolge Menge, Bezeichnung, Typnummer, Hersteller, Bestellnummer (Listen Export).
pub fn artikel_rows(p: &Project, a: &Anlage) -> Vec<Vec<String>> {
    let mut n: std::collections::BTreeMap<usize, usize> = Default::default();
    for o in a.sheets.iter().flat_map(|s| &s.objects) {
        // alle Bauteile der Bauteilliste des Objekts
        for i in crate::bauteilliste::liste(p, o.id) {
            *n.entry(i).or_default() += 1;
        }
    }
    let mut rows: Vec<Vec<String>> = n.into_iter().filter_map(|(i, c)| p.bauteile.get(i).map(|b| vec![c.to_string(), b.name.clone(), b.typ.clone(), b.firma.clone(), b.artikel.clone()])).collect();
    rows.sort_by_key(|r| natural_key(&r[1]));
    rows
}

/// Kennzeichnung eines Objekts mit Bauteil für die Stückliste (Symbol: angezeigte BMK, mit Eigentümer dessen;
/// Klemme: Leiste.Nummer).
fn kennzeichen(anlagen: &[Anlage], a: &Anlage, s: &Sheet, o: &Obj) -> String {
    match &o.kind {
        Kind::Symbol { .. } => crate::eigentuemer::anzeige_in(anlagen, o, &bmk_ctx(a, s)),
        Kind::Terminal { leiste, nr, .. } => format!("{leiste}.{nr}"),
        _ => String::new(),
    }
}

/// Stückliste (Blattart 6) nach Zusatz: 0 Summenliste, 1 Summe+BMK, 2 Einzelliste. Rückgabe: Kopf, Zeilen.
pub fn stueckliste(p: &Project, a: &Anlage, zusatz: u8) -> (Vec<&'static str>, Vec<Vec<String>>) {
    // je Bauteil (Index = Reihenfolge im Projekt) die Kennzeichen der Verwendungen
    let mut teile: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
    for s in &a.sheets {
        for o in &s.objects {
            for i in crate::bauteilliste::liste(p, o.id).into_iter().filter(|i| *i < p.bauteile.len()) {
                teile.entry(i).or_default().push(kennzeichen(&p.anlagen, a, s, o));
            }
        }
    }
    let daten = |i: usize| {
        let b = &p.bauteile[i];
        [b.name.clone(), b.typ.clone(), b.firma.clone(), b.artikel.clone()]
    };
    match zusatz {
        1 => {
            let rows = teile
                .into_iter()
                .map(|(i, mut k)| {
                    let n = k.len();
                    k.retain(|x| !x.is_empty());
                    k.sort_by_key(|x| natural_key(x));
                    let [name, typ, firma, art] = daten(i);
                    vec![k.join(" "), n.to_string(), name, typ, firma, art]
                })
                .collect();
            (vec!["Betriebsmittelkennzeichen", "Menge", "Bezeichnung", "Typnummer", "Hersteller", "Bestellnummer"], rows)
        }
        2 => {
            let mut rows = vec![];
            for (i, mut k) in teile {
                k.sort_by_key(|x| natural_key(x));
                for b in k {
                    let [name, typ, firma, art] = daten(i);
                    rows.push(vec![b, name, typ, firma, art]);
                }
            }
            (vec!["Betriebsmittelkennzeichen", "Bezeichnung", "Typnummer", "Hersteller", "Bestellnummer"], rows)
        }
        _ => {
            let rows = teile
                .into_iter()
                .map(|(i, k)| {
                    let [name, typ, firma, art] = daten(i);
                    vec![name, k.len().to_string(), typ, firma, art]
                })
                .collect();
            (vec!["Bezeichnung", "Menge", "Typnummer", "Hersteller", "Bestellnummer"], rows)
        }
    }
}

/// Inhaltsverzeichnis: alle Blätter der Anlage nach Nummer und Subindex (Seite, Bezeichnung) mit Blattnummer.
pub fn inhalt(a: &Anlage) -> Vec<(u32, Vec<String>)> {
    let mut v: Vec<&Sheet> = a.sheets.iter().collect();
    v.sort_by_key(|s| (s.number, s.subindex));
    v.into_iter().map(|s| (s.number, vec![seite(s), s.name.clone()])).collect()
}

/// Querverweisliste der Anlage: je Kennzeichnung (natürlich sortiert) die Fundstellen „Blatt/Pfad“; Symbole mit
/// Eigentümer stehen unter dessen Kennzeichnung.
pub fn querverweise(p: &Project, a: &Anlage) -> Vec<Vec<String>> {
    let mut map: Vec<(String, Vec<String>)> = vec![];
    for s in &a.sheets {
        if s.blattart() != 3 {
            continue;
        }
        let ctx = bmk_ctx(a, s);
        for o in &s.objects {
            let Kind::Symbol { bmk, ext, pos, .. } = &o.kind else { continue };
            let b = match crate::eigentuemer::kennzeichnung(p, o) {
                Some(k) => k,
                None if ext.querverweise => symbols::display_bmk(bmk, ext.counter, &ctx),
                None => continue,
            };
            if b.trim().is_empty() {
                continue;
            }
            let at = blatt_pfad(s, pos.x);
            match map.iter_mut().find(|(k, _)| *k == b) {
                Some((_, v)) => {
                    if !v.contains(&at) {
                        v.push(at)
                    }
                }
                None => map.push((b, vec![at])),
            }
        }
    }
    map.sort_by_key(|(k, _)| natural_key(k));
    map.into_iter()
        .map(|(k, mut v)| {
            v.sort_by_key(|x| natural_key(x));
            vec![k, v.join(" ")]
        })
        .collect()
}

/// Inhalt einer Tabelle mit berechneten Daten (TSTab „Daten“): 1 Inhaltsverzeichnis ab Blattnummer `ab`,
/// 2 Querverweise ab Eintrag `ab` (ab 1). Rückgabe: Spaltenzahl und Zellen zeilenweise (mit Kopfzeile, falls
/// `kopf`), höchstens `zeilen` Zeilen. None bei Benutzerdaten.
pub fn tabelle(p: &Project, a: &Anlage, art: u8, ab: u32, kopf: bool, zeilen: usize) -> Option<(usize, Vec<String>)> {
    let (head, rows): (&[&str], Vec<Vec<String>>) = match art {
        1 => (&KOPF_INHALT, inhalt(a).into_iter().filter(|(n, _)| *n >= ab).map(|(_, r)| r).collect()),
        2 => (&KOPF_QUER, querverweise(p, a).into_iter().skip(ab.saturating_sub(1) as usize).collect()),
        _ => return None,
    };
    let mut cells: Vec<String> = vec![];
    if kopf {
        cells.extend(head.iter().map(|h| h.to_string()));
    }
    for r in rows {
        cells.extend(r);
    }
    cells.truncate(zeilen.max(1) * head.len());
    Some((head.len(), cells))
}

/// Zeilen der Tabelle „Querverweise“ im Dialog: Kopfzeile auf Wunsch, dann die Einträge ab Nummer `ab` (ab 1).
pub fn quer_zellen(rows: &[Vec<String>], ab: u32, kopf: bool) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = vec![];
    if kopf {
        out.push(KOPF_QUER.iter().map(|s| s.to_string()).collect());
    }
    out.extend(rows.iter().skip(ab.saturating_sub(1) as usize).cloned());
    if out.is_empty() {
        out.push(vec![String::new(); KOPF_QUER.len()]);
    }
    out
}

/// Aufteilung eines Listenblattes: Anzahl Tabellen nebeneinander und Zeilen je Tabelle (mit Kopfzeile).
fn aufteilung(p: &Project, sheet: &Sheet, eintraege: usize) -> (usize, usize) {
    let (fx0, fy0, fx1, fy1) = sheet.frame();
    let zeilen = (((fy1 - p.title_h() - 3.0) - (fy0 + OBEN)) / ZEILE).floor().max(2.0) as usize;
    let max_t = (((fx1 - fx0) - 12.0) / 80.0).floor().max(1.0) as usize;
    let t = match sheet.blattart() {
        2 => (sheet.zusatz.max(1) as usize).min(max_t),
        1 if eintraege > zeilen - 1 => 2.min(max_t),
        _ => 1,
    };
    (t, zeilen)
}

type Liste = (Vec<&'static str>, Vec<f32>, Vec<Vec<String>>);

/// Kopf, Spaltenbreiten (ohne die letzte, die den Rest füllt) und Zeilen eines Listenblattes.
fn liste(p: &Project, sheet: &Sheet, a: &Anlage) -> Option<Liste> {
    Some(match sheet.blattart() {
        1 => (KOPF_INHALT.to_vec(), vec![13.0], inhalt(a).into_iter().map(|(_, r)| r).collect()),
        2 => (KOPF_QUER.to_vec(), vec![29.6], querverweise(p, a)),
        6 => {
            let (head, rows) = stueckliste(p, a, sheet.zusatz);
            // feste Breite je Spalte (Bildschirmaufnahmen des Originals vom 21.11.2026), die letzte füllt den Rest
            let w = head[..head.len() - 1]
                .iter()
                .map(|h| match *h {
                    "Betriebsmittelkennzeichen" => 56.8,
                    "Menge" => 16.8,
                    "Bezeichnung" => 28.9,
                    "Typnummer" => 27.7,
                    _ => 23.1,
                })
                .collect();
            (head, w, rows)
        }
        _ => return None,
    })
}

/// Einträge je Blatt (ohne Kopfzeilen) eines Listenblattes.
fn je_blatt(p: &Project, sheet: &Sheet, eintraege: usize) -> usize {
    let (t, z) = aufteilung(p, sheet, eintraege);
    t * z.saturating_sub(1)
}

/// Wie viele Blätter die Liste dieses Blattes insgesamt braucht (mindestens 1).
pub fn blaetter_noetig(p: &Project, sheet: &Sheet, a: &Anlage) -> usize {
    let Some((_, _, rows)) = liste(p, sheet, a) else { return 1 };
    let n = je_blatt(p, sheet, rows.len()).max(1);
    rows.len().div_ceil(n).max(1)
}

/// Fehlende Folgeblätter eines Listenblattes anlegen (gleiche Art und Zusatz, gleiche Nummer, nächster Subindex),
/// wie das Original nach der Rückfrage „Sollen alle nötigen Blätter erzeugt werden?“. Rückgabe: Anzahl neuer
/// Blätter oder die Meldung des Originals, wenn die nötige Seite schon vergeben ist.
pub fn folgeblaetter_anlegen(p: &mut Project, r: SheetRef) -> Result<usize, String> {
    let Some(a) = p.anlagen.get(r.anlage) else { return Ok(0) };
    let Some(s) = a.sheets.get(r.sheet) else { return Ok(0) };
    let art = s.blattart();
    if !matches!(art, 1 | 2 | 6) {
        return Ok(0);
    }
    // nur das erste Blatt einer zusammenhängenden Folge rechnet
    let erstes = r.sheet - crate::klemmplan::folge_index(a, s);
    let s0 = &a.sheets[erstes];
    let noetig = blaetter_noetig(p, s0, a);
    let vorhanden = a.sheets[erstes..].iter().take_while(|x| x.blattart() == art).count();
    if noetig <= vorhanden {
        return Ok(0);
    }
    let last = &a.sheets[erstes + vorhanden - 1];
    let (nr, sub0) = (last.number, last.subindex);
    let mut neu = vec![];
    for k in 1..=(noetig - vorhanden) {
        let sub = sub0 as usize + k;
        if sub > 255 || a.sheets.iter().any(|x| x.number == nr && x.subindex as usize == sub) {
            let was = if art == 1 { "'s Inhaltsverzeichnis" } else { " die Liste" };
            return Err(format!("Es kann kein Folgeblatt für{was} erstellt werden da die dafür\nbenötigte Seite: {nr}.{sub} schon vergeben ist!\n(Blattnummer: {nr}     Subindex: {sub})"));
        }
        let mut sh = Sheet::new(nr);
        sh.subindex = sub as u8;
        sh.name = s0.name.clone();
        sh.art = art;
        sh.art_fest = true;
        sh.zusatz = s0.zusatz;
        sh.portrait = s0.portrait;
        sh.columns = s0.columns;
        neu.push(sh);
    }
    let n = neu.len();
    let at = erstes + vorhanden;
    let a = &mut p.anlagen[r.anlage];
    for (i, sh) in neu.into_iter().enumerate() {
        a.sheets.insert(at + i, sh);
    }
    Ok(n)
}

/// Berechnetes Listenblatt zeichnen; false, wenn die Blattart hier nicht behandelt wird.
pub fn draw(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage) -> bool {
    let Some((head, widths, rows)) = liste(dc.project, sheet, anlage) else { return false };
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (fx0, fy0, fx1, _) = sheet.frame();
    let stroke = Stroke::new((0.18 * z).max(1.0), Color32::BLACK);
    let (tabellen, n) = aufteilung(dc.project, sheet, rows.len());
    // Lage wie in den Bildschirmaufnahmen des Originals: 5 mm vom Rahmen links, rechts 7,2 mm (Artikelliste) bzw.
    // 5,3 mm, 10 mm unter dem Rahmen; mehrere Tabellen teilen die Breite mit 2,4 mm Abstand
    let rechts = if sheet.blattart() == 6 { 7.2 } else { 5.3 };
    let (x0, x1, y0) = (fx0 + 5.0, fx1 - rechts, fy0 + OBEN);
    let abstand = 2.4;
    let tw = ((x1 - x0) - abstand * (tabellen - 1) as f32) / tabellen as f32;
    let mut w: Vec<f32> = widths.iter().map(|v| v.min(tw * 0.6)).collect();
    w.push((tw - w.iter().sum::<f32>()).max(10.0));
    let off = crate::klemmplan::folge_index(anlage, sheet) * je_blatt(dc.project, sheet, rows.len());
    let kopf: Vec<String> = head.iter().map(|h| h.to_string()).collect();
    for t in 0..tabellen {
        let tx = x0 + t as f32 * (tw + abstand);
        let start = off + t * n.saturating_sub(1);
        draw_grid(p, xf, P::new(tx, y0), P::new(tx + w.iter().sum::<f32>(), y0 + n as f32 * ZEILE), &w, &vec![ZEILE; n], stroke);
        let body = rows.iter().skip(start).take(n.saturating_sub(1));
        for (r, row) in std::iter::once(&kopf).chain(body).enumerate() {
            let mut x = tx;
            for (c, cw) in w.iter().enumerate() {
                if let Some(t) = row.get(c).filter(|t| !t.is_empty()) {
                    let cr = eframe::egui::Rect::from_min_max(xf.s(P::new(x, y0 + r as f32 * ZEILE)), xf.s(P::new(x + cw, y0 + (r + 1) as f32 * ZEILE)));
                    p.with_clip_rect(cr).text(xf.s(P::new(x + 0.8, y0 + (r as f32 + 0.5) * ZEILE)), Align2::LEFT_CENTER, t, times(2.6 * z), Color32::BLACK);
                }
                x += cw;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, bmk: &str, x: f32) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "k".into(), pos: P::new(x, 50.0), rot: 0, mirror_h: false, mirror_v: false, bmk: bmk.into(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn parts_are_summed() {
        let mut p = Project::new_default();
        p.bauteile.push(Bauteil { name: "LS B16".into(), typ: "S201".into(), firma: "ABB".into(), artikel: "123".into(), ..Default::default() });
        for id in [5, 6] {
            p.anlagen[0].sheets[0].objects.push(Obj { id, kind: Kind::Line { a: P::default(), b: P::new(1.0, 1.0) }, pen: 0, fixed: false, group: None, raw: None });
            p.part_of.insert(id, 0);
        }
        assert_eq!(artikel_rows(&p, &p.anlagen[0]), vec![vec!["2", "LS B16", "S201", "ABB", "123"]]);
        assert_eq!(inhalt(&p.anlagen[0])[0].1, ["1".to_string(), "Blatt 1".to_string()]);
    }

    #[test]
    fn stueckliste_varianten() {
        let mut p = Project::new_default();
        p.bauteile.push(Bauteil { name: "Schütz".into(), typ: "A9".into(), firma: "X".into(), artikel: "77".into(), ..Default::default() });
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K2", 30.0));
        p.anlagen[0].sheets[0].objects.push(sym(2, "-K1", 60.0));
        p.part_of.insert(1, 0);
        p.part_of.insert(2, 0);
        let a = &p.anlagen[0];
        let (h, r) = stueckliste(&p, a, 0);
        assert_eq!(h[0], "Bezeichnung");
        assert_eq!(r, vec![vec!["Schütz", "2", "A9", "X", "77"]]);
        let (h, r) = stueckliste(&p, a, 1);
        assert_eq!((h.len(), r[0][0].as_str(), r[0][1].as_str()), (6, "-K1 -K2", "2"));
        let (h, r) = stueckliste(&p, a, 2);
        assert_eq!((h.len(), r.len(), r[0][0].as_str()), (5, 2, "-K1"));
    }

    #[test]
    fn querverweise_und_seiten() {
        let mut p = Project::new_default();
        let mut s2 = Sheet::new(2);
        s2.objects.push(sym(3, "-K1", 200.0));
        s2.objects.push(sym(4, "-F10", 20.0));
        let mut aus = sym(5, "-F2", 20.0);
        if let Kind::Symbol { ext, .. } = &mut aus.kind {
            ext.querverweise = false;
        }
        s2.objects.push(aus);
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1", 20.0));
        p.anlagen[0].sheets.push(s2);
        let q = querverweise(&p, &p.anlagen[0]);
        assert_eq!(q, vec![vec!["-F10".to_string(), "2/1".into()], vec!["-K1".into(), "1/1 2/6".into()]]);
        let (cols, cells) = tabelle(&p, &p.anlagen[0], 2, 2, true, 10).unwrap();
        assert_eq!((cols, cells), (2, ["Objekt", "Blatt/Pfad", "-K1", "1/1 2/6"].map(String::from).to_vec()));
        let mut d = Sheet::new(0);
        d.subindex = 2;
        assert_eq!(seite(&d), "b");
        assert_eq!(tabelle(&p, &p.anlagen[0], 0, 0, true, 3), None);
    }

    #[test]
    fn querverweise_mit_eigentuemer() {
        let mut p = Project::new_default();
        p.anlagen[0].sheets[0].objects.push(sym(1, "-K1", 20.0));
        // Kontakt mit eigener Vorlage und ohne eigene Freigabe: steht trotzdem unter der Spule
        let mut k = sym(2, "-Q1", 60.0);
        if let Kind::Symbol { ext, .. } = &mut k.kind {
            ext.querverweise = false;
        }
        p.anlagen[0].sheets[0].objects.push(k);
        p.anlagen[0].sheets[0].objects.push(sym(3, "-Q2", 200.0));
        assert!(crate::eigentuemer::setzen(&mut p, 1, &[2, 3]));
        assert_eq!(querverweise(&p, &p.anlagen[0]), vec![vec!["-K1".to_string(), "1/1 1/2 1/6".into()]]);
    }

    #[test]
    fn folgeblaetter() {
        let mut p = Project::new_default();
        for i in 0..80 {
            p.anlagen[0].sheets[0].objects.push(sym(100 + i, &format!("-F{i}"), 20.0));
        }
        let mut q = Sheet::new(0);
        q.subindex = 1;
        q.art = 2;
        q.art_fest = true;
        q.zusatz = 1;
        p.anlagen[0].sheets.insert(0, q);
        let n = blaetter_noetig(&p, &p.anlagen[0].sheets[0], &p.anlagen[0]);
        assert!(n >= 2, "{n}");
        assert_eq!(folgeblaetter_anlegen(&mut p, SheetRef { anlage: 0, sheet: 0 }), Ok(n - 1));
        assert_eq!(p.anlagen[0].sheets[1].subindex, 2);
        assert_eq!(p.anlagen[0].sheets[1].blattart(), 2);
        // nichts mehr zu tun
        assert_eq!(folgeblaetter_anlegen(&mut p, SheetRef { anlage: 0, sheet: 1 }), Ok(0));
    }
}
