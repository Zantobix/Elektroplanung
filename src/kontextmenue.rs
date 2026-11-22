//! Sichtbarkeit der Einträge des Blatt-Kontextmenüs (PopupMenu1) wie im Original.
//!
//! Das Original entscheidet nicht nach der Markierung, sondern nach dem Objekt, das beim Drücken der rechten Maustaste
//! vormarkiert war (Zustand der Vormarkierung beim Drücken: nichts, Symbol, Leitung, Verbindungsobjekt mit Unterart
//! Sammelschiene/Verweis/Klemmstelle/Kontaktspiegel, Symboltext, Zeichenobjekt). Daraus schaltet es jeden der 21
//! Einträge sichtbar/unsichtbar; überzählige Trennstriche entfallen (AutoLineReduction der VCL-Menüs). Nur „Löschen“
//! hängt zusätzlich davon ab, ob etwas markiert ist.
//!
//! Regeln (je Eintrag, sonst unsichtbar):
//! - Löschen: etwas markiert oder Rechtsklick auf ein Verbindungsobjekt
//! - Objekt einfügen, Verweis einfügen: freie Fläche (nicht im Symboleditor); Objekt einfügen nur aktiv mit Projektsymbolen
//! - Sammelschiene: Sammelschiene · Ziel setzen: Verweis · Bezeichner setzen: Anschluss im Symboleditor (Element Typ 7)
//! - Bildindex/Optionen/Parameter: Symbol mit Bildnamen/Zusatzoptionen/Parametern
//! - Alternativtext, Eigentümer, Symbol bearbeiten: Symbol (Eigentümer › Suchen nur mit Eigentümer)
//! - Bauteilliste: Symbol und Klemmstelle (0x41933b, 0x41a1d9; Bauteilliste1Click 0x49d57c nimmt beide)
//! - Kontaktspiegel hinzufügen: Symbol, das noch keinen Kontaktspiegel hat
//! - Ausrichtung: Symboltext (Titel/Wert) sowie Text und Textbox · Fixieren: Zeichenobjekte (auch Text, Tabelle, Bild)
//! - Eigenschaften: immer (auf freier Fläche im Symboleditor gesperrt) · Pfadbezeichnung: freie Fläche eines Stromlaufplans
//! - Netz markieren (bewusste Erweiterung, nicht im Original, `netz.rs`): Leitung, am Ende nach einem Trenner

use crate::app::App;
use crate::model::*;
use eframe::egui;

/// Objekt unter der Maus beim Rechtsklick (Vormarkierung beim Drücken im Original).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenueZiel {
    /// freie Blattfläche
    Leer,
    Symbol(u64),
    /// Titel (`wert` = false) bzw. Wert eines Symbols
    Symboltext { id: u64, wert: bool },
    Leitung(u64),
    Sammelschiene(u64),
    Verweis(u64),
    Klemme(u64),
    Kontaktspiegel(u64),
    /// Text bzw. Textbox (Zeichenelement Typ 6/11)
    Text(u64),
    /// übrige Zeichenobjekte: Linie, Kurve, Rechteck, Dreieck, Kreis, Vieleck, Tabelle, Bild
    Grafik(u64),
    /// Anschluss im Symboleditor (Zeichenelement Typ 7)
    Anschluss(u64),
}

