//! Datenmodell: Projekt, Anlagen, Blätter und Zeichenobjekte (Einheiten: Millimeter auf dem Blatt).

use serde::{Deserialize, Serialize};

pub const SHEET_W: f32 = 297.0;
pub const SHEET_H: f32 = 210.0;
pub const FRAME_LEFT: f32 = 5.0;
pub const FRAME_RIGHT: f32 = 5.0;
pub const FRAME_TOP: f32 = 20.0;
pub const FRAME_BOTTOM: f32 = 5.0;
pub const COLUMN_BAR_H: f32 = 5.0;
pub const TITLE_H: f32 = 15.0;
pub const GRID_ORIGIN: f32 = 5.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct P {
    pub x: f32,
    pub y: f32,
}

impl P {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn add(self, o: P) -> P {
        P::new(self.x + o.x, self.y + o.y)
    }
    pub fn sub(self, o: P) -> P {
        P::new(self.x - o.x, self.y - o.y)
    }
    pub fn dist(self, o: P) -> f32 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

/// Abstand eines Punktes zu einer Strecke.
pub fn seg_dist(p: P, a: P, b: P) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 < 1e-9 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
    p.dist(P::new(a.x + t * dx, a.y + t * dy))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Kind {
    Line { a: P, b: P },
    Curve { a: P, c1: P, c2: P, b: P },
    Rect { a: P, b: P },
    Triangle { a: P, b: P },
    Circle { a: P, b: P },
    Text {
        pos: P,
        text: String,
        size: f32,
        align: Align,
        vertical: bool,
        #[serde(default)]
        style: TextStyle,
    },
    TextBox { a: P, b: P, text: String, size: f32 },
    /// Tabelle (TSTab): `header` = erste Zeile fett ("Überschrift"), `fixed_w` = gleiche Spaltenbreiten statt "Autobreite"
    Table {
        a: P,
        b: P,
        rows: usize,
        cols: usize,
        cells: Vec<String>,
        #[serde(default)]
        header: bool,
        #[serde(default)]
        fixed_w: bool,
    },
    /// Bild (TBild): `mode` 0 Gestreckt, 1 Zentriert, 2 Eingepasst (Seitenverhältnis); `frame` = Rahmen
    Image {
        a: P,
        b: P,
        path: String,
        #[serde(default)]
        mode: u8,
        #[serde(default = "yes")]
        frame: bool,
    },
    Symbol {
        key: String,
        pos: P,
        rot: u8,
        mirror_h: bool,
        mirror_v: bool,
        bmk: String,
        value: String,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default)]
        option: u16,
        /// Lage der Betriebsmittelkennzeichnung relativ zum Symbol (aus .epr) und ob hochkant
        #[serde(default)]
        title: Option<(P, bool)>,
        /// Zusatzoptionen als Bitmaske (VSY-Feld 23), siehe SymDef::flag
        #[serde(default)]
        flags: u32,
        #[serde(default)]
        ext: SymExt,
    },
    Poly { pts: Vec<P> },
    Wire {
        pts: Vec<P>,
        leitung: usize,
        /// Verbindungsbezeichnungen (VBG › BZD): Lage der Textmitte links unten in mm und Text, z. B. Aderfarbe "sw"
        #[serde(default)]
        bez: Vec<(P, String)>,
        /// Einstellungen des Verbindungsdialogs (TSVerbindung), siehe `WIRE_*`
        #[serde(default)]
        opts: u8,
    },
    /// Sammelschiene auf dem Blatt (OBJ mit Feld 4 = 0xFFFF): waagerecht von `a` bis `x2`, Name der Schiene (SMS › TIT)
    /// an `a + label`; `xin`/`xout` = Querverweis auf das vorige/nächste Vorkommen derselben Schiene ("Blatt/Pfad")
    Busbar {
        a: P,
        x2: f32,
        name: String,
        label: P,
        #[serde(default)]
        style: u8,
        #[serde(default)]
        sms: u64,
        #[serde(default)]
        xin: String,
        #[serde(default)]
        xout: String,
    },
    /// Klemmstelle. Aus .epr (OBJ mit Feld 4 = 0x2FFFF): Feld 6/7 Lage, 8 Klemmleiste (KLM Feld 13), 9 Nummer,
    /// 11 Byte 0 Etage (A, B, C … bei mehrstöckigen Leisten), 2/3 Textversatz, TIT = Vorlage "%O%B.%C"
    Terminal {
        pos: P,
        leiste: String,
        nr: u32,
        #[serde(default)]
        level: u8,
        #[serde(default)]
        text_off: Option<P>,
        #[serde(default)]
        tpl: String,
    },
    /// Verweis (OBJ im Original): gefülltes Dreieck an `pos`, Spitze in Richtung `dir` (Grad, 0 = rechts,
    /// 90 = oben), Text an `text_pos`; `vertical` = Text hochkant.
    /// `ziel` = Sprungziel (OBJ Feld 8–13, Dialog "Ziel setzen").
    Verweis {
        pos: P,
        text_pos: P,
        text: String,
        dir: u16,
        vertical: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ziel: Option<VerweisZiel>,
    },
}

