//! Netz-Wächter (todo N00, 24.11.2026): Bearbeitungen dürfen elektrische Netze nicht ungewollt kurzschließen.
//!
//! Messung des Zufallsprüfers (`tests/netz_zufall.rs`): 150 von 733 zufälligen Bearbeitungen auf W2 verbanden fremde
//! Netze – meist, weil die Bearbeitung selbst etwas auf eine fremde Leitung setzte (Klemmstelle auf eine Leitung
//! geschoben, Teilstück auf die Flucht einer Schiene gefangen, gedrehte Klemme mit dem Anschluss auf der
//! Nachbarleitung, Querstück durch den Anschluss der Nachbarklemme) und das Nachführen bzw. Aufräumen daraus einen
//! T-Abzweig machte. Das Original kennt keine Netze und lässt das zu; der Nachbau weist solche Bearbeitungen ab wie
//! das Ablegen eines Symbols auf einem anderen (Nutzer 24.11.2026, S02: „Ablegen verhindern, zurückspringen“):
//!
//! - beim Ziehen (Objekt, ganze Leitung, Teilstück, Knick) bleibt der letzte zulässige Stand stehen,
//! - Drehen/Spiegeln und Pfeiltasten werden zurückgenommen,
//! - jeweils mit Meldung in der Statuszeile.
//!
//! **Gewollt** ist eine neue Verbindung, wenn ein freies Ende einer bearbeiteten Leitung (vorher an nichts
//! angeschlossen) auf eine Leitung, Schiene oder einen Anschluss gezogen wird – solche Enden zählen nicht.
//! Verglichen werden die Knoten (Symbolanschlüsse, Klemmstellen, Schienen, Verweise) außer denen der bearbeiteten
//! Objekte: Zwei Knoten, die vorher in verschiedenen Netzen lagen, dürfen nachher nicht im selben liegen. Ebenso
//! abgewiesen wird eine neue Überdeckung einer geänderten Leitung mit einer Leitung bzw. Schiene eines anderen Netzes
//! (zwei Potentiale übereinander gezeichnet).

use std::collections::{HashMap, HashSet};

use crate::canvas::obj_pins;
use crate::model::{Kind, Obj, Sheet, P};
use crate::symbols::SymDef;

/// Knoten der Netze: (Objekt, Anschlussnummer) für Symbolanschlüsse, (Objekt, u32::MAX) für Klemmstellen, Schienen,
/// Verweise; je Knoten die Nummer seines Netzes.
pub fn knoten(s: &Sheet, lib: &[SymDef], ohne_enden: &[(u64, usize)]) -> HashMap<(u64, u32), usize> {
    let mut out = HashMap::new();
    let netze = crate::netz::netze_ohne(s, lib, ohne_enden);
    let pins: Vec<(u64, Vec<P>)> = s.objects.iter().filter(|o| matches!(o.kind, Kind::Symbol { .. })).map(|o| (o.id, obj_pins(o, lib))).collect();
    for (ni, netz) in netze.iter().enumerate() {
        for o in s.objects.iter().filter(|o| netz.contains(&o.id) && !matches!(o.kind, Kind::Wire { .. })) {
            out.insert((o.id, u32::MAX), ni);
        }
        // Enden der Leitungen dieses Netzes (ohne die ausgenommenen)
        for o in s.objects.iter().filter(|o| netz.contains(&o.id)) {
            let Kind::Wire { pts, .. } = &o.kind else { continue };
            if pts.is_empty() {
                continue;
            }
            for (k, e) in [(0usize, pts[0]), (1, pts[pts.len() - 1])] {
                if ohne_enden.contains(&(o.id, k)) {
                    continue;
                }
                for (sym, v) in &pins {
                    if let Some(i) = v.iter().position(|q| q.dist(e) <= crate::mitziehen::PIN_TOL) {
                        out.insert((*sym, i as u32), ni);
                    }
                }
            }
        }
    }
    out
}

/// Netze, die nachher zusammenhängen, vorher aber getrennt waren (Zahl der betroffenen Netze nachher), ohne Knoten
/// der Objekte `ohne`. Mit vertauschten Argumenten: getrennte Netze. Knoten, die nur auf einer Seite vorkommen
/// (Anschluss vorher bzw. nachher frei), zählen nicht.
pub fn verbunden(vorher: &HashMap<(u64, u32), usize>, nachher: &HashMap<(u64, u32), usize>, ohne: &[u64]) -> usize {
    let mut n2v: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (k, a) in vorher {
        if ohne.contains(&k.0) {
            continue;
        }
        if let Some(b) = nachher.get(k) {
            n2v.entry(*b).or_default().insert(*a);
        }
    }
    n2v.values().filter(|s| s.len() > 1).count()
}