impl MenueZiel {
    /// Art eines Objekts auf dem Blatt.
    pub fn von(o: &Obj) -> MenueZiel {
        match o.kind {
            Kind::Symbol { .. } => MenueZiel::Symbol(o.id),
            Kind::Wire { .. } => MenueZiel::Leitung(o.id),
            Kind::Busbar { .. } => MenueZiel::Sammelschiene(o.id),
            Kind::Verweis { .. } => MenueZiel::Verweis(o.id),
            Kind::Terminal { .. } => MenueZiel::Klemme(o.id),
            Kind::Spiegel { .. } => MenueZiel::Kontaktspiegel(o.id),
            Kind::Text { .. } => MenueZiel::Text(o.id),
            _ => MenueZiel::Grafik(o.id),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match *self {
            MenueZiel::Leer => None,
            MenueZiel::Symboltext { id, .. } => Some(id),
            MenueZiel::Symbol(id)
            | MenueZiel::Leitung(id)
            | MenueZiel::Sammelschiene(id)
            | MenueZiel::Verweis(id)
            | MenueZiel::Klemme(id)
            | MenueZiel::Kontaktspiegel(id)
            | MenueZiel::Text(id)
            | MenueZiel::Grafik(id)
            | MenueZiel::Anschluss(id) => Some(id),
        }
    }

    /// Verbindungsobjekt (im Original eigene Liste: Sammelschiene, Verweis, Klemmstelle, Kontaktspiegel).
    fn verbindungsobjekt(&self) -> bool {
        matches!(self, MenueZiel::Sammelschiene(_) | MenueZiel::Verweis(_) | MenueZiel::Klemme(_) | MenueZiel::Kontaktspiegel(_))
    }
}

/// Angaben zum Symbol unter der Maus, von denen Einträge abhängen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Bildnamen (SEI) vorhanden → Bildindex
    pub bilder: bool,
    /// benannte Zusatzoptionen → Optionen
    pub optionen: bool,
    /// Symbolparameter → Parameter
    pub parameter: bool,
    /// schon ein Kontaktspiegel vorhanden → „Kontaktspiegel hinzufügen“ entfällt
    pub hat_spiegel: bool,
    /// Eigentümer gesetzt → Eigentümer › Suchen aktiv
    pub hat_eigentuemer: bool,
}

/// Alles, wovon die Sichtbarkeit abhängt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lage {
    pub ziel: MenueZiel,
    /// etwas markiert
    pub markiert: bool,
    /// Blatt im Symboleditor (im Original Blattart ≥ 'a')
    pub symboleditor: bool,
    /// Blattart Stromlaufplan (3)
    pub stromlaufplan: bool,
    /// Projektsymbole vorhanden (Objekt einfügen aktiv)
    pub projektsymbole: bool,
    pub symbol: SymbolInfo,
}

/// Einträge von PopupMenu1 in der Reihenfolge des Formulars (Trenner = N10, N8, N9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eintrag {
    Loeschen,
    Trenner,
    ObjektEinfuegen,
    VerweisEinfuegen,
    Sammelschiene,
    Bildindex,
    Optionen,
    Parameter,
    Alternativtext,
    Eigentuemer,
    ZielSetzen,
    BezeichnerSetzen,
    Kontaktspiegel,
    Bauteilliste,
    SymbolBearbeiten,
    Ausrichtung,
    Eigenschaften,
    Fixieren,
    Pfadbezeichnung,
    /// Nachbau: alle Leitungen des Netzes markieren (`netz.rs`)
    NetzMarkieren,
}

