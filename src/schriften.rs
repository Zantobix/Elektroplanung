//! Schrift je Text (Original: TFont.Name eines Textes, leer = Standardschrift des Projekts).
//!
//! Gezeichnet wird über egui-Schriftfamilien `ep:<Name>|<Schnitt>`, die ein eigener `FontProvider` beim ersten
//! Gebrauch aus den Schriftordnern des Systems lädt (Windows: %WINDIR%\Fonts und die Benutzerschriften,
//! Linux/macOS: die üblichen Ordner, ohne fontconfig). Gesucht wird über die Namenstabelle der Schriftdateien
//! (Familienname wie bei GDI, Name-ID 1/2, zusätzlich die typografische Familie 16/17), also unabhängig vom
//! Dateinamen. Fehlt eine Schrift, wird Times gezeichnet wie bisher.
//!
//! PDF: Die Schrift wird nicht eingebettet; Serifenlose (Arial, Segoe UI, Tahoma …) werden als Helvetica,
//! Schreibmaschinenschriften (Courier New, Consolas …) als Courier, alles andere als Times ausgegeben.

use eframe::egui::{self, FontFamily, FontId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Vorsilbe der Schriftfamilien für Texte mit eigenem Schriftnamen
pub const VORSILBE: &str = "ep:";

/// Times New Roman (bzw. leer) – dafür gibt es die fest geladenen Familien `times*`.
pub fn ist_times(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    n.is_empty() || n == "times new roman" || n == "times"
}

/// Schriftfamilie für einen Schriftnamen und Schnitt.
pub fn familie(name: &str, bold: bool, italic: bool) -> FontFamily {
    if ist_times(name) {
        let fam = match (bold, italic) {
            (false, false) => "times",
            (true, false) => "times_b",
            (false, true) => "times_i",
            (true, true) => "times_bi",
        };
        return FontFamily::Name(fam.into());
    }
    let schnitt = match (bold, italic) {
        (false, false) => "",
        (true, false) => "b",
        (false, true) => "i",
        (true, true) => "bi",
    };
    FontFamily::Name(format!("{VORSILBE}{}|{schnitt}", name.trim()).into())
}

/// Schrift in `px` Pixeln für einen Schriftnamen (leer/Times = Times wie bisher).
pub fn font_id(name: &str, px: f32, bold: bool, italic: bool) -> FontId {
    FontId::new(px.max(1.0), familie(name, bold, italic))
}

/// Schriftname und Schnitt aus einer Familie `familie` zurückgewinnen (für die PDF-Ausgabe).
pub fn aus_familie(fam: &FontFamily) -> (String, bool, bool) {
    let FontFamily::Name(n) = fam else { return (String::new(), false, false) };
    match &**n {
        "times_b" => return (String::new(), true, false),
        "times_i" => return (String::new(), false, true),
        "times_bi" => return (String::new(), true, true),
        _ => {}
    }
    let Some(rest) = n.strip_prefix(VORSILBE) else { return (String::new(), false, false) };
    let (name, schnitt) = rest.rsplit_once('|').unwrap_or((rest, ""));
    (name.to_string(), schnitt.contains('b'), schnitt.contains('i'))
}

/// Art der PDF-Standardschrift
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfArt {
    Times,
    Helvetica,
    Courier,
}

const SERIFENLOS: &[&str] = &[
    "arial", "helvetica", "segoe", "tahoma", "verdana", "calibri", "liberation sans", "arimo", "dejavu sans", "microsoft sans serif",
    "ms sans serif", "trebuchet", "century gothic", "franklin gothic", "gill sans", "lucida sans", "candara", "corbel", "univers",
    "frutiger", "futura", "din", "isocpeur", "isocteur", "noto sans", "open sans", "roboto", "ubuntu", "carlito", "small fonts", "system",
];
const DICKTEN_GLEICH: &[&str] = &["courier", "consolas", "lucida console", "liberation mono", "cousine", "dejavu sans mono", "fixedsys", "terminal", "mono"];

/// PDF-Standardschrift für einen Schriftnamen.
pub fn pdf_art(name: &str) -> PdfArt {
    let n = name.trim().to_ascii_lowercase();
    if DICKTEN_GLEICH.iter().any(|k| n.contains(k)) {
        PdfArt::Courier
    } else if SERIFENLOS.iter().any(|k| n.starts_with(k)) {
        PdfArt::Helvetica
    } else {
        PdfArt::Times
    }
}

/// Ersatzschriften, wenn eine Schrift nicht installiert ist (metrisch gleiche freie Schriften unter Linux).
fn ersatz(name: &str) -> &'static [&'static str] {
    match name {
        "arial" | "helvetica" | "segoe ui" | "microsoft sans serif" | "ms sans serif" | "tahoma" | "verdana" => &["liberation sans", "arimo", "dejavu sans", "arial"],
        "courier new" | "courier" | "consolas" | "lucida console" => &["liberation mono", "cousine", "dejavu sans mono", "courier new"],
        "calibri" => &["carlito", "liberation sans"],
        "cambria" => &["caladea", "liberation serif"],
        _ => &[],
    }
}

