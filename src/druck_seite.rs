//! Lage des Blattes auf dem Druckerpapier wie in Drucken1Click (0x45d26c, Abschnitt 0x45dc1d–0x45e1b7).
//!
//! Das Original richtet den Drucker je Blatt ein (Hochformat bei BLT +0x80 Bit 0, sonst Querformat), liest
//! HORZRES/VERTRES (bedruckbare Fläche), PHYSICALOFFSETX/Y (nicht bedruckbarer Rand) und LOGPIXELSX und rechnet:
//!
//! 1. Innenmaß = Blatt ohne die Ränder der Anlage (Ränder wie `rahmen::rahmen`, quer gedreht).
//! 2. Ist „An Papiergröße anpassen“ (TForm3 Skal, +0x320) gesetzt, gilt die Antwort „Ja“. Sonst wird – nur wenn das
//!    Innenmaß breiter oder höher als die bedruckbare Fläche ist – einmal je Druckauftrag gefragt (MB_YESNOCANCEL,
//!    Überschrift „Achtung“): „Das Papierformat des Druckers ist zu klein …“. Die Antwort gilt für alle weiteren
//!    Blätter dieses Auftrags. Abbrechen beendet den Druck (bereits gedruckte Blätter bleiben).
//! 3. Ja: Maßstab = kleinerer Wert aus bedruckbarer Höhe / Blatthöhe und Breite / Blattbreite – das ganze Blatt samt
//!    Rändern füllt die bedruckbare Fläche (auch vergrößert, wenn das Papier größer ist). Nein: Maßstab 1.
//! 4. Lage: Der Blattursprung liegt an der Papierkante (Versatz −PHYSICALOFFSET). Wäre der Rand des Blattes dabei
//!    kleiner als der nicht bedruckbare Rand, wird das Blatt verschoben, bis der Rahmen an der bedruckbaren Kante
//!    beginnt (links und oben; rechts und unten wird abgeschnitten).
//!
//! Die Rückfrage „Um das Blatt drucken zu können, muss der Rand verschoben werden! Soll der Rand verschoben werden?“
//! (0x45e147) steht zwar in der Exe, ist aber nicht erreichbar (davor steht ein unbedingter Sprung, kein Sprung führt
//! hin); das Verschieben in Schritt 4 geschieht ohne Rückfrage. Der Nachbau zeigt sie deshalb nicht.
//!
//! Der Nachbau druckt über ein PDF: Die Seite bekommt die Papiergröße, das Blatt wird mit Maßstab und Versatz
//! hineingesetzt und auf die bedruckbare Fläche beschnitten. Papierformat und bedruckbare Fläche kommen vom Drucker
//! (`papier`); ist nichts zu erfahren, bleibt die Seite blattgroß (wie beim PDF-Export).

use crate::meldungen::{self, Antwort, Knoepfe};
use crate::model::{Sheet, SheetRef};

/// Wortlaut des Originals (0x5bb139, mit dem doppelten „für“).
pub const DRUCK_ZU_KLEIN: &str = "Das Papierformat des Druckers ist zu klein für für das zu druckende Format!\nSoll der Ausdruck auf dieses Format verkleinert werden?";

/// Druckerpapier im Hochformat (mm): Größe und nicht bedruckbare Ränder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Papier {
    pub w: f32,
    pub h: f32,
    pub links: f32,
    pub oben: f32,
    pub rechts: f32,
    pub unten: f32,
}

impl Papier {
    /// Papier ohne nicht bedruckbare Ränder.
    pub fn voll(w: f32, h: f32) -> Self {
        Papier { w, h, links: 0.0, oben: 0.0, rechts: 0.0, unten: 0.0 }
    }

    /// Für ein Querformat-Blatt gedreht (wie `rahmen::rahmen`: links ← unten, oben ← links, rechts ← oben,
    /// unten ← rechts).
    pub fn ausgerichtet(&self, hoch: bool) -> Papier {
        if hoch {
            *self
        } else {
            Papier { w: self.h, h: self.w, links: self.unten, oben: self.links, rechts: self.oben, unten: self.rechts }
        }
    }
}

