//! Absturzschutz (Nutzerwunsch 24.11.2026, todo N3): automatisches Zwischenspeichern und Wiederherstellen.
//!
//! - Solange das Projekt ungespeicherte Änderungen hat, schreibt der Nachbau alle `Optionen::sicherung_min` Minuten
//!   eine vollständige Kopie als `.epj` (JSON, nimmt alles auf, auch was das Originalformat nicht kennt) in den
//!   Ordner `wiederherstellung` der Programmeinstellungen – nie neben die Projektdatei. Dazu eine kleine Kopfdatei mit
//!   Projektpfad, Name, Zeit und Prozessnummer.
//! - Nach dem Speichern und beim normalen Beenden wird die Kopie gelöscht.
//! - Findet der Start eine Kopie, deren Programm nicht mehr läuft (Absturz, Stromausfall), fragt er, ob sie
//!   wiederhergestellt werden soll; das Projekt öffnet dann mit seinem alten Pfad und als „geändert“, gespeichert wird
//!   erst auf Wunsch.

use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Ablageordner der Sicherungen.
pub fn ordner() -> PathBuf {
    crate::config_dir().join("wiederherstellung")
}

/// Kopfdaten einer Sicherung.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Kopf {
    /// Pfad der Projektdatei (None = noch nie gespeichert)
    pub pfad: Option<String>,
    pub name: String,
    /// Zeitpunkt der Sicherung (TT.MM.JJJJ hh:mm)
    pub zeit: String,
    pub pid: u32,
}

/// Eine gefundene Sicherung (Kopf und Datei mit dem Projekt).
#[derive(Clone, Debug)]
pub struct Sicherung {
    pub kopf: Kopf,
    pub projekt: PathBuf,
    pub kopf_datei: PathBuf,
}

fn dateien(dir: &Path, pid: u32) -> (PathBuf, PathBuf) {
    (dir.join(format!("{pid}.epj")), dir.join(format!("{pid}.json")))
}

/// Sicherung des Projekts für den Prozess `pid` schreiben (über eine temporäre Datei, wie das Speichern).
pub fn schreiben(dir: &Path, pid: u32, projekt: &Project, pfad: Option<&Path>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (p, k) = dateien(dir, pid);
    let json = serde_json::to_string(projekt).map_err(|e| e.to_string())?;
    crate::meldungen::speichern_ueber_temp(&p, json.as_bytes(), None)?;
    let kopf = Kopf { pfad: pfad.map(|p| p.to_string_lossy().to_string()), name: projekt.name.clone(), zeit: chrono::Local::now().format("%d.%m.%Y %H:%M").to_string(), pid };
    let kj = serde_json::to_string(&kopf).map_err(|e| e.to_string())?;
    crate::meldungen::speichern_ueber_temp(&k, kj.as_bytes(), None)
}

/// Sicherung des Prozesses `pid` entfernen (nach dem Speichern, beim Beenden, nach dem Wiederherstellen).
pub fn entfernen(dir: &Path, pid: u32) {
    let (p, k) = dateien(dir, pid);
    let _ = std::fs::remove_file(p);
    let _ = std::fs::remove_file(k);
}

/// Läuft ein anderes Programm mit der Prozessnummer `pid` noch, dessen Sicherung zuletzt vor `alter` geschrieben
/// wurde? Unter Linux über /proc, sonst gilt eine Sicherung als lebendig, solange sie jünger als drei Intervalle ist
/// (ein laufender Nachbau schreibt sie bei ungespeicherten Änderungen in jedem Intervall neu).
fn laeuft(pid: u32, alter: Duration, intervall: Duration) -> bool {
    #[cfg(target_os = "linux")]
    {
        let _ = (alter, intervall);
        Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        alter < intervall * 3
    }
}

