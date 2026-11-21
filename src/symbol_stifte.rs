//! Stifte einer Symboldefinition (Original: TSymEigenschaften › Reiter „Stifte / Parameter“, Ablage SYM › GRD › SIT).
//!
//! Jedes Element einer Symboldefinition nennt in Feld 0x40 seinen Stift: 0–3 sind die fest eingebauten Stifte
//! (Standard, Gestrichelt, Strich/Punkt, Gepunktet), ab 4 die eigenen Stifte des Symbols. Diese stehen als
//! SIT-Blöcke (40 Byte, Aufbau wie die Projektstifte PRJ › SIT) im GRD des Symbols: Name 0–20, Feld 5 Breite in
//! 0,1 mm, Feld 6 Farbe (TColor), Feld 7 Byte 0 Stiftart, Feld 8 Stiftnummer. Ermittelt an W2.epr (Symbol "Buss":
//! drei Stifte 4–6 "Mittel", "Bus", "Buss", Elemente mit Feld 0x40 = 4, 5, 6).

use crate::eprfile::{put_str, Block};
use crate::model::Stift;
use crate::symbols::SymDef;
use eframe::egui::{Color32, Stroke};
use std::cell::RefCell;

/// Erste Nummer eigener Stifte
pub const ERSTER: u32 = 4;

/// Stifte aus den SIT-Blöcken eines GRD (Nummer, Stift).
pub fn aus_grd(grd: &Block) -> Vec<(u32, Stift)> {
    grd.kids_of("SIT").filter(|b| b.data.len() >= 36).map(|b| (b.i32(8).clamp(0, 255) as u32, crate::epr_projekt::stift_aus(b))).collect()
}