/// Enden der Leitungen `primaer`, die im Stand `vorher` an nichts angeschlossen waren (gewollte Verbindungen).
pub fn freie_enden(vorher: &[Obj], lib: &[SymDef], primaer: &[u64]) -> Vec<(u64, usize)> {
    let mut v = vec![];
    for o in vorher.iter().filter(|o| primaer.contains(&o.id)) {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        if pts.len() < 2 {
            continue;
        }
        for (k, e) in [(0usize, pts[0]), (1, pts[pts.len() - 1])] {
            if crate::leitung_anschluss::freies_ende_in(vorher, lib, o.id, e) {
                v.push((o.id, k));
            }
        }
    }
    v
}

/// Teilstücke eines Objekts (Leitung bzw. Schiene).
fn stuecke(k: &Kind) -> Vec<(P, P)> {
    match k {
        Kind::Wire { pts, .. } => pts.windows(2).map(|w| (w[0], w[1])).collect(),
        Kind::Busbar { a, x2, .. } => vec![(*a, P::new(*x2, a.y))],
        _ => vec![],
    }
}

/// Liegt ein bearbeitetes Symbol (`primaer`) nachher mit seinem Körper auf einem anderen Symbol, auf dem es vorher
/// nicht lag (Überlappung in beiden Richtungen mehr als 0,5 mm)? Nutzer 24.11.2026 (S02): Ablegen verhindern,
/// zurückspringen – MT1: eine Steckdose ließ sich halb auf die Nachbarsteckdose schieben.
pub fn symbol_auf_symbol(vorher: &Sheet, nachher: &Sheet, lib: &[SymDef], primaer: &[u64]) -> bool {
    const M: f32 = 0.5;
    let ueber = |a: (P, P), b: (P, P)| {
        let dx = a.1.x.max(a.0.x).min(b.1.x.max(b.0.x)) - a.0.x.min(a.1.x).max(b.0.x.min(b.1.x));
        let dy = a.1.y.max(a.0.y).min(b.1.y.max(b.0.y)) - a.0.y.min(a.1.y).max(b.0.y.min(b.1.y));
        dx > M && dy > M
    };
    let un = crate::leitung_hindernis::umrisse(nachher, lib);
    let uv = crate::leitung_hindernis::umrisse(vorher, lib);
    let alt = |a: u64, b: u64| {
        let (Some(x), Some(y)) = (uv.iter().find(|u| u.id == a), uv.iter().find(|u| u.id == b)) else { return false };
        ueber(x.r, y.r)
    };
    // nur vorhandene, verschobene Symbole: eingefügte Kopien liegen bewusst versetzt über dem Original
    un.iter().filter(|u| primaer.contains(&u.id) && uv.iter().any(|v| v.id == u.id)).any(|u| un.iter().any(|v| v.id != u.id && !primaer.contains(&v.id) && ueber(u.r, v.r) && !alt(u.id, v.id)))
}

