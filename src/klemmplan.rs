//! Klemmplan (berechnetes Blatt, Blattart 4): Zeilen je Klemme, Verbindungsverfolgung "Von Ziel"/"Nach Ziel",
//! Brücken und die beiden Darstellungen des Originals (Standard und "Alter Klemmenplan").
//!
//! Verfolgung (abgeleitet aus dem Original-Ausdruck des Beispielprojekts, Klemmplan -X1/-X2):
//! - An einer Klemmstelle beginnen alle Verbindungen, deren Anfang oder Ende auf dem Klemmenkreis liegt
//!   (Mitte ± 0,8 mm). Sie werden nach der Lage des Anschlusspunktes sortiert (oben vor links/rechts vor unten,
//!   bei gleicher Lage in Objektreihenfolge).
//! - Jede Verbindung wird bis zum nächsten Anschluss verfolgt: Symbolanschluss (Ziel = BMK und
//!   Anschlussbezeichnung), Klemmstelle einer anderen Klemmleiste (Ziel = Leiste und Nummer), Sammelschiene
//!   (Ziel = "Name (Schiene Name)") oder Klemmstelle derselben Leiste (Brücke, kein Ziel).
//! - Treffen sich Verbindungsenden (Toleranz 0,05 mm), geht es in allen anschließenden Verbindungen weiter.
//!   Endet eine Verbindung auf dem Verlauf einer anderen (Knick oder Strecke), geht es in dieser nur zu ihrem
//!   Anfang weiter – so wie das Original die Ziele zuordnet. Kreuzende Verbindungen sind nicht verbunden.
//! - Von/Nach wie im Original (Klemmplan-Routine 0x4830a8): jede Verbindung an der Klemme belegt eine von vier
//!   Richtungen – vom Klemmenende zum Nachbarpunkt der Verbindung: 0 oben, 1 links, 2 rechts, 3 unten; die erste
//!   Verbindung je Richtung (Objektreihenfolge) zählt, Brücken belegen ihre Richtung ebenfalls. "Ziel Wechsel"
//!   tauscht oben/unten und links/rechts. "Von Ziel" = erste Richtung mit Ziel; "Nach Ziel" = unten, wenn Von
//!   oben ist und unten ein Ziel hat, sonst die nächste Richtung mit Ziel.
//! - Symbole mit Eigentümer (VSY › OWR, z. B. Kontakt eines Relais) erscheinen mit der Kennung des Eigentümers
//!   (Ausdruck Seite 13: "-K1" statt "-Q1+W2"); Quelle ist wie überall `SymExt::eigentuemer` (`eigentuemer.rs`),
//!   also auch ein im Nachbau gesetzter Eigentümer.
//! - Funktion = Funktion der Klemme (Klemmen-Dialog), sonst die Pfadbeschreibung ihres Strompfades (Anleitung 4.2.4).
//! - Reihenfolge der Zeilen: Position in der Klemmleiste, dann Etage (wie das Original: Position · 10 + Etage).
//! - Brücken als Ketten in Spuren (`ketten`), nicht als zusammenhängende Gruppen.
//! - Folgeseiten: direkt aufeinander folgende Klemmplan-Blätter derselben Leiste setzen die Liste fort
//!   (Anleitung 4.2: „Zusammenhängende Blätter ergeben eine zusammenhängende … Liste“).
//! - Pfad = Blattnummer / Strompfad der Symbollage (bei Sammelschienen: ihres Anfangs).

use crate::canvas::{self, DrawCtx};
use crate::model::*;
use crate::symbols::{self, Prim, SymDef};
use eframe::egui::{self, Align2, Color32, Rect, Stroke};
use std::collections::VecDeque;

/// Toleranz für zusammenfallende Punkte in mm
pub const TOL: f32 = 0.05;
/// Radius des Klemmenkreises: Verbindungen enden an Mitte ± 0,8 mm
const KLEMME_R: f32 = 0.8;

/// Ziel einer Klemme: Kennung (BMK bzw. Leiste/Schiene), Anschluss ("KL") und Pfad ("Blatt/Strompfad").
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ziel {
    pub kennung: String,
    pub kl: String,
    pub pfad: String,
}

/// Eine Zeile des Klemmplans (eine Klemme bzw. eine Etage einer Klemme).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Zeile {
    pub nr: u32,
    /// Etage (0 = einstöckige Leiste, 1 = A, 2 = B …)
    pub level: u8,
    /// Bezeichnung wie auf dem Schaltplan ("-X1.3")
    pub label: String,
    pub von: Option<Ziel>,
    pub nach: Option<Ziel>,
    /// Funktion (Klemme bzw. Pfadbeschreibung)
    pub funktion: String,
    /// Position in der Klemmleiste (Reihenfolge)
    pub pos: u32,
}