/// Eine Schriftdatei (bzw. ein Schnitt in einer .ttc-Sammlung)
#[derive(Clone, Debug, PartialEq)]
pub struct Schnitt {
    pub pfad: PathBuf,
    pub index: u32,
    pub bold: bool,
    pub italic: bool,
}

/// Schriftordner des Systems (EP_SCHRIFTEN ersetzt sie, z. B. für Tests).
fn ordner() -> Vec<(PathBuf, usize)> {
    if let Ok(d) = std::env::var("EP_SCHRIFTEN") {
        return std::env::split_paths(&d).map(|p| (p, 4)).collect();
    }
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let mut v = vec![(PathBuf::from(windir).join("Fonts"), 0)];
        if let Ok(l) = std::env::var("LOCALAPPDATA") {
            v.push((PathBuf::from(l).join("Microsoft\\Windows\\Fonts"), 0));
        }
        v
    }
    #[cfg(not(windows))]
    {
        let var = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
        unix_ordner(var("HOME").as_deref(), var("XDG_DATA_HOME").as_deref(), var("XDG_DATA_DIRS").as_deref())
    }
}

/// Schriftordner unter Linux (XDG: $XDG_DATA_HOME bzw. ~/.local/share, $XDG_DATA_DIRS bzw. /usr/local/share und
/// /usr/share, je mit „fonts“, dazu ~/.fonts) und macOS (/System/Library/Fonts samt Supplemental, /Library/Fonts,
/// ~/Library/Fonts), jeweils mit Suchtiefe. Doppelte Ordner fallen weg.
#[cfg_attr(windows, allow(dead_code))]
fn unix_ordner(home: Option<&str>, xdg_home: Option<&str>, xdg_dirs: Option<&str>) -> Vec<(PathBuf, usize)> {
    let mut v: Vec<(PathBuf, usize)> = vec![];
    let mut add = |p: PathBuf, t: usize| {
        if !v.iter().any(|(q, _)| *q == p) {
            v.push((p, t));
        }
    };
    let home = home.map(PathBuf::from);
    if let Some(x) = xdg_home.map(PathBuf::from).or_else(|| home.as_ref().map(|h| h.join(".local/share"))) {
        add(x.join("fonts"), 4);
    }
    if let Some(h) = &home {
        add(h.join(".fonts"), 4);
    }
    for d in xdg_dirs.unwrap_or("/usr/local/share:/usr/share").split(':').filter(|d| !d.is_empty()) {
        add(PathBuf::from(d).join("fonts"), 4);
    }
    // macOS (Supplemental liegt unter /System/Library/Fonts und wird mit der Suchtiefe erfasst)
    add(PathBuf::from("/System/Library/Fonts"), 2);
    add(PathBuf::from("/Library/Fonts"), 2);
    if let Some(h) = &home {
        add(h.join("Library/Fonts"), 2);
    }
    v
}

fn dateien(dir: &Path, tiefe: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if tiefe > 0 {
                dateien(&p, tiefe - 1, out);
            }
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc")) {
            out.push(p);
        }
    }
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(o)?, *b.get(o + 1)?]))
}
fn be32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*b.get(o)?, *b.get(o + 1)?, *b.get(o + 2)?, *b.get(o + 3)?]))
}

fn lies(f: &mut std::fs::File, pos: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    f.seek(SeekFrom::Start(pos)).ok()?;
    let mut b = vec![0u8; len];
    f.read_exact(&mut b).ok()?;
    Some(b)
}