/// Sichtbare Einträge (überzählige Trenner entfernt) wie im Original.
pub fn eintraege(l: &Lage) -> Vec<Eintrag> {
    use Eintrag::*;
    let z = l.ziel;
    let leer = z == MenueZiel::Leer;
    let sym = matches!(z, MenueZiel::Symbol(_));
    let sichtbar = |e: Eintrag| -> bool {
        match e {
            Loeschen => l.markiert || z.verbindungsobjekt(),
            Trenner | Eigenschaften => true,
            ObjektEinfuegen | VerweisEinfuegen => leer && !l.symboleditor,
            Sammelschiene => matches!(z, MenueZiel::Sammelschiene(_)),
            Bildindex => sym && l.symbol.bilder,
            Optionen => sym && l.symbol.optionen,
            Parameter => sym && l.symbol.parameter,
            Alternativtext | Eigentuemer | SymbolBearbeiten => sym,
            Bauteilliste => sym || matches!(z, MenueZiel::Klemme(_)),
            ZielSetzen => matches!(z, MenueZiel::Verweis(_)),
            BezeichnerSetzen => matches!(z, MenueZiel::Anschluss(_)),
            Kontaktspiegel => sym && !l.symbol.hat_spiegel,
            Ausrichtung => matches!(z, MenueZiel::Symboltext { .. } | MenueZiel::Text(_)),
            Fixieren => matches!(z, MenueZiel::Text(_) | MenueZiel::Grafik(_) | MenueZiel::Anschluss(_)),
            Pfadbezeichnung => leer && l.stromlaufplan && !l.symboleditor,
            NetzMarkieren => matches!(z, MenueZiel::Leitung(_)),
        }
    };
    let formular = [
        Loeschen, Trenner, ObjektEinfuegen, VerweisEinfuegen, Sammelschiene, Bildindex, Optionen, Parameter, Alternativtext, Eigentuemer, ZielSetzen,
        BezeichnerSetzen, Kontaktspiegel, Bauteilliste, Trenner, SymbolBearbeiten, Trenner, Ausrichtung, Eigenschaften, Fixieren, Pfadbezeichnung,
        Trenner, NetzMarkieren,
    ];
    let mut v: Vec<Eintrag> = vec![];
    for e in formular.into_iter().filter(|e| sichtbar(*e)) {
        // kein Trenner am Anfang und keine zwei hintereinander
        if e == Trenner && v.last().is_none_or(|l| *l == Trenner) {
            continue;
        }
        v.push(e);
    }
    if v.last() == Some(&Trenner) {
        v.pop();
    }
    v
}

/// „Eigenschaften“ aktiv (im Original auf freier Fläche eines Symbolblattes gesperrt).
pub fn eigenschaften_aktiv(l: &Lage) -> bool {
    !(l.ziel == MenueZiel::Leer && l.symboleditor)
}

/// Blattreiter-Menü (PopupMenu3) wie PopupMenu3Popup 0x480fbc: aktiv sind [Blatt schliessen, Alle Blätter
/// schließen, Alle Blätter dieser Anlage schließen, Eigenschaften]. Das Original prüft den **aktiven** Reiter
/// (TabIndex), nicht den angeklickten: „Blatt schliessen“ nur, wenn einer aktiv ist und kein Symbolblatt
/// (Blattart < 'a'); „… dieser Anlage …“ nur mit aktueller Anlage (Projekt +0xbc, deren Liste +0xec).
pub fn blattreiter_aktiv(aktiv: bool, symbolblatt: bool, anlage: bool) -> [bool; 4] {
    [aktiv && !symbolblatt, true, anlage, true]
}

fn speicher_id() -> egui::Id {
    egui::Id::unique("ep_menueziel")
}

/// Ziel des Rechtsklicks merken (gilt, solange das Kontextmenü offen ist).
pub fn merken(ctx: &egui::Context, z: MenueZiel) {
    ctx.data_mut(|d| d.insert_temp(speicher_id(), z));
}

pub fn gemerkt(ctx: &egui::Context) -> MenueZiel {
    ctx.data(|d| d.get_temp::<MenueZiel>(speicher_id())).unwrap_or(MenueZiel::Leer)
}