/// Brücke zwischen Klemmen derselben Leiste: Zeilen (aufsteigend) und Spur in der Brückenspalte.
#[derive(Clone, Debug, PartialEq)]
pub struct Bruecke {
    pub lane: usize,
    pub rows: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Plan {
    pub zeilen: Vec<Zeile>,
    pub bruecken: Vec<Bruecke>,
    /// Höchste Etage (0/1 = einstöckig)
    pub etagen: u8,
}

/// Ein Treffer am Ende einer verfolgten Verbindung.
enum Treffer {
    Ziel(Ziel),
    /// Klemmstelle derselben Leiste (Index in `Netz::terms`)
    Bruecke(usize),
}

struct Klemmstelle {
    pos: P,
    leiste: String,
    nr: u32,
    level: u8,
    tpl: String,
    opt: crate::klemmen::KlemmOpt,
}

/// Belegung einer Richtung an der Klemme.
enum Slot {
    Ziel(Ziel),
    Bruecke,
}

/// Verbindungsdaten eines Blattes.
struct Netz<'a> {
    wires: Vec<&'a [P]>,
    /// "Verbindung ist Brücke" (VBG Feld 14 Bit 0x100 bzw. Verbindungsdialog) je Verbindung
    bruecke: Vec<bool>,
    /// Symbolanschlüsse: Lage, Ziel
    pins: Vec<(P, Ziel)>,
    terms: Vec<Klemmstelle>,
    /// Sammelschienen: Anfang, Ende x, Ziel
    bars: Vec<(P, f32, Ziel)>,
}

/// Strompfad-Angabe "Blatt/Pfad" für die waagerechte Lage `x`.
pub fn pfad(sheet: &Sheet, x: f32) -> String {
    let (fx0, _, fx1, _) = sheet.frame();
    let cols = sheet.columns.max(1);
    let cw = (fx1 - fx0) / cols as f32;
    let i = if cw > 0.0 { ((x - fx0) / cw).floor().clamp(0.0, (cols - 1) as f32) as usize } else { 0 };
    format!("{}/{}", sheet.number, sheet.col_label(i))
}

/// Anschlussbezeichnung (Textnummer des Anschlusses, VSY › ALT der Instanz, Alternativtext, Symboltext).
pub fn pin_name(def: &SymDef, pin: usize, ext: &SymExt, option: u16) -> String {
    let t = def.pin_info.get(pin).map(|i| i.0).unwrap_or(0);
    if t == 0 {
        return String::new();
    }
    let custom = ext.texts.iter().find(|(n, _)| *n == t).map(|(_, s)| s.clone());
    let alt = if ext.alt > 0 { def.alt_sets.get(ext.alt as usize - 1).and_then(|set| set.iter().find(|(n, _)| *n == t)).map(|(_, s)| s.clone()) } else { None };
    let own = || {
        def.prims.iter().enumerate().find_map(|(i, p)| match p {
            Prim::TextBox(_, _, s, _) if def.text_idx.get(i) == Some(&t) => Some(s.clone()),
            _ => None,
        })
    };
    let s = custom.or(alt).or_else(own).unwrap_or_default();
    symbols::expand_text(&s, def, &ext.params, option).replace("\r\n", " ").replace('\n', " ").trim().to_string()
}


/// Pfadbeschreibung des Strompfades bei `x` (leer, wenn keine eigene vergeben ist).
pub fn pfad_text(sheet: &Sheet, x: f32) -> String {
    let (fx0, _, fx1, _) = sheet.frame();
    let cols = sheet.columns.max(1);
    let cw = (fx1 - fx0) / cols as f32;
    let i = if cw > 0.0 { ((x - fx0) / cw).floor().clamp(0.0, (cols - 1) as f32) as usize } else { 0 };
    sheet.col_labels.get(i).map(|s| s.trim().to_string()).unwrap_or_default()
}

impl<'a> Netz<'a> {
    /// `anlagen`: wo Eigentümer gesucht werden (`eigentuemer::anzeige_in`)
    fn new(anlagen: &[Anlage], anlage: &Anlage, sheet: &'a Sheet, lib: &[SymDef]) -> Self {
        let ctx = canvas::bmk_ctx(anlage, sheet);
        let mut n = Netz { wires: vec![], bruecke: vec![], pins: vec![], terms: vec![], bars: vec![] };
        for o in &sheet.objects {
            match &o.kind {
                Kind::Wire { pts, opts, .. } if pts.len() >= 2 => {
                    n.wires.push(pts);
                    let raw = o.raw.as_deref().filter(|r| r.block.is("VBG") && r.kind == o.kind).map(|r| r.block.i32(14) & 0x100 != 0);
                    n.bruecke.push(raw.unwrap_or(false) || opts & WIRE_BRUECKE != 0);
                }
                Kind::Symbol { key, pos, rot, mirror_h, mirror_v, scale, option, flags, ext, .. } => {
                    let Some(def) = symbols::find(lib, key) else { continue };
                    let kennung = crate::eigentuemer::anzeige_in(anlagen, o, &ctx);
                    let pf = pfad(sheet, pos.x);
                    for (i, q) in def.pins.iter().enumerate() {
                        if !symbols::visible(*option, def.pin_vis.get(i)) || !symbols::flag_visible(*flags, def.pin_info.get(i).map(|x| &x.1)) {
                            continue;
                        }
                        let at = sym_xf(*q, *pos, *rot, *mirror_h, *mirror_v, *scale);
                        n.pins.push((at, Ziel { kennung: kennung.clone(), kl: pin_name(def, i, ext, *option), pfad: pf.clone() }));
                    }
                }
                Kind::Terminal { pos, leiste, nr, level, tpl, opt, .. } => n.terms.push(Klemmstelle { pos: *pos, leiste: leiste.clone(), nr: *nr, level: *level, tpl: tpl.clone(), opt: opt.clone() }),
                Kind::Busbar { a, x2, name, .. } => {
                    let kurz = anlage.schienen.get(name).map(|s| s.kurz.trim().to_string()).filter(|k| !k.is_empty()).unwrap_or_else(|| name.clone());
                    let x0 = a.x.min(*x2);
                    n.bars.push((*a, *x2, Ziel { kennung: format!("{kurz} (Schiene {name})"), kl: String::new(), pfad: pfad(sheet, x0) }));
                }
                _ => {}
            }
        }
        n
    }