/// Lage eines Blattes auf der Druckseite (alles in mm auf dem Papier).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Platz {
    /// Papiergröße (ausgerichtet wie das Blatt)
    pub papier_w: f32,
    pub papier_h: f32,
    /// Maßstab Blatt → Papier
    pub s: f32,
    /// linke obere Blattecke auf dem Papier
    pub x: f32,
    pub y: f32,
    /// bedruckbare Fläche (x0, y0, x1, y1)
    pub clip: (f32, f32, f32, f32),
}

/// Ist das Innenmaß des Blattes (ohne Ränder) größer als die bedruckbare Fläche? Vergleich in 0,1 mm wie im Original.
pub fn zu_klein(blatt: (f32, f32), raender: (f32, f32, f32, f32), p: &Papier) -> bool {
    let z = |v: f32| (v * 10.0).round() as i64;
    let (l, o, r, u) = raender;
    z(blatt.0 - l - r) > z(p.w - p.links - p.rechts) || z(blatt.1 - o - u) > z(p.h - p.oben - p.unten)
}

/// Lage nach Schritt 3/4 (`anpassen` = Antwort „Ja“). `p` ist bereits wie das Blatt ausgerichtet.
pub fn platz(blatt: (f32, f32), raender: (f32, f32, f32, f32), p: &Papier, anpassen: bool) -> Platz {
    let (bw, bh) = (p.w - p.links - p.rechts, p.h - p.oben - p.unten);
    let s = if anpassen && blatt.0 > 0.0 && blatt.1 > 0.0 { (bh / blatt.1).min(bw / blatt.0) } else { 1.0 };
    let (l, o, _, _) = raender;
    // Versatz gegenüber der bedruckbaren Fläche: −PHYSICALOFFSET, höchstens so weit, dass der Rand beginnt
    let mut dx = -p.links;
    if dx + l * s < 0.0 {
        dx = -l * s;
    }
    let mut dy = -p.oben;
    if dy + o * s < 0.0 {
        dy = -o * s;
    }
    Platz { papier_w: p.w, papier_h: p.h, s, x: p.links + dx, y: p.oben + dy, clip: (p.links, p.oben, p.w - p.rechts, p.h - p.unten) }
}

/// Ränder (links, oben, rechts, unten) des Blattes in mm aus seinem Rahmen.
pub fn raender(s: &Sheet) -> (f32, f32, f32, f32) {
    let (w, h) = s.dims();
    let (x0, y0, x1, y1) = s.frame();
    (x0, y0, w - x1, h - y1)
}

/// Lage aller zu druckenden Blätter mit der Rückfrage des Originals. `None` = Abbrechen.
/// Ohne bekanntes Papier gibt es keine Lage (Seite = Blatt).
pub fn einrichten(blaetter: &[(SheetRef, &Sheet)], papier: Option<Papier>, anpassen: bool) -> Option<Vec<(SheetRef, Platz)>> {
    let Some(papier) = papier else { return Some(vec![]) };
    let mut antwort: Option<Antwort> = anpassen.then_some(Antwort::Ja);
    let mut out = vec![];
    for (r, s) in blaetter {
        let p = papier.ausgerichtet(s.portrait);
        let (dims, rd) = (s.dims(), raender(s));
        let a = match antwort {
            Some(a) => a,
            None if zu_klein(dims, rd, &p) => {
                let a = meldungen::zeigen(meldungen::T_ACHTUNG, DRUCK_ZU_KLEIN, Knoepfe::JaNeinAbbrechen, false);
                antwort = Some(a);
                a
            }
            None => Antwort::Nein,
        };
        if a == Antwort::Abbrechen {
            return None;
        }
        out.push((*r, platz(dims, rd, &p, a == Antwort::Ja)));
    }
    Some(out)
}

// ------------------------------------------------------------------ Papier des Druckers

/// Papiergrößen nach Namen (mm, hoch).
pub fn papier_nach_name(name: &str) -> Option<(f32, f32)> {
    let n = name.trim().to_ascii_lowercase();
    let n = n.trim_start_matches("iso_").split(['_', ' ', '/']).next().unwrap_or("");
    Some(match n {
        "a2" => (420.0, 594.0),
        "a3" => (297.0, 420.0),
        "a4" => (210.0, 297.0),
        "a5" => (148.0, 210.0),
        "a6" => (105.0, 148.0),
        "letter" | "na" => (215.9, 279.4),
        "legal" => (215.9, 355.6),
        "tabloid" | "ledger" | "11x17" => (279.4, 431.8),
        _ => return None,
    })
}

