//! Übereinanderliegende Leitungen vereinigen (Nutzer 22.11.2026: „wenn zwei Leitungen übereinander liegen werden
//! diese vereinigt, evtl. wird ein Knoten erstellt“; Prüfliste feinheiten_leitungen.md B2, B6, D1–D3).
//!
//! Vorbild ist das Zusammenfassen überlappender Leitungen in KiCad Eeschema (`SchematicCleanup`, GPL-3.0; nur die
//! Idee, kein Code). Das Dateiformat kennt nur Leitungen als Polygonzüge mit eigener Kennung und Anschlussart je
//! Ende (knoten.rs); deshalb wird nicht zu einem Zug verschmolzen, sondern die doppelte Strecke entfernt:
//!
//! - **Überdeckung:** Zwei Leitungen überdecken sich, wo Teilstücke in gleicher Flucht (Abstand < 0,05 mm) mehr als
//!   0,5 mm gemeinsam haben. Sammelschienen zählen wie Leitungen, weichen aber nie.
//! - **Wer weicht:** die bearbeitete Leitung A – außer die gemeinsame Strecke ist ein Endstück der anderen Leitung B,
//!   aber keins von A (B läuft mit ihrem Ende auf A entlang): dann wird B gekürzt und A bleibt unverändert.
//! - **Endstück:** Läuft die weichende Leitung mit Anfang bzw. Ende auf der anderen entlang, beginnt/endet sie dort,
//!   wo sie die andere verlässt (T-Abzweig → Knoten aus der Geometrie, knoten.rs; Ende auf Ende → Fortsetzung).
//! - **Inneres Stück:** Liegt ein inneres Stück auf der anderen, wird die Leitung dort geteilt: der erste Teil
//!   behält Objekt und Originalblock, jeder weitere bekommt eine Kopie mit neuer Kennung (wie Klemmstelle setzen,
//!   klemme_leitung.rs); Leitung, Stift, Optionen bleiben, Bezeichnungen gehen zum nächstgelegenen Teil. Beide
//!   Teile enden auf der anderen Leitung, die Verbindung bleibt.
//! - **Ganz bedeckt:** Eine Leitung, die ganz auf anderen liegt, entfällt.
//! - **Anschlüsse bleiben:** Lag ein entfallendes Ende auf einem Symbolanschluss, durch den die andere Leitung nur
//!   hindurchläuft, wird die andere dort geteilt, damit der Anschluss angeschlossen bleibt.
//! - Leitungen, die sich nur Ende an Ende fortsetzen oder kreuzen, bleiben unverändert.
//!
//! Aufgerufen nach dem Zeichnen einer Leitung, nach dem Ziehen eines Teilstücks/Knicks und nach dem Verschieben
//! (nur die dabei veränderten Leitungen), im selben Rückgängig-Schritt. Beim bloßen Öffnen/Speichern läuft nichts.

use crate::model::{seg_dist, Kind, Obj, Sheet, P};

/// Abstand, bis zu dem Teilstücke als „in gleicher Flucht“ gelten (mm).
const LAGE: f32 = 0.05;
/// Kleinste gemeinsame Strecke, die als Überdeckung zählt (mm).
pub const MIN: f32 = 0.5;
const EPS: f32 = 0.01;

/// Teilstücke eines Objekts (Leitung bzw. Sammelschiene).
fn stuecke(k: &Kind) -> Vec<(P, P)> {
    match k {
        Kind::Wire { pts, .. } => pts.windows(2).map(|w| (w[0], w[1])).collect(),
        Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
        _ => vec![],
    }
}

/// Bogenlänge bis zu jedem Punkt.
fn laengen(pts: &[P]) -> Vec<f32> {
    let mut v = Vec::with_capacity(pts.len());
    let mut s = 0.0;
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            s += pts[i - 1].dist(*p);
        }
        v.push(s);
    }
    v
}

/// Strecken (Bogenlänge von–bis auf `pts`), auf denen die Leitung auf den Stücken `andere` liegt; aneinander
/// stoßende Strecken zusammengefasst, nur solche länger als `MIN`.
pub fn ueberdeckung(pts: &[P], andere: &[(P, P)]) -> Vec<(f32, f32)> {
    let cum = laengen(pts);
    let mut iv: Vec<(f32, f32)> = vec![];
    for (i, w) in pts.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let l = a.dist(b);
        if l < EPS {
            continue;
        }
        let u = P::new((b.x - a.x) / l, (b.y - a.y) / l);
        let quer = |q: P| ((q.x - a.x) * u.y - (q.y - a.y) * u.x).abs();
        let entlang = |q: P| (q.x - a.x) * u.x + (q.y - a.y) * u.y;
        for &(c, d) in andere {
            if quer(c) > LAGE || quer(d) > LAGE {
                continue;
            }
            let (t0, t1) = (entlang(c), entlang(d));
            let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(l));
            if hi - lo > EPS {
                iv.push((cum[i] + lo, cum[i] + hi));
            }
        }
    }
    iv.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f32, f32)> = vec![];
    for (lo, hi) in iv {
        match out.last_mut() {
            Some(l) if lo <= l.1 + LAGE => l.1 = l.1.max(hi),
            _ => out.push((lo, hi)),
        }
    }
    out.retain(|(lo, hi)| hi - lo > MIN);
    out
}

