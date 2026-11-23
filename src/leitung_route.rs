//! Leitungsführung mit Anschlussrichtung (Maus-Befund M1, 22.11.2026).
//!
//! Der einfache Knick aus `leitung_fuehrung.rs` kennt nur die Lage der Punkte. Liegt das Ziel z. B. oberhalb eines
//! unteren Anschlusses, lief die Leitung senkrecht durch das eigene Symbol nach oben und traf dabei den oberen
//! Anschluss (Knoten, Kontakt kurzgeschlossen). Das Original (Gegenprobe unter Wine, Klickbetrieb) führt die
//! Leitung dagegen aus dem Anschluss in dessen Richtung nach außen, ein Raster weit, dann Z-förmig über die Mitte
//! zum Ziel und tritt dort ebenfalls von außen ein.
//!
//! Regel hier: Der bisherige Weg bleibt, solange er an beiden Enden nicht in das eigene Symbol hineinläuft
//! (senkrecht zur Anschlussrichtung ist erlaubt). Sonst wird der Weg mit Stichen in Anschlussrichtung neu gelegt.

use crate::canvas;
use crate::model::{Kind, Sheet, P};
use crate::symbols::SymDef;

/// Richtung nach außen (Einheitsvektor) des Symbolanschlusses bei `pin`: Anschluss am oberen/unteren bzw.
/// linken/rechten Rand des Umrisses (wie bei den automatischen Verbindungen). Klemmstellen, Leitungsenden und
/// Anschlüsse ohne eindeutige Lage haben keine Richtung.
pub fn anschluss_richtung(sheet: &Sheet, lib: &[SymDef], pin: P) -> Option<P> {
    const T: f32 = 0.2;
    for o in &sheet.objects {
        if !matches!(o.kind, Kind::Symbol { .. }) {
            continue;
        }
        let pins = canvas::obj_pins(o, lib);
        if !pins.iter().any(|q| q.dist(pin) <= 0.05) {
            continue;
        }
        let (mut mn, mut mx) = canvas::obj_bounds(o, lib);
        for p in &pins {
            mn = P::new(mn.x.min(p.x), mn.y.min(p.y));
            mx = P::new(mx.x.max(p.x), mx.y.max(p.y));
        }
        // Eckanschluss eines mehrpoligen Symbols (Maus-Befund M24, 3-polige Sicherung: Pol 5 oben rechts): ein
        // anderer Anschluss genau darunter/darüber (Polpaar) heißt oben/unten, einer auf gleicher Höhe links/rechts
        let senkrecht = pins.iter().any(|q| q.dist(pin) > 0.05 && (q.x - pin.x).abs() <= 0.05);
        let waagerecht = pins.iter().any(|q| q.dist(pin) > 0.05 && (q.y - pin.y).abs() <= 0.05);
        // liegt der Anschluss nicht am Umriss (W2-Leuchte: Anschlussbeschriftungen „1“, „2“, „PE“ ragen über die
        // Anschlüsse hinaus, Maustest Runde 11), gilt die Richtung von der Mitte zum Anschluss (stärkere Achse)
        return richtung_mit_paar(pin, mn, mx, T, senkrecht, waagerecht).or_else(|| {
            let c = P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
            let (dx, dy) = (pin.x - c.x, pin.y - c.y);
            if dx.abs().max(dy.abs()) < 0.5 {
                None
            } else if dy.abs() >= dx.abs() {
                Some(P::new(0.0, dy.signum()))
            } else {
                Some(P::new(dx.signum(), 0.0))
            }
        });
    }
    None
}

/// Wie `richtung_am_umriss`, aber an einer Ecke entscheidet das Polpaar: `senkrecht` = ein anderer Anschluss in
/// derselben Spalte, `waagerecht` = einer in derselben Zeile (nur eines von beiden zählt).
pub fn richtung_mit_paar(p: P, mn: P, mx: P, t: f32, senkrecht: bool, waagerecht: bool) -> Option<P> {
    let ecke_v = (p.y - mn.y).abs() <= t || (p.y - mx.y).abs() <= t;
    let ecke_h = (p.x - mn.x).abs() <= t || (p.x - mx.x).abs() <= t;
    if ecke_v && ecke_h && senkrecht != waagerecht {
        let (a, b) = if senkrecht {
            // nur die Höhe zählt: Umriss in x auf den Punkt verengen
            (P::new(p.x - 1.0, mn.y), P::new(p.x + 1.0, mx.y))
        } else {
            (P::new(mn.x, p.y - 1.0), P::new(mx.x, p.y + 1.0))
        };
        return richtung_am_umriss(p, a, b, t);
    }
    richtung_am_umriss(p, mn, mx, t)
}

