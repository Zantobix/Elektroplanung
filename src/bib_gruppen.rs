//! Bibliotheksverwaltung › Symbolgruppen › „Hinzufügen“: leere Gruppen der Benutzerbibliothek. Symbolgruppen
//! entstehen sonst nur über ihre Symbole; damit eine neue Gruppe ohne Platzhalter-Symbol bestehen kann, merkt sich
//! der Nachbau ihren Pfad in `benutzer_gruppen.json` neben `benutzer_symbole.json`. Symbole.tfi bleibt unverändert.
//! Sobald ein Symbol in der Gruppe liegt, zählt sie wie jede andere; die Liste stört dann nicht.

use crate::app::{TAction, TNode};
use crate::symbols::SymDef;
use std::path::Path;

/// Dateiname im Programmordner (`extra::appdir`).
pub const DATEI: &str = "benutzer_gruppen.json";

/// Gruppenpfad aus „A / B“.
pub fn pfad(text: &str) -> Vec<String> {
    text.split(" / ").map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

pub fn laden(dir: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(dir.join(DATEI)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn speichern(dir: &Path, gruppen: &[Vec<String>]) {
    if let Ok(s) = serde_json::to_string(gruppen) {
        let _ = std::fs::write(dir.join(DATEI), s);
    }
}

/// Gibt es die Gruppe (oder eine Untergruppe davon) schon über Symbole?
fn belegt(lib: &[SymDef], p: &[String]) -> bool {
    lib.iter().any(|d| d.group.len() >= p.len() && d.group[..p.len()] == *p)
}

/// Neue leere Gruppe merken; false, wenn der Name leer ist oder es sie schon gibt.
pub fn hinzufuegen(gruppen: &mut Vec<Vec<String>>, lib: &[SymDef], text: &str) -> bool {
    let p = pfad(text);
    if p.is_empty() || belegt(lib, &p) || gruppen.iter().any(|g| g.len() >= p.len() && g[..p.len()] == *p) {
        return false;
    }
    gruppen.push(p);
    true
}

/// Gruppe „Löschen“ (`neu` = None) oder „Umbenennen“: gemerkte Gruppen mit diesem Pfad (und Untergruppen) folgen.
pub fn aendern(gruppen: &mut Vec<Vec<String>>, alt: &str, neu: Option<&str>) -> bool {
    let a = pfad(alt);
    if a.is_empty() {
        return false;
    }
    let n = neu.map(pfad);
    let vorher = gruppen.clone();
    let mut out: Vec<Vec<String>> = vec![];
    for g in gruppen.drain(..) {
        if g.len() >= a.len() && g[..a.len()] == *a {
            match &n {
                Some(n) if !n.is_empty() => out.push(n.iter().cloned().chain(g[a.len()..].iter().cloned()).collect()),
                _ => {}
            }
        } else {
            out.push(g);
        }
    }
    out.dedup();
    *gruppen = out;
    *gruppen != vorher
}

/// Anzeige „A / B“ aller gemerkten Gruppen, die (noch) kein Symbol haben.
pub fn leere(gruppen: &[Vec<String>], lib: &[SymDef]) -> Vec<String> {
    gruppen.iter().filter(|g| !belegt(lib, g)).map(|g| g.join(" / ")).collect()
}

/// Leere Gruppen als Knoten in den Bibliotheksbaum (unter `prefix`, Knoten-Ids wie `view::group_nodes`)
/// einsetzen; Gruppen stehen alphabetisch vor den Symbolen.
pub fn in_baum(nodes: &mut Vec<TNode>, prefix: &str, gruppen: &[Vec<String>]) {
    for g in gruppen {
        einsetzen(nodes, prefix, g);
    }
}

fn einsetzen(nodes: &mut Vec<TNode>, prefix: &str, g: &[String]) {
    let Some(name) = g.first() else { return };
    let id = format!("{prefix}/{name}");
    let i = match nodes.iter().position(|n| n.id == id && n.bold) {
        Some(i) => i,
        None => {
            let at = nodes.iter().position(|n| !n.bold || n.label.as_str() > name.as_str()).unwrap_or(nodes.len());
            nodes.insert(at, TNode { id: id.clone(), label: name.clone(), bold: true, icon: 1, action: TAction::None, children: vec![] });
            at
        }
    };
    einsetzen(&mut nodes[i].children, &id, &g[1..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, gruppe: &[&str]) -> SymDef {
        let mut d = SymDef::empty(&format!("user:200:{name}"), name, "-A%C");
        d.group = gruppe.iter().map(|s| s.to_string()).collect();
        d
    }

    #[test]
    fn gruppe_ohne_platzhalter_symbol() {
        let lib = vec![sym("Lampe", &["Licht"])];
        let mut g = vec![];
        assert!(hinzufuegen(&mut g, &lib, "Antriebe / Motoren"));
        // doppelt, schon vorhanden (über Symbole) oder leer: nichts
        assert!(!hinzufuegen(&mut g, &lib, "Antriebe / Motoren"));
        assert!(!hinzufuegen(&mut g, &lib, "Antriebe"));
        assert!(!hinzufuegen(&mut g, &lib, "Licht"));
        assert!(!hinzufuegen(&mut g, &lib, "  "));
        assert_eq!(leere(&g, &lib), ["Antriebe / Motoren"]);
        // es entsteht kein Symbol
        assert_eq!(lib.len(), 1);
        // sobald ein Symbol drinliegt, ist sie nicht mehr leer
        assert!(leere(&g, &[sym("M", &["Antriebe", "Motoren"])]).is_empty());
        // Umbenennen und Löschen
        assert!(aendern(&mut g, "Antriebe", Some("Maschinen")));
        assert_eq!(g, [vec!["Maschinen".to_string(), "Motoren".to_string()]]);
        assert!(!aendern(&mut g, "Licht", None));
        assert!(aendern(&mut g, "Maschinen / Motoren", None));
        assert!(g.is_empty());
    }

    #[test]
    fn leere_gruppen_im_baum() {
        let k = |id: &str, label: &str, bold: bool, children: Vec<TNode>| TNode { id: id.into(), label: label.into(), bold, icon: 0, action: TAction::None, children };
        let mut nodes = vec![k("u/Licht", "Licht", true, vec![k("u/Licht/sym/x", "Lampe", false, vec![])]), k("u/sym/y", "Einzeln", false, vec![])];
        in_baum(&mut nodes, "u", &[vec!["Antriebe".into(), "Motoren".into()], vec!["Licht".into(), "Neu".into()], vec!["Zubehör".into()]]);
        let labels: Vec<&str> = nodes.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(labels, ["Antriebe", "Licht", "Zubehör", "Einzeln"]);
        assert_eq!(nodes[0].children[0].id, "u/Antriebe/Motoren");
        assert_eq!(nodes[1].children.iter().map(|n| n.label.as_str()).collect::<Vec<_>>(), ["Neu", "Lampe"]);
    }
}
