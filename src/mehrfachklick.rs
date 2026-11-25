//! Mehrfachklick auf Leitungen in Stufen (todo NR12, Nutzer 24./25.11.2026; Option „Leitung: Mehrfachklick“):
//!
//! | Klicks | Markierung |
//! |---|---|
//! | 1× | nur das Teilstück bis zur nächsten Ecke (Entf löscht nur dieses Stück) |
//! | 2× | über Ecken hinweg bis zum nächsten Knoten (Abzweig, Anschluss, Klemmstelle, Schiene; `netz::kette`) |
//! | 3× | über Knoten hinweg das Netz, ohne über Sammelschienen weiterzugehen |
//! | 4× | zusätzlich die Sammelschiene(n) und alles, was daran hängt (ganzes Potential auf dem Blatt) |
//! | 5× | wieder nur das Teilstück (Kreislauf) |
//!
//! egui meldet nur Doppel- und Dreifachklick, daher ein eigener Zähler: der nächste Klick zählt weiter, wenn er
//! auf dieselbe Leitung fällt, höchstens `ZEIT` s nach dem vorigen und höchstens `WEG` px daneben.

use crate::app::App;
use crate::model::*;

/// Zeit zwischen zwei Klicks derselben Folge in s (egui-Doppelklick: 0,3 s; etwas großzügiger für Dreifach/Vierfach).
pub const ZEIT: f64 = 0.45;
/// Höchstens so weit (px) darf ein Folgeklick neben dem vorigen liegen.
pub const WEG: f32 = 3.0;

/// Stand des Klickzählers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Zaehler {
    pub n: u32,
    pub zeit: f64,
    pub pos: [f32; 2],
    pub id: u64,
}

impl Zaehler {
    /// Klick auf Leitung `id` bei `pos` (px) zur Zeit `t` zählen; Rückgabe: Stufe 1–4.
    pub fn zaehlen(&mut self, id: u64, pos: [f32; 2], t: f64) -> u8 {
        self.zaehlen_bis(id, pos, t, 4)
    }

    /// Wie `zaehlen`, aber höchstens bis Stufe `bis` (Optionen › Markieren „Mehrfachklick bis Stufe“), danach
    /// wieder 1.
    pub fn zaehlen_bis(&mut self, id: u64, pos: [f32; 2], t: f64, bis: u8) -> u8 {
        let nah = (pos[0] - self.pos[0]).hypot(pos[1] - self.pos[1]) <= WEG;
        self.n = if self.n > 0 && id == self.id && nah && t - self.zeit <= ZEIT { self.n + 1 } else { 1 };
        (self.zeit, self.pos, self.id) = (t, pos, id);
        stufe_bis(self.n, bis)
    }
}

/// Stufe zur Klickzahl: 1, 2, 3, 4, dann wieder 1 (5×), 2 (6×) …
pub fn stufe(n: u32) -> u8 {
    stufe_bis(n, 4)
}

/// Stufe zur Klickzahl mit höchster Stufe `bis` (1–4): z. B. bis 2 → 1, 2, 1, 2 …
pub fn stufe_bis(n: u32, bis: u8) -> u8 {
    let bis = bis.clamp(1, 4) as u32;
    ((n.max(1) - 1) % bis + 1) as u8
}

/// Teilstück der Leitung unter `m` (Index des Abschnitts `pts[i]`–`pts[i+1]`), nächstgelegenes bis `tol`.
pub fn teilstueck(pts: &[P], m: P, tol: f32) -> Option<usize> {
    (0..pts.len().saturating_sub(1)).map(|i| (i, seg_dist(m, pts[i], pts[i + 1]))).filter(|(_, d)| *d <= tol).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}