/// Richtung nach außen für einen Punkt auf dem Umriss `mn`..`mx` (Toleranz `t`); bei einem Eckpunkt zählt die
/// längere Ausdehnung des Umrisses (hohes Symbol: oben/unten).
pub fn richtung_am_umriss(p: P, mn: P, mx: P, t: f32) -> Option<P> {
    let oben = (p.y - mn.y).abs() <= t;
    let unten = (p.y - mx.y).abs() <= t;
    let links = (p.x - mn.x).abs() <= t;
    let rechts = (p.x - mx.x).abs() <= t;
    let v = match (oben, unten) {
        (true, false) => Some(P::new(0.0, -1.0)),
        (false, true) => Some(P::new(0.0, 1.0)),
        _ => None,
    };
    let h = match (links, rechts) {
        (true, false) => Some(P::new(-1.0, 0.0)),
        (false, true) => Some(P::new(1.0, 0.0)),
        _ => None,
    };
    let hoch = mx.y - mn.y >= mx.x - mn.x;
    if hoch {
        v.or(h)
    } else {
        h.or(v)
    }
}

fn richtung(a: P, b: P) -> P {
    let d = b.sub(a);
    let l = (d.x * d.x + d.y * d.y).sqrt();
    if l < 1e-4 {
        P::new(0.0, 0.0)
    } else {
        P::new(d.x / l, d.y / l)
    }
}

fn skalar(a: P, b: P) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Läuft der Weg `pts` an einem Ende in das Symbol hinein? `da`/`db` = Richtung nach außen am Anfang/Ende.
pub fn laeuft_hinein(pts: &[P], da: Option<P>, db: Option<P>) -> bool {
    if pts.len() < 2 {
        return false;
    }
    let n = pts.len();
    let anfang = da.map(|d| skalar(richtung(pts[0], pts[1]), d) < -0.5).unwrap_or(false);
    // am Ende muss die Leitung von außen kommen, also entgegen der Richtung nach außen laufen
    let ende = db.map(|d| skalar(richtung(pts[n - 2], pts[n - 1]), d) > 0.5).unwrap_or(false);
    anfang || ende
}

/// Auf Vielfache von `s` ab `bezug` runden.
fn runden(x: f32, bezug: f32, s: f32) -> f32 {
    if s <= 0.0 {
        return x;
    }
    bezug + ((x - bezug) / s).round() * s
}

/// Weg von `a` nach `b` mit Stichen der Länge `s` in Anschlussrichtung (`da`, `db`) und Z-Führung über die Mitte.
pub fn mit_stichen(a: P, da: Option<P>, b: P, db: Option<P>, s: f32) -> Vec<P> {
    let a1 = da.map(|d| P::new(a.x + d.x * s, a.y + d.y * s)).unwrap_or(a);
    let b1 = db.map(|d| P::new(b.x + d.x * s, b.y + d.y * s)).unwrap_or(b);
    let senk = |d: Option<P>| d.map(|d| d.y.abs() > 0.5);
    // erstes Teilstück nach dem Stich quer zur Anschlussrichtung
    let a_senk = senk(da).or(senk(db)).unwrap_or(true);
    let b_senk = senk(db).unwrap_or(a_senk);
    let mut pts = vec![a, a1];
    match (a_senk, b_senk) {
        (true, true) => {
            // beide senkrecht: waagerecht zur Mitte, senkrecht, waagerecht zum Ziel; liegen beide (fast) übereinander,
            // seitlich um zwei Raster ausweichen statt durch das Symbol zu laufen
            let mut mx = runden((a.x + b.x) / 2.0, a.x, s);
            if (a.x - b.x).abs() < 2.0 * s.max(0.1) {
                mx = a.x.max(b.x) + 2.0 * s.max(1.0);
            }
            pts.extend([P::new(mx, a1.y), P::new(mx, b1.y)]);
        }
        (false, false) => {
            let mut my = runden((a.y + b.y) / 2.0, a.y, s);
            if (a.y - b.y).abs() < 2.0 * s.max(0.1) {
                my = a.y.max(b.y) + 2.0 * s.max(1.0);
            }
            pts.extend([P::new(a1.x, my), P::new(b1.x, my)]);
        }
        // senkrechter Anfang, waagerechtes Ende: erst waagerecht bis über/unter das Ziel, dann senkrecht
        (true, false) => pts.push(P::new(b1.x, a1.y)),
        (false, true) => pts.push(P::new(a1.x, b1.y)),
    }
    pts.extend([b1, b]);
    crate::leitung_fuehrung::bereinigen(pts)
}

/// Endgültiger Weg: `weg` (aus der Zugrichtung bzw. den Klicks) behalten, wenn er nicht in ein Symbol hineinläuft,
/// sonst mit Stichen neu legen. Im Klickbetrieb mit gesetzten Eckpunkten (`frei_ab` > 0) wird nur das letzte Stück
/// ab dem letzten Eckpunkt neu gelegt.
pub fn wegfuehrung(weg: Vec<P>, frei_ab: usize, da: Option<P>, db: Option<P>, s: f32) -> Vec<P> {
    if weg.len() < 2 {
        return weg;
    }
    let da = if frei_ab == 0 { da } else { None };
    if !laeuft_hinein(&weg[frei_ab.min(weg.len() - 1)..], da, db) {
        return weg;
    }
    let mut out: Vec<P> = weg[..frei_ab.min(weg.len() - 1)].to_vec();
    let a = weg[frei_ab.min(weg.len() - 1)];
    out.extend(mit_stichen(a, da, *weg.last().unwrap(), db, s));
    crate::leitung_fuehrung::bereinigen(out)
}

