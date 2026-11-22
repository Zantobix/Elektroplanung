//! Kontextmenü „Bauteilliste › Bearbeiten“ an Symbol oder Klemmstelle: Fenster mit der Liste der zugeordneten
//! Bauteile, in die man Bauteile aus dem Baum zieht.
//!
//! Belegt aus dem Original: der Dialog ist TSSpiegel in der Betriebsart 1 (Feld +0x324; Betriebsart 0 ist der
//! Kontaktspiegel). Kopfzeile (0x4b6b6c) „Nr | Bezeichnung | Typ | Intern“. ListeDragOver (0x4b6cb8) nimmt nur in
//! Betriebsart 1 an: gezogen aus dem Reiter Bauteile (TeilListe) oder aus dem Projektbaum, Gruppe Bauteile (6), und
//! nur, wenn das Bauteil im Projekt gefunden wird (0x417b24). ListeDragDrop (0x4b6e80) füllt die letzte (leere)
//! Zeile: Nummer, Name (BTL +0x2c), Typ (+0x54), Intern Nr (+0xa4) und merkt die Kennung des Bauteils für die Zeile.
//!
//! Mehrere Bauteile je Objekt: Das Speicherobjekt eines Symbols bzw. einer Klemmstelle führt die Zuordnung als
//! verkettete Liste (+0xbc, Knoten = Kennung 8 Byte, nächster bei +8). Die Speicherroutine (0x4228c4) schreibt je
//! Knoten einen eigenen Block (0x49117c mit 8 Byte, danach Blockende 0x495e00) – im .epr also ein IDL-Unterblock
//! je Bauteil, in Listenreihenfolge (epr_zusatz.rs). Das Modell führt das erste Bauteil in `Project::part_of`
//! (davon hängen Eigenschaftendialog, Meldungen und Listen ab) und die übrigen in `Project::weitere_bauteile`.

use crate::app::App;
use crate::model::*;
use eframe::egui;

/// Nutzlast beim Ziehen eines Bauteils aus dem Baum: Index in `Project::bauteile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BauteilZug(pub usize);

/// Zustand des Fensters: Objekt (Symbol oder Klemmstelle), zugeordnete Bauteile (Indizes in `Project::bauteile`),
/// gewählte Zeile und – nach „Neues Bauteil …“ – die Anzahl der Bauteile davor (ein neu angelegtes wird angehängt).
#[derive(Clone, Debug, PartialEq)]
pub struct BauteilListeDlg {
    pub obj: u64,
    pub teile: Vec<usize>,
    pub zeile: Option<usize>,
    pub neu_ab: Option<usize>,
}

/// Alle Bauteile eines Objekts in Listenreihenfolge.
pub fn liste(p: &Project, obj: u64) -> Vec<usize> {
    p.part_of.get(&obj).copied().into_iter().chain(p.weitere_bauteile.get(&obj).into_iter().flatten().copied()).collect()
}

/// Bauteile eines Objekts setzen (leer = keine Zuordnung).
pub fn setzen(p: &mut Project, obj: u64, teile: &[usize]) {
    match teile.split_first() {
        Some((erstes, rest)) => {
            p.part_of.insert(obj, *erstes);
            if rest.is_empty() {
                p.weitere_bauteile.remove(&obj);
            } else {
                p.weitere_bauteile.insert(obj, rest.to_vec());
            }
        }
        None => zuordnung_entfernen(p, obj),
    }
}

/// Bauteilliste › Löschen: alle Bauteile des Objekts entfernen.
pub fn zuordnung_entfernen(p: &mut Project, obj: u64) {
    p.part_of.remove(&obj);
    p.weitere_bauteile.remove(&obj);
}