/// Papier aus einer PPD-Datei: `*PaperDimension` und `*ImageableArea` des Formats `name` (Punkte).
pub fn papier_aus_ppd(ppd: &str, name: &str) -> Option<Papier> {
    let werte = |key: &str| -> Option<Vec<f32>> {
        ppd.lines().find_map(|l| {
            let rest = l.strip_prefix(key)?.trim_start();
            let (opt, v) = rest.split_once(':')?;
            if opt.split('/').next()? != name {
                return None;
            }
            let v: Vec<f32> = v.trim().trim_matches('"').split_whitespace().filter_map(|x| x.parse().ok()).collect();
            Some(v)
        })
    };
    let mm = |pt: f32| pt * 25.4 / 72.0;
    let d = werte("*PaperDimension")?;
    let (w, h) = (mm(*d.first()?), mm(*d.get(1)?));
    let mut p = Papier::voll(w, h);
    if let Some(a) = werte("*ImageableArea").filter(|a| a.len() == 4) {
        // l b r t, Ursprung unten links
        p.links = mm(a[0]);
        p.unten = mm(a[1]);
        p.rechts = w - mm(a[2]);
        p.oben = h - mm(a[3]);
    }
    Some(p)
}

/// Gewähltes Papierformat aus `lpoptions -l` (Zeile PageSize/media, Eintrag mit „*“).
pub fn format_aus_lpoptions(text: &str) -> Option<String> {
    text.lines()
        .filter(|l| {
            let k = l.split(['/', ':']).next().unwrap_or("").to_ascii_lowercase();
            k == "pagesize" || k == "media"
        })
        .find_map(|l| l.split_once(':')?.1.split_whitespace().find_map(|t| t.strip_prefix('*').map(str::to_string)))
}