/// Stifte eines Bibliothekssymbols aus seinen Zeichendaten ("VerX", wie `symbols::parse_lib_extras`): nach
/// Elementen, Texten und Bildnamen folgen die SIT-Einträge als verkettete Liste (40 Byte Stift, dann u32 ≠ 0, wenn
/// ein weiterer folgt). Ermittelt an der Original-Datenbank (Symbol "Buss": "Mittel", "Bus", "Buss" = 4, 5, 6).
pub fn aus_bibliothek(raw: &[u8]) -> Vec<(u32, Stift)> {
    let u = |p: usize| raw.get(p..p + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let mut v = vec![];
    if raw.len() < 0x3c || &raw[0..3] != b"Ver" {
        return v;
    }
    let mut p = 0x3c + u(0x14).unwrap_or(0) as usize * 0x50;
    for _ in 0..2 {
        let Some(l) = u(p) else { return v };
        p += 4 + l as usize;
    }
    while let Some(d) = raw.get(p..p + 40) {
        let b = Block::new("SIT", d.to_vec());
        let nr = b.i32(8);
        let name_ok = d[0] != 0 && d[..20].contains(&0);
        if !(ERSTER as i32..=255).contains(&nr) || !(0..=200).contains(&b.i32(5)) || !name_ok {
            break;
        }
        v.push((nr as u32, crate::epr_projekt::stift_aus(&b)));
        if u(p + 40).unwrap_or(0) == 0 || v.len() >= 64 {
            break;
        }
        p += 44;
    }
    v
}

/// SIT-Blöcke für die Stifte eines Symbols; vorhandene Blöcke gleicher Nummer (`orig`) behalten ihre übrigen Bytes.
pub fn sit_bloecke(pens: &[(u32, Stift)], orig: &[&Block]) -> Vec<Block> {
    pens.iter()
        .map(|(nr, s)| {
            let mut b = orig.iter().find(|b| b.i32(8) == *nr as i32).map(|b| (*b).clone()).unwrap_or_else(|| Block::new("SIT", vec![0; 40]));
            b.kids.clear();
            if b.data.len() < 40 {
                b.data.resize(40, 0);
            }
            let alt = crate::epr_projekt::stift_aus(&b);
            if alt.name != s.name {
                put_str(&mut b.data, 0, 20, &s.name);
            }
            if (alt.width * 10.0).round() != (s.width * 10.0).round() {
                b.set_i32(5, (s.width * 10.0).round() as i32);
            }
            if alt.color != s.color {
                b.set_i32(6, crate::epr_projekt::to_tcolor(s.color));
            }
            if alt.style != s.style {
                b.set_i32(7, (b.i32(7) & !0xff) | s.style.min(3) as i32);
            }
            b.set_i32(8, *nr as i32);
            b
        })
        .collect()
}

/// Strich und Stiftart für ein Element mit Stift `st`: eingebaute Stifte (0–3) zeichnen mit dem Strich der
/// Instanz und der Stiftart `st`; eigene Stifte mit ihrer Breite, Art und Farbe (die Farbe nur, solange die Instanz
/// schwarz gezeichnet wird – Markierung und farbige Stifte der Instanz haben Vorrang).
pub fn strich(def: &SymDef, st: u8, stroke: Stroke, z: f32) -> (Stroke, u8) {
    if (st as u32) < ERSTER {
        return (stroke, st);
    }
    let Some((_, pen)) = def.pens.iter().find(|(n, _)| *n == st as u32) else { return (stroke, 0) };
    let mut s = stroke;
    s.width = (pen.width.max(0.05) * z).max(1.0);
    if stroke.color == Color32::BLACK {
        s.color = Color32::from_rgb(pen.color[0], pen.color[1], pen.color[2]);
    }
    (s, pen.style)
}

/// Stifttabelle für den Symboleditor: Index = Stiftnummer; 0–3 die eingebauten Stifte des Projekts, ab 4 die
/// Stifte des Symbols (Lücken als Standardstift).
pub fn tabelle(projekt: &[Stift], pens: &[(u32, Stift)]) -> Vec<Stift> {
    let std = Stift { name: "Standard".into(), style: 0, color: [0, 0, 0], width: 0.25 };
    let mut v: Vec<Stift> = (0..ERSTER as usize).map(|i| projekt.get(i).cloned().unwrap_or_else(|| std.clone())).collect();
    for (nr, s) in pens {
        let i = *nr as usize;
        if i < ERSTER as usize {
            continue;
        }
        while v.len() <= i {
            v.push(Stift { name: format!("Stift {}", v.len()), ..std.clone() });
        }
        v[i] = s.clone();
    }
    v
}

/// Nächste freie Stiftnummer des Symbols.
pub fn neue_nummer(pens: &[(u32, Stift)]) -> u32 {
    pens.iter().map(|(n, _)| *n + 1).max().unwrap_or(ERSTER).max(ERSTER)
}

/// Stift `nr` löschen: Elemente mit diesem Stift zeichnen danach mit dem Standardstift (Rückgabe: geänderte
/// Element-Ids), höhere Nummern rücken nach.
pub fn loeschen(pens: &mut Vec<(u32, Stift)>, nr: u32, elemente: &mut [(u64, &mut usize)]) -> Vec<u64> {
    pens.retain(|(n, _)| *n != nr);
    for (n, _) in pens.iter_mut() {
        if *n > nr {
            *n -= 1;
        }
    }
    let mut geaendert = vec![];
    for (id, pen) in elemente.iter_mut() {
        if **pen == nr as usize {
            **pen = 0;
            geaendert.push(*id);
        } else if **pen > nr as usize {
            **pen -= 1;
            geaendert.push(*id);
        }
    }
    geaendert
}

impl crate::app::App {
    /// Stifttabelle des geöffneten Symboleditors (leer außerhalb des Editors).
    pub fn edit_stifte(&self) -> Vec<Stift> {
        self.sym_edit.as_ref().map(|e| tabelle(&self.project.stifte, &e.pens)).unwrap_or_default()
    }
}

/// Reiter „Stifte / Parameter“ › Liste „Stifte“ (TIconTab Stifte, Knopf „Stift Löschen“): je Stift Nummer, Name,
/// Breite, Stiftart und Farbe. Gelöschte Nummern landen in `geloescht` (die Liste rückt sofort nach).
pub fn liste_ui(ui: &mut eframe::egui::Ui, pens: &mut Vec<(u32, Stift)>, sel: &mut usize, geloescht: &mut Vec<u32>) {
    use eframe::egui;
    const ARTEN: [&str; 4] = ["Durchgängig", "Gestrichelt", "Strich Punkt", "Gepunktet"];
    ui.vertical(|ui| {
        ui.set_width(345.0);
        egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(160))).show(ui, |ui| {
            ui.set_min_size(egui::Vec2::new(341.0, 270.0));
            egui::ScrollArea::vertical().id_salt("symstifte").max_height(270.0).show(ui, |ui| {
                if pens.is_empty() {
                    ui.weak("Keine eigenen Stifte – „Neuer Stift“ in der Symbolleiste legt einen an.");
                }
                for (i, (nr, s)) in pens.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(*sel == i, format!("{nr}")).clicked() {
                            *sel = i;
                        }
                        let r = ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(110.0));
                        if r.gained_focus() {
                            *sel = i;
                        }
                        ui.add(egui::DragValue::new(&mut s.width).range(0.05..=5.0).speed(0.05).max_decimals(2).suffix(" mm"));
                        egui::ComboBox::from_id_salt(("symstiftart", i)).width(86.0).selected_text(ARTEN[(s.style as usize).min(3)]).show_ui(ui, |ui| {
                            for (k, a) in ARTEN.iter().enumerate() {
                                ui.selectable_value(&mut s.style, k as u8, *a);
                            }
                        });
                        let mut c = Color32::from_rgb(s.color[0], s.color[1], s.color[2]);
                        if ui.color_edit_button_srgba(&mut c).changed() {
                            s.color = [c.r(), c.g(), c.b()];
                        }
                    });
                }
            });
        });
        let can = *sel < pens.len();
        if ui.add_enabled(can, egui::Button::new("Stift Löschen").min_size(egui::Vec2::new(99.0, 25.0))).clicked() && can {
            let nr = pens[*sel].0;
            geloescht.push(nr);
            let mut keine: [(u64, &mut usize); 0] = [];
            loeschen(pens, nr, &mut keine);
            *sel = sel.saturating_sub(1);
        }
    });
}