/// Prüft den Stand `nachher` gegen `vorher` (bearbeitete Objekte `primaer`). `None` = zulässig, sonst der Grund.
pub fn pruefen(vorher: &[Obj], nachher: &Sheet, lib: &[SymDef], primaer: &[u64]) -> Option<&'static str> {
    let vs = Sheet { objects: vorher.to_vec(), ..Sheet::new(nachher.number) };
    let frei = freie_enden(vorher, lib, primaer);
    // die freien Enden zählen auch vorher nicht (sie hingen an nichts), so bleiben die Netze vergleichbar
    let kv = knoten(&vs, lib, &[]);
    let kn = knoten(nachher, lib, &frei);
    if verbunden(&kv, &kn, primaer) > 0 {
        return Some("würde fremde Netze verbinden");
    }
    if verbunden(&kn, &kv, primaer) > 0 {
        return Some("würde eine Verbindung trennen");
    }
    if symbol_auf_symbol(&vs, nachher, lib, primaer) {
        return Some("würde ein Symbol auf ein anderes legen");
    }
    // neue Überdeckung einer geänderten Leitung mit einem anderen Netz
    let geaendert: Vec<&Obj> = nachher
        .objects
        .iter()
        .filter(|o| matches!(o.kind, Kind::Wire { .. }) && vorher.iter().find(|x| x.id == o.id).is_none_or(|x| x.kind != o.kind))
        .collect();
    if geaendert.is_empty() {
        return None;
    }
    let netze = crate::netz::netze_ohne(nachher, lib, &frei);
    let netz_von = |id: u64| netze.iter().position(|n| n.contains(&id));
    for o in &geaendert {
        let Kind::Wire { pts, .. } = &o.kind else { continue };
        for x in nachher.objects.iter().filter(|x| x.id != o.id && matches!(x.kind, Kind::Wire { .. } | Kind::Busbar { .. })) {
            if netz_von(x.id) == netz_von(o.id) {
                continue;
            }
            // beide neu (eingefügte Kopie): ihre Lage zueinander stammt aus dem Original
            if !vorher.iter().any(|y| y.id == x.id) && !vorher.iter().any(|y| y.id == o.id) {
                continue;
            }
            if crate::leitung_vereinigen::ueberdeckung(pts, &stuecke(&x.kind)).is_empty() {
                continue;
            }
            // überdeckten sie sich schon vorher, ist es keine Folge dieser Bearbeitung
            let alt = |id: u64| vorher.iter().find(|y| y.id == id).map(|y| stuecke(&y.kind)).unwrap_or_default();
            let a0: Vec<P> = vorher.iter().find(|y| y.id == o.id).and_then(|y| if let Kind::Wire { pts, .. } = &y.kind { Some(pts.clone()) } else { None }).unwrap_or_default();
            if a0.len() >= 2 && !crate::leitung_vereinigen::ueberdeckung(&a0, &alt(x.id)).is_empty() {
                continue;
            }
            return Some("würde Leitungen verschiedener Netze übereinanderlegen");
        }
    }
    None
}

impl crate::app::App {
    /// Netz-Wächter nach einem Schritt einer Bearbeitung (vom gemerkten Stand `netz_vorher` aus, bearbeitete Objekte
    /// `primaer`): zulässig → Stand als letzten gültigen merken, `true`; sonst den letzten gültigen Stand (bzw. den
    /// Ausgangsstand) wiederherstellen, Meldung in der Statuszeile, `false`.
    pub fn netz_waechter(&mut self, primaer: &[u64]) -> bool {
        if !self.opt.netz_waechter || self.netz_vorher.is_empty() {
            return true;
        }
        let Some(s) = self.sheet() else { return true };
        match pruefen(&self.netz_vorher, s, &self.lib, primaer) {
            None => {
                self.netz_gueltig = Some(s.objects.clone());
                true
            }
            Some(grund) => {
                let zurueck = self.netz_gueltig.clone().unwrap_or_else(|| self.netz_vorher.clone());
                self.with_sheet(|s, _| s.objects = zurueck);
                self.info = format!("Nicht ausgeführt: {grund}");
                self.netz_abgewiesen += 1;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Obj;

    fn sym(id: u64, pos: P) -> Obj {
        Obj { id, kind: Kind::Symbol { key: "builtin:schliesser".into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn symbol_nicht_auf_symbol_ablegen() {
        // S02 (Nutzer 24.11.2026, MT1): ein verschobenes Symbol darf nicht neu auf einem anderen liegen
        let lib = crate::symbols::builtin();
        let mut vorher = Sheet::new(1);
        vorher.objects = vec![sym(1, P::new(50.0, 100.0)), sym(2, P::new(100.0, 100.0))];
        let mut nachher = vorher.clone();
        nachher.objects[1] = sym(2, P::new(50.5, 102.0));
        assert_eq!(pruefen(&vorher.objects, &nachher, &lib, &[2]), Some("würde ein Symbol auf ein anderes legen"));
        // daneben: zulässig
        nachher.objects[1] = sym(2, P::new(70.0, 100.0));
        assert_eq!(pruefen(&vorher.objects, &nachher, &lib, &[2]), None);
        // lagen sie schon vorher übereinander, ist das keine Folge dieser Bearbeitung
        let mut v2 = vorher.clone();
        v2.objects[1] = sym(2, P::new(50.5, 101.0));
        nachher.objects[1] = sym(2, P::new(50.5, 102.0));
        assert_eq!(pruefen(&v2.objects, &nachher, &lib, &[2]), None);
    }
}