/// Bauteil `i` wurde aus dem Projekt gelöscht: aus allen Listen nehmen, spätere Indizes nachziehen.
pub fn bauteil_entfernt(p: &mut Project, i: usize) {
    // jedes Objekt nur einmal (es kann in beiden Tabellen stehen)
    let objs: std::collections::BTreeSet<u64> = p.part_of.keys().chain(p.weitere_bauteile.keys()).copied().collect();
    for obj in objs {
        let neu: Vec<usize> = liste(p, obj).into_iter().filter(|v| *v != i).map(|v| if v > i { v - 1 } else { v }).collect();
        setzen(p, obj, &neu);
    }
}

/// Zeilen der Liste wie im Original: Nr, Bezeichnung (Name), Typ, Intern (Nummer).
pub fn zeilen(p: &Project, teile: &[usize]) -> Vec<[String; 4]> {
    teile.iter().filter_map(|i| p.bauteile.get(*i)).enumerate().map(|(n, b)| [(n + 1).to_string(), b.name.clone(), b.typ.clone(), b.nummer.clone()]).collect()
}

/// Ablegen eines gezogenen Bauteils: nur Bauteile des Projekts (wie ListeDragOver), angehängt in der letzten Zeile.
pub fn ablegen(d: &mut BauteilListeDlg, p: &Project, zug: BauteilZug) -> bool {
    if zug.0 >= p.bauteile.len() {
        return false;
    }
    d.teile.push(zug.0);
    d.zeile = Some(d.teile.len() - 1);
    true
}

/// Gewählte Zeile löschen.
pub fn zeile_loeschen(d: &mut BauteilListeDlg) {
    if let Some(z) = d.zeile.filter(|z| *z < d.teile.len()) {
        d.teile.remove(z);
        d.zeile = if d.teile.is_empty() { None } else { Some(z.min(d.teile.len() - 1)) };
    }
}