/// Leitungen des Netzes von `id`, ohne über Sammelschienen weiterzugehen (Stufe 3): das Netz wird auf einer Kopie
/// des Blattes ohne Schienen bestimmt.
pub fn netz_ohne_schienen(s: &Sheet, lib: &[crate::symbols::SymDef], id: u64) -> Vec<u64> {
    let mut ohne = s.clone();
    ohne.objects.retain(|o| !matches!(o.kind, Kind::Busbar { .. }));
    let netz = crate::netz::netz_von(&ohne, lib, id);
    crate::netz::leitungen(&ohne, &netz)
}

/// Ganzes Potential (Stufe 4): Leitungen und Sammelschienen des Netzes von `id` samt allem, was an den Schienen hängt.
pub fn potential(s: &Sheet, lib: &[crate::symbols::SymDef], id: u64) -> Vec<u64> {
    let netz = crate::netz::netz_von(s, lib, id);
    s.objects.iter().filter(|o| netz.contains(&o.id) && matches!(o.kind, Kind::Wire { .. } | Kind::Busbar { .. })).map(|o| o.id).collect()
}

/// Teilstück `i` aus der Leitung nehmen: übrig bleiben die Teile davor und danach (nur mit mindestens zwei Punkten).
pub fn teilstueck_entfernen(pts: &[P], i: usize) -> (Vec<P>, Vec<P>) {
    let vor: Vec<P> = pts[..=i].to_vec();
    let nach: Vec<P> = pts[i + 1..].to_vec();
    (if vor.len() >= 2 { vor } else { vec![] }, if nach.len() >= 2 { nach } else { vec![] })
}