/// Ziel eines Verweises wie im Original (OBJ Feld 8/9 Anlage, 10/11 Blatt, 12/13 Gegenverweis, jeweils als
/// Kennung): hier über den Namen der Anlage, die Blattnummer und die Objekt-Id des Gegenverweises (0 = nur Blatt).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct VerweisZiel {
    pub anlage: String,
    pub blatt: u32,
    #[serde(default)]
    pub obj: u64,
}

/// Einstellungen eines Textes wie im Eigenschaftendialog des Originals (TTextbox).
/// Ablage im Original (ZCN Typ 11): Feld 0–3 Textbox in 0,1 mm, 4 Schriftgrad (pt), 5 Drehung (Grad),
/// 6 Byte 0 Ausrichtung / Bit 16 Rahmen / Bit 24 Fett, 7 Byte 0 Kursiv, 8 Schriftfarbe (TColor), 15 Bit 16 Fixiert.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TextStyle {
    #[serde(default)]
    pub frame: bool,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub color: [u8; 3],
    /// Drehung in Grad (0 = waagerecht; 90 = hochkant von unten nach oben)
    #[serde(default)]
    pub rotation: i16,
    /// Umbruch (im Original nur bei Textboxen aktiv)
    #[serde(default)]
    pub wrap: bool,
    /// Textboxabmessung (links oben, rechts unten) in mm, falls bekannt
    #[serde(default)]
    pub rect: Option<(P, P)>,
    /// Schriftart (leer = Projektschrift)
    #[serde(default)]
    pub font: String,
}

/// Weitere Symboleinstellungen wie im Eigenschaftendialog des Originals (TSymbol).
/// Ablage im Original (VSY): 12/15 Ausrichtung Titel/Wert (Hi-Word) + Bit 24 hochkant, 13/14 Wertversatz,
/// 20 Freigaben (Bit 0 = keine Querverweise, Bit 24 = Sicherungsübersicht), 21 Alternativtext,
/// Unterblöcke FKT (Funktion), SPA (Parameterwerte), ALT (benutzerdefinierte Texte).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymExt {
    #[serde(default)]
    pub title_align: u8,
    #[serde(default)]
    pub value_off: Option<P>,
    #[serde(default)]
    pub value_align: u8,
    /// 0 = Standard, n = Alternativtext-Satz n
    #[serde(default)]
    pub alt: u16,
    #[serde(default = "yes")]
    pub querverweise: bool,
    #[serde(default)]
    pub sicherung: bool,
    #[serde(default)]
    pub funktion: String,
    #[serde(default = "minus_one")]
    pub rack: i32,
    /// Umriss relativ zur Lage in mm (VSY Feld 32–35: links, oben, rechts, unten in 0,1 mm) – wie das Original
    /// ihn für Markierung und Treffer benutzt; None = aus der Symboldefinition berechnen
    #[serde(default)]
    pub bounds: Option<(P, P)>,
    /// Parameterwerte der Instanz (VSY › SPA: Feld 0 = Parameternummer, STR = Wert), ersetzen %<Name> in Symboltexten
    #[serde(default)]
    pub params: Vec<(u32, String)>,
    /// Zähler für %C in der BMK-Vorlage (VSY Feld 38; 0 = keiner) – das Original speichert die BMK als Vorlage,
    /// z. B. "-X1.%C+%P", und zeigt "-X1.1+W2"
    #[serde(default)]
    pub counter: i32,
    /// Benutzerdefinierte Symboltexte der Instanz (VSY › ALT: Feld 0 = Textnummer, STR = Text), z. B. "PE"
    /// statt "N" im Symbol "Verbindung"
    #[serde(default)]
    pub texts: Vec<(u32, String)>,
}

