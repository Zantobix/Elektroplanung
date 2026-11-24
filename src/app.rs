//! Hauptfenster: Zustand und Aktionen. Die Oberfläche selbst steht in `view.rs`.

use crate::canvas;
use crate::model::*;
use crate::symbols::{self, SymDef};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Rasterweiten der Rasterauswahl; ob gefangen wird, schaltet der Gitterknopf (`App::grid`)
pub const RASTER: [(&str, f32); 7] = [
    ("1,0mm", 1.0),
    ("2,5mm", 2.5),
    ("3,5mm", 3.5),
    ("5,0mm", 5.0),
    ("7,0mm", 7.0),
    ("10,0mm", 10.0),
    ("14,0mm", 14.0),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Klemmstelle,
    Line,
    Curve,
    Rect,
    Triangle,
    Circle,
    Text,
    TextBox,
    Table,
    Image,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Projekt,
    Symbole,
    Bauteile,
}

#[derive(Clone)]
pub enum Drag {
    None,
    /// Verschieben: Rasterpunkt beim Drücken und bisher angewandte Gesamtverschiebung (kein Wandern durch Ausrichtung)
    Move { start: P, applied: P, moved: bool },
    Rubber { start: P },
    Create { tool: Tool, start: P },
    /// Leitung von einem Anschluss aus (Ziehen oder Klickbetrieb wie im Original), siehe leitung_fuehrung.rs
    Wire { f: crate::leitung_fuehrung::Fuehrung },
    /// Neue Kurve zwischen den vier Klicks, siehe kurve.rs
    Kurve(crate::kurve::NeueKurve),
    /// Griff (0 Anfang, 1/2 Kontrollpunkte, 3 Ende) einer markierten Kurve ziehen
    KurvenGriff { id: u64, griff: usize },
    Resize { id: u64, corner: usize },
    /// Punkt eines Zeichenobjekts ziehen (zeichen_punkt.rs); `ab` = Maus beim Drücken, bis sie sich bewegt
    Zeichenpunkt { id: u64, punkt: crate::zeichen_punkt::Punkt, ab: Option<P> },
    /// Rahmen zur Größenänderung um die ganze Markierung: Griff 0–7 (links oben im Uhrzeigersinn),
    /// Umriss und Objekte beim Beginn des Ziehens
    Scale { handle: usize, start: (P, P), orig: Vec<Obj> },
    /// Titel (`wert` = false) bzw. Wert eines Symbols verschieben: Maus und Lage (relativ zum Symbol) beim Drücken
    Beschriftung { id: u64, wert: bool, start: P, orig: P },
    Pan { last: Pos2 },
    /// Rechte Maustaste gedrückt (rechts_ziehen.rs): Rasterpunkt und Bildschirmlage beim Drücken; `kopie` = auf
    /// einem Objekt (Ziehen legt eine Kopie ab), sonst öffnet das Loslassen das Kontextmenü; `schieben` = auf
    /// freier Fläche: Ansichtsversatz beim Drücken (Ziehen verschiebt die vergrößerte Ansicht), `geschoben` = es
    /// wurde verschoben (dann kein Kontextmenü)
    Rechts { start: P, druck: Pos2, kopie: bool, schieben: Option<Vec2>, geschoben: bool },
    /// Teilstück `idx` einer Leitung quer verschieben, Enden bleiben angeschlossen (segment_ziehen.rs)
    /// `knick`: statt des Teilstücks `idx` wird der innere Eckpunkt `idx` gezogen (knick_ziehen.rs)
    Teilstueck { id: u64, idx: usize, orig: Vec<P>, start: P, moved: bool, knick: bool },
}

pub enum TextTarget {
    NewText(P),
    EditObj(u64),
}

pub enum Dialog {
    Text { title: String, value: String, target: TextTarget },
    Symbol { id: u64, bmk: String, value: String, option: u16, options: Vec<String> },
    Project { name: String, kunde: String, owner: String, von: String, ort: String, znr: String, done: bool },
    Sheet { r: SheetRef, name: String, number: String, columns: String },
    Options,
    Info,
    Message(String),
    Table { id: u64, rows: String, cols: String, cells: Vec<String> },
}

/// Aktion, die nach der Rückfrage "Geänderte Daten Speichern?" ausgeführt wird.
#[derive(Clone, Debug)]
pub enum Pending {
    New,
    Open,
    OpenPath(PathBuf),
    Quit,
}

pub struct Shot {
    pub path: PathBuf,
    pub frames: u32,
    pub requested: bool,
    pub done: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct View {
    pub zoom: f32,
    pub offset: Vec2,
    pub user: bool,
    pub last_size: Vec2,
    pub last_portrait: bool,
    /// Seit wann (egui-Zeit, s) die Maus beim Ziehen in der Randzone liegt (Rand-Mitschieben erst nach `VERWEILEN`)
    pub rand_seit: Option<f64>,
}

#[derive(Clone)]
pub struct TNode {
    pub id: String,
    pub label: String,
    pub bold: bool,
    /// Knotensymbol, Werte siehe `baum::ico` (0 nichts, 1 Buch, 2 Buch+Stern, 3 Blatt, 4 Anlage, 5… Projektgruppen)
    pub icon: u8,
    pub action: TAction,
    pub children: Vec<TNode>,
}

#[derive(Clone)]
pub enum TAction {
    None,
    Symbol(String),
    Sheet(SheetRef),
    Stift(usize),
    Leitung(usize),
    Potential(usize),
    Bauteil(usize),
    Platz(usize),
    /// Projektbild (PRJ › BLD)
    Bild(usize),
    /// Sammelschiene bzw. Klemmleiste einer Anlage (Anlage, Index)
    Schiene(usize, usize),
    Klemmleiste(usize, usize),
}

pub struct App {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub lib: Vec<SymDef>,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
    pub tabs: Vec<SheetRef>,
    pub active: usize,
    pub tool: Tool,
    pub zoom_tool: bool,
    pub pen: usize,
    pub raster_idx: usize,
    pub grid: bool,
    pub path_align: bool,
    pub auto_conn: bool,
    pub size_frame: bool,
    pub fix_active: bool,
    pub leitung: usize,
    pub klemme: usize,
    pub selection: Vec<u64>,
    pub clipboard: Vec<Obj>,
    pub view: View,
    /// Ansicht vor dem Öffnen des Symboleditors (wird nach Übernehmen/Abbruch wiederhergestellt)
    pub view_before_edit: Option<View>,
    /// offene Rückfrage "Geänderte Daten Speichern?" und die Aktion danach
    pub ask_save: Option<Pending>,
    /// Beenden ohne weitere Rückfrage erlaubt (nach Ja/Nein)
    pub quit_ok: bool,
    /// Ansicht je Blatt (Zoom/Verschiebung); ein noch nicht gezeigtes Blatt wird eingepasst wie im Original
    pub sheet_views: std::collections::HashMap<SheetRef, View>,
    pub view_sheet: Option<SheetRef>,
    pub drag: Drag,
    pub placing: Option<String>,
    /// offenes Variantenrad beim Platzieren eines Symbols mit mehreren Bildern
    pub ring: Option<crate::variantenrad::Ring>,
    pub lib_dir: Option<PathBuf>,
    pub lib_info: String,
    /// Bibliothek kam aus dem Zwischenspeicher (Datenbank gesperrt) → regelmäßig neu versuchen
    pub lib_stale: bool,
    pub lib_retry: f64,
    pub mouse_mm: P,
    pub right_tab: RightTab,
    /// erster sichtbarer Blattreiter (Reiter blättern wie im Original)
    pub tab_first: usize,
    pub tree_open: HashSet<String>,
    pub tree_sel: Option<String>,
    pub search: String,
    /// Zustand des Suchfelds im Projektexplorer (Verlauf, „Aa“)
    pub suchfeld: crate::baum::SuchFeld,
    /// Suchen über alle Blätter, Projekt prüfen, Bildexport (suche.rs)
    pub suche: crate::suche::SucheState,
    pub dialog: Option<Dialog>,
    /// Eigenschaftendialoge wie im Original (Symbol, Text, Blatt)
    pub props: Option<crate::props::Props>,
    /// Werkzeugfenster Ausrichtung offen, Index des Referenzobjekts
    pub align_win: bool,
    /// Programmoptionen (Dialog "Option", dauerhaft gespeichert) und offener Dialog
    pub opt: crate::optionen::Optionen,
    pub opt_dlg: Option<crate::optionen::OptDlg>,
    /// Dialoge Projekteigenschaften, Projektinfo, Drucken, Potential
    pub dlg2: Option<crate::dialoge::Dlg2>,
    /// Blattauswahl und Drucker für den nächsten Ausdruck
    pub print_sel: Option<Vec<SheetRef>>,
    pub print_printer: Option<String>,
    /// Druckdialog „An Papiergröße anpassen“ (TForm3 Skal)
    pub print_skal: bool,
    /// zuletzt im Dialog Symboleigenschaften gewählte Freigaben
    pub sym_frg_default: (bool, bool),
    pub align_ref: usize,
    pub info: String,
    pub shot: Option<Shot>,
    pub sheet_color: Color32,
    pub grid_color: Color32,
    pub invert_wheel: bool,
    pub images: HashMap<String, egui::TextureHandle>,
    pub sheet_screen_rect: Rect,
    /// Zeichenfläche im letzten Bild (für Zoom per Tastatur)
    pub canvas_area: Rect,
    /// Ziehen zum Umsortieren: Blatt im Projektbaum, Blattreiter, Seitenreiter des Symboleditors
    pub tree_drag: Option<SheetRef>,
    pub tab_drag: Option<usize>,
    pub page_drag: Option<usize>,
    pub export_req: Option<PathBuf>,
    /// PDF erzeugen: (Datei, danach drucken)
    pub pdf_req: Option<(PathBuf, bool)>,
    pub sym_edit: Option<crate::extra::SymEdit>,
    pub xdlg: Option<crate::extra::XDialog>,
    /// Bauteil wird aus „Bauteil zuordnen“ heraus angelegt → danach diesem Symbol zuordnen
    pub part_assign_after: Option<u64>,
    pub fav_overrides: std::collections::BTreeMap<String, bool>,
    /// Leere Gruppen der Benutzerbibliothek (Bibliotheksverwaltung › Hinzufügen, bib_gruppen.rs)
    pub lib_gruppen: Vec<Vec<String>>,
    pub recent: Vec<String>,
    pub show_library: bool,
    /// Suche nach einer neueren Version (update_pruefung.rs)
    pub update: crate::update_pruefung::Pruefung,
    /// Absturzschutz: Zwischenspeichern (wiederherstellung.rs)
    pub sicherung: crate::wiederherstellung::Stand,
    /// letzte Eingaben für den Fehlerbericht (fehlerbericht.rs) und Auftrag „Bericht anlegen“ (F12)
    pub protokoll: crate::fehlerbericht::Protokoll,
    pub bericht_req: bool,
    /// Importfenster für DXF (dxf_dialog.rs)
    pub dxf_dlg: Option<crate::dxf_dialog::DxfDialog>,
    /// weitere Dialoge (dialoge3.rs)
    pub dlg3: Option<crate::dialoge3::Dlg3>,
    /// Bibliotheksverwaltung: Reiter und Synchronisierung
    pub lib_tab: usize,
    pub lib_sync: (String, bool, bool, usize, String),
    pub lib_filter: String,
    /// Lage des letzten Rechtsklicks auf dem Blatt (für "Verweis einfügen")
    pub mouse_ctx: P,
    pub right_w: f32,
    /// gehaltene Tasten (nur ohne Texteingabe): S kehrt die Strompfadausrichtung, A die automatischen
    /// Verbindungen vorübergehend um, Umschalt schaltet den Rasterfang ab
    pub hold_s: bool,
    pub hold_a: bool,
    pub no_snap: bool,
    /// Taste S/A gedrückt und noch nicht losgelassen: (Taste, während des Haltens benutzt)
    pub hold_press: [Option<bool>; 2],
    /// Zustand der Tasten Leertaste/Strg+Tab/Strg+Ziffer und der Anzeige „Heute“ (blatt_tasten.rs)
    pub bt: crate::blatt_tasten::Stand,
    /// Eigentümer › Setzen: diese Symbole übernehmen die Kennzeichnung des als Nächstes angeklickten Symbols
    /// (Blatt der Ziele, Ziel-Ids) – der Eigentümer darf auf einem anderen Blatt liegen (z. B. Spule zum Kontakt)
    pub owner_pick: Option<(SheetRef, Vec<u64>)>,
    /// Objekte des Blattes vor der laufenden Änderung (netz_folgen.rs: Leitungsnetz wird von hier aus nachgeführt)
    pub netz_vorher: Vec<Obj>,
    /// letzter vom Netz-Wächter zugelassener Stand der laufenden Bearbeitung (netz_waechter.rs)
    pub netz_gueltig: Option<Vec<Obj>>,
    /// Zahl der vom Netz-Wächter abgewiesenen Bearbeitungsschritte (für Tests und Messung)
    pub netz_abgewiesen: usize,
    /// Leitungsformen je Quadrant (leitung_gedaechtnis.rs, nach QElectroTech)
    pub formen: crate::leitung_gedaechtnis::Gedaechtnis,
    /// Blattstände zu Gestenbeginn (neueste zuletzt): kehren alle Objekte außer Leitungen in einen davon zurück, gelten
    /// dessen Leitungen (Rückkehr, Kriterium L10; netz_folgen.rs)
    pub rueckkehr: Vec<(SheetRef, Vec<Obj>)>,
    /// die laufende Geste steht auf einem gemerkten Stand (dann räumt das Loslassen nicht auf)
    pub rueckkehr_gilt: bool,
    /// Text für die System-Zwischenablage nach „Kopieren“ (view.rs gibt ihn an egui weiter): Unter Linux meldet
    /// egui Strg+V nur, wenn dort Text liegt – sonst blieb Einfügen per Tastatur wirkungslos (Maustest Runde 11)
    pub ablage_text: Option<String>,
    /// offenes Fenster „Fehler melden / Wunsch äußern“ (fehlermeldung.rs)
    pub meldung: Option<crate::fehlermeldung::Meldung>,
    /// Eigenschaften mehrerer markierter Objekte (Stift, Fixierung, Textgröße/-stil)
    pub multi_props: Option<crate::mehrfach::MultiProps>,
    /// Symbol wird aus der Bibliothek auf das Blatt gezogen (Loslassen außerhalb bricht ab)
    pub lib_drag: bool,
    /// Sammelschiene (Anlage, Index) wird aus dem Projektbaum gezogen (schiene_platzieren.rs)
    pub schiene_drag: Option<(usize, usize)>,
    /// weitere Dialoge (zusatz.rs: Parameter, Projektgrafik)
    pub zusatz: crate::zusatz::Zusatz,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>) -> Self {
        crate::setup_style(&cc.egui_ctx);
        // SVG-Icons der Werkzeugleisten
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut tree_open = HashSet::new();
        for k in [
            "sym/fav",
            "sym/fav/Kontakte/Schalter",
            "sym/fav/Motore",
            "sym/fav/Relais",
            "sym/fav/Schutzeinrichtung",
            "sym/fav/Siemens",
            "sym/fav/Signaleinrichtung",
            "sym/fav/Signaleinrichtung/Lampen",
            "prj/a0",
            "prj/a0/bl",
        ] {
            tree_open.insert(k.to_string());
        }
        Self {
            project: Project::new_default(),
            path: None,
            dirty: false,
            lib: vec![],
            undo: vec![],
            redo: vec![],
            tabs: vec![SheetRef { anlage: 0, sheet: 0 }],
            active: 0,
            tool: Tool::Select,
            zoom_tool: false,
            pen: 0,
            raster_idx: 5,
            grid: true,
            path_align: true,
            auto_conn: true,
            size_frame: true,
            fix_active: true,
            leitung: 0,
            klemme: 0,
            selection: vec![],
            clipboard: vec![],
            view_before_edit: None,
            ask_save: None,
            quit_ok: false,
            sheet_views: Default::default(),
            view_sheet: None,
            view: View { zoom: 4.0, offset: Vec2::ZERO, user: false, last_size: Vec2::ZERO, last_portrait: false, rand_seit: None },
            drag: Drag::None,
            placing: None,
            ring: None,
            mouse_mm: P::new(65.0, 45.0),
            right_tab: RightTab::Symbole,
            tab_first: 0,
            tree_open,
            tree_sel: None,
            search: String::new(),
            suchfeld: Default::default(),
            suche: Default::default(),
            dialog: None,
            props: None,
            align_win: false,
            opt: Default::default(),
            opt_dlg: None,
            lib_stale: false,
            lib_retry: 0.0,
            dlg2: None,
            print_sel: None,
            print_printer: None,
            print_skal: false,
            sym_frg_default: (true, false),
            align_ref: 0,
            info: String::new(),
            shot: shot.map(|path| Shot { path, frames: 0, requested: false, done: Arc::new(AtomicBool::new(false)) }),
            sheet_color: Color32::from_rgb(238, 221, 221),
            grid_color: crate::optionen::RASTER_MITTEL,
            invert_wheel: false,
            images: HashMap::new(),
            sheet_screen_rect: Rect::NOTHING,
            canvas_area: Rect::NOTHING,
            tree_drag: None,
            tab_drag: None,
            page_drag: None,
            export_req: None,
            pdf_req: None,
            sym_edit: None,
            xdlg: None,
            part_assign_after: None,
            fav_overrides: Default::default(),
            lib_gruppen: vec![],
            recent: vec![],
            show_library: false,
            update: Default::default(),
            sicherung: Default::default(),
            protokoll: Default::default(),
            bericht_req: false,
            dxf_dlg: None,
            dlg3: None,
            lib_tab: 0,
            lib_sync: (String::new(), true, true, 0, String::new()),
            lib_filter: String::new(),
            mouse_ctx: P::new(50.0, 50.0),
            right_w: 441.0,
            hold_s: false,
            hold_a: false,
            no_snap: false,
            hold_press: [None, None],
            bt: Default::default(),
            owner_pick: None,
            netz_vorher: vec![],
            netz_gueltig: None,
            netz_abgewiesen: 0,
            formen: Default::default(),
            rueckkehr: vec![],
            rueckkehr_gilt: false,
            ablage_text: None,
            meldung: None,
            multi_props: None,
            zusatz: Default::default(),
            lib_drag: false,
            schiene_drag: None,
            lib_dir: None,
            lib_info: String::new(),
        }
        .with_library(cc)
    }