/// Punkt bei Bogenlänge `s`, auf einen nahen Fangpunkt gezogen (keine Rundungsreste an den Teilungsstellen).
fn punkt_bei(pts: &[P], cum: &[f32], s: f32, fang: &[P]) -> P {
    let i = (0..pts.len() - 1).find(|&i| s <= cum[i + 1] + 1e-4).unwrap_or(pts.len() - 2);
    let l = cum[i + 1] - cum[i];
    let t = if l < 1e-6 { 0.0 } else { ((s - cum[i]) / l).clamp(0.0, 1.0) };
    let p = P::new(pts[i].x + (pts[i + 1].x - pts[i].x) * t, pts[i].y + (pts[i + 1].y - pts[i].y) * t);
    fang.iter().chain(pts.iter()).copied().filter(|q| q.dist(p) < 2e-3).min_by(|a, b| a.dist(p).total_cmp(&b.dist(p))).unwrap_or(p)
}

/// Teil der Leitung zwischen den Bogenlängen `s0` und `s1`.
fn teil(pts: &[P], cum: &[f32], s0: f32, s1: f32, fang: &[P]) -> Vec<P> {
    let mut v = vec![punkt_bei(pts, cum, s0, fang)];
    v.extend(pts.iter().zip(cum).filter(|(_, c)| **c > s0 + EPS && **c < s1 - EPS).map(|(p, _)| *p));
    v.push(punkt_bei(pts, cum, s1, fang));
    crate::leitung_fuehrung::bereinigen(v)
}

/// Was von der Leitung `pts` ohne die Strecken `iv` übrig bleibt (leer = ganz bedeckt).
pub fn rest(pts: &[P], iv: &[(f32, f32)], fang: &[P]) -> Vec<Vec<P>> {
    if pts.len() < 2 {
        return vec![];
    }
    let cum = laengen(pts);
    let total = cum[cum.len() - 1];
    let mut out = vec![];
    let mut s = 0.0;
    for &(lo, hi) in iv.iter().chain(std::iter::once(&(total, total))) {
        if lo - s > EPS {
            let t = teil(pts, &cum, s, lo, fang);
            if t.len() >= 2 {
                out.push(t);
            }
        }
        s = s.max(hi);
    }
    out
}

fn beruehrt_ende(iv: &[(f32, f32)], pts: &[P]) -> bool {
    let total = laengen(pts).last().copied().unwrap_or(0.0);
    iv.iter().any(|(lo, hi)| *lo <= EPS || *hi >= total - EPS)
}

fn pts_von(sheet: &Sheet, id: u64) -> Option<Vec<P>> {
    sheet.objects.iter().find(|o| o.id == id).and_then(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts.clone()) } else { None })
}

/// Ergebnis von `vereinigen`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ergebnis {
    /// entfallene Leitungen
    pub entfernt: Vec<u64>,
    /// neue Teile: (Herkunft, neue Id)
    pub neu: Vec<(u64, u64)>,
    /// gekürzte bzw. geteilte Leitungen (Herkunft, behält ihre Id)
    pub geaendert: Vec<u64>,
}

impl Ergebnis {
    pub fn leer(&self) -> bool {
        self.entfernt.is_empty() && self.neu.is_empty() && self.geaendert.is_empty()
    }
}

/// Leitung `id` durch die Teile `teile` ersetzen: der erste behält Objekt und Originalblock, weitere bekommen eine
/// Kopie des Blocks mit neuer Kennung; keine Teile = Leitung entfällt.
fn ersetzen(sheet: &mut Sheet, id: u64, teile: Vec<Vec<P>>, next: &mut u64, erg: &mut Ergebnis) -> Vec<u64> {
    let Some(i) = sheet.objects.iter().position(|o| o.id == id) else { return vec![] };
    if teile.is_empty() {
        sheet.objects.remove(i);
        erg.entfernt.push(id);
        return vec![];
    }
    let o = sheet.objects[i].clone();
    let Kind::Wire { leitung, bez, opts, .. } = &o.kind else { return vec![] };
    // Bezeichnungen zum nächstgelegenen Teil
    let naehe = |p: P, z: &[P]| z.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f32::MAX, f32::min);
    let zu = |p: P| (0..teile.len()).min_by(|a, b| naehe(p, &teile[*a]).total_cmp(&naehe(p, &teile[*b]))).unwrap_or(0);
    let mut neue = vec![];
    for (k, t) in teile.iter().enumerate() {
        let b: Vec<(P, String)> = bez.iter().filter(|(p, _)| zu(*p) == k).cloned().collect();
        let kind = Kind::Wire { pts: t.clone(), leitung: *leitung, bez: b, opts: *opts };
        if k == 0 {
            sheet.objects[i].kind = kind;
            erg.geaendert.push(id);
            continue;
        }
        let mut raw = o.raw.clone();
        if let Some(r) = raw.as_mut() {
            // eigene Kennung (VBG Byte 40); `kind` der alten Leitung, damit der Schreiber Punkte und Enden nachträgt
            if r.block.data.len() >= 48 {
                r.block.data[40..48].copy_from_slice(&crate::kennungen::neu().to_le_bytes());
            }
        }
        let nid = *next;
        *next += 1;
        sheet.objects.push(Obj { id: nid, kind, pen: o.pen, fixed: o.fixed, group: o.group, raw });
        erg.neu.push((id, nid));
        neue.push(nid);
    }
    neue
}