impl Default for SymExt {
    fn default() -> Self {
        SymExt { title_align: 0, value_off: None, value_align: 0, alt: 0, querverweise: true, sicherung: false, funktion: String::new(), rack: -1, bounds: None, params: vec![], counter: 0, texts: vec![] }
    }
}

fn yes() -> bool {
    true
}
fn minus_one() -> i32 {
    -1
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Obj {
    pub id: u64,
    pub kind: Kind,
    #[serde(default)]
    pub pen: usize,
    #[serde(default)]
    pub fixed: bool,
    #[serde(default)]
    pub group: Option<u64>,
    /// Originalblock aus einer .epr-Datei (für verlustfreies Zurückschreiben)
    #[serde(skip)]
    pub raw: Option<Box<RawObj>>,
}

/// Originaldaten eines aus .epr gelesenen Objekts: Block und die beim Import erzeugte Form.
#[derive(Clone, Debug, PartialEq)]
pub struct RawObj {
    pub block: crate::eprfile::Block,
    pub kind: Kind,
    pub pen: usize,
    pub fixed: bool,
}

/// Originaldaten eines Blattes: BLT-Block mit allen Unterblöcken und der Zuordnung Unterblock → Objekt-Id.
#[derive(Clone, Debug, PartialEq)]
pub struct RawSheet {
    pub block: crate::eprfile::Block,
    pub map: Vec<Option<u64>>,
    pub name: String,
    pub number: u32,
    pub portrait: bool,
    pub subindex: u8,
    pub ort: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub number: u32,
    pub columns: u32,
    pub objects: Vec<Obj>,
    /// Hochformat (Blatt_Format "Hochformat"/"Querformat" im Original-Dialog)
    #[serde(default)]
    pub portrait: bool,
    /// Eigene Strompfadbezeichnungen (leer = 1, 2, 3 …)
    #[serde(default)]
    pub col_labels: Vec<String>,
    #[serde(default)]
    pub ort: String,
    /// Rahmenhöhe ohne Spaltenleiste in mm (BLT-Feld 24 in 0,1 mm; 0 = Standard 180 mm quer)
    #[serde(default)]
    pub frame_h: f32,
    /// Subindex (BLT Feld 23 Byte 0; Feld 23 Bit 8… = Blattnummer)
    #[serde(default)]
    pub subindex: u8,
    /// Strompfade von der Anlage übernommen (Dialog zeigt -1)
    #[serde(default)]
    pub col_auto: bool,
    /// Blattart (BLT Feld 22 Byte 0): 3 = Stromlaufplan, 4 = Klemmplan, 5 = Sicherungsübersicht; 0 = unbekannt/neu
    #[serde(default)]
    pub art: u8,
    /// Klemmplan: Kennung der Klemmleiste (BLT Feld 32 ab Bit 8 = KLM Feld 13)
    #[serde(default)]
    pub klm: i32,
    /// Pfadbeschreibungen: Begrenzungslinien zwischen den Strompfaden über das ganze Blatt (TPfadDialog)
    #[serde(default)]
    pub pfad_linien: bool,
    /// Erstellt / Geändert dieses Blattes (BLT Byte 112 / 120, Delphi-Datum) – das Schriftfeld zeigt sie
    #[serde(default)]
    pub erstellt: String,
    #[serde(default)]
    pub geaendert: String,
    #[serde(skip)]
    pub raw: Option<Box<RawSheet>>,
}

impl Sheet {
    pub fn new(number: u32) -> Self {
        Self { name: format!("Blatt {number}"), number, columns: 8, objects: Vec::new(), portrait: false, col_labels: vec![], ort: String::new(), frame_h: 0.0, subindex: 0, col_auto: false, art: 0, klm: 0, pfad_linien: false, erstellt: today(), geaendert: String::new(), raw: None }
    }

    /// Blattgröße in mm (A4 quer oder hoch).
    pub fn dims(&self) -> (f32, f32) {
        if self.portrait {
            (SHEET_H, SHEET_W)
        } else {
            (SHEET_W, SHEET_H)
        }
    }

    /// Rahmen (x0, y0, x1, y1) in mm. Quer: links/rechts/unten 5; oben = unten − Rahmenhöhe − Spaltenleiste
    /// (BLT-Feld 24: 1900 → 10 mm wie im Original-Ausdruck "Altbau EG", Standard 1800 → 20 mm wie ein neues Blatt im Original);
    /// hoch: links 10, oben/rechts/unten 5 (gemessen im Original-PDF "Elektroplanung Wohnung 2", Seite 1).
    pub fn frame(&self) -> (f32, f32, f32, f32) {
        let (w, h) = self.dims();
        if self.portrait {
            (10.0, 5.0, w - 5.0, h - 5.0)
        } else {
            let fh = if self.frame_h > 50.0 { self.frame_h } else { h - FRAME_BOTTOM - FRAME_TOP - COLUMN_BAR_H };
            let top = (h - FRAME_BOTTOM - fh - COLUMN_BAR_H).clamp(5.0, 60.0);
            (FRAME_LEFT, top, w - FRAME_RIGHT, h - FRAME_BOTTOM)
        }
    }

    pub fn col_label(&self, i: usize) -> String {
        self.col_labels.get(i).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| (i + 1).to_string())
    }

    /// Mitte des Strompfades, der `x` am nächsten liegt – dieselbe Teilung wie die Strompfad-Spaltenleiste
    /// (Rahmen links bis rechts durch die Zahl der Strompfade). None bei Blättern ohne Spaltenleiste.
    pub fn path_center_x(&self, x: f32) -> Option<f32> {
        if self.art == 4 || self.art == 5 {
            return None;
        }
        let (fx0, _, fx1, _) = self.frame();
        Some(path_center(fx0, fx1, self.columns.max(1), x))
    }
}