impl App {
    /// Markiertes Teilstück (Stufe 1), falls die Markierung genau diese Leitung ist.
    pub fn teil_auswahl(&self) -> Option<(u64, usize)> {
        let (id, i) = self.zusatz.teil?;
        let ok = self.selection == [id] && self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.id == id && matches!(&o.kind, Kind::Wire { pts, .. } if i + 1 < pts.len())));
        ok.then_some((id, i))
    }

    /// Klick auf die Leitung `id` bei `m` (mm, Fangweite `tol`), Stufe aus dem Zähler.
    pub fn mehrfachklick(&mut self, id: u64, m: P, tol: f32, stufe: u8) {
        self.zusatz.teil = None;
        let Some(s) = self.sheet().cloned() else { return };
        let s = &s;
        let lib = self.lib.clone();
        match stufe {
            1 => {
                let i = s.objects.iter().find(|o| o.id == id).and_then(|o| if let Kind::Wire { pts, .. } = &o.kind { teilstueck(pts, m, tol) } else { None });
                self.selection = vec![id];
                self.zusatz.teil = i.map(|i| (id, i));
                self.info = "Teilstück (Doppelklick: bis zum Knoten, 3×: Netz, 4×: mit Sammelschiene)".into();
            }
            2 => {
                let ids = crate::netz::kette(s, &lib, id);
                self.info = format!("Bis zum Knoten: {} Leitung(en) (3×: Netz, 4×: mit Sammelschiene)", ids.len());
                self.selection = ids;
            }
            3 => {
                let ids = netz_ohne_schienen(s, &lib, id);
                self.info = format!("Netz ohne Sammelschiene: {} Leitung(en) (4×: mit Sammelschiene)", ids.len());
                self.selection = ids;
            }
            _ => {
                let ids = potential(s, &lib, id);
                let n = s.objects.iter().filter(|o| ids.contains(&o.id) && matches!(o.kind, Kind::Busbar { .. })).count();
                self.info = format!("Ganzes Potential: {} Leitung(en), {n} Sammelschiene(n) (nächster Klick: Teilstück)", ids.len() - n);
                self.selection = ids;
            }
        }
    }

    /// Entf mit markiertem Teilstück: nur dieses Stück löschen; die Teile davor und danach bleiben eigene Leitungen.
    /// Rückgabe: ob ein Teilstück gelöscht wurde.
    pub fn teilstueck_loeschen(&mut self) -> bool {
        let Some((id, i)) = self.teil_auswahl() else { return false };
        self.checkpoint();
        let neu_id = self.project.alloc_id();
        self.with_sheet(|s, _| {
            let Some(k) = s.objects.iter().position(|o| o.id == id) else { return };
            let Kind::Wire { pts, leitung, bez, opts } = s.objects[k].kind.clone() else { return };
            let (vor, nach) = teilstueck_entfernen(&pts, i);
            match (vor.is_empty(), nach.is_empty()) {
                (true, true) => {
                    s.objects.remove(k);
                }
                (false, true) | (true, false) => {
                    let rest = if vor.is_empty() { nach } else { vor };
                    if let Kind::Wire { pts, .. } = &mut s.objects[k].kind {
                        *pts = rest;
                    }
                }
                (false, false) => {
                    let o = &mut s.objects[k];
                    if let Kind::Wire { pts, .. } = &mut o.kind {
                        *pts = vor;
                    }
                    // zweiter Teil: Kopie des Originalblocks mit eigener Kennung wie beim Teilen an einer Klemme
                    let mut raw = o.raw.clone();
                    if let Some(r) = raw.as_mut() {
                        if r.block.data.len() >= 48 {
                            r.block.data[40..48].copy_from_slice(&crate::kennungen::neu().to_le_bytes());
                        }
                    }
                    let (pen, fixed, group) = (o.pen, o.fixed, o.group);
                    s.objects.push(Obj { id: neu_id, kind: Kind::Wire { pts: nach, leitung, bez: vec![], opts }, pen, fixed, group, raw });
                    let _ = bez;
                }
            }
        });
        self.selection.clear();
        self.zusatz.teil = None;
        self.info = "Teilstück gelöscht".into();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> P {
        P::new(x, y)
    }

    #[test]
    fn stufen_im_kreis() {
        assert_eq!((1..=9).map(stufe).collect::<Vec<_>>(), vec![1, 2, 3, 4, 1, 2, 3, 4, 1]);
        assert_eq!((1..=5).map(|n| stufe_bis(n, 2)).collect::<Vec<_>>(), vec![1, 2, 1, 2, 1]);
        assert_eq!((1..=3).map(|n| stufe_bis(n, 0)).collect::<Vec<_>>(), vec![1, 1, 1]);
        let mut z = Zaehler::default();
        assert_eq!(z.zaehlen(5, [10.0, 10.0], 1.0), 1);
        assert_eq!(z.zaehlen(5, [11.0, 10.0], 1.2), 2);
        assert_eq!(z.zaehlen(5, [11.0, 11.0], 1.4), 3);
        assert_eq!(z.zaehlen(5, [11.0, 11.0], 1.6), 4);
        assert_eq!(z.zaehlen(5, [11.0, 11.0], 1.8), 1);
        // zu spät, zu weit, andere Leitung: neu bei 1
        assert_eq!(z.zaehlen(5, [11.0, 11.0], 3.0), 1);
        assert_eq!(z.zaehlen(5, [30.0, 11.0], 3.1), 1);
        assert_eq!(z.zaehlen(6, [30.0, 11.0], 3.2), 1);
    }

    #[test]
    fn teilstueck_finden_und_entfernen() {
        let pts = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(20.0, 10.0)];
        assert_eq!(teilstueck(&pts, p(10.2, 5.0), 0.5), Some(1));
        assert_eq!(teilstueck(&pts, p(5.0, 3.0), 0.5), None);
        assert_eq!(teilstueck_entfernen(&pts, 1), (vec![p(0.0, 0.0), p(10.0, 0.0)], vec![p(10.0, 10.0), p(20.0, 10.0)]));
        assert_eq!(teilstueck_entfernen(&pts, 0), (vec![], vec![p(10.0, 0.0), p(10.0, 10.0), p(20.0, 10.0)]));
        assert_eq!(teilstueck_entfernen(&[p(0.0, 0.0), p(1.0, 0.0)], 0), (vec![], vec![]));
    }
}