/// Namen aus einer `name`-Tabelle: (Familie, Schnitt) für Name-ID 1/2 und 16/17 (englisch bevorzugt).
pub fn namen(tab: &[u8]) -> Vec<(String, String)> {
    let n = be16(tab, 2).unwrap_or(0) as usize;
    let strings = be16(tab, 4).unwrap_or(0) as usize;
    // je Name-ID: (Güte, Text)
    let mut beste: HashMap<u16, (u8, String)> = HashMap::new();
    for i in 0..n {
        let r = 6 + i * 12;
        let (Some(plat), Some(enc), Some(lang), Some(id), Some(len), Some(off)) =
            (be16(tab, r), be16(tab, r + 2), be16(tab, r + 4), be16(tab, r + 6), be16(tab, r + 8), be16(tab, r + 10))
        else {
            break;
        };
        if !matches!(id, 1 | 2 | 16 | 17) {
            continue;
        }
        let Some(raw) = tab.get(strings + off as usize..strings + off as usize + len as usize) else { continue };
        let (guete, text) = match plat {
            3 | 0 if plat == 0 || enc == 1 || enc == 10 => {
                let u: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                (if lang == 0x409 { 3 } else { 1 }, String::from_utf16_lossy(&u))
            }
            1 if enc == 0 => (if lang == 0 { 2 } else { 0 }, raw.iter().map(|&c| c as char).collect()),
            _ => continue,
        };
        if beste.get(&id).is_none_or(|(g, _)| guete > *g) {
            beste.insert(id, (guete, text));
        }
    }
    let get = |id: u16| beste.get(&id).map(|(_, t)| t.trim().to_string()).filter(|t| !t.is_empty());
    let mut v = vec![];
    if let Some(f) = get(1) {
        v.push((f, get(2).unwrap_or_else(|| "Regular".into())));
    }
    if let Some(f) = get(16) {
        v.push((f, get(17).or_else(|| get(2)).unwrap_or_else(|| "Regular".into())));
    }
    v
}

/// Schnitte einer Datei: (Index in der Sammlung, Familie, Schnittname). Nur Schriften mit Zeichentabelle und Umrissen.
fn datei_lesen(pfad: &Path) -> Vec<(u32, String, String)> {
    let Ok(mut f) = std::fs::File::open(pfad) else { return vec![] };
    let Some(kopf) = lies(&mut f, 0, 12) else { return vec![] };
    let starts: Vec<u32> = if &kopf[0..4] == b"ttcf" {
        let n = be32(&kopf, 8).unwrap_or(0).min(64) as usize;
        let Some(t) = lies(&mut f, 12, n * 4) else { return vec![] };
        (0..n).filter_map(|i| be32(&t, i * 4)).collect()
    } else {
        vec![0]
    };
    let mut out = vec![];
    for (idx, start) in starts.into_iter().enumerate() {
        let Some(h) = lies(&mut f, start as u64, 12) else { continue };
        let tag = be32(&h, 0).unwrap_or(0);
        if tag != 0x0001_0000 && &h[0..4] != b"OTTO" && &h[0..4] != b"true" {
            continue;
        }
        let nt = be16(&h, 4).unwrap_or(0).min(100) as usize;
        let Some(dir) = lies(&mut f, start as u64 + 12, nt * 16) else { continue };
        let tabelle = |name: &[u8]| (0..nt).find(|i| &dir[i * 16..i * 16 + 4] == name).map(|i| (be32(&dir, i * 16 + 8).unwrap_or(0), be32(&dir, i * 16 + 12).unwrap_or(0)));
        if tabelle(b"cmap").is_none() || (tabelle(b"glyf").is_none() && tabelle(b"CFF ").is_none()) {
            continue;
        }
        let Some((off, len)) = tabelle(b"name") else { continue };
        let Some(tab) = lies(&mut f, off as u64, (len as usize).min(1 << 18)) else { continue };
        for (fam, sub) in namen(&tab) {
            out.push((idx as u32, fam, sub));
        }
    }
    out
}