/// Strompfadausrichtung wie im Original: Schritt = (rechts − links) / Pfade, Mitte = links + (k + 0,5) · Schritt;
/// gewählt wird der Pfad, in dessen Mitte `x` am nächsten liegt (ohne Abstandsgrenze).
pub fn path_center(left: f32, right: f32, cols: u32, x: f32) -> f32 {
    let n = cols.max(1);
    let step = (right - left) / n as f32;
    if step <= 0.0 {
        return x;
    }
    let k = ((x - left) / step - 0.5).round().clamp(0.0, (n - 1) as f32);
    left + (k + 0.5) * step
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Anlage {
    pub name: String,
    pub sheets: Vec<Sheet>,
    pub sammelschienen: Vec<String>,
    pub klemmleisten: Vec<String>,
    /// Klemmen je Klemmleiste aus .epr (KLM › KFO): Kennung (KLM Feld 13), Name, Klemmennummern
    #[serde(default)]
    pub klemmen: Vec<KlemmInfo>,
    /// Einstellungen der Sammelschienen (TSSchiene) je Name
    #[serde(default)]
    pub schienen: std::collections::BTreeMap<String, SchieneSet>,
    /// Einstellungen des Anlagendialogs (TSAnlage)
    #[serde(default)]
    pub set: AnlSettings,
    /// ANL-Block aus .epr (Name des Originals, Unterblöcke)
    #[serde(skip)]
    pub raw: Option<Box<(crate::eprfile::Block, String)>>,
}

/// Klemmleiste aus .epr: KLM Feld 13 = Kennung (Verweis aus dem Klemmplan-Blatt), KFO Feld 4 = Klemmennummer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct KlemmInfo {
    pub id: i32,
    pub name: String,
    pub nrs: Vec<u32>,
}

/// Verbindungsdialog (TSVerbindung): Bits in `Kind::Wire::opts`.
pub const WIRE_ZIEL_A: u8 = 1;
pub const WIRE_ZIEL_B: u8 = 2;
pub const WIRE_BRUECKE: u8 = 4;
pub const WIRE_HOCHKANT: u8 = 8;
pub const WIRE_MARKIERUNG: u8 = 16;
pub const WIRE_UNTERSTRICH: u8 = 32;
pub const WIRE_DURCH: u8 = 64;

/// Sammelschiene (TSSchiene): Kürzel, Potential, Stift und Schalter des Originaldialogs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SchieneSet {
    pub kurz: String,
    pub potential: String,
    pub stift: usize,
    pub quer: bool,
    pub pfeile: bool,
    pub klemmen: bool,
    pub schleife: bool,
    pub autoende: bool,
    pub einspeisung: bool,
}

impl Default for SchieneSet {
    fn default() -> Self {
        SchieneSet { kurz: String::new(), potential: String::new(), stift: 0, quer: true, pfeile: true, klemmen: false, schleife: false, autoende: false, einspeisung: false }
    }
}

/// Projektbild (PRJ › BLD › JPG): Name (BLD 0–20), Nummer (BLD Feld 5), JPEG-Daten.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bild {
    pub name: String,
    pub nr: i32,
    #[serde(default)]
    pub jpeg: Vec<u8>,
}