    fn with_library(mut self, cc: &eframe::CreationContext<'_>) -> Self {
        let saved: Option<PathBuf> = cc.storage.and_then(|s| s.get_string("lib_dir")).map(PathBuf::from);
        if let Some(o) = cc.storage.and_then(|s| s.get_string("optionen")).and_then(|j| serde_json::from_str(&j).ok()) {
            self.opt = o;
        }
        self.apply_start_options();
        // Startprojekt (noch ohne Datei) bekommt wie „Datei › Neu“ die Projektvorgaben (Rand Links 20 mm)
        if self.path.is_none() {
            self.project.anlagen[0].set = self.opt.anlage();
            crate::rahmen::abgleichen(&mut self.project);
        }
        if let Some(st) = cc.storage {
            if let Some(f) = st.get_string("favs") {
                for l in f.lines() {
                    if let Some((k, v)) = l.rsplit_once('\t') {
                        self.fav_overrides.insert(k.to_string(), v == "1");
                    }
                }
            }
            if let Some(r) = st.get_string("recent") {
                self.recent = r.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).take(5).collect();
            }
        }
        self.load_library(saved.as_deref());
        if let Some(tb) = crate::extra::load_default_titleblock() {
            self.project.titleblock = tb;
        }
        if std::env::args().any(|a| a == "--dump-lib") {
            eprintln!("{}", self.lib_info);
            for d in &self.lib {
                if d.fav {
                    eprintln!("{} {:?} bmk={} pins={:?} title={:?}", d.name, d.group, d.bmk, d.pins, d.title);
                    for p in &d.prims {
                        if let symbols::Prim::TextBox(..) = p {
                            eprintln!("   {:?}", p);
                        }
                    }
                }
            }
        }
        let args: Vec<String> = std::env::args().collect();
        if let Some(f) = args.iter().position(|a| a == "--open").and_then(|i| args.get(i + 1)) {
            self.open_path(PathBuf::from(f), false);
            if let Some(t) = args.iter().position(|a| a == "--sheet").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<usize>().ok()) {
                let mut n = 0;
                'o: for (ai, an) in self.project.anlagen.iter().enumerate() {
                    for si in 0..an.sheets.len() {
                        if n == t {
                            self.tabs = vec![SheetRef { anlage: ai, sheet: si }];
                            self.active = 0;
                            break 'o;
                        }
                        n += 1;
                    }
                }
            }
        } else if let Some(f) = args.iter().skip(1).find(|a| !a.starts_with("--") && (a.ends_with(".epr") || a.ends_with(".epj"))) {
            self.open_path(PathBuf::from(f), false);
        }
        // Prüfhilfe für Bildschirmfotos: Eigenschaftendialog eines Objekts (Text oder BMK) bzw. des Blattes öffnen
        if let Some(label) = args.iter().position(|a| a == "--props").and_then(|i| args.get(i + 1)).cloned() {
            let hit = self.sheet().and_then(|s| {
                s.objects.iter().find(|o| match &o.kind {
                    Kind::Text { text, .. } => text == &label,
                    Kind::Symbol { bmk, .. } => bmk == &label,
                    _ => false,
                })
            }).map(|o| o.id);
            match (hit, label.as_str()) {
                (Some(id), _) => {
                    self.selection = vec![id];
                    self.edit_props(id);
                }
                (None, "Anlage") => self.open_anlage_props(self.cur().map(|r| r.anlage).unwrap_or(0)),
                (None, "Option") => self.open_optionen(),
                (None, "Projekt") => self.open_projekt_dialog(),
                (None, "Drucken") => self.open_druck_dialog(),
                (None, "Projektinfo") => self.dlg2 = Some(crate::dialoge::Dlg2::Info),
                (None, "Blatt") => {
                    if let Some(r) = self.cur() {
                        self.open_sheet_props(r);
                    }
                }
                _ => {}
            }
        }
        if let Some(f) = args.iter().position(|a| a == "--pdf").and_then(|i| args.get(i + 1)) {
            self.pdf_req = Some((PathBuf::from(f), false));
        }
        if let Some(n) = args.iter().position(|a| a == "--edit-symbol").and_then(|i| args.get(i + 1)) {
            if let Some(k) = self.lib.iter().find(|d| &d.name == n).map(|d| d.key.clone()) {
                self.open_symbol_editor(Some(k));
                if args.iter().any(|a| a == "--symeigen") {
                    self.open_sym_eigenschaften();
                }
            }
        }
        if std::env::args().any(|a| a == "--demo") {
            self.demo();
        }
        self
    }

    /// Beispielschaltung (nur mit --demo, zum Testen der Symboldarstellung).
    fn demo(&mut self) {
        let names = ["Sicherung", "Schließer", "Öffner", "Leistungskontakt", "Relais", "Lampe", "Motorschutz", "Motor"];
        let mut x = 30.0;
        for n in names {
            if let Some(k) = self.lib.iter().find(|d| d.name == n && (d.fav || d.haupt == 300)).map(|d| d.key.clone()) {
                self.place_symbol(k, P::new(x, 80.0));
                x += 32.0;
            }
        }
        let l = self.leitung;
        self.add_obj(Kind::Wire { pts: vec![P::new(15.0, 45.0), P::new(280.0, 45.0)], leitung: l, bez: vec![], opts: 0 });
        self.add_obj(Kind::Text { pos: P::new(14.0, 45.0), text: "L1".into(), size: 3.0, align: Align::Right, vertical: false, style: Default::default() });
        self.selection.clear();
        self.dirty = false;
    }

    /// Symbolbibliothek laden: Original-Datenbank, sonst eingebaute Ersatzsymbole.
    /// Gesperrte Datenbank alle 5 s erneut versuchen (das Original sperrt sie, solange es läuft).
    pub fn retry_library(&mut self, now: f64) {
        if !self.lib_stale || now < self.lib_retry {
            return;
        }
        self.lib_retry = now + 5.0;
        let dir = self.lib_dir.clone();
        for d in symbols::db_candidates(dir.as_deref()) {
            if symbols::load_db(&d).is_ok() {
                self.lib_stale = false;
                self.load_library(Some(&d));
                return;
            }
        }
    }

    pub fn load_library(&mut self, dir: Option<&std::path::Path>) {
        let cache = crate::config_dir().join("bibliothek_cache.json");
        let mut errors = vec![];
        let mut loaded = false;
        for d in symbols::db_candidates(dir) {
            match symbols::load_db(&d) {
                Ok(lib) => {
                    self.lib_info = format!("Bibliothek: {} Symbole aus {}", lib.len(), d.display());
                    if let Ok(s) = serde_json::to_string(&lib) {
                        let _ = std::fs::create_dir_all(cache.parent().unwrap());
                        let _ = std::fs::write(&cache, s);
                    }
                    self.lib = lib;
                    self.lib_dir = Some(d);
                    loaded = true;
                    break;
                }
                Err(e) => errors.push(format!("{}: {e}", d.display())),
            }
        }
        if !loaded {
            // Das Original läuft und sperrt die Datenbank: zuletzt gelesene Bibliothek verwenden
            match std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<Vec<SymDef>>(&s).ok()) {
                Some(mut lib) if !lib.is_empty() => {
                    // Zwischenspeicher älterer Versionen: Optionen/Alternativtexte aus den Originaldaten nachtragen;
                    // ganz ohne Originaldaten → später erneut aus der Datenbank lesen
                    let mut stale = false;
                    for d in lib.iter_mut().filter(|d| d.key.starts_with("db:")) {
                        if d.raw.is_empty() {
                            stale = true;
                        } else if d.flag_names.is_empty() && d.alt_sets.is_empty() {
                            let (alt, opts) = symbols::parse_lib_extras(&d.raw);
                            d.alt_sets = alt;
                            d.flag_names = opts;
                        }
                    }
                    self.lib_stale = stale || !errors.is_empty();
                    self.lib_info = format!("Bibliothek: {} Symbole (zwischengespeichert, Datenbank gesperrt – wird neu gelesen, sobald frei)", lib.len());
                    self.lib = lib;
                }
                _ => {
                    self.lib = symbols::builtin();
                    self.lib_info = if errors.is_empty() { "Original-Datenbank nicht gefunden – Ersatzsymbole".into() } else { format!("Datenbank nicht lesbar ({}) – Ersatzsymbole", errors.join("; ")) };
                }
            }
        }

        let extra: Vec<SymDef> = self.project.symdefs.iter().filter(|d| symbols::find(&self.lib, &d.key).is_none()).cloned().collect();
        self.lib.extend(extra);
        self.load_user_library();
    }

    /// Neues BMK aus der Vorlage des Symbols (mit Zähler, eindeutig im Projekt).
    pub fn new_bmk(&self, def: &SymDef) -> String {
        let sheet_no = self.sheet().map(|s| s.number).unwrap_or(1);
        symbols::freie_bmk(&def.bmk, sheet_no, &symbols::vergebene_bmk(&self.project))
    }

    /// Gewählte Rasterweite (mm), auch bei ausgeschaltetem Rasterfang.
    pub fn raster(&self) -> f32 {
        RASTER[self.raster_idx.min(RASTER.len() - 1)].1
    }

    /// Stichlänge an Anschlüssen beim Führen von Leitungen: ein halbes Raster, bei feinem Raster (unter 5 mm) aber
    /// mindestens 2,5 mm und ein Vielfaches des Rasters (Nachzeichnen 24.11.2026: bei Raster 1 mm entstanden
    /// 0,5-mm-Endstücke, und eine Leitung in den Nachbarpol lief auf Anschlusshöhe über den fremden Pol).
    pub fn stich(&self) -> f32 {
        let r = self.raster();
        if r >= 5.0 {
            r / 2.0
        } else {
            r * (2.5 / r).ceil()
        }
    }

    /// Wirksamer Rasterfang (mm): 0 = aus (Gitterknopf aus), dann nur Rundung auf 0,1 mm.
    pub fn fang(&self) -> f32 {
        if self.grid {
            self.raster()
        } else {
            0.0
        }
    }

    pub fn cur(&self) -> Option<SheetRef> {
        if self.sym_edit.is_some() {
            return Some(SheetRef { anlage: usize::MAX, sheet: 0 });
        }
        self.tabs.get(self.active).copied()
    }

    pub fn sheet(&self) -> Option<&Sheet> {
        if let Some(e) = &self.sym_edit {
            return Some(&e.sheet);
        }
        self.cur().and_then(|r| self.project.sheet(r))
    }

    pub fn checkpoint(&mut self) {
        self.rueckkehr_gilt = false;
        self.undo.push(self.project.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        self.project.geaendert = today();
        self.mark_sheet_changed();
    }

    /// Hat der letzte Rückgängig-Schritt auf dem aktuellen Blatt nichts verändert (z. B. ein Ziehen, das der
    /// Netz-Wächter ganz abgewiesen hat), wird er entfernt – sonst nähme Strg+Z „nichts“ zurück (Nachzeichnen
    /// 24.11.2026, Befund 82).
    pub fn leeren_schritt_entfernen(&mut self) {
        let (Some(r), Some(s)) = (self.cur(), self.sheet()) else { return };
        if self.undo.last().and_then(|p| p.sheet(r)).is_some_and(|alt| alt.objects == s.objects) {
            self.undo.pop();
        }
    }

    pub fn do_undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.project, p));
            self.selection.clear();
            self.fix_tabs();
        }
    }

    pub fn do_redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.project, p));
            self.selection.clear();
            self.fix_tabs();
        }
    }

    pub fn fix_tabs(&mut self) {
        let prj = &self.project;
        self.tabs.retain(|r| prj.sheet(*r).is_some());
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
    }

    pub fn snap(&self, p: P) -> P {
        let r = self.fang();
        // Umschalt gehalten: kein Rasterfang (wie im Original), nur auf 0,1 mm runden
        if r <= 0.0 || self.no_snap {
            return P::new((p.x * 10.0).round() / 10.0, (p.y * 10.0).round() / 10.0);
        }
        // absolute Rasterpunkte ab dem Ursprung des Blattes (Rand oder Blattecke) wie im Original
        crate::raster::fang(p, r, self.raster_ursprung())
    }

    /// Nächster Anschlusspunkt (Symbol, Klemme, Leitungsende) in Reichweite.
    pub fn pin_near(&self, p: P, tol: f32, exclude: &[u64]) -> Option<P> {
        let sheet = self.sheet()?;
        let mut best: Option<(f32, P)> = None;
        for o in &sheet.objects {
            if exclude.contains(&o.id) {
                continue;
            }
            for q in canvas::obj_pins(o, &self.lib) {
                let d = q.dist(p);
                if d <= tol && best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, q));
                }
            }
        }
        // Klemmstelle = ein Anschluss: die Leitungsenden an ihrem Kreis (0,8 mm vor dem Mittelpunkt) fangen auf
        // die Klemme selbst (Nutzer-Befund Runde 11: drei Fangpunkte je Klemme)
        best.map(|b| crate::klemme_leitung::klemme_zu(sheet, b.1).unwrap_or(b.1))
    }

    pub fn with_sheet<R>(&mut self, f: impl FnOnce(&mut Sheet, &mut u64) -> R) -> Option<R> {
        if let Some(e) = &mut self.sym_edit {
            let mut next = self.project.next_id;
            let out = f(&mut e.sheet, &mut next);
            self.project.next_id = next;
            return Some(out);
        }
        let r = self.cur()?;
        let mut next = self.project.next_id;
        let out = self.project.sheet_mut(r).map(|s| f(s, &mut next));
        self.project.next_id = next;
        out
    }

    pub fn add_obj(&mut self, kind: Kind) -> u64 {
        self.checkpoint();
        let pen = self.pen;
        let id = self.project.alloc_id();
        self.with_sheet(|s, _| s.objects.push(Obj { id, kind, pen, fixed: false, group: None, raw: None }));
        self.edit_tag_new(id);
        self.selection = vec![id];
        id
    }

    /// Höchste Nummer + 1 der Klemmleiste `leiste` der aktuellen Anlage (Klemmleisten gehören zu einer Anlage; W2
    /// hat „-X2“ in drei Anlagen). Das Werkzeug Klemmstelle nimmt den Zähler der Leiste, siehe klemme_leitung.rs.
    pub fn next_terminal(&self, leiste: &str) -> u32 {
        let mut max = 0;
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        if let Some(a) = self.project.anlagen.get(a) {
            for s in &a.sheets {
                for o in &s.objects {
                    if let Kind::Terminal { leiste: l, nr, .. } = &o.kind {
                        if l == leiste {
                            max = max.max(*nr);
                        }
                    }
                }
            }
        }
        max + 1
    }

    pub fn klemmleisten(&self) -> Vec<String> {
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        self.project.anlagen.get(a).map(|a| a.klemmleisten.clone()).unwrap_or_default()
    }

    /// Wirksame Strompfadausrichtung: Schalter der Werkzeugleiste, mit gehaltener Taste S umgekehrt.
    pub fn path_align_eff(&self) -> bool {
        self.path_align ^ self.hold_s
    }

    /// Wirksame automatische Verbindungen: Schalter der Werkzeugleiste, mit gehaltener Taste A umgekehrt.
    pub fn auto_conn_eff(&self) -> bool {
        self.auto_conn ^ self.hold_a
    }

    /// Symbol platzieren: erster Anschluss liegt auf dem Raster. Wie im Original wird eine Leitung, auf die das
    /// Symbol gesetzt wird, immer aufgetrennt; mit automatischen Verbindungen werden freie Anschlüsse verbunden.
    pub fn place_symbol(&mut self, key: String, at: P) {
        self.place_symbol_opt(key, at, 0);
    }

    /// Wie `place_symbol`, mit Bild (Bildindex/Seite), z. B. aus dem Variantenrad.
    pub fn place_symbol_opt(&mut self, key: String, at: P, option: u16) {
        let Some(def) = symbols::find(&self.lib, &key).cloned() else { return };
        let option = if (option as usize) < def.options.len() { option } else { 0 };
        // Drehung per Leertaste beim Platzieren (blatt_tasten.rs)
        let rot = self.bt.drehung % 4;
        let pin0 = def.pins.first().map(|q| sym_xform(*q, P::default(), rot, false, false)).unwrap_or_default();
        let mut pos = self.snap(at.add(pin0)).sub(pin0);
        pos = self.align_to_path(pos);
        let bmk = self.new_bmk(&def);
        if !self.project.symdefs.iter().any(|d| d.key == def.key) {
            self.project.symdefs.push(def.clone());
        }
        // BMK-Lage wie das Original sie speichert (VSY Feld 10/11, Ausrichtung Feld 12): rechtsbündig am Titelanker
        // der Bibliothek bzw. links neben dem Symbol. Ohne eigene Lage übernahm der Schreiber die Werte eines
        // Vorlageblocks (hochkant, im Symbol) – das Original zeigte die BMK dann winzig und senkrecht.
        let (title, title_align) = symbol_title_rot(&def, rot, option);
        let ext = SymExt { querverweise: def.frg.0, sicherung: def.frg.1, title_align, ..Default::default() };
        let id = self.add_obj(Kind::Symbol { key: def.key.clone(), pos, rot, mirror_h: false, mirror_v: false, bmk, value: String::new(), scale: 1.0, option, title: Some((title, false)), flags: 0, ext });
        // nur die Anschlüsse des gewählten Bildes (ohne Zusatzoptionen)
        let wpins: Vec<P> = symbols::anschluss_lagen(&def, option, 0).into_iter().map(|(_, q)| sym_xform(q, pos, rot, false, false)).collect();
        self.insert_into_wire(&wpins);
        // durchquert danach noch eine Leitung das Symbol ohne passenden Anschluss: Meldung des Originals (0x47bf14)
        if self.sheet().is_some_and(|s| crate::symbol_in_leitung::nicht_integrierbar(s, &self.lib, id)) {
            crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_X, crate::symbol_in_leitung::MELDUNG);
        }
        if self.auto_conn_eff() {
            // gehört zum selben Rückgängig-Schritt wie das Setzen (add_obj hat ihn schon angelegt)
            self.auto_connect_nocp(&[id]);
        }
    }

    /// Strompfadausrichtung wie im Original: die Lage (Bezugspunkt) des Symbols waagerecht auf die Mitte des
    /// nächsten Strompfades setzen (Teilung wie die Spaltenleiste, ohne Abstandsgrenze).
    pub fn align_to_path(&self, pos: P) -> P {
        if !self.path_align_eff() || self.sym_edit.is_some() {
            return pos;
        }
        match self.sheet().and_then(|s| s.path_center_x(pos.x)) {
            Some(x) => P::new(x, pos.y),
            None => pos,
        }
    }

    /// Automatische Verbindungen für die Objekte `ids` mit eigenem Rückgängig-Schritt (nur wenn etwas entsteht).
    pub fn auto_connect(&mut self, ids: &[u64]) {
        let Some(sheet) = self.sheet() else { return };
        if auto_wires(sheet, &self.lib, ids, AUTO_CONN_MAX).is_empty() {
            return;
        }
        self.checkpoint();
        self.auto_connect_nocp(ids);
    }

    /// Wie `auto_connect`, ohne Rückgängig-Schritt (Aufrufer hat ihn schon angelegt). Liefert die Zahl der Leitungen.
    pub fn auto_connect_nocp(&mut self, ids: &[u64]) -> usize {
        let Some(sheet) = self.sheet() else { return 0 };
        let wires = auto_wires(sheet, &self.lib, ids, AUTO_CONN_MAX);
        let n = wires.len();
        if n == 0 {
            return 0;
        }
        let leitung = self.leitung;
        self.with_sheet(|s, next| {
            for (a, b) in wires {
                let id = *next;
                *next += 1;
                s.objects.push(Obj { id, kind: Kind::Wire { pts: vec![a, b], leitung, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
            }
        });
        self.dirty = true;
        n
    }

    /// Liegen die beiden äußeren Symbolanschlüsse auf derselben Leitung, wird die Leitung dazwischen aufgetrennt.
    pub fn insert_into_wire(&mut self, pins: &[P]) {
        if pins.len() < 2 {
            return;
        }
        let (a, b) = (pins[0], pins[pins.len() - 1]);
        self.with_sheet(|s, next| {
            let mut new_objs = vec![];
            let mut remove = vec![];
            for o in &s.objects {
                if let Kind::Wire { pts, leitung, .. } = &o.kind {
                    for (i, w) in pts.windows(2).enumerate() {
                        if seg_dist(a, w[0], w[1]) < 0.05 && seg_dist(b, w[0], w[1]) < 0.05 {
                            let (n, f) = if w[0].dist(a) <= w[0].dist(b) { (a, b) } else { (b, a) };
                            let mut p1: Vec<P> = pts[..=i].to_vec();
                            p1.push(n);
                            let mut p2: Vec<P> = vec![f];
                            p2.extend_from_slice(&pts[i + 1..]);
                            remove.push(o.id);
                            for mut part in [p1, p2] {
                                part.dedup_by(|x, y| x.dist(*y) < 0.01);
                                if part.len() >= 2 {
                                    let id = *next;
                                    *next += 1;
                                    new_objs.push(Obj { id, kind: Kind::Wire { pts: part, leitung: *leitung, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None });
                                }
                            }
                            break;
                        }
                    }
                }
            }
            s.objects.retain(|o| !remove.contains(&o.id));
            s.objects.extend(new_objs);
        });
    }

    /// Rahmen zur Größenänderung um die Markierung (einzelne Zeichenelemente behalten ihre Eckgriffe; ein einzelnes
    /// Symbol bzw. ein einzelner Text bekommt vier Eckgriffe zum proportionalen Skalieren, siehe groesse.rs).
    /// Umriss (min, max) in mm.
    pub fn sel_frame(&self) -> Option<(P, P)> {
        if !self.size_frame || self.tool != Tool::Select || self.placing.is_some() {
            return None;
        }
        let objs = self.selected_objs();
        // mit markierten Leitungen keine Skalierungsecken (Nutzerwunsch 23.11.2026: Leitungen werden nicht skaliert,
        // die Ecken stören beim Greifen von Leitungen und Anschlüssen)
        if objs.iter().any(|o| matches!(o.kind, Kind::Wire { .. })) {
            return None;
        }
        crate::groesse::rahmen(&objs, &self.lib)
    }

    pub fn selected_objs(&self) -> Vec<Obj> {
        self.sheet().map(|s| s.objects.iter().filter(|o| self.selection.contains(&o.id)).cloned().collect()).unwrap_or_default()
    }

    pub fn expand_groups(&mut self) {
        let Some(s) = self.sheet() else { return };
        let groups: HashSet<u64> = s.objects.iter().filter(|o| self.selection.contains(&o.id)).filter_map(|o| o.group).collect();
        if groups.is_empty() {
            return;
        }
        let extra: Vec<u64> = s.objects.iter().filter(|o| o.group.map(|g| groups.contains(&g)).unwrap_or(false)).map(|o| o.id).collect();
        for id in extra {
            if !self.selection.contains(&id) {
                self.selection.push(id);
            }
        }
    }

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        let sel = self.selection.clone();
        self.with_sheet(|s, _| s.objects.retain(|o| !sel.contains(&o.id)));
        self.selection.clear();
    }

    pub fn copy(&mut self) {
        // Kontaktspiegel kopiert das Original nicht mit (kontaktspiegel.rs)
        let objs: Vec<Obj> = self.selected_objs().into_iter().filter(|o| !matches!(o.kind, Kind::Spiegel { .. })).collect();
        if !objs.is_empty() {
            self.ablage_text = Some(format!("Elektroplanung: {} Objekt(e) kopiert", objs.len()));
            self.clipboard = objs;
        }
    }

    pub fn paste(&mut self) {
        if self.clipboard.is_empty() || self.cur().is_none() {
            return;
        }
        self.checkpoint();
        // Versatz ein Vielfaches des Rasters, mindestens 10 mm: bei feinem Raster (2,5 mm) verdeckte die Kopie
        // sonst das Original fast ganz (Maustest Runde 11)
        let d = if self.fang() > 0.0 { self.fang() * (10.0 / self.fang()).ceil().max(1.0) } else { 10.0 };
        let clip = self.clipboard.clone();
        // Versetzt nur, wenn die Kopie sonst genau auf vorhandenen Objekten läge (auf einem anderen, leeren Blatt
        // bleibt sie an derselben Stelle), und nie über den Zeichnungsrahmen hinaus (Nachzeichnen 24.11.2026:
        // Grundriss auf ein neues Blatt eingefügt ragte 10 mm über den Rahmen)
        let d = {
            let lib = self.lib.clone();
            let lage = |o: &Obj| canvas::obj_bounds(o, &lib);
            let nah = |a: P, b: P| (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3;
            let s = self.sheet().expect("Blatt");
            let liegt_auf = clip.iter().any(|c| {
                let (ca, cb) = lage(c);
                s.objects.iter().any(|o| {
                    std::mem::discriminant(&o.kind) == std::mem::discriminant(&c.kind) && {
                        let (oa, ob) = lage(o);
                        nah(oa, ca) && nah(ob, cb)
                    }
                })
            });
            if liegt_auf {
                let ids: Vec<u64> = clip.iter().map(|o| o.id).collect();
                let v = crate::blatt_grenze::begrenzen(s, &lib, &clip, &ids, P::default(), P::new(d, d));
                if v.x.abs() < 1e-3 && v.y.abs() < 1e-3 {
                    crate::blatt_grenze::begrenzen(s, &lib, &clip, &ids, P::default(), P::new(-d, -d))
                } else {
                    v
                }
            } else {
                P::default()
            }
        };
        let mut new_sel = vec![];
        let mut gmap: HashMap<u64, u64> = HashMap::new();
        for mut o in clip {
            o.id = self.project.alloc_id();
            // ohne Originalblock: beim Speichern neuer Block mit neuer Kennung (sonst stünden OBJ/VBG/VSY-Kennungen
            // doppelt in der Datei, wie beim Rechts-Ziehen)
            o.raw = None;
            if let Some(g) = o.group {
                let ng = *gmap.entry(g).or_insert_with(|| 1_000_000 + o.id);
                o.group = Some(ng);
            }
            o.kind.translate(d);
            let new_bmk = if let Kind::Symbol { key, .. } = &o.kind {
                symbols::find(&self.lib, key).map(|def| self.new_bmk(def))
            } else {
                None
            };
            if let (Kind::Symbol { bmk, .. }, Some(nb)) = (&mut o.kind, new_bmk) {
                *bmk = nb;
            }
            // Klemmstelle: nächste freie Nummer ihrer Leiste (Maustest Runde 12: Kopie hieß wieder „X1.1“)
            if let Kind::Terminal { leiste, nr, .. } = &o.kind {
                let n = self.next_terminal(leiste);
                if n != *nr {
                    if let Kind::Terminal { nr, .. } = &mut o.kind {
                        *nr = n;
                    }
                }
            }
            new_sel.push(o.id);
            self.with_sheet(|s, _| s.objects.push(o));
        }
        self.selection = new_sel;
        for o in &mut self.clipboard {
            o.kind.translate(d);
        }
    }

    /// op: 1 drehen, 2 vertikal spiegeln, 4 horizontal spiegeln
    pub fn transform_selection(&mut self, op: u8) {
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        let sel = self.selection.clone();
        let lib = self.lib.clone();
        // angeschlossene Leitungen folgen den gedrehten/gespiegelten Anschlüssen (C2, netz_folgen.rs)
        self.netz_merken();
        self.with_sheet(|s, _| {
            // Original (ToolButton17Click 0x465994): gemeinsame Mitte aller markierten Objekte (drehen_mitte.rs);
            // Einfügepunkte von Symbolen, Texten, Klemmstellen und Verweisen wandern dann mit
            let gemeinsam = crate::drehen_mitte::gemeinsame_mitte(s.objects.iter().filter(|o| sel.contains(&o.id)), &lib);
            for o in s.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
                let (mn, mx) = canvas::obj_bounds(o, &lib);
                let c = gemeinsam.unwrap_or_else(|| P::new(((mn.x + mx.x) / 2.0 / 0.5).round() * 0.5, ((mn.y + mx.y) / 2.0 / 0.5).round() * 0.5));
                let f = |p: &mut P| match op {
                    1 => {
                        let (dx, dy) = (p.x - c.x, p.y - c.y);
                        *p = P::new(c.x - dy, c.y + dx);
                    }
                    2 => p.x = 2.0 * c.x - p.x,
                    _ => p.y = 2.0 * c.y - p.y,
                };
                // gemeinsame Mitte: Einfügepunkte mitdrehen bzw. -spiegeln (Verweis samt Textlage)
                if gemeinsam.is_some() {
                    match &mut o.kind {
                        Kind::Symbol { pos, .. } | Kind::Text { pos, .. } | Kind::Terminal { pos, .. } => f(pos),
                        Kind::Verweis { pos, text_pos, .. } => {
                            let alt = *pos;
                            f(pos);
                            *text_pos = text_pos.add(pos.sub(alt));
                        }
                        _ => {}
                    }
                }
                match &mut o.kind {
                    Kind::Symbol { rot, mirror_h, mirror_v, ext, .. } => {
                        let old = (*rot, *mirror_h, *mirror_v);
                        match op {
                            1 => *rot = (*rot + 1) % 4,
                            2 => *mirror_h = !*mirror_h,
                            _ => *mirror_v = !*mirror_v,
                        }
                        ext.bounds = ext.bounds.map(|b| retransform_bounds(b, old, (*rot, *mirror_h, *mirror_v)));
                    }
                    Kind::Text { vertical, .. } => {
                        if op == 1 {
                            *vertical = !*vertical
                        }
                    }
                    Kind::Terminal { .. } | Kind::Busbar { .. } | Kind::Spiegel { .. } => {}
                    Kind::Verweis { .. } => crate::verweise::drehen(&mut o.kind, op),
                    Kind::Line { a, b }
                    | Kind::Rect { a, b }
                    | Kind::Triangle { a, b }
                    | Kind::Circle { a, b }
                    | Kind::Table { a, b, .. }
                    | Kind::Image { a, b, .. } => {
                        f(a);
                        f(b)
                    }
                    Kind::Curve { a, c1, c2, b } => {
                        f(a);
                        f(c1);
                        f(c2);
                        f(b)
                    }
                    Kind::Wire { pts, .. } | Kind::Poly { pts } => pts.iter_mut().for_each(f),
                }
            }
        });
        self.netz_nachfuehren(&sel, false);
        self.aufraeumen();
        // Netz-Wächter: Drehen/Spiegeln, das fremde Netze kurzschließen würde, zurücknehmen (netz_waechter.rs)
        let ok = !self.opt.netz_waechter || self.sheet().is_none_or(|s| crate::netz_waechter::pruefen(&self.netz_vorher, s, &self.lib, &sel).is_none());
        if !ok {
            let grund = self.sheet().and_then(|s| crate::netz_waechter::pruefen(&self.netz_vorher, s, &self.lib, &sel)).unwrap_or_default();
            self.do_undo();
            self.redo.pop();
            self.selection = sel;
            self.info = format!("Nicht gedreht/gespiegelt: {grund}");
            self.netz_abgewiesen += 1;
        }
    }

    pub fn group_selection(&mut self, on: bool) {
        if self.selection.len() < if on { 2 } else { 1 } {
            return;
        }
        self.checkpoint();
        let sel = self.selection.clone();
        let gid = self.project.alloc_id() + 1_000_000;
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
                o.group = if on { Some(gid) } else { None };
            }
        });
    }

    pub fn set_fixed(&mut self, v: bool) {
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        let sel = self.selection.clone();
        self.with_sheet(|s, _| s.objects.iter_mut().filter(|o| sel.contains(&o.id)).for_each(|o| o.fixed = v));
    }

    pub fn set_text_align(&mut self, al: Option<Align>, vert: Option<bool>) {
        let sel = self.selection.clone();
        self.checkpoint();
        self.with_sheet(|s, _| {
            for o in s.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
                if let Kind::Text { align, vertical, .. } = &mut o.kind {
                    if let Some(a) = al {
                        *align = a;
                    }
                    if let Some(v) = vert {
                        *vertical = v;
                    }
                }
            }
        });
    }

    pub fn new_sheet(&mut self) {
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        if self.project.anlagen.get(a).is_none() {
            crate::meldungen::hinweis(crate::meldungen::T_ACHTUNG, crate::meldungen::KEINE_ANLAGE_FUER_BLATT);
            return;
        }
        self.checkpoint();
        let Some(anl) = self.project.anlagen.get_mut(a) else { return };
        let n = anl.sheets.iter().map(|s| s.number).max().unwrap_or(0) + 1;
        // Ausrichtung nach den Projektvorgaben (Original 0x403ce4: Blatt +0x80 = Option „Hochformat“)
        let mut sh = Sheet::new(n);
        // Ausrichtung nach den Projektvorgaben; Strompfade „wie Anlage“ (Nachzeichnen 24.11.2026, S8: neues Blatt mit
        // 8 statt 20 Strompfaden bis zum Neuladen)
        sh.portrait = self.opt.hochformat;
        sh.col_auto = true;
        sh.columns = Some(anl.set.pfade).filter(|c| (1..=40).contains(c)).map(|c| c as u32).unwrap_or(8);
        anl.sheets.push(sh);
        crate::rahmen::abgleichen_anlage(anl);
        let r = SheetRef { anlage: a, sheet: anl.sheets.len() - 1 };
        self.open_sheet(r);
    }

    /// Hinzufügen › Blatt wie im Original: neues Blatt anlegen und gleich seinen Eigenschaftendialog öffnen
    /// (Art, Bezeichnung, Nummer, Ausrichtung); Abbruch nimmt das Blatt wieder weg (Maus-Befund M18).
    pub fn new_sheet_dialog(&mut self) {
        let vorher = self.project.anlagen.iter().map(|a| a.sheets.len()).sum::<usize>();
        self.new_sheet();
        if self.project.anlagen.iter().map(|a| a.sheets.len()).sum::<usize>() == vorher {
            return;
        }
        if let Some(r) = self.cur() {
            self.open_sheet_props_dialog(r);
            if let Some(crate::props::Props::Sheet(s)) = self.props.as_mut() {
                s.neu = true;
                // Vorgaben des Dialogs wie Blatteinfgen1Click 0x461960 (blatt_neu.rs): Strompfade „-1“, Querformat
                s.strompfade = "-1".into();
                // Ausrichtung: das Original schlägt Querformat vor; der Nachbau die des vorigen Blattes der Anlage
                // (Nachzeichnen 24.11.2026, B4/S8: neues Blatt zwischen lauter Hochformat-Blättern kam quer)
                s.portrait = r.sheet.checked_sub(1).and_then(|i| self.project.anlagen.get(r.anlage)?.sheets.get(i)).map(|v| v.portrait).unwrap_or(false);
                s.vorgabe = [s.name.clone(), s.nummer.clone(), s.subindex.clone()];
                s.vorher = (s.art, s.anlage_wahl);
            }
        }
    }

    pub fn new_anlage(&mut self) {
        self.checkpoint();
        let n = self.project.anlagen.len() + 1;
        let mut sheet = Sheet::new(1);
        sheet.portrait = self.opt.hochformat;
        sheet.columns = self.opt.pfade.max(1) as u32;
        sheet.col_auto = true;
        self.project.anlagen.push(Anlage { name: format!("Anlage {n}"), sheets: vec![sheet], sammelschienen: vec![], klemmleisten: crate::klemmen::erste_leiste().0, klemmen: crate::klemmen::erste_leiste().1, schienen: Default::default(), set: self.opt.anlage(), raw: None });
        let a = self.project.anlagen.len() - 1;
        crate::rahmen::abgleichen_anlage(&mut self.project.anlagen[a]);
        self.tree_open.insert(format!("prj/a{a}"));
        self.tree_open.insert(format!("prj/a{a}/bl"));
        self.open_sheet(SheetRef { anlage: a, sheet: 0 });
    }

    pub fn open_sheet(&mut self, r: SheetRef) {
        if let Some(i) = self.tabs.iter().position(|t| *t == r) {
            self.active = i;
        } else {
            // nach Anlage und Blatt einsortieren (Nachzeichnen 24.11.2026, B5/S1: neues Blatt der Anlage 1 stand
            // hinter den Blättern der Anlage 2, bis zum Neuladen)
            let i = self.tabs.iter().position(|t| (t.anlage, t.sheet) > (r.anlage, r.sheet)).unwrap_or(self.tabs.len());
            self.tabs.insert(i, r);
            self.active = i;
        }
        self.selection.clear();
    }

    pub fn new_project(&mut self) {
        self.sym_edit = None;
        // Statuszeile des vorigen Projekts nicht stehen lassen („Ladestatus: 39 Blätter geladen“)
        self.info = "Neues Projekt".into();
        self.project = Project::new_default();
        // Anlage 1 bekommt wie im Original die Projektvorgaben (Ränder, Strompfade, Schriftgrößen)
        self.project.anlagen[0].set = self.opt.anlage();
        crate::rahmen::abgleichen(&mut self.project);
        if let Some(tb) = crate::optionen::schriftfeld_neues_projekt(&self.opt) {
            self.project.titleblock = tb;
        }
        // Standardschrift der Projektvorgaben (Original 0x4055d8: leer = Times New Roman)
        if self.opt.schrift != "Times New Roman" {
            self.project.schrift = self.opt.schrift.clone();
        }
        self.path = None;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        self.bt = Default::default();
        self.tabs = vec![SheetRef { anlage: 0, sheet: 0 }];
        self.active = 0;
        self.selection.clear();
        self.pen = 0;
        self.leitung = 0;
        self.klemme = 0;
        self.view.user = false;
        self.view.last_size = Vec2::ZERO;
        self.sheet_views.clear();
        self.view_sheet = None;
    }

    /// Blatt innerhalb seiner Anlage verschieben (Ziehen im Baum oder an den Reitern); Reiter und gemerkte
    /// Ansichten folgen dem Blatt.
    pub fn move_sheet(&mut self, anlage: usize, from: usize, to: usize) {
        let Some(a) = self.project.anlagen.get_mut(anlage) else { return };
        if from == to || from >= a.sheets.len() || to >= a.sheets.len() {
            return;
        }
        self.checkpoint();
        let a = &mut self.project.anlagen[anlage];
        let s = a.sheets.remove(from);
        a.sheets.insert(to, s);
        let map = |i: usize| -> usize {
            if i == from {
                to
            } else if from < to && i > from && i <= to {
                i - 1
            } else if to < from && i >= to && i < from {
                i + 1
            } else {
                i
            }
        };
        for t in &mut self.tabs {
            if t.anlage == anlage {
                t.sheet = map(t.sheet);
            }
        }
        // Reiter der Anlage in die neue Blattreihenfolge bringen (ihre Plätze in der Reiterleiste bleiben)
        let active = self.tabs.get(self.active).copied();
        let slots: Vec<usize> = (0..self.tabs.len()).filter(|i| self.tabs[*i].anlage == anlage).collect();
        let mut own: Vec<SheetRef> = slots.iter().map(|i| self.tabs[*i]).collect();
        own.sort_by_key(|t| t.sheet);
        for (slot, t) in slots.into_iter().zip(own) {
            self.tabs[slot] = t;
        }
        if let Some(a) = active {
            self.active = self.tabs.iter().position(|t| *t == a).unwrap_or(0);
        }
        let views: Vec<(SheetRef, View)> = self.sheet_views.drain().collect();
        for (mut r, v) in views {
            if r.anlage == anlage {
                r.sheet = map(r.sheet);
            }
            self.sheet_views.insert(r, v);
        }
        if let Some(r) = self.view_sheet.as_mut() {
            if r.anlage == anlage {
                r.sheet = map(r.sheet);
            }
        }
        self.dirty = true;
    }

    /// Aktion ausführen; bei ungespeicherten Änderungen zuerst fragen wie das Original ("Geänderte Daten Speichern?").
    pub fn guarded(&mut self, p: Pending) {
        // Neu: offene Editoren zuerst abschließen wie das Original (meldungen.rs)
        if matches!(p, Pending::New | Pending::Quit) && !self.editoren_abschliessen() {
            return;
        }
        if self.dirty {
            self.ask_save = Some(p);
        } else {
            self.run_pending(p);
        }
    }

    pub fn run_pending(&mut self, p: Pending) {
        match p {
            Pending::New => self.new_project(),
            Pending::Open => self.open(false),
            Pending::OpenPath(path) => self.open_path(path, false),
            Pending::Quit => self.quit_ok = true,
        }
    }

    pub fn new_leitung(&mut self) {
        self.checkpoint();
        // Vorgaben und Name „Leitung <Kennung>“ wie im Original (neu_anlegen.rs)
        let k = crate::neu_anlegen::naechste_kennung(&self.project);
        self.project.leitungen.push(crate::neu_anlegen::leitung_vorgabe(k));
        self.leitung = self.project.leitungen.len() - 1;
    }

    pub fn new_potential(&mut self) {
        self.checkpoint();
        // Name „Potential <Kennung>“ wie im Original (0x4052c4)
        let n = crate::neu_anlegen::naechste_kennung(&self.project);
        self.project.potentiale.push(format!("Potential {n}"));
        self.info = format!("Potential {n} angelegt");
    }

    pub fn new_pen(&mut self) {
        // im Symboleditor: neuer Stift des Symbols
        if let Some(e) = &mut self.sym_edit {
            let nr = crate::symbol_stifte::neue_nummer(&e.pens);
            e.pens.push((nr, Stift { name: format!("Stift {nr}"), style: 0, color: [0, 0, 160], width: 0.25 }));
            self.pen = nr as usize;
            return;
        }
        self.checkpoint();
        let n = self.project.stifte.len() + 1;
        self.project.stifte.push(Stift { name: format!("Stift {n}"), style: 0, color: [0, 0, 160], width: 0.25 });
        self.pen = self.project.stifte.len() - 1;
    }

    pub fn new_klemmleiste(&mut self) {
        let a = self.cur().map(|r| r.anlage).unwrap_or(0);
        self.checkpoint();
        // mit Kennung (KLM Feld 13), damit Klemmstellen und Klemmplan-Blätter sie im .epr finden
        if let Some(i) = crate::klemmen::new_leiste(&mut self.project, a) {
            self.klemme = i;
            self.tree_open.insert(format!("prj/a{a}/kl"));
        }
    }

    /// Neue Sammelschiene (Hinzufügen-Menü, Werkzeugleiste) wie im Original (Gegenprobe Wine 22.11.2026): nur im
    /// Projektbaum anlegen (Name „Schiene n“) und ihren Eigenschaftendialog öffnen; aufs Blatt kommt sie durch
    /// Ziehen aus dem Baum (schiene_platzieren.rs). Bisher entstand eine Leitung mit Text auf dem Blatt.
    pub fn new_busbar(&mut self) {
        let Some(a) = self.cur().map(|r| r.anlage) else { return };
        self.checkpoint();
        let Some(anl) = self.project.anlagen.get_mut(a) else { return };
        // Vorgaben des Original-Dialogs: Name „Schiene n“, Kürzel „L<n>“ (Schlüssel, TIT), keine Zuordnung,
        // Querverweise an, Pfeile aus
        let name = crate::schiene_neu::anlegen(anl);
        self.tree_open.insert(format!("prj/a{a}/ss"));
        // Original legt die Schiene erst bei OK an: Abbrechen nimmt sie wieder heraus
        self.neu_merken();
        self.open_schiene_dialog(a, &name);
    }

    pub fn save_as(&mut self) {
        // Vorschlag: Dateiname bzw. Projektname, Ordner der Datei bzw. des zuletzt geöffneten Projekts (Nachzeichnen
        // 24.11.2026, S9: „Projekt.epr“ im zuletzt benutzten Systemordner – beim Nutzer der Ordner der Originalpläne)
        let stem = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| dateiname_aus(&self.project.name));
        let ordner = self.path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).or_else(|| self.recent.first().and_then(|r| std::path::Path::new(r).parent().map(|d| d.to_path_buf()))).filter(|d| d.is_dir());
        let mut dlg = rfd::FileDialog::new();
        if let Some(d) = ordner {
            dlg = dlg.set_directory(d);
        }
        let dlg = dlg
            // Filtertext wie im Original (0x416cfc: „Elektroplanungs Projekt|*.epr“), JSON als Erweiterung
            .add_filter("Elektroplanungs Projekt", &["epr"])
            .add_filter("Elektroplanung Projekt (JSON)", &["epj"])
            .set_file_name(format!("{stem}.epr"));
        if let Some(p) = dlg.save_file() {
            self.path = Some(p);
            self.save();
        }
    }

    pub fn save(&mut self) {
        let Some(p) = self.path.clone() else {
            self.save_as();
            return;
        };
        let is_epr = crate::vorlage::is_epr_format(&p);
        let mut skipped = vec![];
        let res = if is_epr {
            // Vor dem ersten Überschreiben einer Originaldatei eine Sicherung anlegen
            let bak = p.with_extension("epr.bak");
            let backup = (self.opt.backup && p.exists() && !bak.exists()).then_some(bak.as_path());
            skipped = crate::epr_write::unsupported(&self.project);
            // Projekt-/Anlagenangaben, deren Ablage im Originalformat unbekannt ist
            skipped.extend(crate::epr_projekt::nicht_gespeichert(&self.project));
            // wie das Original: über eine temporäre Datei; ohne Sicherung wird nicht gespeichert (meldungen.rs)
            match crate::epr_write::to_bytes(&self.project) {
                Ok(d) => {
                    if let Err(m) = crate::meldungen::speichern_ueber_temp(&p, &d, backup) {
                        crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, &m);
                        return;
                    }
                    Ok(())
                }
                Err(_) => {
                    crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, crate::meldungen::S_DATENFEHLER);
                    return;
                }
            }
        } else {
            match serde_json::to_string_pretty(&self.project) {
                Ok(s) => {
                    if let Err(m) = crate::meldungen::speichern_ueber_temp(&p, s.as_bytes(), None) {
                        crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, &m);
                        return;
                    }
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        };
        match res {
            Ok(()) => {
                self.dirty = false;
                self.raster_merken();
                self.info = format!("Gespeichert: {}", p.display());
                if !skipped.is_empty() {
                    self.dialog = Some(Dialog::Message(format!(
                        "Gespeichert. Folgendes kann das Originalformat (.epr) noch nicht aufnehmen und wurde nicht geschrieben:\n{}\n\nZum vollständigen Speichern zusätzlich als .epj sichern.",
                        skipped.join("\n")
                    )));
                }
            }
            // nur noch Fehler beim Aufbereiten der Daten (Schreibfehler meldet speichern_ueber_temp)
            Err(_) => {
                crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, crate::meldungen::S_ALLGEMEIN);
            }
        }
    }

    pub fn open(&mut self, add: bool) {
        let dlg = rfd::FileDialog::new().add_filter("Elektroplanung Projekt", &["epj", "epr", "epv"]).add_filter("Neues Format", &["epj"]).add_filter("Altes Format", &["epr"]);
        let Some(p) = dlg.pick_file() else { return };
        self.open_path(p, add);
    }

    /// Projekt laden: .epr (Originalformat, wird übernommen) oder .epj (eigenes Format).
    pub fn open_path(&mut self, p: PathBuf, add: bool) {
        let is_epr = crate::vorlage::is_epr_format(&p);
        // Prüfungen des Originals vor dem Einlesen: Datei lesbar, Kennung und Version (meldungen.rs)
        let bytes = if is_epr { Some(std::fs::read(&p)) } else { None };
        if let Some((titel, text)) = bytes.as_ref().and_then(|b| crate::meldungen::ladefehler_vorab(b, add)) {
            crate::meldungen::warnung(titel, &text);
            return;
        }
        // Statuszeile des Originals während des Lesens (bleibt bei einem Fehler stehen)
        // (0x426310: „Projekt wird geöffnet“ beim Öffnen, „Projekt wird gelesen“ beim Hinzufügen)
        self.info = if add { crate::meldungen::L_LESEN } else { crate::meldungen::L_OEFFNEN }.into();
        let res = if is_epr {
            crate::epr::import(&p)
        } else {
            std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Project>(&s).map_err(|e| e.to_string()))
        };
        if res.is_ok() {
            self.remember_recent(&p);
        }
        match res {
            Ok(prj) => {
                if add {
                    self.add_project(prj);
                } else {
                    self.new_project();
                    self.formen.leeren();
                    self.rueckkehr.clear();
                    self.project = prj;
                    crate::rahmen::abgleichen(&mut self.project);
                    let extra: Vec<SymDef> = self.project.symdefs.iter().filter(|d| symbols::find(&self.lib, &d.key).is_none()).cloned().collect();
                    self.lib.extend(extra);
                    self.path = Some(p);
                    let gemerkt = self.raster_laden();
                    self.fix_tabs();
                    // Wie das Original: alle Blätter als Reiter öffnen, erstes aktiv, Explorer auf "Projekt"
                    self.tabs.clear();
                    for (ai, a) in self.project.anlagen.iter().enumerate() {
                        for si in 0..a.sheets.len() {
                            self.tabs.push(SheetRef { anlage: ai, sheet: si });
                        }
                    }
                    self.active = 0;
                    self.right_tab = RightTab::Projekt;
                    if !gemerkt {
                        self.raster_erkennen();
                    }
                    // Fehler in den Daten (TErrliste): Prüfsummen/Struktur wie das Original, Blöcke ohne Darstellung
                    let mut lines = bytes.as_ref().and_then(|b| b.as_ref().ok()).map(|b| crate::meldungen::ladefehler_liste(b)).unwrap_or_default();
                    lines.extend(crate::meldungen::bildfehler(&self.project));
                    // Fehler im Sinne des Originals: Statuszeile „Fehlerhaftes Projekt“
                    let fehlerhaft = !lines.is_empty();
                    lines.extend(crate::epr::load_warnings(&self.project));
                    if !lines.is_empty() {
                        self.dlg3 = Some(crate::dialoge3::Dlg3::Errliste { lines });
                    }
                    self.info = if fehlerhaft { crate::meldungen::L_FEHLERHAFT.to_string() } else { format!("Ladestatus: {} Blätter geladen", self.project.total_sheets()) };
                    self.tree_open.retain(|k| !k.starts_with("prj/"));
                }
            }
            Err(e) => crate::meldungen::warnung(crate::meldungen::T_ACHTUNG_FEHLER, &format!("{}\n{e}", crate::meldungen::L_FEHLER)),
        }
    }

    /// Anlagen eines anderen Projekts anhängen (Datei › Projekt hinzufügen).
    pub fn add_project(&mut self, prj: Project) {
        self.checkpoint();
        // wie das Original (0x4961c8): nur Symbole übernehmen, die auf den Blättern der übernommenen Anlagen stehen
        // und dem Projekt noch fehlen
        let benutzt: std::collections::HashSet<&str> = prj
            .anlagen
            .iter()
            .flat_map(|a| &a.sheets)
            .flat_map(|s| &s.objects)
            .filter_map(|o| if let Kind::Symbol { key, .. } = &o.kind { Some(key.as_str()) } else { None })
            .collect();
        for d in prj.symdefs.iter().filter(|d| benutzt.contains(d.key.as_str())) {
            if !self.project.symdefs.iter().any(|x| x.key == d.key) {
                self.project.symdefs.push(d.clone());
            }
            if symbols::find(&self.lib, &d.key).is_none() {
                self.lib.push(d.clone());
            }
        }
        let base = self.project.anlagen.len();
        let off = self.project.next_id;
        for mut a in prj.anlagen {
            // Originalblöcke beziehen sich auf die andere Datei: im neuen Projekt neu erzeugen
            a.raw = None;
            for s in &mut a.sheets {
                s.raw = None;
                for o in &mut s.objects {
                    o.id += off;
                    o.raw = None;
                }
            }
            self.project.anlagen.push(a);
        }
        self.project.next_id += prj.next_id + 1;
        self.open_sheet(SheetRef { anlage: base, sheet: 0 });
    }

    pub fn title(&self) -> String {
        let name = if !self.project.name.is_empty() {
            self.project.name.clone()
        } else if let Some(p) = &self.path {
            p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
        } else {
            "Neues Projekt".into()
        };
        // im Symboleditor: „Bibliothekssymbol '…' bearbeiten“ usw. wie der Reiter des Originals
        if let Some(e) = &self.sym_edit {
            return format!("Elektroplanung {}", e.sheet.name);
        }
        let kennung = self.project.raw.as_ref().map(|f| f.magic).filter(|_| self.path.as_deref().is_some_and(crate::vorlage::is_epr_format));
        format!("{}{}", crate::meldungen::fenstertitel(&name, self.path.as_deref(), kennung, false), if self.dirty { " *" } else { "" })
    }

    pub fn not_yet(&mut self, what: &str) {
        self.dialog = Some(Dialog::Message(format!("„{what}“ ist in dieser Version noch nicht enthalten.")));
    }

    /// Blattnummerierung: Blätter jeder Anlage fortlaufend ab 1 nummerieren.
    pub fn renumber(&mut self) {
        self.checkpoint();
        for a in &mut self.project.anlagen {
            for (i, s) in a.sheets.iter_mut().enumerate() {
                s.number = i as u32 + 1;
            }
        }
        self.info = "Blätter neu nummeriert".into();
    }

    /// Listenexport: Betriebsmittelliste als CSV (Semikolon getrennt).
    /// Tools › Listen Export: Dialog wie im Original (Liste wählen, Vorschau, Export), siehe `crate::listen`.
    pub fn list_export(&mut self) {
        self.dlg3 = Some(crate::dialoge3::Dlg3::ListExport { art: 0, format: 0, msg: String::new() });
    }

    pub fn hit_top(&self, m: P, tol: f32) -> Option<u64> {
        let s = self.sheet()?;
        let treffer = |o: &&Obj| self.selectable(o) && canvas::hit(o, m, tol, &self.lib);
        // Klemmstelle vor den Leitungen, die an ihrem Kreis enden (Maustest Runde 11: Doppelklick und Kontextmenü
        // an der Klemme trafen die Leitung)
        s.objects.iter().rev().filter(|o| matches!(o.kind, Kind::Terminal { .. })).find(treffer).or_else(|| s.objects.iter().rev().find(treffer)).map(|o| o.id)
    }

    /// Ziel eines Doppelklicks bei `m` (Treffertoleranz `tol`): getroffenes Objekt, sonst eines bis 12/5 · `tol`
    /// daneben (B34: knapp neben eine Sammelschiene öffnete die Blatteigenschaften); `None` = freie Blattfläche.
    pub fn doppelklick_ziel(&self, m: P, tol: f32) -> Option<u64> {
        self.hit_top(m, tol).or_else(|| self.hit_top(m, tol * 12.0 / 5.0))
    }

    /// Lässt sich das Objekt markieren? Im Editor nur sichtbare; mit „Fixierung aktivieren“ keine fixierten
    /// (wie im Original weder per Klick noch per Aufziehen).
    pub fn selectable(&self, o: &Obj) -> bool {
        self.edit_visible(o.id) && !(o.fixed && self.fix_active)
    }

    pub fn edit_props(&mut self, id: u64) {
        if self.open_props(id) || self.open_props3(id) || self.spiegel_props(id) {
            return;
        }
        let Some(o) = self.sheet().and_then(|s| s.objects.iter().find(|o| o.id == id)).cloned() else { return };
        match &o.kind {
            Kind::Symbol { bmk, value, option, key, .. } => {
                let options = symbols::find(&self.lib, key).map(|d| d.options.clone()).unwrap_or_default();
                self.dialog = Some(Dialog::Symbol { id, bmk: bmk.clone(), value: value.clone(), option: *option, options })
            }
            Kind::Text { text, .. } => {
                self.dialog = Some(Dialog::Text { title: "Text bearbeiten".into(), value: text.clone(), target: TextTarget::EditObj(id) })
            }
            Kind::Table { rows, cols, cells, .. } => self.dialog = Some(Dialog::Table { id, rows: rows.to_string(), cols: cols.to_string(), cells: cells.clone() }),
            Kind::Terminal { nr, .. } => self.dialog = Some(Dialog::Text { title: "Klemmennummer".into(), value: nr.to_string(), target: TextTarget::EditObj(id) }),
            Kind::Image { .. } => {
                if let Some(p) = rfd::FileDialog::new().add_filter("Bilder", &["png", "jpg", "jpeg", "bmp"]).pick_file() {
                    let ps = p.to_string_lossy().to_string();
                    self.checkpoint();
                    self.with_sheet(|s, _| {
                        if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                            if let Kind::Image { path, .. } = &mut o.kind {
                                *path = ps;
                            }
                        }
                    });
                }
            }
            _ => {}
        }
    }

    pub fn open_sheet_props(&mut self, r: SheetRef) {
        self.open_sheet_props_dialog(r);
        if self.props.is_some() {
            return;
        }
        if let Some(s) = self.project.sheet(r) {
            self.dialog = Some(Dialog::Sheet { r, name: s.name.clone(), number: s.number.to_string(), columns: s.columns.to_string() });
        }
    }

    pub fn project_props(&mut self) {
        self.open_projekt_dialog();
    }

    #[allow(dead_code)]
    pub fn project_props_alt(&mut self) {
        let p = &self.project;
        self.dialog = Some(Dialog::Project {
            name: p.name.clone(),
            kunde: p.kunde.clone(),
            owner: p.owner.clone(),
            von: p.von.clone(),
            ort: p.ort.clone(),
            znr: p.znr.clone(),
            done: p.abgeschlossen,
        });
    }

    pub fn apply_dialog(&mut self, dlg: Dialog) {
        match dlg {
            Dialog::Text { value, target, .. } => match target {
                TextTarget::NewText(pos) => {
                    if !value.trim().is_empty() {
                        self.add_obj(Kind::Text { pos, text: value, size: 3.5, align: Align::Left, vertical: false, style: Default::default() });
                    }
                }
                TextTarget::EditObj(id) => {
                    self.checkpoint();
                    self.with_sheet(|s, _| {
                        if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                            match &mut o.kind {
                                Kind::Text { text, .. } => *text = value,
                                Kind::Terminal { nr, .. } => *nr = value.trim().parse().unwrap_or(*nr),
                                _ => {}
                            }
                        }
                    });
                }
            },
            Dialog::Symbol { id, bmk: nb, value: nv, option: no, .. } => {
                self.checkpoint();
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Symbol { bmk, value, option, .. } = &mut o.kind {
                            *bmk = nb;
                            *value = nv;
                            *option = no;
                        }
                    }
                });
            }
            Dialog::Project { name, kunde, owner, von, ort, znr, done } => {
                self.checkpoint();
                let p = &mut self.project;
                p.name = name;
                p.kunde = kunde;
                p.owner = owner;
                p.von = von;
                p.ort = ort;
                p.znr = znr;
                p.abgeschlossen = done;
            }
            Dialog::Sheet { r, name, number, columns } => {
                let num: u32 = number.trim().parse().unwrap_or(0);
                if num == 0 {
                    self.dialog = Some(Dialog::Message("Die Blattnummer darf nicht kleiner als 1 sein!".into()));
                    return;
                }
                let dup = self.project.anlagen.get(r.anlage).map(|a| a.sheets.iter().enumerate().any(|(i, s)| i != r.sheet && s.number == num)).unwrap_or(false);
                if dup {
                    self.dialog = Some(Dialog::Message("Ein Blatt mit dieser Blattnummer ist bei dieser Anlage schon vorhanden!".into()));
                    return;
                }
                self.checkpoint();
                if let Some(s) = self.project.sheet_mut(r) {
                    s.name = name;
                    s.number = num;
                    s.columns = columns.trim().parse().unwrap_or(8).clamp(1, 40);
                }
            }
            Dialog::Table { id, rows, cols, cells } => {
                self.checkpoint();
                let r: usize = rows.parse().unwrap_or(1).clamp(1, 50);
                let c: usize = cols.parse().unwrap_or(1).clamp(1, 20);
                self.with_sheet(|s, _| {
                    if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                        if let Kind::Table { rows, cols, cells: cc, .. } = &mut o.kind {
                            *rows = r;
                            *cols = c;
                            *cc = cells;
                            cc.resize(r * c, String::new());
                        }
                    }
                });
            }
            _ => {}
        }
    }

    /// Markierte Objekte ausrichten: 0 links, 1 rechts, 2 oben, 3 unten, 4 waagerecht mittig, 5 senkrecht mittig.
    pub fn align_selection(&mut self, how: u8) {
        let objs = self.selected_objs();
        if objs.len() < 2 {
            return;
        }
        let bs: Vec<(u64, P, P)> = objs.iter().map(|o| {
            let (a, b) = canvas::obj_bounds(o, &self.lib);
            (o.id, a, b)
        }).collect();
        let first = bs[0];
        let target = match how {
            0 => first.1.x,
            1 => first.2.x,
            2 => first.1.y,
            3 => first.2.y,
            4 => (first.1.y + first.2.y) / 2.0,
            _ => (first.1.x + first.2.x) / 2.0,
        };
        self.checkpoint();
        let fix = self.fix_active;
        self.with_sheet(|s, _| {
            for (id, a, b) in &bs {
                let d = match how {
                    0 => P::new(target - a.x, 0.0),
                    1 => P::new(target - b.x, 0.0),
                    2 => P::new(0.0, target - a.y),
                    3 => P::new(0.0, target - b.y),
                    4 => P::new(0.0, target - (a.y + b.y) / 2.0),
                    _ => P::new(target - (a.x + b.x) / 2.0, 0.0),
                };
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == *id) {
                    if !(o.fixed && fix) {
                        o.kind.translate(d);
                    }
                }
            }
        });
    }

    /// Markierung als Symbol: markierte Zeichenelemente werden zu einem Projektsymbol, Klemmstellen zu Anschlüssen.
    pub fn selection_to_symbol(&mut self) {
        let objs = self.selected_objs();
        // MarkierungzuSymbol1Click 0x46c58c: ohne Markierung (+0x714 = 0) geschieht nichts; die Meldung kommt nur,
        // wenn unter den markierten Objekten keines umwandelbar ist
        if objs.is_empty() {
            return;
        }
        let pts: Vec<P> = objs.iter().flat_map(|o| { let (a, b) = canvas::obj_bounds(o, &self.lib); [a, b] }).collect();
        let (mn, mx) = canvas::pts_bounds(&pts);
        let c = self.snap(P::new((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0));
        let l = |p: P| p.sub(c);
        let mut prims = vec![];
        let mut pins = vec![];
        for o in &objs {
            let st = self.project.stifte.get(o.pen).map(|s| s.style).unwrap_or(0);
            match &o.kind {
                Kind::Line { a, b } => prims.push(symbols::Prim::Line(l(*a), l(*b), st)),
                Kind::Curve { a, c1, c2, b } => prims.push(symbols::Prim::Bezier(l(*a), l(*c1), l(*c2), l(*b), st)),
                Kind::Rect { a, b } => {
                    let (p0, p1) = canvas::norm_rect(*a, *b);
                    prims.push(symbols::Prim::Poly(vec![l(p0), l(P::new(p1.x, p0.y)), l(p1), l(P::new(p0.x, p1.y))], st))
                }
                Kind::Triangle { a, b } => prims.push(symbols::Prim::Poly(canvas::triangle_pts(*a, *b).iter().map(|p| l(*p)).collect(), st)),
                Kind::Circle { a, b } => prims.push(symbols::Prim::Ellipse(l(*a), l(*b), st)),
                Kind::Poly { pts } => prims.push(symbols::Prim::Poly(pts.iter().map(|p| l(*p)).collect(), st)),
                Kind::Wire { pts, .. } => {
                    for w in pts.windows(2) {
                        prims.push(symbols::Prim::Line(l(w[0]), l(w[1]), 0));
                    }
                }
                Kind::Text { pos, text, size, .. } => prims.push(symbols::Prim::Text(l(*pos), text.clone(), *size)),
                Kind::Terminal { pos, .. } => pins.push(l(*pos)),
                _ => {}
            }
        }
        if prims.is_empty() && pins.is_empty() {
            crate::meldungen::hinweis(crate::meldungen::T_ACHTUNG, crate::meldungen::MARKIERUNG_LEER);
            return;
        }
        let n = self.project.symdefs.iter().filter(|d| d.haupt == 0).count() + 1;
        let key = format!("proj:{}", self.project.alloc_id());
        let def = symbols::SymDef {
            key: key.clone(),
            name: format!("Symbol {n}"),
            group: vec!["Projekt".into()],
            haupt: 0,
            fav: false,
            bmk: "-A%C".into(),
            prims,
            pins,
            title: None,
            vis: vec![],
            pin_vis: vec![],
            pin_info: vec![],
            flag: vec![],
            flag_names: vec![],
            text_idx: vec![],
            alt_sets: vec![],
            raw: vec![],
            frg: (true, false),
            params: vec![],
            value_pos: None,
            text_mode: vec![],
            fill: vec![],
            pens: vec![],
            options: vec![],
            versatz: Default::default(),
        };
        self.checkpoint();
        self.project.symdefs.push(def.clone());
        self.lib.push(def.clone());
        // wie das Original: erst fragen, ob das neue Symbol die markierten Objekte ersetzen soll (Nein = nur anlegen)
        if !crate::meldungen::frage(crate::meldungen::T_SOLL_ICH, crate::meldungen::MARKIERUNG_FRAGE) {
            self.info = format!("{} im Projekt angelegt (Benutzerbauteile › Projekt)", def.name);
            return;
        }
        let sel = self.selection.clone();
        self.with_sheet(|s, _| s.objects.retain(|o| !sel.contains(&o.id)));
        let bmk = self.new_bmk(&def);
        let id = self.project.alloc_id();
        let pen = self.pen;
        self.with_sheet(|s, _| {
            s.objects.push(Obj {
                id,
                kind: Kind::Symbol { key, pos: c, rot: 0, mirror_h: false, mirror_v: false, bmk, value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() },
                pen,
                fixed: false,
                group: None, raw: None
            })
        });
        self.selection = vec![id];
        self.info = format!("{} im Projekt angelegt (Benutzerbauteile › Projekt)", def.name);
    }

    pub fn make_kind(&self, tool: Tool, a: P, b: P) -> Option<Kind> {
        if a.dist(b) < 0.01 {
            return None;
        }
        Some(match tool {
            Tool::Line => Kind::Line { a, b },
            // Zeichenfläche: vier Klicks wie im Original (kurve.rs); hier nur die gerade Ausgangslage
            Tool::Curve => Kind::Curve { a, c1: a, c2: b, b },
            Tool::Rect => Kind::Rect { a, b },
            Tool::Triangle => Kind::Triangle { a, b },
            Tool::Circle => Kind::Circle { a, b },
            // wie im Original ein Text mit Box (ZCN 11): Rahmen an, Umbruch aus, 8 pt – Dialog TTextbox
            Tool::TextBox => crate::textbox::neu(a, b, ""),
            // wie im Original 2 × 2, ohne Überschrift, Autobreite – Dialog TSTab (tabelle.rs)
            Tool::Table => crate::tabelle::neu(a, b),
            Tool::Image => Kind::Image { a, b, path: String::new(), mode: 0, frame: true },
            _ => return None,
        })
    }
}