/// Fett/Kursiv aus dem Schnittnamen ("Bold Italic", "Fett Kursiv", "Oblique" …).
fn schnitt_aus(sub: &str) -> (bool, bool) {
    let s = sub.to_ascii_lowercase();
    (s.contains("bold") || s.contains("fett") || s.contains("black") || s.contains("heavy"), s.contains("italic") || s.contains("oblique") || s.contains("kursiv"))
}

type Verzeichnis = HashMap<String, Vec<Schnitt>>;

/// Verzeichnis aller installierten Schriften (einmal je Programmlauf aufgebaut, Schlüssel = Familie klein).
pub fn verzeichnis() -> &'static Verzeichnis {
    static V: OnceLock<Verzeichnis> = OnceLock::new();
    V.get_or_init(|| {
        let mut files = vec![];
        for (d, t) in ordner() {
            dateien(&d, t, &mut files);
        }
        let mut v: Verzeichnis = HashMap::new();
        for p in files {
            for (index, fam, sub) in datei_lesen(&p) {
                let (bold, italic) = schnitt_aus(&sub);
                let e = v.entry(fam.to_ascii_lowercase()).or_default();
                let s = Schnitt { pfad: p.clone(), index, bold, italic };
                if !e.contains(&s) {
                    e.push(s);
                }
            }
        }
        v
    })
}

/// Namen aller installierten Schriftfamilien (für den Schriftdialog), sortiert.
pub fn familien_namen() -> Vec<String> {
    static N: OnceLock<Vec<String>> = OnceLock::new();
    N.get_or_init(|| {
        let mut files = vec![];
        for (d, t) in ordner() {
            dateien(&d, t, &mut files);
        }
        // Namen in der Schreibweise der Datei (das Verzeichnis kennt nur Kleinbuchstaben)
        let mut v: Vec<String> = files.iter().flat_map(|p| datei_lesen(p)).map(|(_, f, _)| f).filter(|f| !f.starts_with('.')).collect();
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    })
    .clone()
}

/// Besten Schnitt aus einer Liste wählen: genau passend, sonst gleiche Stärke, sonst Normal, sonst irgendeinen.
pub fn waehle(liste: &[Schnitt], bold: bool, italic: bool) -> Option<&Schnitt> {
    liste
        .iter()
        .find(|s| s.bold == bold && s.italic == italic)
        .or_else(|| liste.iter().find(|s| s.bold == bold && !s.italic))
        .or_else(|| liste.iter().find(|s| !s.bold && !s.italic))
        .or_else(|| liste.first())
}

/// Schriftdatei für Name und Schnitt suchen (mit Ersatzschriften).
pub fn finden(name: &str, bold: bool, italic: bool) -> Option<Schnitt> {
    let v = verzeichnis();
    let n = name.trim().to_ascii_lowercase();
    std::iter::once(n.as_str()).chain(ersatz(&n).iter().copied()).find_map(|k| v.get(k).and_then(|l| waehle(l, bold, italic)).cloned())
}

/// Gelesene Schriftdateien (Pfad → Inhalt), damit jede Datei nur einmal im Speicher liegt
fn datei_inhalt(p: &Path) -> Option<Arc<Vec<u8>>> {
    static C: OnceLock<Mutex<HashMap<PathBuf, Arc<Vec<u8>>>>> = OnceLock::new();
    let c = C.get_or_init(Default::default);
    if let Some(d) = c.lock().ok()?.get(p) {
        return Some(d.clone());
    }
    let d = Arc::new(std::fs::read(p).ok()?);
    c.lock().ok()?.insert(p.to_path_buf(), d.clone());
    Some(d)
}

/// Lädt die Familien `ep:<Name>|<Schnitt>` beim ersten Gebrauch; dahinter steht die Times-Kette als Ersatz
/// (fehlende Schrift oder fehlende Zeichen).
pub struct Lader {
    pub ersatz: Vec<egui::epaint::text::FontInsert>,
}