thread_local! {
    /// Stifttabelle, die `canvas::pen_stroke` statt der Projektstifte benutzt (Symboleditor)
    static ERSATZ: RefCell<Option<Vec<Stift>>> = const { RefCell::new(None) };
}

/// Solange der Wächter lebt, zeichnet `canvas::pen_stroke` mit `tabelle` statt mit den Projektstiften.
pub struct Ersatz;

impl Ersatz {
    pub fn setzen(tabelle: Vec<Stift>) -> Ersatz {
        ERSATZ.with(|e| *e.borrow_mut() = Some(tabelle));
        Ersatz
    }
}

impl Drop for Ersatz {
    fn drop(&mut self) {
        ERSATZ.with(|e| *e.borrow_mut() = None);
    }
}

/// Stift `pen` aus der Ersatztabelle, falls eine gesetzt ist.
pub fn ersatz(pen: usize) -> Option<Option<Stift>> {
    ERSATZ.with(|e| e.borrow().as_ref().map(|t| t.get(pen).cloned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stift(n: &str, w: f32, c: [u8; 3], s: u8) -> Stift {
        Stift { name: n.into(), style: s, color: c, width: w }
    }

    #[test]
    fn sit_hin_und_zurueck() {
        let pens = vec![(4, stift("Mittel", 0.4, [0, 0, 0], 0)), (5, stift("Bus", 0.4, [128, 128, 128], 1))];
        let bl = sit_bloecke(&pens, &[]);
        assert_eq!(bl.len(), 2);
        assert_eq!(bl[1].i32(8), 5);
        assert_eq!(bl[1].i32(6), 0x808080);
        let mut grd = Block::new("GRD", vec![100, 0, 0, 0, 25, 0, 0, 0]);
        grd.kids = bl;
        assert_eq!(aus_grd(&grd), pens);
    }

    #[test]
    fn unveraenderte_bytes_bleiben() {
        // Feld 7 mit weiteren Bits wie im Original ("Rot": 6029312 = 0x5C0000)
        let mut b = Block::new("SIT", vec![0; 40]);
        put_str(&mut b.data, 0, 20, "Rot");
        b.set_i32(5, 2);
        b.set_i32(7, 6029312);
        b.set_i32(8, 4);
        let pens = aus_grd(&Block { kids: vec![b.clone()], ..Block::new("GRD", vec![]) });
        assert_eq!(sit_bloecke(&pens, &[&b])[0].data, b.data);
        let mut neu = pens.clone();
        neu[0].1.style = 1;
        assert_eq!(sit_bloecke(&neu, &[&b])[0].i32(7), 6029312 | 1);
    }

    #[test]
    fn tabelle_und_loeschen() {
        let prj = crate::model::Project::new_default().stifte;
        let mut pens = vec![(4, stift("A", 0.5, [255, 0, 0], 0)), (5, stift("B", 0.3, [0, 0, 0], 2))];
        let t = tabelle(&prj, &pens);
        assert_eq!(t.len(), 6);
        assert_eq!(t[1].name, "Gestrichelt");
        assert_eq!(t[5].name, "B");
        assert_eq!(neue_nummer(&pens), 6);
        assert_eq!(neue_nummer(&[]), 4);
        let (mut a, mut b, mut c) = (4usize, 5usize, 1usize);
        let ids = loeschen(&mut pens, 4, &mut [(1, &mut a), (2, &mut b), (3, &mut c)]);
        assert_eq!((a, b, c), (0, 4, 1));
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(pens, vec![(4, stift("B", 0.3, [0, 0, 0], 2))]);
    }

    #[test]
    fn strich_eigener_stift() {
        let mut d = crate::symbols::builtin().into_iter().next().unwrap();
        d.pens = vec![(4, stift("Rot", 0.5, [255, 0, 0], 1))];
        let s = Stroke::new(1.0, Color32::BLACK);
        let (k, art) = strich(&d, 4, s, 10.0);
        assert_eq!((k.width, k.color, art), (5.0, Color32::from_rgb(255, 0, 0), 1));
        assert_eq!(strich(&d, 2, s, 10.0), (s, 2));
        // markiert: Farbe der Markierung bleibt
        let m = Stroke::new(1.0, Color32::BLUE);
        assert_eq!(strich(&d, 4, m, 10.0).0.color, Color32::BLUE);
    }
}