/// Rechtwinklige Leitungsführung (erst in Hauptrichtung, dann quer).
pub fn route(a: P, b: P) -> Vec<P> {
    if (a.x - b.x).abs() < 0.01 || (a.y - b.y).abs() < 0.01 {
        return vec![a, b];
    }
    let corner = if (b.y - a.y).abs() >= (b.x - a.x).abs() { P::new(a.x, b.y) } else { P::new(b.x, a.y) };
    vec![a, corner, b]
}

/// Lage (relativ zum Symbol) und Ausrichtung der BMK eines neu gesetzten Symbols: am Titelanker der Bibliothek
/// (Element Typ 9), sonst links neben dem Umriss, jeweils rechtsbündig und mittig zur Ankerhöhe.
pub fn symbol_title(def: &SymDef) -> (P, u8) {
    symbol_title_rot(def, 0, 0)
}

/// Wie `symbol_title`, für ein gedrehtes Symbol (Drehung in 90°-Schritten, Bild/Option): ungedreht am Titelanker,
/// gedreht links bzw. über dem gedrehten Umriss, damit die BMK nicht im Symbol liegt.
pub fn symbol_title_rot(def: &SymDef, rot: u8, option: u16) -> (P, u8) {
    let (mn, mx) = symbols::bounds_opt(def, option);
    if rot % 4 == 0 {
        return match symbols::title_lage(def, option) {
            Some(t) => (P::new(t.x.min(mn.x - 0.8), t.y - 1.25), 2),
            None => (P::new(mn.x - 1.0, (mn.y + mx.y) / 2.0 - 1.25), 2),
        };
    }
    let c = [P::new(mn.x, mn.y), P::new(mx.x, mn.y), P::new(mx.x, mx.y), P::new(mn.x, mx.y)];
    let pts: Vec<P> = c.iter().map(|q| sym_xf(*q, P::default(), rot, false, false, 1.0)).collect();
    let (rmn, rmx) = crate::canvas::pts_bounds(&pts);
    // waagerecht liegendes Symbol: BMK mittig darüber
    (P::new((rmn.x + rmx.x) / 2.0, rmn.y - 3.5), 1)
}