/// Leitung `y` weicht der Leitung bzw. Schiene `c` auf den Strecken `iv`. Entfallende Enden auf Anschlüssen
/// (`anker`), durch die `c` nur hindurchläuft: `c` wird dort geteilt. Liefert die neuen Teile von `y`.
fn weichen(sheet: &mut Sheet, y: u64, c: u64, iv: &[(f32, f32)], anker: &[P], next: &mut u64, erg: &mut Ergebnis) -> Vec<u64> {
    let Some(pts) = pts_von(sheet, y) else { return vec![] };
    let fang: Vec<P> = sheet.objects.iter().find(|o| o.id == c).map(|o| stuecke(&o.kind).into_iter().flat_map(|(a, b)| [a, b]).collect()).unwrap_or_default();
    let total = laengen(&pts).last().copied().unwrap_or(0.0);
    let mut verloren = vec![];
    if iv.first().is_some_and(|i| i.0 <= EPS) {
        verloren.push(pts[0]);
    }
    if iv.last().is_some_and(|i| i.1 >= total - EPS) {
        verloren.push(pts[pts.len() - 1]);
    }
    let neue = ersetzen(sheet, y, rest(&pts, iv, &fang), next, erg);
    for e in verloren.into_iter().filter(|e| anker.iter().any(|q| q.dist(*e) < LAGE)) {
        let Some(cp) = pts_von(sheet, c) else { continue };
        if !crate::knoten::im_innern(&cp, e) {
            continue;
        }
        // Teilungsstelle auf `c` (Bogenlänge des Lotpunkts)
        let cum = laengen(&cp);
        let Some(k) = (0..cp.len() - 1).min_by(|a, b| seg_dist(e, cp[*a], cp[*a + 1]).total_cmp(&seg_dist(e, cp[*b], cp[*b + 1]))) else { continue };
        let s = cum[k] + cp[k].dist(e);
        let teile = vec![teil(&cp, &cum, 0.0, s, &[e]), teil(&cp, &cum, s, cum[cum.len() - 1], &[e])];
        ersetzen(sheet, c, teile, next, erg);
    }
    neue
}

/// Übereinanderliegende Leitungen vereinigen: die Leitungen `ids` (gerade bearbeitet) gegen alle anderen Leitungen
/// und Sammelschienen des Blattes. `anker` = Symbolanschlüsse, `next` = nächste freie Objekt-Id.
pub fn vereinigen(sheet: &mut Sheet, ids: &[u64], anker: &[P], next: &mut u64) -> Ergebnis {
    vereinigen_netze(sheet, ids, anker, next, None)
}

/// Wie `vereinigen`; mit `lib` nur Leitungen bzw. Schienen **desselben Netzes** (netz.rs): Überdecken sich Leitungen
/// verschiedener Netze, entstünde beim Vereinigen ein Abzweig und damit ein Kurzschluss (todo N00) – sie bleiben, der
/// Netz-Wächter weist solche Bearbeitungen ab (netz_waechter.rs).
pub fn vereinigen_netze(sheet: &mut Sheet, ids: &[u64], anker: &[P], next: &mut u64, lib: Option<&[crate::symbols::SymDef]>) -> Ergebnis {
    let mut erg = Ergebnis::default();
    let mut offen: Vec<u64> = ids.to_vec();
    let mut betroffen: Vec<u64> = ids.to_vec();
    let mut runden = 0;
    while let Some(a) = offen.pop() {
        'neu: loop {
            runden += 1;
            if runden > 10_000 {
                return erg;
            }
            let Some(ap) = pts_von(sheet, a) else { break };
            if ap.len() < 2 {
                break;
            }
            let astk: Vec<(P, P)> = ap.windows(2).map(|w| (w[0], w[1])).collect();
            let netze = lib.map(|l| crate::netz::netze(sheet, l));
            let gleiches_netz = |b: u64| netze.as_ref().is_none_or(|n| n.iter().any(|v| v.contains(&a) && v.contains(&b)));
            // erstes Objekt, das sich mit A überdeckt: wer weicht wem auf welchen Strecken
            let fund = sheet.objects.iter().filter(|o| o.id != a && gleiches_netz(o.id)).find_map(|o| {
                let iva = ueberdeckung(&ap, &stuecke(&o.kind));
                if iva.is_empty() {
                    return None;
                }
                // B weicht, wenn die gemeinsame Strecke ein Endstück von B ist, aber keins von A
                if let Kind::Wire { pts: bp, .. } = &o.kind {
                    let ivb = ueberdeckung(bp, &astk);
                    if !beruehrt_ende(&iva, &ap) && beruehrt_ende(&ivb, bp) && !rest(bp, &ivb, &[]).is_empty() {
                        return Some((o.id, a, ivb));
                    }
                }
                Some((a, o.id, iva))
            });
            let Some((y, c, iv)) = fund else { break };
            let neue = weichen(sheet, y, c, &iv, anker, next, &mut erg);
            if betroffen.contains(&y) {
                offen.extend(&neue);
                betroffen.extend(&neue);
            }
            continue 'neu;
        }
    }
    erg.geaendert.retain(|id| !erg.entfernt.contains(id));
    erg.geaendert.dedup();
    erg
}