impl App {
    /// Objekt unter `m` für das Kontextmenü: zuerst Titel/Wert eines Symbols (wie die Vormarkierung), dann das
    /// oberste markierbare Objekt.
    pub fn menue_ziel(&self, m: P, tol: f32) -> MenueZiel {
        if let Some((id, wert, _, _)) = self.beschriftung_unter(m, tol * 0.4) {
            return MenueZiel::Symboltext { id, wert };
        }
        let Some(id) = self.hit_top(m, tol) else { return MenueZiel::Leer };
        let Some(o) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)) else { return MenueZiel::Leer };
        // im Symboleditor sind alle Objekte Zeichenelemente, Anschlüsse (Klemmstellen dort) Typ 7
        match o.kind {
            Kind::Terminal { .. } if self.sym_edit.is_some() => MenueZiel::Anschluss(id),
            _ => MenueZiel::von(o),
        }
    }

    /// Lage für `eintraege` aus dem aktuellen Zustand.
    pub fn menue_lage(&self, ziel: MenueZiel) -> Lage {
        let mut symbol = SymbolInfo::default();
        if let MenueZiel::Symbol(id) = ziel {
            let s = self.sheet();
            if let Some(Kind::Symbol { key, ext, .. }) = s.and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| &o.kind) {
                let def = crate::symbols::find(&self.lib, key);
                symbol.bilder = def.is_some_and(|d| !d.options.is_empty());
                symbol.optionen = def.is_some_and(|d| d.flag_names.iter().any(|n| !n.is_empty()));
                symbol.parameter = def.is_some_and(|d| !d.params.is_empty());
                symbol.hat_eigentuemer = ext.eigentuemer != 0;
            }
            symbol.hat_spiegel = s.is_some_and(|s| s.objects.iter().any(|o| matches!(o.kind, Kind::Spiegel { owner, .. } if owner == id)));
        }
        Lage {
            ziel,
            markiert: !self.selection.is_empty(),
            symboleditor: self.sym_edit.is_some(),
            stromlaufplan: self.sheet().is_some_and(|s| s.blattart() == 3),
            projektsymbole: !self.project.symdefs.is_empty(),
            symbol,
        }
    }

    /// Ausrichtung › Links/Mittig/Rechts bzw. Hochkant/Waagerecht auf Titel oder Wert eines Symbols. Wie im
    /// Original: ist das Symbol markiert, gilt es für denselben Text aller markierten Symbole, sonst nur für dieses.
    /// Hochkant gilt im Nachbau für Titel und Wert gemeinsam (ein Merker je Symbol).
    pub fn symboltext_ausrichten(&mut self, id: u64, wert: bool, al: Option<u8>, hochkant: Option<bool>) {
        let ids: Vec<u64> = if self.selection.contains(&id) { self.selection.clone() } else { vec![id] };
        self.checkpoint();
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| ids.contains(&o.id)) {
                symboltext_setzen(&mut o.kind, wert, al, hochkant);
            }
        });
        self.dirty = true;
    }

    /// Eigentümer › Suchen wie im Original (Suchen2Click 0x480ba4): Blatt des Eigentümers öffnen (auch in einer
    /// anderen Anlage) und nur ihn markieren.
    pub fn eigentuemer_suchen(&mut self, id: u64) {
        let owner = match self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| &o.kind) {
            Some(Kind::Symbol { ext, .. }) => ext.eigentuemer,
            _ => 0,
        };
        if owner == 0 {
            return;
        }
        let Some((r, _)) = self.project.find_obj(owner) else {
            self.info = "Eigentümer nicht gefunden".into();
            return;
        };
        if self.cur() != Some(r) {
            self.open_sheet(r);
        }
        self.selection = vec![owner];
        self.info = format!("Eigentümer: {}", crate::kontaktspiegel::lage(&self.project, owner).unwrap_or_default());
    }

    /// Eigenschaften eines Symboltextes wie im Original: Dialog „Texteigenschaft“ (TTextchange) für Titel bzw. Wert;
    /// gibt ein Eigentümer den Titel vor, erscheint stattdessen der Hinweis des Originals.
    pub fn symboltext_eigenschaften(&mut self, id: u64, wert: bool) {
        let Some(Kind::Symbol { bmk, value, ext, .. }) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).map(|o| o.kind.clone()) else { return };
        if !wert && ext.eigentuemer != 0 {
            crate::meldungen::hinweis(crate::meldungen::T_GEHT_NICHT, crate::meldungen::TITEL_VOM_EIGENTUEMER);
            return;
        }
        let text = if wert { value } else { bmk };
        self.dlg3 = Some(crate::dialoge3::Dlg3::TextChange { ids: vec![id], text, wert: Some(wert) });
    }
}