/// Kantenlänge der Griffe des Rahmens zur Größenänderung in mm (Original 2,4 mm).
pub const FRAME_HANDLE: f32 = 2.4;

/// Größte Suchweite der automatischen Verbindungen in mm.
pub const AUTO_CONN_MAX: f32 = 200.0;

/// Ein Anschluss gilt als verbunden, wenn ein Leitungsende oder ein anderer Anschluss so nah liegt (mm).
const CONN_TOL: f32 = 0.05;

/// Automatische Verbindungen wie im Original: für jeden noch freien Anschluss der Objekte `ids` die Richtung nach
/// außen bestimmen (Anschluss am linken/rechten/oberen/unteren Rand des Umrisses, Toleranz 0,2 mm) und eine gerade
/// Leitung bis zum nächsten Ziel auf diesem Strahl ziehen: Anschluss eines anderen Objekts oder eine Leitung, die
/// den Strahl kreuzt. Liefert die neuen Leitungen (Anfang, Ende).
pub fn auto_wires(sheet: &Sheet, lib: &[SymDef], ids: &[u64], max: f32) -> Vec<(P, P)> {
    let mut out: Vec<(P, P)> = vec![];
    // Anschlüsse und Leitungsstrecken aller Objekte (mit Objekt-Id)
    let pins: Vec<(u64, P)> = sheet.objects.iter().flat_map(|o| canvas::obj_pins(o, lib).into_iter().map(move |p| (o.id, p))).collect();
    let segs: Vec<(P, P)> = sheet
        .objects
        .iter()
        .filter_map(|o| if let Kind::Wire { pts, .. } = &o.kind { Some(pts) } else { None })
        .flat_map(|pts| pts.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
        .collect();
    let umrisse = crate::leitung_hindernis::umrisse(sheet, lib);
    for o in sheet.objects.iter().filter(|o| ids.contains(&o.id)) {
        if !matches!(o.kind, Kind::Symbol { .. } | Kind::Terminal { .. }) {
            continue;
        }
        let own = canvas::obj_pins(o, lib);
        if own.is_empty() {
            continue;
        }
        let (mut mn, mut mx) = canvas::obj_bounds(o, lib);
        for p in &own {
            mn = P::new(mn.x.min(p.x), mn.y.min(p.y));
            mx = P::new(mx.x.max(p.x), mx.y.max(p.y));
        }
        let tall = mx.y - mn.y >= mx.x - mn.x;
        for &p in &own {
            let connected = pins.iter().any(|(id, q)| *id != o.id && q.dist(p) <= CONN_TOL) || out.iter().any(|(a, b)| a.dist(p) <= CONN_TOL || b.dist(p) <= CONN_TOL);
            if connected {
                continue;
            }
            for d in pin_dirs(p, mn, mx, tall) {
                // nächstes Ziel auf dem Strahl p + t·d (t > 0)
                let mut best: Option<f32> = None;
                let mut take = |t: f32| {
                    if t > CONN_TOL && t <= max && best.map(|b| t < b).unwrap_or(true) {
                        best = Some(t);
                    }
                };
                for (id, q) in &pins {
                    if *id == o.id {
                        continue;
                    }
                    let v = q.sub(p);
                    let t = v.x * d.x + v.y * d.y;
                    let off = (v.x * d.y - v.y * d.x).abs();
                    if off <= CONN_TOL {
                        take(t);
                    }
                }
                for (a, b) in segs.iter().chain(out.iter()) {
                    if let Some(t) = ray_seg(p, d, *a, *b) {
                        take(t);
                    }
                }
                // ein fremdes Symbol auf dem Strahl vor dem Ziel: keine Verbindung (Nachzeichnen 24.11.2026, Blatt
                // -F3: der Strahl vom N des Motors lief 145 mm quer über das Blatt durch einen Taster bis zur PE-Leitung)
                let durch = umrisse.iter().filter(|u| u.id != o.id).filter_map(|u| strahl_in_rechteck(p, d, u.r)).fold(f32::MAX, f32::min);
                let best = best.filter(|t| *t < durch);
                if let Some(t) = best {
                    out.push((p, P::new(p.x + d.x * t, p.y + d.y * t)));
                    break;
                }
            }
        }
    }
    out
}

/// Strecke `t` (> 0), nach der der achsparallele Strahl `p + t·d` ins Innere des Rechtecks `r` eintritt (0,2 mm Rand
/// nach innen, damit ein Strahl am Rand bzw. in einen Anschluss am Rand nicht zählt).
fn strahl_in_rechteck(p: P, d: P, r: crate::leitung_hindernis::Rechteck) -> Option<f32> {
    const R: f32 = 0.2;
    let (mn, mx) = (P::new(r.0.x.min(r.1.x) + R, r.0.y.min(r.1.y) + R), P::new(r.0.x.max(r.1.x) - R, r.0.y.max(r.1.y) - R));
    if mn.x >= mx.x || mn.y >= mx.y {
        return None;
    }
    if d.y.abs() < 0.5 {
        if p.y <= mn.y || p.y >= mx.y {
            return None;
        }
        let t = if d.x > 0.0 { mn.x - p.x } else { p.x - mx.x };
        (t > 0.0).then_some(t)
    } else {
        if p.x <= mn.x || p.x >= mx.x {
            return None;
        }
        let t = if d.y > 0.0 { mn.y - p.y } else { p.y - mx.y };
        (t > 0.0).then_some(t)
    }
}

/// Richtungen nach außen für einen Anschluss am Umriss (mn, mx); an Ecken zuerst entlang der längeren Seite des
/// Umrisses (hohes Symbol → oben/unten).
fn pin_dirs(p: P, mn: P, mx: P, tall: bool) -> Vec<P> {
    const T: f32 = 0.2;
    let mut v = vec![];
    let mut h = vec![];
    if (p.y - mn.y).abs() <= T {
        v.push(P::new(0.0, -1.0));
    }
    if (p.y - mx.y).abs() <= T {
        v.push(P::new(0.0, 1.0));
    }
    if (p.x - mn.x).abs() <= T {
        h.push(P::new(-1.0, 0.0));
    }
    if (p.x - mx.x).abs() <= T {
        h.push(P::new(1.0, 0.0));
    }
    // Umriss ohne Ausdehnung in einer Richtung: dort keine eindeutige Richtung nach außen
    if v.len() > 1 {
        v.clear();
    }
    if h.len() > 1 {
        h.clear();
    }
    if tall {
        v.extend(h);
        v
    } else {
        h.extend(v);
        h
    }
}

/// Schnitt des Strahls p + t·d (d Einheitsvektor) mit der Strecke a–b: Abstand t oder None.
fn ray_seg(p: P, d: P, a: P, b: P) -> Option<f32> {
    let e = b.sub(a);
    let den = d.x * e.y - d.y * e.x;
    let w = a.sub(p);
    if den.abs() < 1e-6 {
        // parallel: nur wenn die Strecke auf dem Strahl liegt – ihr erster Punkt vor dem Anschluss
        if (w.x * d.y - w.y * d.x).abs() > CONN_TOL {
            return None;
        }
        let ta = w.x * d.x + w.y * d.y;
        let tb = (b.x - p.x) * d.x + (b.y - p.y) * d.y;
        let t = ta.min(tb);
        return if t > CONN_TOL { Some(t) } else { None };
    }
    let t = (w.x * e.y - w.y * e.x) / den;
    let u = (w.x * d.y - w.y * d.x) / den;
    if (-1e-4..=1.0 + 1e-4).contains(&u) {
        Some(t)
    } else {
        None
    }
}

/// Griffe des Rahmens zur Größenänderung (links oben im Uhrzeigersinn: Ecke, Mitte oben, Ecke, Mitte rechts …).
pub fn frame_handles(mn: P, mx: P) -> [P; 8] {
    let cx = (mn.x + mx.x) / 2.0;
    let cy = (mn.y + mx.y) / 2.0;
    [P::new(mn.x, mn.y), P::new(cx, mn.y), P::new(mx.x, mn.y), P::new(mx.x, cy), P::new(mx.x, mx.y), P::new(cx, mx.y), P::new(mn.x, mx.y), P::new(mn.x, cy)]
}

/// Streckung beim Ziehen des Griffs `handle` nach `cur`: (Festpunkt = gegenüberliegender Griff, Faktor x, Faktor y).
pub fn frame_scale(handle: usize, mn: P, mx: P, cur: P) -> (P, f32, f32) {
    let h = frame_handles(mn, mx);
    let g = h[handle % 8];
    let anchor = h[(handle + 4) % 8];
    let fac = |c: f32, g: f32, a: f32| if (g - a).abs() < 0.01 { 1.0 } else { ((c - a) / (g - a)).max(0.05) };
    let (sx, sy) = match handle % 8 {
        1 | 5 => (1.0, fac(cur.y, g.y, anchor.y)),
        3 | 7 => (fac(cur.x, g.x, anchor.x), 1.0),
        _ => (fac(cur.x, g.x, anchor.x), fac(cur.y, g.y, anchor.y)),
    };
    (anchor, sx, sy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: u64, key: &str, pos: P) -> Obj {
        Obj { id, kind: Kind::Symbol { key: key.into(), pos, rot: 0, mirror_h: false, mirror_v: false, bmk: String::new(), value: String::new(), scale: 1.0, option: 0, title: None, flags: 0, ext: Default::default() }, pen: 0, fixed: false, group: None, raw: None }
    }

    fn wire(id: u64, a: P, b: P) -> Obj {
        Obj { id, kind: Kind::Wire { pts: vec![a, b], leitung: 0, bez: vec![], opts: 0 }, pen: 0, fixed: false, group: None, raw: None }
    }

    #[test]
    fn strompfad_mitte() {
        // A4 quer: Rahmen 5 … 292, 8 Pfade → Schritt 35,875; Mitten auf 0,1 mm gerundet (Dateigenauigkeit)
        let m = |k: f32| ((5.0 + k * (292.0 - 5.0) / 8.0) * 10.0f32).round() / 10.0;
        assert!((path_center(5.0, 292.0, 8, 6.0) - m(0.5)).abs() < 1e-4);
        assert!((path_center(5.0, 292.0, 8, 60.0) - m(1.5)).abs() < 1e-4);
        assert!((path_center(5.0, 292.0, 8, 400.0) - m(7.5)).abs() < 1e-4);
        assert!((path_center(5.0, 292.0, 8, -50.0) - m(0.5)).abs() < 1e-4);
        assert_eq!(path_center(5.0, 292.0, 8, 6.0), 22.9);
        let s = Sheet::new(1);
        let (fx0, _, fx1, _) = s.frame();
        assert!((s.path_center_x(100.0).unwrap() - path_center(fx0, fx1, 8, 100.0)).abs() < 1e-4);
    }

    #[test]
    fn automatische_verbindung_zum_anschluss_und_zur_leitung() {
        let lib = symbols::builtin();
        let mut s = Sheet::new(1);
        // Schließer bei (50, 100): Anschlüsse (50, 93) oben und (50, 107) unten
        s.objects.push(sym(1, "builtin:schliesser", P::new(50.0, 100.0)));
        // darüber eine waagerechte Leitung, darunter ein zweiter Schließer
        s.objects.push(wire(2, P::new(10.0, 60.0), P::new(150.0, 60.0)));
        s.objects.push(sym(3, "builtin:schliesser", P::new(50.0, 140.0)));
        let w = auto_wires(&s, &lib, &[1], AUTO_CONN_MAX);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(w.iter().any(|(a, b)| a.dist(P::new(50.0, 93.0)) < 1e-3 && b.dist(P::new(50.0, 60.0)) < 1e-3));
        assert!(w.iter().any(|(a, b)| a.dist(P::new(50.0, 107.0)) < 1e-3 && b.dist(P::new(50.0, 133.0)) < 1e-3));
        // bereits verbundener Anschluss bekommt keine weitere Leitung
        s.objects.push(wire(4, P::new(50.0, 93.0), P::new(50.0, 60.0)));
        let w = auto_wires(&s, &lib, &[1], AUTO_CONN_MAX);
        assert_eq!(w.len(), 1, "{w:?}");
        // nichts in Reichweite → keine Leitung
        let mut s2 = Sheet::new(1);
        s2.objects.push(sym(1, "builtin:schliesser", P::new(50.0, 100.0)));
        assert!(auto_wires(&s2, &lib, &[1], AUTO_CONN_MAX).is_empty());
    }

    #[test]
    fn rahmen_streckung() {
        let (mn, mx) = (P::new(10.0, 10.0), P::new(30.0, 20.0));
        // rechte untere Ecke auf (50, 30): Festpunkt links oben, Faktor 2
        let (a, sx, sy) = frame_scale(4, mn, mx, P::new(50.0, 30.0));
        assert_eq!(a, P::new(10.0, 10.0));
        assert!((sx - 2.0).abs() < 1e-5 && (sy - 2.0).abs() < 1e-5);
        // Mitte links: nur waagerecht, Festpunkt rechte Seite
        let (a, sx, sy) = frame_scale(7, mn, mx, P::new(20.0, 99.0));
        assert_eq!(a.x, 30.0);
        assert!((sx - 0.5).abs() < 1e-5 && sy == 1.0);
        // Linie wird gestreckt, Symbol nur verschoben
        let mut l = Kind::Line { a: P::new(10.0, 10.0), b: P::new(30.0, 20.0) };
        l.scale_about(P::new(10.0, 10.0), 2.0, 2.0);
        assert_eq!(l, Kind::Line { a: P::new(10.0, 10.0), b: P::new(50.0, 30.0) });
        let mut k = sym(1, "builtin:schliesser", P::new(20.0, 15.0)).kind;
        k.scale_about(P::new(10.0, 10.0), 2.0, 2.0);
        assert!(matches!(k, Kind::Symbol { pos, scale, .. } if pos == P::new(30.0, 20.0) && scale == 1.0));
    }
}

#[cfg(test)]
mod blatt_dialog_tests {
    /// Maus-Befund M18: Hinzufügen › Blatt öffnet den Eigenschaftendialog des neuen Blattes; Abbruch entfernt es.
    #[test]
    fn neues_blatt_mit_dialog_und_abbruch() {
        let mut app = crate::blatt_tasten::test_app();
        let vorher = app.project.anlagen[0].sheets.len();
        app.new_sheet_dialog();
        assert_eq!(app.project.anlagen[0].sheets.len(), vorher + 1);
        assert!(matches!(&app.props, Some(crate::props::Props::Sheet(s)) if s.neu));
        // Abbruch wie im Dialog: Blatt wieder weg, kein Wiederholen-Schritt
        let redo = app.redo.len();
        app.props = None;
        app.do_undo();
        app.redo.pop();
        assert_eq!(app.project.anlagen[0].sheets.len(), vorher);
        assert_eq!(app.redo.len(), redo);
    }
}

/// Dateiname aus dem Projektnamen (ohne Zeichen, die Dateisysteme nicht erlauben); leer → „Projekt“.
pub fn dateiname_aus(name: &str) -> String {
    let s: String = name.trim().chars().map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
    if s.trim().is_empty() {
        "Projekt".into()
    } else {
        s
    }
}

#[cfg(test)]
mod dateiname_tests {
    #[test]
    fn dateiname_aus_projektname() {
        assert_eq!(super::dateiname_aus("Wohnung 2"), "Wohnung 2");
        assert_eq!(super::dateiname_aus("A/B: C?"), "A_B_ C_");
        assert_eq!(super::dateiname_aus("  "), "Projekt");
    }
}