/// Paare von Leitungen (bzw. Leitung und Schiene), die sich überdecken (Bericht, ändert nichts).
pub fn paare(sheet: &Sheet) -> Vec<(u64, u64, f32)> {
    let mut out = vec![];
    for (i, o) in sheet.objects.iter().enumerate() {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        for (j, x) in sheet.objects.iter().enumerate() {
            if j == i || (j < i && matches!(x.kind, Kind::Wire { .. })) {
                continue;
            }
            let iv = ueberdeckung(pts, &stuecke(&x.kind));
            if !iv.is_empty() {
                out.push((o.id, x.id, iv.iter().map(|(a, b)| b - a).sum()));
            }
        }
    }
    out
}

impl crate::app::App {
    /// Übereinanderliegende Leitungen nach Bearbeitung der Leitungen `ids` vereinigen (ohne eigenen
    /// Rückgängig-Schritt). Markierte Leitungen bleiben samt ihren neuen Teilen markiert.
    pub fn leitungen_vereinigen(&mut self, ids: &[u64]) -> Ergebnis {
        let Some(anker) = self.sheet().map(|s| s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).flat_map(|o| crate::canvas::obj_pins(o, &self.lib)).collect::<Vec<P>>()) else {
            return Ergebnis::default();
        };
        let lib = self.lib.clone();
        let nur_netz = self.opt.netz_waechter;
        let erg = self.with_sheet(|s, next| vereinigen_netze(s, ids, &anker, next, nur_netz.then_some(&lib[..]))).unwrap_or_default();
        if !erg.leer() {
            self.selection.retain(|id| !erg.entfernt.contains(id));
            for (von, neu) in &erg.neu {
                if self.selection.contains(von) {
                    self.selection.push(*neu);
                }
            }
            self.info = "Übereinanderliegende Leitungen vereinigt".into();
        }
        erg
    }

    /// Leitungen des aktuellen Blattes, die seit dem letzten Rückgängig-Schritt neu sind oder andere Punkte haben.
    pub fn geaenderte_leitungen(&self) -> Vec<u64> {
        if self.sym_edit.is_some() {
            return vec![];
        }
        let (Some(r), Some(s)) = (self.cur(), self.sheet()) else { return vec![] };
        let alt = self.undo.last().and_then(|p| p.sheet(r));
        s.objects
            .iter()
            .filter_map(|o| {
                let Kind::Wire { pts, .. } = &o.kind else { return None };
                let vorher = alt.and_then(|a| a.objects.iter().find(|x| x.id == o.id)).map(|x| matches!(&x.kind, Kind::Wire { pts: q, .. } if q == pts));
                (vorher != Some(true)).then_some(o.id)
            })
            .collect()
    }

    /// Beim Loslassen der linken Taste: nach Teilstück-/Knick-Ziehen bzw. Verschieben übereinanderliegende
    /// Leitungen vereinigen (gleicher Rückgängig-Schritt).
    pub fn nach_ziehen_vereinigen(&mut self) {
        match &self.drag {
            crate::app::Drag::Teilstueck { moved: true, .. } | crate::app::Drag::Move { moved: true, .. } | crate::app::Drag::Scale { .. } => {}
            _ => return,
        }
        self.aufraeumen();
    }

    /// Nach jeder Bearbeitung von Leitungen (gleicher Rückgängig-Schritt): übereinanderliegende vereinigen,
    /// Überstände kürzen, stoßende Leitungen zu einer zusammenfassen (leitung_zusammen.rs).
    pub fn aufraeumen(&mut self) {
        // Geste endet auf einem gemerkten Stand (Rückkehr, netz_folgen.rs): der ist schon aufgeräumt und bleibt
        if std::mem::take(&mut self.rueckkehr_gilt) {
            return;
        }
        let ids = self.geaenderte_leitungen();
        if ids.is_empty() {
            return;
        }
        self.aufraeum_schritt(|a| {
            a.leitungen_vereinigen(&ids);
        });
        self.aufraeum_schritt(|a| a.ueberstaende_kuerzen());
        self.aufraeum_schritt(|a| {
            let ids = a.geaenderte_leitungen();
            a.leitungen_glaetten(&ids);
        });
        self.aufraeum_schritt(|a| {
            let ids = a.geaenderte_leitungen();
            a.leitungen_zusammenfassen(&ids);
        });
        self.aufraeum_schritt(|a| a.rechtwinklig_halten());
    }

    /// Ein Schritt des Aufräumens; verbindet er fremde Netze (Netz-Wächter, z. B. Glätten einer Leitung durch den
    /// Anschluss der Nachbarklemme, W2 Blatt 10), wird er zurückgenommen.
    fn aufraeum_schritt(&mut self, f: impl FnOnce(&mut Self)) {
        if !self.opt.netz_waechter {
            f(self);
            return;
        }
        let Some(vorher) = self.sheet().map(|s| s.objects.clone()) else { return };
        let sel = self.selection.clone();
        f(self);
        let schlecht = self.sheet().is_some_and(|s| crate::netz_waechter::pruefen(&vorher, s, &self.lib, &[]).is_some());
        if schlecht {
            self.with_sheet(|s, _| s.objects = vorher);
            self.selection = sel;
        }
    }

    /// Absicherung nach dem Aufräumen (Nutzerbild Runde 13: schräge Leitung nach dem Verschieben einer Lampe):
    /// Geänderte Leitungen, die vor der Bearbeitung rechtwinklig waren und es jetzt nicht mehr sind, werden zwischen
    /// ihren Enden rechtwinklig neu geführt (Stiche aus den Anschlüssen, um Symbole herum).
    pub fn rechtwinklig_halten(&mut self) {
        let (Some(r), Some(s)) = (self.cur(), self.sheet()) else { return };
        let Some(alt) = self.undo.last().and_then(|p| p.sheet(r)) else { return };
        let stich = self.stich();
        let lib = &self.lib;
        let umr = crate::leitung_hindernis::umrisse(s, lib);
        let mut neu: Vec<(u64, Vec<P>)> = vec![];
        for o in &s.objects {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            if pts.len() < 2 || crate::netz_folgen::rechtwinklig(pts) {
                continue;
            }
            let war = match alt.objects.iter().find(|x| x.id == o.id).map(|x| &x.kind) {
                Some(Kind::Wire { pts: q, .. }) => crate::netz_folgen::rechtwinklig(q),
                // neue Leitungen: gezeichnete sind immer rechtwinklig gemeint
                _ => true,
            };
            if !war {
                continue;
            }
            let (a, b) = (pts[0], pts[pts.len() - 1]);
            let da = crate::leitung_route::anschluss_richtung(s, lib, a);
            let db = crate::leitung_route::anschluss_richtung(s, lib, b);
            let hind = crate::leitung_hindernis::wirksam(&crate::leitung_hindernis::hindernisse(&umr, &[], &[]), a, b);
            let weg = crate::leitung_route::mit_stichen(a, da, b, db, stich);
            neu.push((o.id, crate::leitung_hindernis::umgehen(&weg, da, db, &hind, 2.0 * stich, false)));
        }
        if neu.is_empty() {
            return;
        }
        self.with_sheet(|s, _| {
            for (id, p) in neu {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *pts = p;
                }
            }
        });
    }

    /// Leitungsenden, die vor dem Ziehen verbunden waren und jetzt frei sind (Maustest Runde 12: Teilstück auf eine
    /// andere Leitung gezogen, die angeschlossene Zuleitung hing danach mit einem Stück frei unter dem Abzweig):
    /// bis zum nächsten Punkt kürzen, an dem die Leitung verbunden ist. Ohne solchen Punkt bleibt sie.
    pub fn ueberstaende_kuerzen(&mut self) {
        let (Some(r), Some(s)) = (self.cur(), self.sheet()) else { return };
        if self.sym_edit.is_some() {
            return;
        }
        let Some(alt) = self.undo.last().and_then(|p| p.sheet(r)) else { return };
        let lib = &self.lib;
        let mut neu: Vec<(u64, Vec<P>)> = vec![];
        for o in &s.objects {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            let Some(Kind::Wire { pts: q, .. }) = alt.objects.iter().find(|x| x.id == o.id).map(|x| &x.kind) else { continue };
            if pts.len() < 2 || q.len() < 2 {
                continue;
            }
            let mut p = pts.clone();
            for anfang in [true, false] {
                let (e, e_alt) = if anfang { (p[0], q[0]) } else { (p[p.len() - 1], q[q.len() - 1]) };
                let war = !crate::leitung_anschluss::freies_ende(alt, lib, o.id, e_alt);
                if !war || !crate::leitung_anschluss::freies_ende(s, lib, o.id, e) {
                    continue;
                }
                let mut zug = p.clone();
                if !anfang {
                    zug.reverse();
                }
                if let Some(k) = gekuerzt_bis_anschluss(s, lib, o.id, &zug) {
                    p = if anfang { k } else { k.into_iter().rev().collect() };
                }
            }
            if p != *pts && p.len() >= 2 {
                neu.push((o.id, p));
            }
        }
        if neu.is_empty() {
            return;
        }
        self.with_sheet(|s, _| {
            for (id, p) in neu {
                if let Some(Kind::Wire { pts, .. }) = s.objects.iter_mut().find(|o| o.id == id).map(|o| &mut o.kind) {
                    *pts = p;
                }
            }
        });
    }
}