/// Ausrichtung (0 Links, 1 Mittig, 2 Rechts) bzw. Hochkant eines Symboltextes setzen.
pub fn symboltext_setzen(k: &mut Kind, wert: bool, al: Option<u8>, hochkant: Option<bool>) {
    let Kind::Symbol { title, ext, .. } = k else { return };
    if let Some(a) = al {
        if wert {
            ext.value_align = a.min(2);
        } else {
            ext.title_align = a.min(2);
        }
    }
    if let (Some(v), Some((_, vert))) = (hochkant, title.as_mut()) {
        *vert = v;
    }
}

/// Häkchen im Untermenü Ausrichtung (Links, Mittig, Rechts, Hochkant, Waagerecht) wie im Original.
pub fn ausrichtung_haken(k: &Kind, wert: Option<bool>) -> [bool; 5] {
    let (al, vert) = match (k, wert) {
        (Kind::Symbol { title, ext, .. }, Some(w)) => (if w { ext.value_align } else { ext.title_align }, title.map(|t| t.1).unwrap_or(false)),
        (Kind::Text { align, vertical, .. }, _) => (*align as u8, *vertical),
        _ => return [false; 5],
    };
    [al == 0, al == 1, al == 2, vert, !vert]
}

#[cfg(test)]
mod tests {
    use super::Eintrag::*;
    use super::*;

    fn lage(ziel: MenueZiel) -> Lage {
        Lage { ziel, markiert: ziel != MenueZiel::Leer, symboleditor: false, stromlaufplan: true, projektsymbole: true, symbol: SymbolInfo::default() }
    }

    #[test]
    fn freie_flaeche() {
        assert_eq!(eintraege(&lage(MenueZiel::Leer)), vec![ObjektEinfuegen, VerweisEinfuegen, Trenner, Eigenschaften, Pfadbezeichnung]);
        // Klemmplan o. Ä.: keine Pfadbezeichnung
        let mut l = lage(MenueZiel::Leer);
        l.stromlaufplan = false;
        assert_eq!(eintraege(&l), vec![ObjektEinfuegen, VerweisEinfuegen, Trenner, Eigenschaften]);
        // Symboleditor: nur (gesperrte) Eigenschaften
        l.symboleditor = true;
        assert_eq!(eintraege(&l), vec![Eigenschaften]);
        assert!(!eigenschaften_aktiv(&l));
        // mit Markierung (Rechtsklick daneben) zusätzlich Löschen
        let mut l = lage(MenueZiel::Leer);
        l.markiert = true;
        assert_eq!(eintraege(&l)[..2], [Loeschen, Trenner]);
    }

    #[test]
    fn blattreiter() {
        assert_eq!(blattreiter_aktiv(true, false, true), [true; 4]);
        assert_eq!(blattreiter_aktiv(true, true, true), [false, true, true, true]);
        assert_eq!(blattreiter_aktiv(false, false, false), [false, true, false, true]);
    }

    #[test]
    fn symbol() {
        let mut l = lage(MenueZiel::Symbol(1));
        assert_eq!(eintraege(&l), vec![Loeschen, Trenner, Alternativtext, Eigentuemer, Kontaktspiegel, Bauteilliste, Trenner, SymbolBearbeiten, Trenner, Eigenschaften]);
        l.symbol = SymbolInfo { bilder: true, optionen: true, parameter: true, hat_spiegel: true, hat_eigentuemer: true };
        assert_eq!(eintraege(&l), vec![Loeschen, Trenner, Bildindex, Optionen, Parameter, Alternativtext, Eigentuemer, Bauteilliste, Trenner, SymbolBearbeiten, Trenner, Eigenschaften]);
    }