/// Sammelschiene unter `m` (Toleranz `tol`): Höhe und waagerechte Ausdehnung.
pub fn schiene_bei(sheet: &Sheet, m: P, tol: f32) -> Option<(f32, f32, f32)> {
    sheet.objects.iter().find_map(|o| match &o.kind {
        Kind::Busbar { a, x2, .. } => {
            let (x1, x2) = (a.x.min(*x2), a.x.max(*x2));
            ((m.y - a.y).abs() <= tol && m.x >= x1 - tol && m.x <= x2 + tol).then_some((a.y, x1, x2))
        }
        _ => None,
    })
}

/// Zeigt ein Anschluss bei `a` mit Richtung `da` zur Schiene in Höhe `y` (oder ist seine Richtung unbekannt)?
pub fn zeigt_zur_schiene(a: P, da: Option<P>, y: f32) -> bool {
    da.is_none_or(|d| d.y == 0.0 || (y - a.y) * d.y > 0.0)
}

/// Leitungsende auf einer Sammelschiene (Maus-Befund M3): der Leiter trifft die Schiene senkrecht in der Flucht
/// des letzten festen Punktes `l` (innerhalb der Schiene), statt auf den nächsten Rasterpunkt zu springen.
pub fn ende_auf_schiene(l: P, schiene: (f32, f32, f32)) -> P {
    let (y, x1, x2) = schiene;
    P::new(l.x.clamp(x1, x2), y)
}

/// Wie `ende_auf_schiene`, aber die Flucht von `l` nur, wenn die Maus `m` weniger als `halb` daneben liegt, sonst
/// der Rasterwert `q` der Maus (Runde 13: das Ende sprang sonst weit von der Maus weg).
pub fn ende_auf_schiene_fang(l: P, m: P, q: P, halb: f32, schiene: (f32, f32, f32)) -> P {
    let (y, x1, x2) = schiene;
    P::new(entlang(m.x, l.x, q.x, halb, x1 - 1e-3, x2 + 1e-3).clamp(x1, x2), y)
}

/// Richtung nach außen an einem Leitungsende `e`, das im Innern einer Leitung bzw. auf einer Schiene liegt: quer zum
/// getroffenen Stück, zur Seite von `von` (dort kommt die neue Leitung her). So tritt sie senkrecht auf und läuft
/// nicht ein Stück auf der anderen entlang.
pub fn richtung_auf_leitung(sheet: &Sheet, e: P, von: P) -> Option<P> {
    richtung_auf_leitung_ohne(sheet, e, von, u64::MAX)
}

/// Wie `richtung_auf_leitung`, ohne die Leitung `ohne` (die gerade geführte selbst).
pub fn richtung_auf_leitung_ohne(sheet: &Sheet, e: P, von: P, ohne: u64) -> Option<P> {
    const T: f32 = 0.05;
    for o in sheet.objects.iter().filter(|o| o.id != ohne) {
        let stuecke: Vec<(P, P)> = match &o.kind {
            Kind::Wire { pts, .. } => pts.windows(2).map(|w| (w[0], w[1])).collect(),
            Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
            _ => continue,
        };
        for (a, b) in stuecke {
            if crate::model::seg_dist(e, a, b) > T {
                continue;
            }
            if (a.y - b.y).abs() < 1e-3 {
                let s = if von.y < e.y { -1.0 } else { 1.0 };
                return Some(P::new(0.0, s));
            }
            if (a.x - b.x).abs() < 1e-3 {
                let s = if von.x < e.x { -1.0 } else { 1.0 };
                return Some(P::new(s, 0.0));
            }
        }
    }
    None
}

/// Leitungsende auf dem waagerechten bzw. senkrechten Teilstück einer Leitung unter `m`: in der Flucht des letzten
/// festen Punktes `l`, sonst der Lotfußpunkt der Maus.
pub fn ende_auf_leitung(sheet: &Sheet, l: P, m: P, tol: f32) -> Option<P> {
    ende_auf_leitung_fang(sheet, l, m, m, tol, f32::INFINITY)
}

/// Lage entlang einer Achse: die Flucht `bezug`, wenn die Maus `m` weniger als `halb` daneben liegt und sie im
/// Bereich `lo..hi` liegt, sonst der Rasterwert `q`; immer auf `lo..hi` begrenzt.
fn entlang(m: f32, bezug: f32, q: f32, halb: f32, lo: f32, hi: f32) -> f32 {
    let v = if (m - bezug).abs() < halb && bezug > lo && bezug < hi { bezug } else { q };
    v.clamp(lo, hi)
}