    /// Ziel auf eine Klemme: Kennung = Leiste, Anschluss = Nummer ohne Etage (Ausdruck Seite 22: "-X1 8")
    fn term_ziel(&self, sheet: &Sheet, i: usize) -> Ziel {
        let t = &self.terms[i];
        Ziel { kennung: t.leiste.clone(), kl: t.nr.to_string(), pfad: pfad(sheet, t.pos.x) }
    }

    /// Was liegt am Punkt `p`? (Anschluss, Klemmstelle außer `origin`, Sammelschiene); `Err(())` = die
    /// Ursprungsklemme selbst (Verfolgung endet ohne Ergebnis).
    fn hit(&self, sheet: &Sheet, p: P, origin: usize) -> Result<Option<Treffer>, ()> {
        if let Some((_, z)) = self.pins.iter().find(|(q, _)| q.dist(p) <= TOL) {
            return Ok(Some(Treffer::Ziel(z.clone())));
        }
        let bar = |p: P| self.bars.iter().find(|(a, x2, _)| (p.y - a.y).abs() <= TOL && p.x >= a.x.min(*x2) - TOL && p.x <= a.x.max(*x2) + TOL).map(|(_, _, z)| z.clone());
        if let Some(i) = self.terms.iter().position(|t| t.pos.dist(p) <= KLEMME_R + TOL) {
            if i == origin {
                // kurze Verbindung innerhalb des Klemmenkreises auf eine Sammelschiene (Ausdruck Seite 20, Klemme 1B)
                return bar(p).map(|z| Some(Treffer::Ziel(z))).ok_or(());
            }
            return Ok(Some(if self.terms[i].leiste == self.terms[origin].leiste { Treffer::Bruecke(i) } else { Treffer::Ziel(self.term_ziel(sheet, i)) }));
        }
        if let Some((_, _, z)) = self.bars.iter().find(|(a, x2, _)| (p.y - a.y).abs() <= TOL && p.x >= a.x.min(*x2) - TOL && p.x <= a.x.max(*x2) + TOL) {
            return Ok(Some(Treffer::Ziel(z.clone())));
        }
        Ok(None)
    }