/// Sicherungen anderer, nicht mehr laufender Programme (`eigen` = eigene Prozessnummer), neueste zuerst.
pub fn verwaiste(dir: &Path, eigen: u32, intervall: Duration) -> Vec<Sicherung> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<(SystemTime, Sicherung)> = rd
        .flatten()
        .filter_map(|e| {
            let k = e.path();
            if k.extension().and_then(|x| x.to_str()) != Some("json") {
                return None;
            }
            let kopf: Kopf = serde_json::from_slice(&std::fs::read(&k).ok()?).ok()?;
            let projekt = k.with_extension("epj");
            let zeit = std::fs::metadata(&projekt).and_then(|m| m.modified()).ok()?;
            let alter = SystemTime::now().duration_since(zeit).unwrap_or_default();
            (kopf.pid != eigen && !laeuft(kopf.pid, alter, intervall)).then_some((zeit, Sicherung { kopf, projekt, kopf_datei: k }))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().map(|(_, s)| s).collect()
}

/// Zustand des Zwischenspeicherns im laufenden Programm.
#[derive(Default)]
pub struct Stand {
    /// letzte Sicherung bzw. Beginn der ungespeicherten Änderungen
    pub seit: Option<Instant>,
    /// eine Sicherung dieses Programms liegt im Ordner
    pub geschrieben: bool,
}

impl crate::app::App {
    /// Je Bild: bei ungespeicherten Änderungen nach Ablauf des Intervalls sichern; ohne Änderungen (gespeichert) die
    /// Sicherung entfernen.
    pub fn zwischenspeichern(&mut self) {
        let min = self.opt.sicherung_min;
        if min == 0 || self.sym_edit.is_some() {
            return;
        }
        let dir = ordner();
        if !self.dirty {
            self.sicherung.seit = None;
            if self.sicherung.geschrieben {
                entfernen(&dir, std::process::id());
                self.sicherung.geschrieben = false;
            }
            return;
        }
        let jetzt = Instant::now();
        let seit = *self.sicherung.seit.get_or_insert(jetzt);
        if jetzt.duration_since(seit) >= Duration::from_secs(min as u64 * 60) {
            self.sicherung.seit = Some(jetzt);
            match schreiben(&dir, std::process::id(), &self.project, self.path.as_deref()) {
                Ok(()) => self.sicherung.geschrieben = true,
                Err(e) => self.info = format!("Zwischenspeichern fehlgeschlagen: {e}"),
            }
        }
    }

    /// Beim Start: Sicherungen abgestürzter Programme anbieten (neueste zuerst); Ja öffnet sie mit dem alten Pfad als
    /// geändertes Projekt, Nein verwirft sie.
    pub fn wiederherstellen_anbieten(&mut self) {
        let dir = ordner();
        let intervall = Duration::from_secs(self.opt.sicherung_min.max(1) as u64 * 60);
        for s in verwaiste(&dir, std::process::id(), intervall) {
            let wo = s.kopf.pfad.as_deref().unwrap_or("(noch nicht gespeichert)");
            let text = format!(
                "Elektroplanung wurde nicht ordnungsgemäß beendet.\n\nUngespeicherte Änderungen an „{}“\n{wo}\nvom {} wiederherstellen?\n\n(Nein verwirft die Sicherung.)",
                s.kopf.name, s.kopf.zeit
            );
            if crate::meldungen::frage("Wiederherstellen", &text) {
                self.open_path(s.projekt.clone(), false);
                self.path = s.kopf.pfad.as_ref().map(PathBuf::from);
                // die Sicherungsdatei gehört nicht in „zuletzt geöffnet“, wohl aber das Projekt
                let sicherung = s.projekt.to_string_lossy().to_string();
                self.recent.retain(|r| *r != sicherung);
                if let Some(p) = self.path.clone() {
                    self.remember_recent(&p);
                }
                self.dirty = true;
                self.info = format!("Wiederhergestellt: {} (Stand {})", s.kopf.name, s.kopf.zeit);
                let _ = std::fs::remove_file(&s.projekt);
                let _ = std::fs::remove_file(&s.kopf_datei);
                // nur eine Sicherung öffnen; weitere beim nächsten Start
                return;
            }
            let _ = std::fs::remove_file(&s.projekt);
            let _ = std::fs::remove_file(&s.kopf_datei);
        }
    }

    /// Beim normalen Beenden: eigene Sicherung entfernen.
    pub fn sicherung_beenden(&mut self) {
        entfernen(&ordner(), std::process::id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schreiben_finden_entfernen() {
        let dir = std::env::temp_dir().join(format!("ep_wh_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut p = Project::new_default();
        p.name = "Testprojekt".into();
        // Prozessnummer, die sicher nicht läuft (Linux: kein /proc-Eintrag; sonst: frische Datei, Intervall 0)
        let tot = u32::MAX - 7;
        schreiben(&dir, tot, &p, Some(Path::new("/tmp/x.epr"))).unwrap();
        // eigene Sicherung zählt nicht
        schreiben(&dir, std::process::id(), &p, None).unwrap();
        let v = verwaiste(&dir, std::process::id(), Duration::ZERO);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kopf.name, "Testprojekt");
        assert_eq!(v[0].kopf.pfad.as_deref(), Some("/tmp/x.epr"));
        let q: Project = serde_json::from_slice(&std::fs::read(&v[0].projekt).unwrap()).unwrap();
        assert_eq!(q.name, "Testprojekt");
        entfernen(&dir, tot);
        entfernen(&dir, std::process::id());
        assert!(verwaiste(&dir, std::process::id(), Duration::ZERO).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