impl egui::epaint::text::FontProvider for Lader {
    fn fonts_for_family(&self, family: &FontFamily) -> Vec<egui::epaint::text::FontInsert> {
        let FontFamily::Name(n) = family else { return vec![] };
        if !n.starts_with(VORSILBE) {
            return vec![];
        }
        let (name, bold, italic) = aus_familie(family);
        let mut v = vec![];
        if let Some(s) = finden(&name, bold, italic) {
            if let Some(d) = datei_inhalt(&s.pfad) {
                let blob: egui::epaint::text::Blob = d;
                let data = egui::FontData::from_blob(blob, s.index);
                v.push(egui::epaint::text::FontInsert::new(&format!("datei:{}#{}", s.pfad.display(), s.index), data, vec![]));
            }
        }
        // Times im passenden Schnitt bevorzugt
        let times = match (bold, italic) {
            (true, false) => "times_b",
            (false, true) => "times_i",
            (true, true) => "times_bi",
            _ => "times",
        };
        v.extend(self.ersatz.iter().filter(|f| f.name == times).cloned());
        v.extend(self.ersatz.iter().filter(|f| f.name != times && !f.name.starts_with("times_")).cloned());
        v
    }
}

/// Lader an den Kontext hängen; `defs` = bereits gesetzte Schriften, `kette` = Namen der Ersatzkette.
pub fn einrichten(ctx: &egui::Context, defs: &egui::FontDefinitions, kette: &[String]) {
    let mut ersatz = vec![];
    for name in ["times", "times_b", "times_i", "times_bi"].iter().map(|s| s.to_string()).chain(kette.iter().cloned()) {
        if let Some(d) = defs.font_data.get(&name) {
            if !ersatz.iter().any(|f: &egui::epaint::text::FontInsert| f.name == name) {
                ersatz.push(egui::epaint::text::FontInsert::new(&name, (**d).clone(), vec![]));
            }
        }
    }
    ctx.add_font_provider(Arc::new(Lader { ersatz }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn familie_hin_und_zurueck() {
        assert_eq!(familie("", true, false), FontFamily::Name("times_b".into()));
        assert_eq!(familie("Times New Roman", false, false), FontFamily::Name("times".into()));
        let f = familie("Arial", true, true);
        assert_eq!(f, FontFamily::Name("ep:Arial|bi".into()));
        assert_eq!(aus_familie(&f), ("Arial".to_string(), true, true));
        assert_eq!(aus_familie(&familie("Courier New", false, false)), ("Courier New".to_string(), false, false));
        assert_eq!(aus_familie(&FontFamily::Name("times_i".into())), (String::new(), false, true));
    }

    #[test]
    fn pdf_standardschriften() {
        assert_eq!(pdf_art("Arial"), PdfArt::Helvetica);
        assert_eq!(pdf_art("Segoe UI"), PdfArt::Helvetica);
        assert_eq!(pdf_art("Courier New"), PdfArt::Courier);
        assert_eq!(pdf_art("Consolas"), PdfArt::Courier);
        assert_eq!(pdf_art("Times New Roman"), PdfArt::Times);
        assert_eq!(pdf_art("Garamond"), PdfArt::Times);
    }

    #[test]
    fn schnitt_waehlen() {
        let s = |b, i| Schnitt { pfad: PathBuf::from(format!("{b}{i}")), index: 0, bold: b, italic: i };
        let l = vec![s(false, false), s(true, false)];
        assert_eq!(waehle(&l, true, true).unwrap(), &s(true, false));
        assert_eq!(waehle(&l, false, true).unwrap(), &s(false, false));
        assert_eq!(schnitt_aus("Bold Italic"), (true, true));
        assert_eq!(schnitt_aus("Regular"), (false, false));
    }

    /// Namenstabelle mit einem Windows-Eintrag (Familie) und einem Mac-Eintrag (Schnitt) lesen.
    #[test]
    fn namenstabelle() {
        let fam: Vec<u8> = "Arial".encode_utf16().flat_map(|c| c.to_be_bytes()).collect();
        let sub = b"Bold".to_vec();
        let mut t = vec![];
        t.extend(0u16.to_be_bytes());
        t.extend(2u16.to_be_bytes());
        t.extend((6u16 + 24).to_be_bytes());
        for (plat, enc, lang, id, len, off) in [(3u16, 1u16, 0x409u16, 1u16, fam.len() as u16, 0u16), (1, 0, 0, 2, sub.len() as u16, fam.len() as u16)] {
            for x in [plat, enc, lang, id, len, off] {
                t.extend(x.to_be_bytes());
            }
        }
        t.extend(&fam);
        t.extend(&sub);
        assert_eq!(namen(&t), vec![("Arial".to_string(), "Bold".to_string())]);
    }

    /// Schriftordner unter Linux/macOS (auf jedem System prüfbar, da reine Pfadrechnung).
    #[test]
    fn ordner_linux_macos() {
        let v: Vec<String> = unix_ordner(Some("/home/u"), None, None).into_iter().map(|(p, _)| p.to_string_lossy().replace('\\', "/")).collect();
        assert_eq!(
            v,
            ["/home/u/.local/share/fonts", "/home/u/.fonts", "/usr/local/share/fonts", "/usr/share/fonts", "/System/Library/Fonts", "/Library/Fonts", "/home/u/Library/Fonts"]
        );
        // XDG-Variablen ersetzen die Vorgaben, doppelte Ordner fallen weg
        let v: Vec<String> = unix_ordner(None, Some("/x/data"), Some("/opt/share:/usr/share:/usr/share:")).into_iter().map(|(p, _)| p.to_string_lossy().replace('\\', "/")).collect();
        assert_eq!(v, ["/x/data/fonts", "/opt/share/fonts", "/usr/share/fonts", "/System/Library/Fonts", "/Library/Fonts"]);
    }

    /// Fehlt eine Schrift, liefert der Lader die Times-Kette im passenden Schnitt (fett: times_b zuerst), für
    /// fremde Familien nichts.
    #[test]
    fn rueckfall_auf_times() {
        let data = egui::FontDefinitions::default().font_data.values().next().unwrap().as_ref().clone();
        let ersatz = ["times", "times_b", "times_i", "segoe"].iter().map(|n| egui::epaint::text::FontInsert::new(n, data.clone(), vec![])).collect();
        let l = Lader { ersatz };
        use egui::epaint::text::FontProvider;
        let namen = |f: FontFamily| l.fonts_for_family(&f).into_iter().map(|i| i.name).collect::<Vec<_>>();
        assert!(finden("Gibt Es Nicht 4711", false, false).is_none());
        assert_eq!(namen(familie("Gibt Es Nicht 4711", true, false)), ["times_b", "times", "segoe"]);
        assert_eq!(namen(familie("Gibt Es Nicht 4711", false, false)), ["times", "segoe"]);
        assert!(namen(FontFamily::Name("times".into())).is_empty());
        assert!(namen(FontFamily::Proportional).is_empty());
        // gezeichnet: gleiche Laufweite wie Times
        let ctx = egui::Context::default();
        crate::setup_style(&ctx);
        let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(400.0))), ..Default::default() };
        let _ = ctx.run_ui(input(), |_| {});
        let mut breiten = (0.0, 0.0);
        let _ = ctx.run_ui(input(), |ui| {
            let w = |f: FontId| ui.painter().layout_no_wrap("Wiiiiiiiiide".into(), f, egui::Color32::BLACK).size().x;
            breiten = (w(font_id("Gibt Es Nicht 4711", 20.0, false, false)), w(font_id("", 20.0, false, false)));
        });
        assert!((breiten.0 - breiten.1).abs() < 0.01, "{breiten:?}");
    }

    /// Mit installierter Arial (Windows) wird sie gefunden und ist eine gültige Schrift für egui.
    #[test]
    fn arial_finden_und_zeichnen() {
        let t = std::time::Instant::now();
        let Some(s) = finden("Arial", false, false) else { return };
        eprintln!("Schriftverzeichnis: {:?}, {} Familien", t.elapsed(), verzeichnis().len());
        assert!(!s.bold);
        assert!(finden("arial", true, false).is_some_and(|b| b.bold));
        let ctx = egui::Context::default();
        crate::setup_style(&ctx);
        let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(400.0))), ..Default::default() };
        let _ = ctx.run_ui(input(), |_| {});
        let mut breiten = (0.0, 0.0);
        let _ = ctx.run_ui(input(), |ui| {
            let w = |f: FontId| ui.painter().layout_no_wrap("Wiiiiiiiiiiiiiiide".into(), f, egui::Color32::BLACK).size().x;
            breiten = (w(font_id("Arial", 20.0, false, false)), w(font_id("", 20.0, false, false)));
        });
        // Arial hat andere Laufweiten als Times (i schmaler)
        assert!(breiten.0 < breiten.1 * 0.95, "{breiten:?}");
    }
}