    /// Verbindung `w` von der Klemmstelle `origin` aus verfolgen (`from_start`: die Verbindung beginnt an der
    /// Klemme). Ergebnis: Treffer in Suchreihenfolge (nächste zuerst).
    fn trace(&self, sheet: &Sheet, origin: usize, w: usize, from_start: bool) -> Vec<Treffer> {
        let mut out = vec![];
        let mut seen = vec![false; self.wires.len()];
        seen[w] = true;
        let first = if from_start { *self.wires[w].last().unwrap() } else { self.wires[w][0] };
        let mut queue = VecDeque::from([first]);
        while let Some(p) = queue.pop_front() {
            match self.hit(sheet, p, origin) {
                Err(()) => continue,
                Ok(Some(t)) => {
                    out.push(t);
                    continue;
                }
                Ok(None) => {}
            }
            for (k, pts) in self.wires.iter().enumerate() {
                if seen[k] {
                    continue;
                }
                let (a, b) = (pts[0], *pts.last().unwrap());
                if a.dist(p) <= TOL {
                    seen[k] = true;
                    queue.push_back(b);
                } else if b.dist(p) <= TOL {
                    seen[k] = true;
                    queue.push_back(a);
                } else if pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) <= TOL) {
                    // Ende auf dem Verlauf einer anderen Verbindung: weiter zu deren Anfang
                    seen[k] = true;
                    queue.push_back(a);
                }
            }
        }
        out
    }

    /// Belegung der vier Richtungen (0 oben, 1 links, 2 rechts, 3 unten; mit "Ziel Wechsel" getauscht) und
    /// Brücken einer Klemmstelle.
    fn klemme(&self, sheet: &Sheet, i: usize) -> ([Option<Slot>; 4], Vec<usize>) {
        let c = self.terms[i].pos;
        let mut slots: [Option<Slot>; 4] = Default::default();
        let mut br = vec![];
        for (k, pts) in self.wires.iter().enumerate() {
            let n = pts.len();
            let from_start = if pts[0].dist(c) <= KLEMME_R + TOL {
                true
            } else if pts[n - 1].dist(c) <= KLEMME_R + TOL {
                false
            } else {
                continue;
            };
            // Richtung wie im Original: vom Klemmenende zum benachbarten Punkt der Verbindung (0,1 mm genau)
            let (p0, p1) = if from_start { (pts[0], pts[1]) } else { (pts[n - 1], pts[n - 2]) };
            let r = |v: f32| (v * 10.0).round() as i32;
            let mut dir = if r(p0.x) == r(p1.x) {
                if r(p0.y) < r(p1.y) {
                    3
                } else {
                    0
                }
            } else if r(p0.x) < r(p1.x) {
                2
            } else {
                1
            };
            if self.terms[i].opt.ziel_wechsel {
                dir = 3 - dir;
            }
            // Klemme derselben Leiste: Brücke nur über "Verbindung ist Brücke", sonst ein Ziel wie "-X2 2"
            // (Ausdruck Seite 39; Original: Verbindung +0x38 Bit 0x100)
            let hits: Vec<Treffer> = self
                .trace(sheet, i, k, from_start)
                .into_iter()
                .map(|t| match t {
                    Treffer::Bruecke(j) if !self.bruecke[k] => Treffer::Ziel(self.term_ziel(sheet, j)),
                    t => t,
                })
                .collect();
            br.extend(hits.iter().filter_map(|t| if let Treffer::Bruecke(j) = t { Some(*j) } else { None }));
            if slots[dir].is_none() {
                slots[dir] = match hits.into_iter().next() {
                    Some(Treffer::Ziel(z)) => Some(Slot::Ziel(z)),
                    Some(Treffer::Bruecke(_)) => Some(Slot::Bruecke),
                    None => None,
                };
            }
        }
        (slots, br)
    }
}

/// Von/Nach aus den vier Richtungen wie im Original.
fn von_nach(slots: &[Option<Slot>; 4]) -> (Option<Ziel>, Option<Ziel>) {
    let ziel = |d: usize| match &slots[d] {
        Some(Slot::Ziel(z)) => Some(z.clone()),
        _ => None,
    };
    let Some(v) = (0..4).find(|d| ziel(*d).is_some()) else { return (None, None) };
    let n = if v == 0 && ziel(3).is_some() { Some(3) } else { (v + 1..4).find(|d| ziel(*d).is_some()) };
    (ziel(v), n.and_then(ziel))
}

/// Klemmplan der Klemmleiste `leiste` einer Anlage berechnen (Klemmen aller Blätter der Anlage); Eigentümer
/// (`SymExt::eigentuemer`) werden nur in dieser Anlage gesucht – mit Projekt: `plan_projekt`.
pub fn plan(anlage: &Anlage, leiste: &str, lib: &[SymDef]) -> Plan {
    plan_in(std::slice::from_ref(anlage), anlage, leiste, lib)
}

/// Klemmplan wie `plan`, Eigentümer im ganzen Projekt gesucht (auch in anderen Anlagen).
pub fn plan_projekt(p: &Project, anlage: &Anlage, leiste: &str, lib: &[SymDef]) -> Plan {
    plan_in(&p.anlagen, anlage, leiste, lib)
}