/// Anlagendialog des Originals (TSAnlage). Ablage im ANL-Block: Name 0–55, Kurzform 55–66, Erstellt von ab 66;
/// Feld 38/39 Blattformat (0,1 mm), 40–43 Blattränder oben/unten/links/rechts (0,1 mm, Hochformat),
/// 44 Strompfade, 45 Strompfadhöhe (0,1 mm), 46 Schrift Titel, 47 Wert, 48 Verbindungen, 49 Ab '0', 50 Pfadbezeichnung.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnlSettings {
    pub kurz: String,
    pub von: String,
    pub znr: String,
    pub format: (i32, i32),
    pub raender: [i32; 4],
    pub pfade: i32,
    pub pfad_hoehe: i32,
    pub ab_null: bool,
    pub size_titel: i32,
    pub size_wert: i32,
    pub size_verbg: i32,
    pub size_pfad: i32,
}

impl Default for AnlSettings {
    fn default() -> Self {
        // Werte eines neuen Projekts (wie in W2.epr bei allen Anlagen)
        AnlSettings { kurz: String::new(), von: String::new(), znr: String::new(), format: (2100, 2970), raender: [50, 50, 100, 50], pfade: 8, pfad_hoehe: 50, ab_null: false, size_titel: 10, size_wert: 8, size_verbg: 6, size_pfad: 10 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Leitung {
    pub name: String,
    pub color: [u8; 3],
    pub width_mm: f32,
    /// Adern: Querschnitt in mm², Anzahl, Kennzeichnung (0 keine, 1 Zahlen, 2 farbig) – wie Dialog TLeitung
    #[serde(default)]
    pub quer: f32,
    #[serde(default)]
    pub adern: u32,
    #[serde(default)]
    pub kennz: u8,
    /// Strichart (LTG Feld 10: 0 Standard, 1 Gestrichelt, 2 Strich/Punkt, 3 Gepunktet), z. B. PE strichpunktiert
    #[serde(default)]
    pub style: u8,
    /// Kennung aus .epr (LTG Feld 9), auf die Verbindungen in VBG Feld 13 verweisen
    #[serde(default)]
    pub id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Stift {
    pub name: String,
    /// 0 = durchgezogen, 1 = gestrichelt, 2 = Strich/Punkt, 3 = gepunktet
    pub style: u8,
    pub color: [u8; 3],
    #[serde(default = "quarter")]
    pub width: f32,
}

fn quarter() -> f32 {
    0.25
}

/// Bauteil (Dialog TSBauteil des Originals: Gruppe, Name, Typ, Firma, Artikel, Maße, Nummer).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bauteil {
    pub gruppe: String,
    pub name: String,
    pub typ: String,
    pub firma: String,
    pub artikel: String,
    pub breite: String,
    pub hoehe: String,
    pub tiefe: String,
    pub nummer: String,
}

/// Platzhaltergruppe: Platzhalter %{Name} in Texten werden durch den Wert ersetzt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Platzhalter {
    pub name: String,
    pub werte: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub name: String,
    pub kunde: String,
    pub owner: String,
    pub von: String,
    pub ort: String,
    pub znr: String,
    pub erstellt: String,
    pub geaendert: String,
    pub abgeschlossen: bool,
    pub anlagen: Vec<Anlage>,
    pub leitungen: Vec<Leitung>,
    pub potentiale: Vec<String>,
    pub stifte: Vec<Stift>,
    pub next_id: u64,
    /// Im Projekt verwendete Symbole (damit das Projekt auch ohne Bibliothek vollständig ist)
    #[serde(default)]
    pub symdefs: Vec<crate::symbols::SymDef>,
    /// Projekteigenes Schriftfeld (leer = Standard-Schriftfeld)
    #[serde(default)]
    pub titleblock: Vec<crate::titleblock::TbOwned>,
    #[serde(default)]
    pub bauteile: Vec<Bauteil>,
    /// Bauteilzuordnung: Objekt-Id des Symbols → Index in `bauteile`
    #[serde(default)]
    pub part_of: std::collections::BTreeMap<u64, usize>,
    #[serde(default)]
    pub platzhalter: Vec<Platzhalter>,
    /// Standardschrift des Projekts (leer = Times New Roman), "Alter Klemmenplan", Datum des letzten Ausdrucks
    #[serde(default)]
    pub schrift: String,
    #[serde(default)]
    pub alter_klemmenplan: bool,
    #[serde(default)]
    pub gedruckt: String,
    /// Nennspannung / Nennstrom je Potential (Dialog TPotential)
    #[serde(default)]
    pub pot_werte: std::collections::BTreeMap<String, (String, String)>,
    /// Bilder des Projekts (PRJ › BLD)
    #[serde(default)]
    pub bilder: Vec<Bild>,
    /// Schriftfeldhöhe in mm (TSchriftfeld "Höhe", 0 = Standard 15 mm) und Maßangaben relativ/absolut
    #[serde(default)]
    pub tb_hoehe: f32,
    #[serde(default)]
    pub tb_absolut: bool,
    /// Gelesene Originaldatei (.epr) – Grundlage für das Zurückschreiben
    #[serde(skip)]
    pub raw: Option<Box<crate::eprfile::EprFile>>,
}

impl Project {
    /// Höhe des Schriftfeldes in mm (Standard 15 mm wie im Original).
    pub fn title_h(&self) -> f32 {
        if self.tb_hoehe > 3.0 {
            self.tb_hoehe
        } else {
            TITLE_H
        }
    }

    /// Platzhalter %{Name} aus den Platzhaltergruppen ersetzen.
    pub fn expand_placeholders(&self, s: &str) -> String {
        if !s.contains("%{") {
            return s.to_string();
        }
        let mut out = s.to_string();
        for g in &self.platzhalter {
            for (k, v) in &g.werte {
                out = out.replace(&format!("%{{{k}}}"), v);
            }
        }
        out
    }
}

pub fn today() -> String {
    chrono::Local::now().format("%d.%m.%Y").to_string()
}

impl Project {
    /// Neues Projekt wie im Original (FUN_004055d8): Anlage 1, Blatt 1, Potentiale, Stifte, Leitung 1, Klemme 1.
    pub fn new_default() -> Self {
        let pot = ["L1", "L2", "L3", "N", "PE", "24VDC", "OV"];
        Self {
            name: String::new(),
            kunde: String::new(),
            owner: String::new(),
            von: String::new(),
            ort: String::new(),
            znr: String::new(),
            erstellt: today(),
            geaendert: "---------".into(),
            abgeschlossen: false,
            anlagen: vec![Anlage {
                name: "Anlage 1".into(),
                sheets: vec![Sheet::new(1)],
                sammelschienen: vec![],
                klemmleisten: vec!["Klemme 1".into()],
                klemmen: vec![],
                schienen: Default::default(),
                set: Default::default(),
                raw: None,
            }],
            leitungen: vec![Leitung { name: "Leitung 1".into(), color: [0, 0, 0], width_mm: 0.25, quer: 1.5, adern: 3, kennz: 0, style: 0, id: 0 }],
            potentiale: pot.iter().map(|s| s.to_string()).collect(),
            stifte: vec![
                Stift { name: "Standard".into(), style: 0, color: [0, 0, 0], width: 0.25 },
                Stift { name: "Gestrichelt".into(), style: 1, color: [0, 0, 0], width: 0.25 },
                Stift { name: "Strich/Punkt".into(), style: 2, color: [0, 0, 0], width: 0.25 },
                Stift { name: "Gepunktet".into(), style: 3, color: [0, 0, 0], width: 0.25 },
            ],
            next_id: 1,
            symdefs: vec![],
            titleblock: vec![],
            bauteile: vec![],
            part_of: Default::default(),
            platzhalter: vec![],
            schrift: String::new(),
            alter_klemmenplan: false,
            gedruckt: String::new(),
            pot_werte: Default::default(),
            bilder: vec![],
            tb_hoehe: 0.0,
            tb_absolut: false,
            raw: None,
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn sheet(&self, r: SheetRef) -> Option<&Sheet> {
        self.anlagen.get(r.anlage)?.sheets.get(r.sheet)
    }
    pub fn sheet_mut(&mut self, r: SheetRef) -> Option<&mut Sheet> {
        self.anlagen.get_mut(r.anlage)?.sheets.get_mut(r.sheet)
    }

    pub fn total_sheets(&self) -> usize {
        self.anlagen.iter().map(|a| a.sheets.len()).sum()
    }

    /// Blatt und Objekt mit der Id `id` (projektweit eindeutig).
    pub fn find_obj(&self, id: u64) -> Option<(SheetRef, &Obj)> {
        self.anlagen.iter().enumerate().find_map(|(ai, a)| {
            a.sheets.iter().enumerate().find_map(|(si, s)| s.objects.iter().find(|o| o.id == id).map(|o| (SheetRef { anlage: ai, sheet: si }, o)))
        })
    }

    /// Sprungziel eines Verweises: zuerst der Gegenverweis (Objekt-Id), sonst Anlage (Name) und Blatt (Nummer).
    /// Rückgabe: Blatt und – falls vorhanden – Objekt-Id des Gegenverweises.
    pub fn resolve_ziel(&self, z: &VerweisZiel) -> Option<(SheetRef, Option<u64>)> {
        if z.obj != 0 {
            if let Some((r, _)) = self.find_obj(z.obj) {
                return Some((r, Some(z.obj)));
            }
        }
        let ai = self.anlagen.iter().position(|a| a.name == z.anlage)?;
        let si = self.anlagen[ai].sheets.iter().position(|s| s.number == z.blatt)?;
        Some((SheetRef { anlage: ai, sheet: si }, None))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SheetRef {
    pub anlage: usize,
    pub sheet: usize,
}

/// Transformation eines Symbol-Lokalpunktes (Drehung in 90°-Schritten, Spiegelung).
pub fn sym_xform(p: P, pos: P, rot: u8, mh: bool, mv: bool) -> P {
    sym_xf(p, pos, rot, mh, mv, 1.0)
}

/// Wie `sym_xform`, mit Maßstab.
pub fn sym_xf(p: P, pos: P, rot: u8, mh: bool, mv: bool, scale: f32) -> P {
    let mut x = p.x * scale;
    let mut y = p.y * scale;
    if mh {
        x = -x;
    }
    if mv {
        y = -y;
    }
    let (rx, ry) = match rot % 4 {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    };
    P::new(pos.x + rx, pos.y + ry)
}

impl Kind {
    /// Alle Stützpunkte (für Verschieben).
    pub fn translate(&mut self, d: P) {
        let mv = |p: &mut P| {
            p.x += d.x;
            p.y += d.y;
        };
        match self {
            Kind::Line { a, b } | Kind::Rect { a, b } | Kind::Triangle { a, b } | Kind::Circle { a, b } => {
                mv(a);
                mv(b)
            }
            Kind::Curve { a, c1, c2, b } => {
                mv(a);
                mv(c1);
                mv(c2);
                mv(b)
            }
            Kind::TextBox { a, b, .. } | Kind::Table { a, b, .. } | Kind::Image { a, b, .. } => {
                mv(a);
                mv(b)
            }
            Kind::Text { pos, style, .. } => {
                mv(pos);
                if let Some((a, b)) = style.rect.as_mut() {
                    mv(a);
                    mv(b);
                }
            }
            Kind::Symbol { pos, .. } | Kind::Terminal { pos, .. } => mv(pos),
            Kind::Wire { pts, .. } | Kind::Poly { pts } => pts.iter_mut().for_each(mv),
            Kind::Verweis { pos, text_pos, .. } => {
                mv(pos);
                mv(text_pos)
            }
            Kind::Busbar { a, x2, .. } => {
                mv(a);
                *x2 += d.x;
            }
        }
        if let Kind::Wire { bez, .. } = self {
            bez.iter_mut().for_each(|(p, _)| mv(p));
        }
    }

    /// Rahmen zur Größenänderung: Objekt um `anchor` mit den Faktoren `sx`/`sy` strecken. Punktbasierte Objekte
    /// (Linien, Verbindungen, Rechtecke, Kreise, Kurven, Dreiecke, Textboxen, Tabellen, Bilder, Polygone,
    /// Sammelschienen) werden gestreckt; Symbole, Texte, Klemmstellen und Verweise nur in ihrer Lage verschoben.
    pub fn scale_about(&mut self, anchor: P, sx: f32, sy: f32) {
        let f = |p: P| P::new(anchor.x + (p.x - anchor.x) * sx, anchor.y + (p.y - anchor.y) * sy);
        let m = |p: &mut P| *p = f(*p);
        match self {
            Kind::Line { a, b }
            | Kind::Rect { a, b }
            | Kind::Triangle { a, b }
            | Kind::Circle { a, b }
            | Kind::TextBox { a, b, .. }
            | Kind::Table { a, b, .. }
            | Kind::Image { a, b, .. } => {
                m(a);
                m(b)
            }
            Kind::Curve { a, c1, c2, b } => {
                m(a);
                m(c1);
                m(c2);
                m(b)
            }
            Kind::Poly { pts } => pts.iter_mut().for_each(m),
            Kind::Wire { pts, bez, .. } => {
                pts.iter_mut().for_each(m);
                bez.iter_mut().for_each(|(p, _)| m(p));
            }
            Kind::Busbar { a, x2, .. } => {
                m(a);
                *x2 = anchor.x + (*x2 - anchor.x) * sx;
            }
            Kind::Symbol { pos, .. } | Kind::Text { pos, .. } | Kind::Terminal { pos, .. } | Kind::Verweis { pos, .. } => {
                let d = f(*pos).sub(*pos);
                self.translate(d);
            }
        }
    }

    /// Zwei-Punkt-Rechteck (falls vorhanden) für Größenänderung.
    pub fn frame_points(&self) -> Option<(P, P)> {
        match self {
            Kind::Line { a, b }
            | Kind::Rect { a, b }
            | Kind::Triangle { a, b }
            | Kind::Circle { a, b }
            | Kind::TextBox { a, b, .. }
            | Kind::Table { a, b, .. }
            | Kind::Image { a, b, .. } => Some((*a, *b)),
            Kind::Curve { a, b, .. } => Some((*a, *b)),
            _ => None,
        }
    }

    pub fn set_frame_points(&mut self, na: P, nb: P) {
        match self {
            Kind::Line { a, b }
            | Kind::Rect { a, b }
            | Kind::Triangle { a, b }
            | Kind::Circle { a, b }
            | Kind::TextBox { a, b, .. }
            | Kind::Table { a, b, .. }
            | Kind::Image { a, b, .. } => {
                *a = na;
                *b = nb
            }
            Kind::Curve { a, c1, c2, b } => {
                let (oa, ob) = (*a, *b);
                let map = |p: P| -> P {
                    let sx = if (ob.x - oa.x).abs() > 1e-4 { (p.x - oa.x) / (ob.x - oa.x) } else { 0.0 };
                    let sy = if (ob.y - oa.y).abs() > 1e-4 { (p.y - oa.y) / (ob.y - oa.y) } else { 0.0 };
                    P::new(na.x + sx * (nb.x - na.x), na.y + sy * (nb.y - na.y))
                };
                *c1 = map(*c1);
                *c2 = map(*c2);
                *a = na;
                *b = nb;
            }
            _ => {}
        }
    }
}

/// Umriss eines Symbols (relativ zur Lage) von einer Lage/Spiegelung in eine andere überführen –
/// wie das Original den gespeicherten Umriss beim Drehen und Spiegeln mitführt.
pub fn retransform_bounds(b: (P, P), old: (u8, bool, bool), new: (u8, bool, bool)) -> (P, P) {
    let inv = |p: P| -> P {
        // Drehung zurück, dann Spiegelung zurück (Umkehrung von sym_xf)
        let (x, y) = match old.0 % 4 {
            0 => (p.x, p.y),
            1 => (p.y, -p.x),
            2 => (-p.x, -p.y),
            _ => (-p.y, p.x),
        };
        P::new(if old.1 { -x } else { x }, if old.2 { -y } else { y })
    };
    let corners = [b.0, P::new(b.1.x, b.0.y), b.1, P::new(b.0.x, b.1.y)];
    let pts: Vec<P> = corners.iter().map(|c| sym_xform(inv(*c), P::default(), new.0, new.1, new.2)).collect();
    let mut mn = P::new(f32::MAX, f32::MAX);
    let mut mx = P::new(f32::MIN, f32::MIN);
    for p in pts {
        mn = P::new(mn.x.min(p.x), mn.y.min(p.y));
        mx = P::new(mx.x.max(p.x), mx.y.max(p.y));
    }
    (mn, mx)
}

/// Spiegelbits des Originals (VSY Feld 18: Bit 16 = "Spiegeln Horizontal" = an der senkrechten Achse,
/// Bit 8 = "Spiegeln Vertikal" = an der waagerechten Achse; das Original dreht zuerst und spiegelt danach)
/// in die Spiegelung vor der Drehung umrechnen, die `sym_xf` benutzt.
pub fn mirror_from_bits(rot: u8, b16: bool, b8: bool) -> (bool, bool) {
    if rot % 2 == 0 {
        (b16, b8)
    } else {
        (b8, b16)
    }
}

/// Umkehrung von `mirror_from_bits`: (Bit 16, Bit 8) aus Drehung und Spiegelung vor der Drehung.
pub fn mirror_to_bits(rot: u8, mh: bool, mv: bool) -> (bool, bool) {
    if rot % 2 == 0 {
        (mh, mv)
    } else {
        (mv, mh)
    }
}