    #[test]
    fn symboltext_und_text() {
        let e = eintraege(&lage(MenueZiel::Symboltext { id: 1, wert: false }));
        assert_eq!(e, vec![Loeschen, Trenner, Ausrichtung, Eigenschaften]);
        // Symboltext ohne Markierung: kein Löschen
        let mut l = lage(MenueZiel::Symboltext { id: 1, wert: true });
        l.markiert = false;
        assert_eq!(eintraege(&l), vec![Ausrichtung, Eigenschaften]);
        assert_eq!(eintraege(&lage(MenueZiel::Text(2))), vec![Loeschen, Trenner, Ausrichtung, Eigenschaften, Fixieren]);
    }

    #[test]
    fn verbindungen_und_grafik() {
        assert_eq!(eintraege(&lage(MenueZiel::Leitung(1))), vec![Loeschen, Trenner, Eigenschaften, Trenner, NetzMarkieren]);
        assert_eq!(eintraege(&lage(MenueZiel::Sammelschiene(1))), vec![Loeschen, Trenner, Sammelschiene, Trenner, Eigenschaften]);
        assert_eq!(eintraege(&lage(MenueZiel::Verweis(1))), vec![Loeschen, Trenner, ZielSetzen, Trenner, Eigenschaften]);
        assert_eq!(eintraege(&lage(MenueZiel::Klemme(1))), vec![Loeschen, Trenner, Bauteilliste, Trenner, Eigenschaften]);
        assert_eq!(eintraege(&lage(MenueZiel::Kontaktspiegel(1))), vec![Loeschen, Trenner, Eigenschaften]);
        // Verbindungsobjekte zeigen Löschen auch ohne Markierung, Leitungen nicht
        let mut l = lage(MenueZiel::Verweis(1));
        l.markiert = false;
        assert_eq!(eintraege(&l)[0], Loeschen);
        let mut l = lage(MenueZiel::Leitung(1));
        l.markiert = false;
        assert_eq!(eintraege(&l), vec![Eigenschaften, Trenner, NetzMarkieren]);
        assert_eq!(eintraege(&lage(MenueZiel::Grafik(1))), vec![Loeschen, Trenner, Eigenschaften, Fixieren]);
        let mut l = lage(MenueZiel::Anschluss(1));
        l.symboleditor = true;
        assert_eq!(eintraege(&l), vec![Loeschen, Trenner, BezeichnerSetzen, Trenner, Eigenschaften, Fixieren]);
    }

    #[test]
    fn ziel_je_objektart() {
        let o = |kind: Kind| Obj { id: 7, kind, pen: 0, fixed: false, group: None, raw: None };
        let p = P::default();
        assert_eq!(MenueZiel::von(&o(Kind::Line { a: p, b: p })), MenueZiel::Grafik(7));
        assert_eq!(MenueZiel::von(&o(Kind::Wire { pts: vec![p, p], leitung: 0, bez: vec![], opts: 0 })), MenueZiel::Leitung(7));
        assert_eq!(MenueZiel::von(&o(Kind::Spiegel { pos: p, owner: 1, rows: vec![], auto: true })), MenueZiel::Kontaktspiegel(7));
    }

    #[test]
    fn symboltext_ausrichtung() {
        let mut k = Kind::Symbol {
            key: String::new(),
            pos: P::default(),
            rot: 0,
            mirror_h: false,
            mirror_v: false,
            bmk: "-K1".into(),
            value: "230V".into(),
            scale: 1.0,
            option: 0,
            title: Some((P::new(1.0, 2.0), false)),
            flags: 0,
            ext: SymExt::default(),
        };
        symboltext_setzen(&mut k, true, Some(2), None);
        symboltext_setzen(&mut k, false, Some(1), Some(true));
        let Kind::Symbol { title, ext, .. } = &k else { unreachable!() };
        assert_eq!((ext.title_align, ext.value_align, title.unwrap().1), (1, 2, true));
        assert_eq!(ausrichtung_haken(&k, Some(true)), [false, false, true, true, false]);
        assert_eq!(ausrichtung_haken(&k, Some(false)), [false, true, false, true, false]);
    }
}