/// Leitungsende auf einer Leitung unter der Maus `m` (Runde 13, Nutzer: „wenn eine Leitung auf einer Leitung
/// gesetzt wird, soll diese nicht irgendwo platziert werden können, sondern am Raster orientiert“): der Punkt liegt
/// auf dem getroffenen Teilstück, entlang des Stücks auf dem Rasterwert `q` der Maus – oder in der Flucht des
/// Bezugspunkts `l`, wenn die Maus weniger als `halb` daneben liegt. (Vorher: immer in der Flucht von `l`, auch
/// wenn die Maus weit weg war – das Ende sprang bis zu 60 mm.)
pub fn ende_auf_leitung_fang(sheet: &Sheet, l: P, m: P, q: P, tol: f32, halb: f32) -> Option<P> {
    for o in &sheet.objects {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if (a.y - b.y).abs() < 1e-3 {
                let (x1, x2) = (a.x.min(b.x), a.x.max(b.x));
                if (m.y - a.y).abs() <= tol && m.x >= x1 - tol && m.x <= x2 + tol {
                    return Some(P::new(entlang(m.x, l.x, q.x, halb, x1, x2), a.y));
                }
            } else if (a.x - b.x).abs() < 1e-3 {
                let (y1, y2) = (a.y.min(b.y), a.y.max(b.y));
                if (m.x - a.x).abs() <= tol && m.y >= y1 - tol && m.y <= y2 + tol {
                    return Some(P::new(a.x, entlang(m.y, l.y, q.y, halb, y1, y2)));
                }
            }
        }
    }
    None
}

impl crate::app::App {
    /// Ende der gerade gezeichneten Leitung bei der Maus `m`: Anschluss in Reichweite, sonst senkrecht auf eine
    /// Sammelschiene unter der Maus, sonst der Rasterpunkt.
    pub fn leitungsende(&self, f: &crate::leitung_fuehrung::Fuehrung, m: P, z: f32) -> P {
        let tol = 6.0 / z;
        if let Some(p) = self.pin_near(m, tol, &[]) {
            return p;
        }
        if let Some((sheet, s)) = self.sheet().and_then(|sh| schiene_bei(sh, m, tol).map(|s| (sh, s))) {
            // Maus-Befund M16: zeigt der Anfangsanschluss (noch ohne Eckpunkte) von der Schiene weg, führte die
            // Flucht des Anschlusses um das Symbol herum und auf der Schiene zurück – dann senkrecht an der Maus
            let da = anschluss_richtung(sheet, &self.lib, f.start());
            let bezug = if f.pts.len() == 1 && !zeigt_zur_schiene(f.start(), da, s.0) { self.snap(m) } else { f.bahn.last().copied().unwrap_or_else(|| f.letzter()) };
            return ende_auf_schiene_fang(bezug, m, self.snap(m), self.raster() / 2.0, s);
        }
        // auf einer Leitung: auf dem Raster bzw. in der Flucht des Bezugspunkts (Knoten)
        let bezug = f.bahn.last().copied().unwrap_or_else(|| f.letzter());
        if let Some(p) = self.sheet().and_then(|s| ende_auf_leitung_fang(s, bezug, m, self.snap(m), tol, self.raster() / 2.0)) {
            return p;
        }
        // Flucht des Bezugspunkts (Anfang bzw. letzter Knick) vor dem Raster: liegt die Maus weniger als ein halbes
        // Raster neben ihr, bleibt das Ende in der Flucht (Maustest Runde 12: Lampenanschluss bei x = 202,3, Raster
        // 10 mm – das erste gefangene Ende sprang auf x = 205, die Leitung lief erst nach rechts statt nach unten)
        // Runde 13: je Achse auch auf die Fluchten von Anschlüssen und Leitungsenden (fang.rs)
        let bezug = f.bahn.last().copied().unwrap_or_else(|| f.letzter());
        self.fang_mit_fluchten(m, &[bezug, f.start()])
    }