fn plan_in(anlagen: &[Anlage], anlage: &Anlage, leiste: &str, lib: &[SymDef]) -> Plan {
    let mut plan = Plan::default();
    let mut links: Vec<((u32, u8), (u32, u8))> = vec![];
    let row_of = |plan: &mut Plan, nr: u32, level: u8, label: String, pos: u32| -> usize {
        if let Some(r) = plan.zeilen.iter().position(|z| z.nr == nr && z.level == level) {
            return r;
        }
        plan.zeilen.push(Zeile { nr, level, label, von: None, nach: None, funktion: String::new(), pos });
        plan.zeilen.len() - 1
    };
    for sheet in &anlage.sheets {
        if sheet.art == 4 || sheet.art == 5 {
            continue;
        }
        let netz = Netz::new(anlagen, anlage, sheet, lib);
        for i in 0..netz.terms.len() {
            let t = &netz.terms[i];
            if t.leiste != leiste {
                continue;
            }
            let label = crate::epr::terminal_label(&t.tpl, &t.leiste, t.nr, t.level);
            let r = row_of(&mut plan, t.nr, t.level, label, t.opt.pos(t.nr));
            let (slots, br) = netz.klemme(sheet, i);
            let (von, nach) = von_nach(&slots);
            let z = &mut plan.zeilen[r];
            if z.von.is_none() {
                z.von = von;
                z.nach = nach;
            } else if z.nach.is_none() {
                z.nach = von;
            }
            if z.funktion.is_empty() {
                z.funktion = if t.opt.funktion.trim().is_empty() { pfad_text(sheet, t.pos.x) } else { t.opt.funktion.trim().to_string() };
            }
            for j in br {
                links.push(((t.nr, t.level), (netz.terms[j].nr, netz.terms[j].level)));
            }
        }
    }
    // Klemmen ohne Klemmstelle auf einem Blatt (nur in der Leiste eingetragen)
    if let Some(k) = anlage.klemmen.iter().find(|k| k.name == leiste) {
        for nr in &k.nrs {
            if !plan.zeilen.iter().any(|z| z.nr == *nr) {
                plan.zeilen.push(Zeile { nr: *nr, level: 0, label: crate::epr::terminal_label("", leiste, *nr, 0), von: None, nach: None, funktion: String::new(), pos: *nr });
            }
        }
    }
    plan.zeilen.sort_by_key(|z| (z.pos, z.level, z.nr));
    plan.etagen = plan.zeilen.iter().map(|z| z.level).max().unwrap_or(0);
    plan.bruecken = ketten(&plan, &links);
    plan
}

/// Brücken als Ketten wie im Original (Klemmplan-Routine 0x4830a8 sammelt je Brückenverbindung das Paar
/// Klemme/Gegenklemme, 0x49a86c ordnet die Paare zu Ketten): jede Brücke ist eine Strecke zwischen zwei Zeilen.
/// Die Strecken werden nach ihrer oberen Zeile abgearbeitet und kommen in die erste Spur, deren letzte Kette eine
/// ihrer beiden Klemmen enthält (dann wird die Kette verlängert) oder schon darüber endet (dann beginnt dort eine
/// neue Kette). Eine Klemme kann so Ende einer Kette und Anfang einer anderen in einer zweiten Spur sein – so
/// zeigt es der Original-Ausdruck des Beispielprojekts (Klemmplan -X2 der Heizungssteuerung: 4A–7A in der
/// zweiten, 7A–12A in der ersten Spur), wo eine Zusammenfassung zu Gruppen eine einzige Linie 4A–12A ergäbe;
/// Klemmplan -X1 (Seite 12): Brücken 3–5 und 4–5 bilden eine Kette 3, 4, 5. Abgeleitet aus beiden Ausdrucken,
/// die Routine 0x49a86c selbst ist nur teilweise entschlüsselt.
pub fn ketten(plan: &Plan, links: &[((u32, u8), (u32, u8))]) -> Vec<Bruecke> {
    let idx = |k: (u32, u8)| plan.zeilen.iter().position(|z| (z.nr, z.level) == k);
    let mut strecken: Vec<(usize, usize)> = links
        .iter()
        .filter_map(|(a, b)| Some((idx(*a)?, idx(*b)?)))
        .filter(|(a, b)| a != b)
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    strecken.sort();
    strecken.dedup();
    // je Spur die Ketten (Zeilen aufsteigend); die letzte Kette einer Spur bestimmt, ob die Spur frei ist
    let mut spuren: Vec<Vec<Vec<usize>>> = vec![];
    for (oben, unten) in strecken {
        // Spur frei (letzte Kette endet über der Strecke) oder letzte Kette enthält eine ihrer Klemmen
        let teilt = |sp: &Vec<Vec<usize>>| sp.last().map(|k| k.contains(&oben) || k.contains(&unten)).unwrap_or(false);
        let frei = |sp: &Vec<Vec<usize>>| sp.last().and_then(|k| k.last()).map(|e| *e < oben).unwrap_or(true);
        match spuren.iter().position(|sp| teilt(sp) || frei(sp)) {
            Some(l) if teilt(&spuren[l]) => {
                let k = spuren[l].last_mut().unwrap();
                k.extend([oben, unten]);
                k.sort();
                k.dedup();
            }
            Some(l) => spuren[l].push(vec![oben, unten]),
            None => spuren.push(vec![vec![oben, unten]]),
        }
    }
    let mut v: Vec<Bruecke> = spuren.into_iter().enumerate().flat_map(|(lane, ks)| ks.into_iter().map(move |rows| Bruecke { lane, rows })).collect();
    v.sort_by_key(|b| (b.rows[0], b.lane));
    v
}

/// Folgeseite eines berechneten Blattes: Anzahl der unmittelbar vorangehenden Blätter derselben Art (beim
/// Klemmplan derselben Klemmleiste) in der Anlage – sie zeigen die Liste davor (Anleitung 4.2: zusammenhängende
/// Blätter ergeben eine zusammenhängende Liste).
pub fn folge_index(anlage: &Anlage, sheet: &Sheet) -> usize {
    let i = anlage.sheets.iter().position(|s| std::ptr::eq(s, sheet)).or_else(|| anlage.sheets.iter().position(|s| s.number == sheet.number && s.subindex == sheet.subindex && s.art == sheet.art));
    let Some(i) = i else { return 0 };
    anlage.sheets[..i].iter().rev().take_while(|s| s.art == sheet.art && (sheet.art != 4 || s.klm == sheet.klm)).count()
}