fn ausgabe(cmd: &str, args: &[&str]) -> Option<String> {
    let o = std::process::Command::new(cmd).args(args).env("LC_ALL", "C").output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Papier des Druckers (`None` = Standarddrucker). Linux/macOS über CUPS (`lpoptions`, PPD in /etc/cups/ppd),
/// Windows über `Get-PrintConfiguration` (ohne nicht bedruckbare Ränder).
#[cfg(not(windows))]
pub fn papier(drucker: Option<&str>) -> Option<Papier> {
    let name = match drucker {
        Some(n) => n.to_string(),
        None => ausgabe("lpstat", &["-d"])?.split(':').nth(1)?.trim().to_string(),
    };
    if name.is_empty() {
        return None;
    }
    let format = format_aus_lpoptions(&ausgabe("lpoptions", &["-p", &name, "-l"])?)?;
    if let Some(p) = std::fs::read_to_string(format!("/etc/cups/ppd/{name}.ppd")).ok().and_then(|ppd| papier_aus_ppd(&ppd, &format)) {
        return Some(p);
    }
    papier_nach_name(&format).map(|(w, h)| Papier::voll(w, h))
}

#[cfg(windows)]
pub fn papier(drucker: Option<&str>) -> Option<Papier> {
    let ziel = match drucker {
        Some(n) => format!("'{}'", n.replace('\'', "''")),
        None => "(Get-CimInstance Win32_Printer -Filter 'Default=true').Name".to_string(),
    };
    let t = ausgabe("powershell", &["-NoProfile", "-Command", &format!("(Get-PrintConfiguration -PrinterName {ziel}).PaperSize")])?;
    papier_nach_name(t.trim()).map(|(w, h)| Papier::voll(w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: Papier = Papier { w: 210.0, h: 297.0, links: 4.0, oben: 4.0, rechts: 4.0, unten: 4.0 };

    #[test]
    fn a4_blatt_auf_a4_passt() {
        // W2 quer: Ränder links 5, oben 10, rechts 5, unten 5; Innenmaß 287 × 195 auf 289 × 202 bedruckbar
        let p = A4.ausgerichtet(false);
        assert_eq!((p.w, p.h), (297.0, 210.0));
        let r = (5.0, 10.0, 5.0, 5.0);
        assert!(!zu_klein((297.0, 210.0), r, &p));
        let pl = platz((297.0, 210.0), r, &p, false);
        assert_eq!((pl.s, pl.x, pl.y), (1.0, 0.0, 0.0));
        assert_eq!(pl.clip, (4.0, 4.0, 293.0, 206.0));
    }

    #[test]
    fn rand_kleiner_als_druckrand_verschiebt() {
        // Rand 2 mm, Druckrand 4 mm: Rahmen beginnt an der bedruckbaren Kante
        let pl = platz((200.0, 280.0), (2.0, 3.0, 2.0, 2.0), &A4, false);
        assert_eq!((pl.x, pl.y), (2.0, 1.0));
    }

    #[test]
    fn a3_auf_a4_verkleinert() {
        let p = A4.ausgerichtet(false);
        let r = (5.0, 10.0, 5.0, 5.0);
        assert!(zu_klein((420.0, 297.0), r, &p));
        let pl = platz((420.0, 297.0), r, &p, true);
        // kleinerer Maßstab: Breite 289/420 = 0,688; Höhe 202/297 = 0,680
        assert!((pl.s - 202.0 / 297.0).abs() < 1e-6);
        // links Rand 5·s = 3,4 < 4: Blatt rückt nach innen, bis der Rahmen an der bedruckbaren Kante beginnt;
        // oben 10·s = 6,8 ≥ 4: an der Papierkante
        assert!((pl.x + 5.0 * pl.s - 4.0).abs() < 1e-5);
        assert_eq!(pl.y, 0.0);
        // Nein: 1:1, rechts/unten abgeschnitten
        assert_eq!(platz((420.0, 297.0), r, &p, false).s, 1.0);
    }

    #[test]
    fn anpassen_vergroessert() {
        let a3 = Papier::voll(297.0, 420.0).ausgerichtet(false);
        let pl = platz((297.0, 210.0), (5.0, 10.0, 5.0, 5.0), &a3, true);
        assert!((pl.s - 420.0 / 297.0).abs() < 1e-6);
    }

    #[test]
    fn rueckfrage_einmal_je_auftrag() {
        let mut s = Sheet::new(1);
        s.raender = crate::rahmen::RAENDER_W2;
        let a5 = Papier::voll(148.0, 210.0);
        let r = SheetRef { anlage: 0, sheet: 0 };
        let r2 = SheetRef { anlage: 0, sheet: 1 };
        meldungen::test_antworten(&[Antwort::Nein]);
        let v = einrichten(&[(r, &s), (r2, &s)], Some(a5), false).unwrap();
        assert_eq!(meldungen::test_gezeigt(), vec![(meldungen::T_ACHTUNG.to_string(), DRUCK_ZU_KLEIN.to_string())]);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|(_, p)| p.s == 1.0));
        // Anpassen gesetzt: keine Rückfrage, verkleinert
        meldungen::test_antworten(&[]);
        let v = einrichten(&[(r, &s)], Some(a5), true).unwrap();
        assert!(meldungen::test_gezeigt().is_empty());
        assert!(v[0].1.s < 1.0);
        // Abbrechen
        meldungen::test_antworten(&[Antwort::Abbrechen]);
        assert!(einrichten(&[(r, &s)], Some(a5), false).is_none());
        // ohne Papier: nichts zu tun
        assert_eq!(einrichten(&[(r, &s)], None, false), Some(vec![]));
    }

    #[test]
    fn ppd_und_lpoptions() {
        let ppd = "*PaperDimension A4/A4: \"595 842\"\n*ImageableArea A4/A4: \"12 12 583 830\"\n*ImageableArea Letter/Letter: \"0 0 1 1\"\n";
        let p = papier_aus_ppd(ppd, "A4").unwrap();
        assert!((p.w - 209.9).abs() < 0.1 && (p.h - 297.0).abs() < 0.1);
        assert!((p.links - 4.23).abs() < 0.01 && (p.oben - 4.23).abs() < 0.01);
        assert_eq!(format_aus_lpoptions("PageSize/Media Size: Letter *A4 A3\nDuplex/2-Sided: *None"), Some("A4".into()));
        assert_eq!(papier_nach_name("iso_a3_297x420mm"), Some((297.0, 420.0)));
        assert_eq!(papier_nach_name("A4"), Some((210.0, 297.0)));
    }
}