    /// Weg der gerade gezeichneten Leitung `f` bis `ende` mit Anschlussrichtungen (Vorschau und Ergebnis).
    pub fn leitungsweg(&self, f: &crate::leitung_fuehrung::Fuehrung, ende: P) -> Vec<P> {
        let w = self.leitungsweg_roh(f, ende);
        let Some(s) = self.sheet() else { return w };
        // läge ein Teilstück auf einer vorhandenen Leitung oder Schiene (Maustest Runde 11: W2 erweitert, die neue
        // Leitung lief senkrecht auf der Leitung von -E7.6 zum Abgriff), die andere Winkelführung nehmen, wenn sie
        // frei ist und kein Symbol durchquert; im Klickbetrieb bleiben die gesetzten Ecken
        // Nachzeichnen 24.11.2026: ebenso, wenn der Weg über einen fremden Anschluss läuft; die Ersatzwege dürfen
        // das auch nicht, und neben den beiden Winkeln gibt es Z-Wege auf anderen Höhen (nächstgelegene zuerst) –
        // vorher lief eine Leitung in den Nachbarpol eines LS waagerecht über dessen ersten Pol
        let am_anschluss = anschluss_richtung(s, &self.lib, ende).is_some() || liegt_auf_leitung(s, ende);
        let w = if !f.klickbetrieb && f.bahn.is_empty() && (ueberlappt(s, &w) || am_anschluss && ueber_fremdem_anschluss(s, &self.lib, &w)) {
            let hind: Vec<_> = crate::leitung_hindernis::umrisse(s, &self.lib).into_iter().map(|u| u.r).collect();
            let hind = crate::leitung_hindernis::wirksam(&hind, f.start(), ende);
            let (a, e) = (f.start(), ende);
            let (da, db) = (anschluss_richtung(s, &self.lib, a), anschluss_richtung(s, &self.lib, e));
            // Ende auf einer Leitung/Schiene: senkrecht darauf zulaufen (wie im Rohweg)
            let db = db.or_else(|| richtung_auf_leitung(s, e, f.bahn.last().copied().unwrap_or(a)));
            let mut kand = vec![vec![a, P::new(a.x, e.y), e], vec![a, P::new(e.x, a.y), e]];
            kand.extend(z_wege(a, da, e, db, self.stich_frei(s, a, e), self.raster(), &w));
            // längere Stiche: endet der Stich auf einer fremden Leitung (Nachzeichnen 24.11.2026: PE-Pin nach oben,
            // Stich genau auf der L-Leitung über der Anschlussreihe – PE und L wurden verbunden)
            kand.extend((2..=4).map(|k| self.leitungsweg_roh_mit(f, ende, self.stich() * k as f32)));
            kand.into_iter()
                .map(crate::leitung_fuehrung::bereinigen)
                .find(|c| {
                    !ueberlappt(s, c)
                        && crate::leitung_hindernis::kreuzungen(c, &hind) == 0
                        && !laeuft_hinein(c, da, db)
                        && !ueber_fremdem_anschluss(s, &self.lib, c)
                })
                .unwrap_or(w)
        } else {
            w
        };
        let w = crate::klemme_leitung::enden_an_klemmen(s, w);
        // gegen Umspringen: Form des letzten Bildes halten, solange sie nicht deutlich schlechter ist
        let w = self.formhalt(f, ende, w);
        crate::klemme_leitung::enden_an_klemmen(s, w)
    }

    fn leitungsweg_roh(&self, f: &crate::leitung_fuehrung::Fuehrung, ende: P) -> Vec<P> {
        // Stich so lang, dass sein Ende nicht auf (oder dicht neben) einer fremden Leitung liegt (Nachzeichnen
        // 24.11.2026: PE-Pin nach oben, die L-Leitung lief genau einen Stich über der Anschlussreihe – der Weg
        // begann auf ihr und wurde mit ihr verbunden)
        let stich = self.sheet().map(|s| self.stich_frei(s, f.start(), ende)).unwrap_or_else(|| self.stich());
        self.leitungsweg_roh_mit(f, ende, stich)
    }

    /// Kleinste Stichlänge ab `stich()` (in Rasterschritten, höchstens 20), bei der die Stichenden am Anfang `a` und
    /// am Ende `e` mindestens 1,5 mm von jeder Leitung und Schiene entfernt sind, die nicht durch diesen Punkt bzw.
    /// den Anschluss selbst läuft.
    pub fn stich_frei(&self, s: &Sheet, a: P, e: P) -> f32 {
        let basis = self.stich();
        let schritt = self.raster().max(0.5);
        let (da, db) = (anschluss_richtung(s, &self.lib, a), anschluss_richtung(s, &self.lib, e));
        let mut strecken: Vec<(P, P)> = vec![];
        for o in &s.objects {
            match &o.kind {
                Kind::Wire { pts, .. } => strecken.extend(pts.windows(2).map(|w| (w[0], w[1]))),
                Kind::Busbar { a, x2, .. } => strecken.push((*a, P::new(*x2, a.y))),
                _ => {}
            }
        }
        let abstand = |q: P, u: P, v: P| {
            let (x, y) = (q.x.clamp(u.x.min(v.x), u.x.max(v.x)), q.y.clamp(u.y.min(v.y), u.y.max(v.y)));
            q.dist(P::new(x, y))
        };
        let frei = |p: P, d: Option<P>, l: f32| {
            let Some(d) = d else { return true };
            let q = P::new(p.x + d.x * l, p.y + d.y * l);
            strecken.iter().filter(|(u, v)| abstand(p, *u, *v) > 0.05).all(|(u, v)| abstand(q, *u, *v) >= 1.5)
        };
        let mut l = basis;
        for _ in 0..20 {
            if frei(a, da, l) && frei(e, db, l) {
                return l;
            }
            l += schritt;
        }
        basis
    }