/// Zug `zug` (freies Ende vorn) bis zum ersten Punkt kürzen, an dem ein anderes Objekt anliegt (Ende einer anderen
/// Leitung, Anschluss, Klemmstelle, Schiene); `None`, wenn es keinen gibt oder er am anderen Ende liegt.
fn gekuerzt_bis_anschluss(s: &Sheet, lib: &[crate::symbols::SymDef], id: u64, zug: &[P]) -> Option<Vec<P>> {
    let tol = crate::mitziehen::PIN_TOL;
    let mut punkte: Vec<P> = vec![];
    for o in s.objects.iter().filter(|o| o.id != id) {
        match &o.kind {
            Kind::Wire { pts, .. } if pts.len() >= 2 => {
                punkte.push(pts[0]);
                punkte.push(pts[pts.len() - 1]);
            }
            Kind::Terminal { pos, .. } => punkte.push(*pos),
            _ => punkte.extend(crate::canvas::obj_pins(o, lib)),
        }
    }
    for i in 0..zug.len() - 1 {
        let (a, b) = (zug[i], zug[i + 1]);
        let treffer = punkte
            .iter()
            .filter(|q| crate::model::seg_dist(**q, a, b) <= tol && q.dist(zug[0]) > tol)
            .min_by(|x, y| x.dist(a).total_cmp(&y.dist(a)))
            .copied();
        if let Some(t) = treffer {
            if i + 2 == zug.len() && t.dist(b) <= tol {
                return None;
            }
            let mut out = vec![t];
            out.extend_from_slice(&zug[i + 1..]);
            return Some(crate::leitung_fuehrung::bereinigen(out));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    fn leitung(id: u64, pts: Vec<P>) -> Obj {
        Obj { id, kind: Kind::Wire { pts, leitung: 2, bez: vec![], opts: 5 }, pen: 3, fixed: false, group: None, raw: None }
    }

    fn blatt(ls: Vec<Vec<P>>) -> Sheet {
        let mut s = Sheet::new(1);
        for (i, pts) in ls.into_iter().enumerate() {
            s.objects.push(leitung(i as u64 + 1, pts));
        }
        s
    }

    fn pts(s: &Sheet, id: u64) -> Vec<P> {
        pts_von(s, id).unwrap_or_default()
    }

    fn lauf(s: &mut Sheet, ids: &[u64], anker: &[P]) -> Ergebnis {
        let mut next = 100;
        vereinigen(s, ids, anker, &mut next)
    }

    /// Keine zwei Leitungen überdecken sich mehr.
    fn sauber(s: &Sheet) {
        assert!(paare(s).is_empty(), "Überdeckung bleibt: {:?}", paare(s));
    }

    #[test]
    fn ueberdeckung_misst_gemeinsame_strecke() {
        let a = [p(0.0, 0.0), p(50.0, 0.0), p(50.0, 30.0)];
        assert_eq!(ueberdeckung(&a, &[(p(20.0, 0.0), p(80.0, 0.0))]), vec![(20.0, 50.0)]);
        // über den Knick hinweg zusammengefasst
        assert_eq!(ueberdeckung(&a, &[(p(40.0, 0.0), p(50.0, 0.0)), (p(50.0, 0.0), p(50.0, 10.0))]), vec![(40.0, 60.0)]);
        // Kreuzung, Fortsetzung Ende an Ende und bis 0,5 mm: keine Überdeckung
        assert!(ueberdeckung(&a, &[(p(25.0, -10.0), p(25.0, 10.0))]).is_empty());
        assert!(ueberdeckung(&a, &[(p(50.0, 30.0), p(50.0, 60.0))]).is_empty());
        assert!(ueberdeckung(&a, &[(p(49.6, 0.0), p(80.0, 0.0))]).is_empty());
        // parallel daneben
        assert!(ueberdeckung(&a, &[(p(0.0, 1.0), p(50.0, 1.0))]).is_empty());
    }

    #[test]
    fn endstueck_endet_wo_es_die_andere_verlaesst() {
        // B (2) kommt von oben und läuft mit ihrem Ende auf A (1) nach links: B endet am Abzweig, A bleibt
        let mut s = blatt(vec![vec![p(0.0, 0.0), p(100.0, 0.0)], vec![p(60.0, -30.0), p(60.0, 0.0), p(30.0, 0.0)]]);
        let e = lauf(&mut s, &[1], &[]);
        assert_eq!(pts(&s, 1), vec![p(0.0, 0.0), p(100.0, 0.0)]);
        assert_eq!(pts(&s, 2), vec![p(60.0, -30.0), p(60.0, 0.0)]);
        assert_eq!(e.geaendert, vec![2]);
        // Knoten am Abzweig (T)
        assert_eq!(crate::knoten::punkte(&s), vec![p(60.0, 0.0)]);
        sauber(&s);
        // dieselbe Lage, aber B wurde bearbeitet: B weicht ebenso
        let mut s = blatt(vec![vec![p(0.0, 0.0), p(100.0, 0.0)], vec![p(60.0, -30.0), p(60.0, 0.0), p(30.0, 0.0)]]);
        lauf(&mut s, &[2], &[]);
        assert_eq!(pts(&s, 2), vec![p(60.0, -30.0), p(60.0, 0.0)]);
        assert_eq!(pts(&s, 1), vec![p(0.0, 0.0), p(100.0, 0.0)]);
    }

    #[test]
    fn anfangsstueck_gemeinsam_vom_anschluss() {
        // beide beginnen am selben Anschluss und laufen 20 mm gemeinsam: die bearbeitete beginnt am Abzweig
        let mut s = blatt(vec![vec![p(0.0, 0.0), p(80.0, 0.0)], vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 40.0)]]);
        lauf(&mut s, &[2], &[p(0.0, 0.0)]);
        assert_eq!(pts(&s, 2), vec![p(20.0, 0.0), p(20.0, 40.0)]);
        assert_eq!(pts(&s, 1), vec![p(0.0, 0.0), p(80.0, 0.0)]);
        assert_eq!(crate::knoten::punkte(&s), vec![p(20.0, 0.0)]);
        sauber(&s);
    }

    #[test]
    fn inneres_stueck_wird_geteilt() {
        // Bild 2: Teilstück von A (1) auf B (2) gezogen – A läuft von oben auf B, 40 mm entlang, wieder nach unten
        let mut s = blatt(vec![vec![p(10.0, -30.0), p(10.0, 0.0), p(50.0, 0.0), p(50.0, 30.0)], vec![p(0.0, 0.0), p(100.0, 0.0)]]);
        if let Kind::Wire { bez, .. } = &mut s.objects[0].kind {
            bez.push((p(52.0, 20.0), "sw".into()));
        }
        let e = lauf(&mut s, &[1], &[]);
        assert_eq!(pts(&s, 1), vec![p(10.0, -30.0), p(10.0, 0.0)]);
        assert_eq!(e.neu, vec![(1, 100)]);
        assert_eq!(pts(&s, 100), vec![p(50.0, 0.0), p(50.0, 30.0)]);
        assert_eq!(pts(&s, 2), vec![p(0.0, 0.0), p(100.0, 0.0)]);
        // Eigenschaften übernommen, Bezeichnung beim näheren Teil
        let o = s.objects.iter().find(|o| o.id == 100).unwrap();
        assert!(matches!(&o.kind, Kind::Wire { leitung: 2, opts: 5, bez, .. } if bez.len() == 1));
        assert_eq!(o.pen, 3);
        let mut k = crate::knoten::punkte(&s);
        k.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert_eq!(k, vec![p(10.0, 0.0), p(50.0, 0.0)]);
        sauber(&s);
    }

    #[test]
    fn ganz_bedeckte_leitung_entfaellt() {
        let mut s = blatt(vec![vec![p(20.0, 0.0), p(40.0, 0.0)], vec![p(0.0, 0.0), p(100.0, 0.0)]]);
        let e = lauf(&mut s, &[1], &[]);
        assert_eq!(e.entfernt, vec![1]);
        assert_eq!(s.objects.len(), 1);
        // auf zwei Leitungen, die sich fortsetzen, verteilt: entfällt ebenso
        let mut s = blatt(vec![vec![p(20.0, 0.0), p(60.0, 0.0), p(60.0, 20.0)], vec![p(0.0, 0.0), p(60.0, 0.0)], vec![p(60.0, 0.0), p(60.0, 50.0)]]);
        lauf(&mut s, &[1], &[]);
        assert!(pts_von(&s, 1).is_none());
        assert_eq!(s.objects.len(), 2);
    }

    #[test]
    fn bearbeitete_ueber_kurzer_wird_geteilt() {
        // die bearbeitete A (1) liegt über der ganzen kürzeren B (2): A wird um B herum geteilt, B bleibt
        let mut s = blatt(vec![vec![p(0.0, 0.0), p(100.0, 0.0)], vec![p(30.0, 0.0), p(60.0, 0.0)]]);
        lauf(&mut s, &[1], &[]);
        assert_eq!(pts(&s, 2), vec![p(30.0, 0.0), p(60.0, 0.0)]);
        assert_eq!(pts(&s, 1), vec![p(0.0, 0.0), p(30.0, 0.0)]);
        assert_eq!(pts(&s, 100), vec![p(60.0, 0.0), p(100.0, 0.0)]);
        // Fortsetzungen: kein Knoten
        assert!(crate::knoten::punkte(&s).is_empty());
        sauber(&s);
    }

    #[test]
    fn anschluss_bleibt_angeschlossen() {
        // A (1) vom Anschluss bei x=40 nach links, ganz auf B (2), die durch den Anschluss hindurchläuft
        let mut s = blatt(vec![vec![p(40.0, 0.0), p(10.0, 0.0)], vec![p(0.0, 0.0), p(100.0, 0.0)]]);
        lauf(&mut s, &[1], &[p(40.0, 0.0)]);
        assert!(pts_von(&s, 1).is_none());
        assert_eq!(pts(&s, 2), vec![p(0.0, 0.0), p(40.0, 0.0)]);
        assert_eq!(pts(&s, 100), vec![p(40.0, 0.0), p(100.0, 0.0)]);
    }

    #[test]
    fn schiene_weicht_nie() {
        let mut s = blatt(vec![vec![p(80.0, 20.0), p(120.0, 20.0), p(120.0, 150.0)]]);
        s.objects.push(Obj { id: 9, kind: Kind::Busbar { a: p(10.0, 20.0), x2: 300.0, name: "L1".into(), label: p(0.0, -4.0), style: 0, sms: 0, xin: String::new(), xout: String::new(), lage: 0, versatz: 0.0 }, pen: 0, fixed: false, group: None, raw: None });
        lauf(&mut s, &[1], &[]);
        assert_eq!(pts(&s, 1), vec![p(120.0, 20.0), p(120.0, 150.0)]);
        sauber(&s);
    }

    #[test]
    fn fortsetzung_und_kreuzung_bleiben() {
        let orig = vec![vec![p(0.0, 0.0), p(50.0, 0.0)], vec![p(50.0, 0.0), p(100.0, 0.0)], vec![p(20.0, -10.0), p(20.0, 10.0)]];
        let mut s = blatt(orig.clone());
        let e = lauf(&mut s, &[1, 2, 3], &[]);
        assert!(e.leer());
        assert_eq!(s, blatt(orig));
    }

    #[test]
    fn zwei_gleiche_leitungen_eine_bleibt() {
        let mut s = blatt(vec![vec![p(0.0, 0.0), p(0.0, 20.0), p(50.0, 20.0)], vec![p(0.0, 0.0), p(0.0, 20.0), p(50.0, 20.0)]]);
        lauf(&mut s, &[1, 2], &[]);
        assert_eq!(s.objects.len(), 1);
        sauber(&s);
    }

    #[test]
    fn mehrfach_ueberdeckt() {
        // A (1) liegt mit zwei inneren Stücken auf B (2) und C (3): drei Teile, alle angeschlossen
        let mut s = blatt(vec![
            vec![p(0.0, -20.0), p(0.0, 0.0), p(30.0, 0.0), p(30.0, 20.0), p(60.0, 20.0), p(60.0, 40.0)],
            vec![p(-10.0, 0.0), p(40.0, 0.0)],
            vec![p(20.0, 20.0), p(80.0, 20.0)],
        ]);
        let e = lauf(&mut s, &[1], &[]);
        assert_eq!(e.neu.len(), 2);
        let mut alle: Vec<Vec<P>> = s.objects.iter().filter(|o| o.id == 1 || o.id >= 100).map(|o| pts(&s, o.id)).collect();
        alle.sort_by(|a, b| a[0].x.total_cmp(&b[0].x).then(a[0].y.total_cmp(&b[0].y)));
        assert_eq!(alle, vec![vec![p(0.0, -20.0), p(0.0, 0.0)], vec![p(30.0, 0.0), p(30.0, 20.0)], vec![p(60.0, 20.0), p(60.0, 40.0)]]);
        sauber(&s);
    }

    #[test]
    fn verschiedene_netze_werden_nicht_vereinigt() {
        // N00: zwei Leitungen ohne Verbindung (verschiedene Netze), die bearbeitete liegt ein Stück auf der anderen.
        // Vereinigen machte daraus einen Abzweig (Kurzschluss); mit Netzprüfung bleiben beide.
        let s0 = blatt(vec![vec![p(0.0, 0.0), p(0.0, 10.0), p(30.0, 10.0), p(30.0, 20.0)], vec![p(10.0, 30.0), p(10.0, 10.0), p(20.0, 10.0), p(20.0, -10.0)]]);
        let mut s = s0.clone();
        let mut next = 100;
        let erg = vereinigen_netze(&mut s, &[1], &[], &mut next, Some(&[]));
        assert!(erg.leer());
        assert_eq!(s.objects, s0.objects);
        // ohne Netzprüfung (bisher) wird vereinigt
        let mut s = s0.clone();
        assert!(!vereinigen(&mut s, &[1], &[], &mut next).leer());
    }
}