impl App {
    /// Bauteilliste › Bearbeiten (Bauteilliste1Click 0x49d57c): nur für Symbole und Klemmstellen.
    pub fn bauteilliste_oeffnen(&mut self, obj: u64) {
        let ok = self.sheet().is_some_and(|s| s.objects.iter().any(|o| o.id == obj && matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. })));
        if ok {
            self.zusatz.bauteilliste = Some(BauteilListeDlg { obj, teile: liste(&self.project, obj), zeile: None, neu_ab: None });
        }
    }

    /// Fenster zeichnen (nicht modal wie im Original, damit man aus dem Baum ziehen kann) und übernehmen.
    pub fn bauteilliste_ui(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.zusatz.bauteilliste.take() else { return };
        // nach „Neues Bauteil …“: das neu angelegte Bauteil anhängen
        if let Some(n) = d.neu_ab {
            if self.project.bauteile.len() > n {
                d.teile.push(self.project.bauteile.len() - 1);
                d.zeile = Some(d.teile.len() - 1);
                d.neu_ab = None;
            }
        }
        let mut done: Option<bool> = None;
        let mut neu = false;
        let namen: Vec<String> = self.project.bauteile.iter().map(|b| format!("{} {}", b.name, b.typ).trim().to_string()).collect();
        egui::Window::new("Bauteilliste").collapsible(false).resizable(false).default_width(300.0).show(ctx, |ui| {
            let rows = zeilen(&self.project, &d.teile);
            let (_, gezogen) = ui.dnd_drop_zone::<BauteilZug, _>(egui::Frame::default().inner_margin(4.0), |ui| {
                ui.set_min_size(egui::Vec2::new(290.0, 120.0));
                egui::Grid::new("bt_liste").striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Nr", "Bezeichnung", "Typ", "Intern"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (z, r) in rows.iter().enumerate() {
                        let gewaehlt = d.zeile == Some(z);
                        if ui.selectable_label(gewaehlt, r[0].as_str()).clicked() {
                            d.zeile = Some(z);
                        }
                        for c in &r[1..] {
                            ui.label(c.as_str());
                        }
                        ui.end_row();
                    }
                });
                if rows.is_empty() {
                    ui.weak("Bauteil aus dem Reiter „Bauteile“ hierher ziehen");
                }
            });
            if let Some(z) = gezogen {
                ablegen(&mut d, &self.project, *z);
            }
            // Nachbau: Hinzufügen ohne Ziehen und neues Bauteil
            ui.horizontal(|ui| {
                let mut i = usize::MAX;
                egui::ComboBox::from_id_salt("bt_liste_wahl").width(170.0).selected_text("Bauteil hinzufügen …").show_ui(ui, |ui| {
                    for (k, n) in namen.iter().enumerate() {
                        ui.selectable_value(&mut i, k, n);
                    }
                });
                if i < namen.len() {
                    ablegen(&mut d, &self.project, BauteilZug(i));
                }
                if ui.button("Neues Bauteil …").clicked() {
                    neu = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(d.zeile.is_some(), egui::Button::new("Zeile löschen")).clicked() {
                    zeile_loeschen(&mut d);
                }
                ui.add_space(30.0);
                if ui.button("Abbruch").clicked() {
                    done = Some(false);
                }
                if ui.button("OK").clicked() {
                    done = Some(true);
                }
            });
        });
        if neu {
            d.neu_ab = Some(self.project.bauteile.len());
            self.part_assign_after = None;
            self.new_bauteil_dialog();
        }
        match done {
            None => self.zusatz.bauteilliste = Some(d),
            Some(false) => {}
            Some(true) => {
                if liste(&self.project, d.obj) != d.teile {
                    self.checkpoint();
                    setzen(&mut self.project, d.obj, &d.teile);
                    self.dirty = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projekt() -> Project {
        let mut p = Project::new_default();
        for (n, t, k) in [("Schütz", "DILM12", "K1"), ("Hilfsschalter", "DILA-XHI11", "K1.1"), ("Klemme", "UK5", "X1")] {
            p.bauteile.push(Bauteil { name: n.into(), typ: t.into(), nummer: k.into(), ..Default::default() });
        }
        p
    }

    #[test]
    fn ablegen_und_zeilen() {
        let p = projekt();
        let mut d = BauteilListeDlg { obj: 5, teile: vec![], zeile: None, neu_ab: None };
        assert!(zeilen(&p, &d.teile).is_empty());
        // unbekanntes Bauteil wird nicht angenommen
        assert!(!ablegen(&mut d, &p, BauteilZug(3)));
        assert!(ablegen(&mut d, &p, BauteilZug(0)));
        assert!(ablegen(&mut d, &p, BauteilZug(1)));
        assert_eq!(
            zeilen(&p, &d.teile),
            vec![["1".to_string(), "Schütz".into(), "DILM12".into(), "K1".into()], ["2".to_string(), "Hilfsschalter".into(), "DILA-XHI11".into(), "K1.1".into()]]
        );
        d.zeile = Some(0);
        zeile_loeschen(&mut d);
        assert_eq!((d.teile.clone(), d.zeile), (vec![1], Some(0)));
    }

    #[test]
    fn mehrere_bauteile_je_objekt() {
        let mut p = projekt();
        setzen(&mut p, 7, &[0, 1, 2]);
        assert_eq!(liste(&p, 7), vec![0, 1, 2]);
        assert_eq!(p.part_of.get(&7), Some(&0), "erstes Bauteil wie bisher in part_of");
        setzen(&mut p, 8, &[2]);
        // Bauteil 1 löschen: aus der Liste, spätere rücken nach
        bauteil_entfernt(&mut p, 1);
        assert_eq!(liste(&p, 7), vec![0, 1]);
        assert_eq!(liste(&p, 8), vec![1]);
        // erstes gelöscht: das nächste rückt nach vorn
        bauteil_entfernt(&mut p, 0);
        assert_eq!(liste(&p, 7), vec![0]);
        assert!(p.weitere_bauteile.is_empty());
        zuordnung_entfernen(&mut p, 7);
        assert!(liste(&p, 7).is_empty());
        setzen(&mut p, 8, &[]);
        assert!(p.part_of.is_empty());
    }
}