    /// Wie `leitungsweg_roh` mit der Stichlänge `stich`.
    fn leitungsweg_roh_mit(&self, f: &crate::leitung_fuehrung::Fuehrung, ende: P, stich: f32) -> Vec<P> {
        let weg = f.weg(ende);
        let Some(sheet) = self.sheet() else { return weg };
        let da = anschluss_richtung(sheet, &self.lib, f.start());
        // Ende auf einer Leitung bzw. Schiene: senkrecht darauf zulaufen
        let von = f.bahn.last().copied().unwrap_or_else(|| f.letzter());
        let db = anschluss_richtung(sheet, &self.lib, ende).or_else(|| richtung_auf_leitung(sheet, ende, von));
        // Symbole als Hindernisse (M29, leitung_hindernis.rs), auch Start- und Zielsymbol; die Zugrichtung bleibt
        let hind: Vec<_> = crate::leitung_hindernis::umrisse(sheet, &self.lib).into_iter().map(|u| u.r).collect();
        let umgehen = |w: &[P], da: Option<P>| crate::leitung_hindernis::umgehen(w, da, db, &hind, self.raster(), true);
        // Ziehen vom Anschluss: stetige Führung mit Hysterese (leitung_zug.rs)
        if !f.klickbetrieb && f.schiene.is_none() {
            return umgehen(&f.zugweg(ende, da, db, stich), da);
        }
        // im Klickbetrieb bleiben die gesetzten Eckpunkte (Anfang + Ecken = pts.len() - 1 Teilstücke davor)
        let frei_ab = if f.klickbetrieb { f.pts.len() - 1 } else { 0 };
        // Stich ein halbes Raster (Original unter Wine: deutlich kürzer als ein Raster; genaue Länge offen)
        let w = wegfuehrung(weg, frei_ab, da, db, stich);
        if frei_ab == 0 {
            return umgehen(&w, da);
        }
        // nur das Stück ab dem letzten gesetzten Eckpunkt
        let Some(j) = w.iter().rposition(|q| q.dist(f.letzter()) < 0.01) else { return w };
        let mut out = w[..j].to_vec();
        out.extend(umgehen(&w[j..], None));
        crate::leitung_fuehrung::bereinigen(out)
    }
}