/// Zeilen je Klemmplan-Blatt (gleiche Rechnung wie beim Zeichnen).
pub fn rows_per_sheet(project: &Project, sheet: &Sheet) -> usize {
    let (_, fy0, _, fy1) = sheet.frame();
    let (y0, y1) = (fy0 + 10.0, fy1 - project.title_h() - 4.0);
    ((((y1 - y0 - 9.0) / 4.4).floor().max(3.0)) as usize).saturating_sub(2)
}

/// Klemmplan zeichnen (Original-Ausdruck Seite 12 bzw. "Alter Klemmenplan" in den Projekteigenschaften).
pub fn draw(dc: &DrawCtx, sheet: &Sheet, anlage: &Anlage) {
    let p = dc.painter;
    let xf = dc.xf;
    let z = xf.z;
    let (fx0, fy0, fx1, fy1) = sheet.frame();
    let stroke = Stroke::new((0.18 * z).max(1.0), Color32::BLACK);
    let text = |at: P, t: &str, em: f32, a: Align2| {
        if !t.is_empty() {
            p.text(xf.s(at), a, t, canvas::times(em * z), Color32::BLACK);
        }
    };
    let ki = anlage.klemmen.iter().find(|k| k.id == sheet.klm);
    let name = ki.map(|k| k.name.as_str()).unwrap_or("");
    let plan = plan_projekt(dc.project, anlage, name, dc.lib);
    let alt = dc.project.alter_klemmenplan;
    let mehr = plan.etagen > 1;
    let (x0, x1) = (fx0 + 4.0, fx1 - 4.0);
    let y0 = fy0 + 10.0;
    let y1 = fy1 - dc.project.title_h() - 4.0;
    // Spalten wie im Original (FUN 0x48e530, Fall Klemmplan): Standard 9 Spalten "Funktion | Kennung KL Pfad |
    // Nr | Brücken | Kennung KL Pfad"; alter Klemmenplan 8 Spalten "Kennung Anschluß Pfad | | Klemme |
    // Kennung Anschluß Pfad" (ohne Funktion und Brücken)
    let base: &[f32] = if alt { &[57.0, 18.0, 30.0, 6.0, 30.0, 57.0, 18.0, 64.0] } else { &[57.0, 33.6, 9.4, 42.0, 9.5, 20.5, 35.0, 8.0, 64.0] };
    let f = (x1 - x0) / base.iter().sum::<f32>();
    let widths: Vec<f32> = base.iter().map(|w| w * f).collect();
    let mut xs = vec![x0];
    for w in &widths {
        xs.push(xs.last().unwrap() + w);
    }
    let (th, rh) = (9.0, 4.4);
    let n = (((y1 - y0 - th) / rh).floor().max(3.0)) as usize;
    let ybot = y0 + th + n as f32 * rh;
    p.rect_stroke(Rect::from_min_max(xf.s(P::new(x0, y0)), xf.s(P::new(x1, ybot))), 0.0, stroke, egui::StrokeKind::Middle);
    p.line_segment([xf.s(P::new(x0, y0 + th)), xf.s(P::new(x1, y0 + th))], stroke);
    text(P::new((x0 + x1) / 2.0, y0 + th / 2.0), name, 2.6, Align2::CENTER_CENTER);
    // Gruppenzeile: senkrechte Linien nur an den Gruppengrenzen
    let (von_col, nach_col, groups): (usize, usize, &[usize]) = if alt { (0, 5, &[3, 4, 5]) } else { (1, 6, &[1, 4, 5, 6]) };
    for (i, x) in xs.iter().enumerate().skip(1).take(widths.len() - 1) {
        let top = if groups.contains(&i) { y0 + th } else { y0 + th + rh };
        p.line_segment([xf.s(P::new(*x, top)), xf.s(P::new(*x, ybot))], stroke);
    }
    for r in 1..n {
        let y = y0 + th + r as f32 * rh;
        let (a, b) = if r == 1 && !alt { (xs[1], x1) } else { (x0, x1) };
        p.line_segment([xf.s(P::new(a, y)), xf.s(P::new(b, y))], stroke);
    }
    let gy = y0 + th + rh / 2.0;
    text(P::new(xs[von_col] + 0.8, gy), "Von Ziel", 2.2, Align2::LEFT_CENTER);
    text(P::new(xs[nach_col] + 0.8, gy), "Nach Ziel", 2.2, Align2::LEFT_CENTER);
    if mehr && !alt {
        text(P::new(xs[6] - 0.8, gy), "Brücken", 2.2, Align2::RIGHT_CENTER);
    }
    let abc: String = (0..plan.etagen.min(26)).map(|i| (b'A' + i) as char).collect();
    let head: Vec<&str> = if alt {
        vec!["Kennung", "Anschluß", "Pfad", "", "Klemme", "Kennung", "Anschluß", "Pfad"]
    } else {
        vec!["Funktion", "Kennung", "KL", "Pfad", "Nr", if mehr { abc.as_str() } else { "Brücken" }, "Kennung", "KL", "Pfad"]
    };
    let centered = |c: usize| if alt { c == 1 || c == 6 } else { c == 2 || c == 4 || c == 5 || c == 7 };
    let hy = y0 + th + 1.5 * rh;
    for (c, h) in head.iter().enumerate() {
        if centered(c) && !(mehr && c == 5) {
            text(P::new((xs[c] + xs[c + 1]) / 2.0, hy), h, 2.2, Align2::CENTER_CENTER);
        } else {
            text(P::new(xs[c] + 0.8, hy), h, 2.2, Align2::LEFT_CENTER);
        }
    }
    let row_y = |r: usize| y0 + th + (r as f32 + 2.5) * rh;
    let rows = n.saturating_sub(2);
    // Zellen einer Zeile: (Spalte, Text)
    let ziel_cells = |c0: usize, zz: &Option<Ziel>| -> Vec<(usize, String)> {
        match zz {
            Some(zz) => vec![(c0, zz.kennung.clone()), (c0 + 1, zz.kl.clone()), (c0 + 2, zz.pfad.clone())],
            None => vec![],
        }
    };
    // Folgeseite: vorangehende Klemmplan-Blätter derselben Leiste zeigen die ersten Zeilen
    let off = folge_index(anlage, sheet) * rows;
    for r in 0..rows {
        let y = row_y(r);
        let Some(zl) = plan.zeilen.get(off + r) else {
            if !alt {
                p.circle_stroke(xf.s(P::new(xs[5] + 2.0, y)), 0.8 * z, stroke);
            }
            continue;
        };
        let mut cells = vec![];
        if alt {
            cells.extend(ziel_cells(0, &zl.von));
            cells.push((4, zl.label.clone()));
            cells.extend(ziel_cells(5, &zl.nach));
        } else {
            cells.push((0, zl.funktion.clone()));
            cells.extend(ziel_cells(1, &zl.von));
            cells.extend(ziel_cells(6, &zl.nach));
            let first_of_nr = r == 0 || plan.zeilen[off + r - 1].nr != zl.nr;
            if first_of_nr {
                text(P::new(xs[5] - 0.8, y), &zl.nr.to_string(), 2.2, Align2::RIGHT_CENTER);
            }
            let dx = if mehr { zl.level.max(1) as f32 * 3.0 - 1.0 } else { 2.0 };
            p.circle_stroke(xf.s(P::new(xs[5] + dx, y)), 0.8 * z, stroke);
        }
        for (c, t) in cells {
            if centered(c) {
                text(P::new((xs[c] + xs[c + 1]) / 2.0, y), &t, 2.2, Align2::CENTER_CENTER);
            } else {
                text(P::new(xs[c] + 0.8, y), &t, 2.2, Align2::LEFT_CENTER);
            }
        }
    }
    // Brücken: Punkt je Klemme, senkrechte Linie von der ersten bis zur letzten (Spuren nebeneinander)
    // bei mehrstöckigen Leisten rechts neben den Etagen-Kreisen (Ausdruck Seite 39)
    if !alt {
        let x0 = if mehr { xs[5] + plan.etagen as f32 * 3.0 + 1.2 } else { xs[5] + 4.2 };
        for b in &plan.bruecken {
            let x = x0 + b.lane as f32 * 3.3;
            let vis: Vec<usize> = b.rows.iter().copied().filter(|r| *r >= off && *r < off + rows).map(|r| r - off).collect();
            let (Some(lo), Some(hi)) = (vis.first(), vis.last()) else { continue };
            p.line_segment([xf.s(P::new(x, row_y(*lo))), xf.s(P::new(x, row_y(*hi)))], stroke);
            for r in &vis {
                p.circle_filled(xf.s(P::new(x, row_y(*r))), (0.35 * z).max(1.5), Color32::BLACK);
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
    fn term(id: u64, x: f32, y: f32, leiste: &str, nr: u32) -> Obj {
        obj(id, Kind::Terminal { pos: P::new(x, y), leiste: leiste.into(), nr, level: 0, text_off: None, tpl: String::new(), opt: Default::default() })
    }
    fn wire(id: u64, pts: &[(f32, f32)]) -> Obj {
        obj(id, Kind::Wire { pts: pts.iter().map(|(x, y)| P::new(*x, *y)).collect(), leitung: 0, bez: vec![], opts: 0 })
    }

    /// Ziele über Verbindungsenden und Knicke; kreuzende Verbindungen sind nicht verbunden; Brücken.
    #[test]
    fn tracing_rules() {
        let mut s = Sheet::new(2);
        s.art = 3;
        s.objects = vec![
            term(1, 50.0, 50.0, "-X1", 1),
            term(2, 50.0, 70.0, "-X1", 2),
            term(3, 150.0, 50.0, "-X9", 7),
            term(4, 150.0, 90.0, "-X9", 8),
            // -X1:1 rechts → Knick → -X9:7 (Von)
            wire(10, &[(50.8, 50.0), (100.0, 50.0), (100.0, 50.0), (149.2, 50.0)]),
            // -X1:1 unten → -X1:2 oben (Brücke: "Verbindung ist Brücke")
            obj(11, Kind::Wire { pts: vec![P::new(50.0, 50.8), P::new(50.0, 69.2)], leitung: 0, bez: vec![], opts: WIRE_BRUECKE }),
            // -X1:2 rechts: kreuzt die senkrechte Verbindung 12 ohne Verbindung, endet an -X9:8 unten links
            wire(12, &[(120.0, 40.0), (120.0, 100.0)]),
            wire(13, &[(50.8, 70.0), (130.0, 70.0), (130.0, 90.0), (149.2, 90.0)]),
            // -X1:2 unten → endet auf dem Verlauf von 12: weiter zu dessen Anfang (nichts) – kein Ziel
            wire(14, &[(50.0, 70.8), (50.0, 80.0), (120.0, 80.0)]),
        ];
        let a = Anlage { name: "A".into(), sheets: vec![s], sammelschienen: vec![], klemmleisten: vec![], klemmen: vec![], schienen: Default::default(), set: Default::default(), raw: None };
        let plan = plan(&a, "-X1", &[]);
        assert_eq!(plan.zeilen.len(), 2);
        let z1 = &plan.zeilen[0];
        assert_eq!(z1.von.as_ref().map(|z| (z.kennung.as_str(), z.kl.as_str())), Some(("-X9", "7")));
        assert!(z1.nach.is_none());
        assert_eq!(plan.zeilen[1].von.as_ref().map(|z| z.kl.as_str()), Some("8"));
        assert_eq!(plan.bruecken, vec![Bruecke { lane: 0, rows: vec![0, 1] }]);
    }

    /// Ketten wie im Original-Ausdruck (Klemmplan -X2, zweistöckig): 1A–2A, 2B–4B–6B, 7A–…–12A in der ersten,
    /// 4A–7A, 7B–8B, 10B–12B in der zweiten Spur.
    #[test]
    fn bruecken_als_ketten_mehrstoeckig() {
        let mut plan = Plan::default();
        for nr in 1..=12 {
            for level in 1..=2 {
                plan.zeilen.push(Zeile { nr, level, pos: nr, ..Default::default() });
            }
        }
        let a = |n: u32| (n, 1u8);
        let b = |n: u32| (n, 2u8);
        let mut links = vec![(a(2), a(1)), (b(4), b(2)), (b(6), b(4)), (b(7), b(8)), (b(8), b(7)), (b(12), b(10))];
        links.extend((5..=12).map(|n| (a(n), a(n - 1))));
        let k: Vec<(usize, Vec<usize>)> = ketten(&plan, &links).into_iter().map(|b| (b.lane, b.rows)).collect();
        // Zeile = (Nr − 1) · 2 + Etage − 1
        assert_eq!(k, vec![(0, vec![0, 2]), (0, vec![3, 7, 11]), (1, vec![6, 8, 10, 12]), (0, vec![12, 14, 16, 18, 20, 22]), (1, vec![13, 15]), (1, vec![19, 23])]);
        // einstöckig: eine Kette wird verlängert, wo sie endet; überlappende Brücken liegen nebeneinander
        let mut p1 = Plan::default();
        for nr in 1..=6 {
            p1.zeilen.push(Zeile { nr, pos: nr, ..Default::default() });
        }
        let e = |n: u32| (n, 0u8);
        let k: Vec<(usize, Vec<usize>)> = ketten(&p1, &[(e(1), e(2)), (e(2), e(3)), (e(2), e(4)), (e(3), e(5))]).into_iter().map(|b| (b.lane, b.rows)).collect();
        assert_eq!(k, vec![(0, vec![0, 1, 2, 3, 4])]);
        // Seite 12: 3–5 und 4–5 eine Kette; 3–5 und 4–6 nebeneinander
        let k: Vec<(usize, Vec<usize>)> = ketten(&p1, &[(e(1), e(3)), (e(2), e(3)), (e(4), e(6)), (e(5), e(6))]).into_iter().map(|b| (b.lane, b.rows)).collect();
        assert_eq!(k, vec![(0, vec![0, 1, 2]), (0, vec![3, 4, 5])]);
        let k: Vec<(usize, Vec<usize>)> = ketten(&p1, &[(e(1), e(3)), (e(2), e(4))]).into_iter().map(|b| (b.lane, b.rows)).collect();
        assert_eq!(k, vec![(0, vec![0, 2]), (1, vec![1, 3])]);
    }
}