/// Liegt ein Teilstück von `pts` auf einer vorhandenen Leitung oder Sammelschiene (gemeinsame Strecke über 0,5 mm in
/// derselben Flucht)? Berühren an einem Punkt und Kreuzen zählen nicht.
pub fn ueberlappt(sheet: &Sheet, pts: &[P]) -> bool {
    const T: f32 = 0.05;
    let mut strecken: Vec<(P, P)> = vec![];
    for o in &sheet.objects {
        match &o.kind {
            Kind::Wire { pts: w, .. } => strecken.extend(w.windows(2).map(|v| (v[0], v[1]))),
            Kind::Busbar { a, x2, .. } => strecken.push((*a, P::new(*x2, a.y))),
            _ => {}
        }
    }
    pts.windows(2).any(|w| {
        let (a, b) = (w[0], w[1]);
        strecken.iter().any(|(c, d)| {
            if (a.y - b.y).abs() < T && (c.y - d.y).abs() < T && (a.y - c.y).abs() < T {
                a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x)) > 0.5
            } else if (a.x - b.x).abs() < T && (c.x - d.x).abs() < T && (a.x - c.x).abs() < T {
                a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y)) > 0.5
            } else {
                false
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }
    const UNTEN: Option<P> = Some(P { x: 0.0, y: 1.0 });
    const OBEN: Option<P> = Some(P { x: 0.0, y: -1.0 });

    #[test]
    fn unterer_anschluss_zu_hoeherem_ziel_wie_original() {
        // Q1:14 bei (40,100) (Richtung unten), K1:A1 bei (100,90) (Richtung oben): der alte Weg lief senkrecht nach
        // oben durch das Kontaktsymbol
        let alt = vec![p(40.0, 100.0), p(40.0, 90.0), p(100.0, 90.0)];
        assert!(laeuft_hinein(&alt, UNTEN, OBEN));
        let neu = wegfuehrung(alt, 0, UNTEN, OBEN, 5.0);
        assert_eq!(neu, vec![p(40.0, 100.0), p(40.0, 105.0), p(70.0, 105.0), p(70.0, 85.0), p(100.0, 85.0), p(100.0, 90.0)]);
        assert!(!laeuft_hinein(&neu, UNTEN, OBEN));
    }

    #[test]
    fn gerader_weg_bleibt() {
        // beide unteren Anschlüsse auf gleicher Höhe: waagerecht ist erlaubt (quer zur Anschlussrichtung)
        let w = vec![p(40.0, 100.0), p(100.0, 100.0)];
        assert_eq!(wegfuehrung(w.clone(), 0, UNTEN, UNTEN, 5.0), w);
        // unterer Anschluss zu tiefer liegendem oberen Anschluss: senkrecht hinunter ist richtig
        let w = vec![p(40.0, 100.0), p(40.0, 140.0)];
        assert_eq!(wegfuehrung(w.clone(), 0, UNTEN, OBEN, 5.0), w);
    }

    #[test]
    fn uebereinander_weicht_seitlich_aus() {
        // Ziel genau über dem unteren Anschluss desselben Symbols: nicht durch das Symbol
        let neu = mit_stichen(p(40.0, 100.0), UNTEN, p(40.0, 80.0), OBEN, 5.0);
        assert!(neu.iter().any(|q| q.x > 40.0));
        assert!(!laeuft_hinein(&neu, UNTEN, OBEN));
    }

    #[test]
    fn klickbetrieb_nur_letztes_stueck() {
        // Eckpunkt (60,120) vom Nutzer gesetzt, das letzte Stück liefe von unten in einen unteren Anschluss
        let w = vec![p(40.0, 100.0), p(40.0, 120.0), p(60.0, 120.0), p(60.0, 100.0)];
        let neu = wegfuehrung(w, 2, UNTEN, OBEN, 5.0);
        // die Ecke des Nutzers bleibt, das Stück zum Ziel weicht seitlich aus und kommt von oben
        assert_eq!(&neu[..2], &[p(40.0, 100.0), p(40.0, 120.0)]);
        assert_eq!(neu.last(), Some(&p(60.0, 100.0)));
        assert!(!laeuft_hinein(&neu, UNTEN, OBEN));
    }

    #[test]
    fn eckpol_mehrpolig_nach_paar() {
        // 3-polige Sicherung, Umriss 14 × 10 breit: Pol 5 oben rechts mit Pol 6 darunter → oben (M24)
        let (mn, mx) = (p(-7.0, 0.0), p(7.0, 10.0));
        assert_eq!(richtung_mit_paar(p(7.0, 0.0), mn, mx, 0.2, true, false), OBEN);
        assert_eq!(richtung_mit_paar(p(7.0, 10.0), mn, mx, 0.2, true, false), UNTEN);
        // ohne Paar wie bisher (breit → seitlich)
        assert_eq!(richtung_mit_paar(p(7.0, 0.0), mn, mx, 0.2, false, false), Some(p(1.0, 0.0)));
    }

    #[test]
    fn anschluss_zeigt_zur_schiene() {
        // Schiene oben (y = 20): oberer Anschluss bei y = 50 zeigt hin, unterer zeigt weg (M16)
        assert!(zeigt_zur_schiene(p(40.0, 50.0), OBEN, 20.0));
        assert!(!zeigt_zur_schiene(p(40.0, 120.0), UNTEN, 20.0));
        // Schiene unten: unterer Anschluss zeigt hin
        assert!(zeigt_zur_schiene(p(40.0, 120.0), UNTEN, 200.0));
        // seitlicher oder unbekannter Anschluss: Flucht wie bisher
        assert!(zeigt_zur_schiene(p(40.0, 120.0), Some(p(1.0, 0.0)), 20.0));
        assert!(zeigt_zur_schiene(p(40.0, 120.0), None, 20.0));
    }

    #[test]
    fn leiter_trifft_schiene_senkrecht() {
        // F1:1 bei x = 41,3 (nicht auf dem Raster), Schiene y = 20 von 10 bis 200
        assert_eq!(ende_auf_schiene(p(41.3, 50.0), (20.0, 10.0, 200.0)), p(41.3, 20.0));
        // außerhalb der Schiene: an deren Ende
        assert_eq!(ende_auf_schiene(p(5.0, 50.0), (20.0, 10.0, 200.0)), p(10.0, 20.0));
    }

    #[test]
    fn leiter_trifft_leitung_in_der_flucht() {
        let mut s = Sheet::new(1);
        s.objects.push(crate::model::Obj { id: 1, kind: Kind::Wire { pts: vec![p(10.0, 20.0), p(200.0, 20.0)], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
        assert_eq!(ende_auf_leitung(&s, p(58.8, 45.0), p(55.0, 20.2), 1.0), Some(p(58.8, 20.0)));
        assert_eq!(ende_auf_leitung(&s, p(58.8, 45.0), p(55.0, 25.0), 1.0), None);
    }

    #[test]
    fn richtung_am_umriss_eckpunkt() {
        // hohes Symbol: Anschluss an der unteren rechten Ecke zählt als unten
        assert_eq!(richtung_am_umriss(p(10.0, 20.0), p(0.0, 0.0), p(10.0, 20.0), 0.2), UNTEN);
        assert_eq!(richtung_am_umriss(p(5.0, 10.0), p(0.0, 0.0), p(10.0, 20.0), 0.2), None);
    }
}

/// Liegt ein Anschluss eines Symbols (außer Anfang und Ende des Weges) auf dem Weg `w`?
pub fn ueber_fremdem_anschluss(sheet: &Sheet, lib: &[SymDef], w: &[P]) -> bool {
    let (Some(&a), Some(&e)) = (w.first(), w.last()) else { return false };
    let auf = |q: P, u: P, v: P| {
        let t = 0.05;
        q.x >= u.x.min(v.x) - t && q.x <= u.x.max(v.x) + t && q.y >= u.y.min(v.y) - t && q.y <= u.y.max(v.y) + t
    };
    sheet
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Symbol { .. }))
        .flat_map(|o| canvas::obj_pins(o, lib))
        .filter(|q| q.dist(a) > 0.05 && q.dist(e) > 0.05)
        .any(|q| w.windows(2).any(|s| auf(q, s[0], s[1])))
}

/// Z-Wege von `a` nach `e` mit Stichen in Anschlussrichtung und dem Querstück auf verschiedenen Höhen (bei
/// senkrechten Anschlüssen) bzw. Fluchten (bei waagerechten) zwischen den Stichen, im Abstand max(Raster, Stich);
/// die dem längsten Querstück von `bisher` nächstgelegenen zuerst.
fn z_wege(a: P, da: Option<P>, e: P, db: Option<P>, stich: f32, raster: f32, bisher: &[P]) -> Vec<Vec<P>> {
    let senk = |d: Option<P>| d.map(|d| d.y.abs() > 0.5).unwrap_or(true);
    let waag = |d: Option<P>| d.map(|d| d.x.abs() > 0.5).unwrap_or(true);
    let st = |p: P, d: Option<P>| d.map(|d| P::new(p.x + d.x * stich, p.y + d.y * stich)).unwrap_or(p);
    let (sa, se) = (st(a, da), st(e, db));
    let schritt = raster.max(stich).max(0.5);
    let mut out = vec![];
    for waagerechtes_querstueck in [true, false] {
        if waagerechtes_querstueck && !(senk(da) && senk(db)) || !waagerechtes_querstueck && !(waag(da) && waag(db)) {
            continue;
        }
        let quer = |s: &[P]| if waagerechtes_querstueck { (s[0].y - s[1].y).abs() < 0.01 } else { (s[0].x - s[1].x).abs() < 0.01 };
        let (u0, u1) = if waagerechtes_querstueck { (sa.y, se.y) } else { (sa.x, se.x) };
        let bezug = bisher
            .windows(2)
            .filter(|s| quer(s))
            .max_by(|s, t| s[0].dist(s[1]).total_cmp(&t[0].dist(t[1])))
            .map(|s| if waagerechtes_querstueck { s[0].y } else { s[0].x })
            .unwrap_or((u0 + u1) / 2.0);
        let (lo, hi) = (u0.min(u1), u0.max(u1));
        let mut stufen = vec![];
        let mut u = hi;
        while u >= lo - 0.01 && stufen.len() < 200 {
            stufen.push(u);
            u -= schritt;
        }
        stufen.sort_by(|x, y| (x - bezug).abs().total_cmp(&(y - bezug).abs()));
        for u in stufen {
            out.push(if waagerechtes_querstueck {
                vec![a, sa, P::new(sa.x, u), P::new(se.x, u), se, e]
            } else {
                vec![a, sa, P::new(u, sa.y), P::new(u, se.y), se, e]
            });
        }
    }
    // U-Umwege seitlich um das eigene bzw. ein fremdes Symbol (beide Anschlüsse senkrecht bzw. beide waagerecht):
    // vom Stich quer bis zur Flucht u, dort entlang bis zur Höhe des Zielstichs, dann zum Ziel (Nachzeichnen
    // 24.11.2026: PE-Pin oben, Schiene unten – rechts versperrte die Nachbarsteckdose, links war frei)
    let mut seitlich = vec![];
    for k in 1..=30 {
        for vz in [-1.0f32, 1.0] {
            let d = vz * k as f32 * schritt;
            if senk(da) && senk(db) {
                let u = sa.x + d;
                seitlich.push(vec![a, sa, P::new(u, sa.y), P::new(u, se.y), se, e]);
            } else if waag(da) && waag(db) {
                let u = sa.y + d;
                seitlich.push(vec![a, sa, P::new(sa.x, u), P::new(se.x, u), se, e]);
            }
        }
    }
    out.extend(seitlich);
    out
}

/// Liegt `q` auf einer Leitung oder Sammelschiene (Ende einer gezogenen Leitung, die dort anschließt)?
pub fn liegt_auf_leitung(sheet: &Sheet, q: P) -> bool {
    let auf = |u: P, v: P| {
        let t = 0.05;
        q.x >= u.x.min(v.x) - t && q.x <= u.x.max(v.x) + t && q.y >= u.y.min(v.y) - t && q.y <= u.y.max(v.y) + t
    };
    sheet.objects.iter().any(|o| match &o.kind {
        Kind::Wire { pts, .. } => pts.windows(2).any(|w| auf(w[0], w[1])),
        Kind::Busbar { a, x2, .. } => auf(*a, P::new(*x2, a.y)),
        _ => false,
    })
}
