//! Kopieren und Einfügen (Strg+C/Strg+V) gegen das Beispielprojekt (W2.epr): die eingefügten Objekte bekommen
//! neue Kennungen, die gespeicherte Datei enthält keine Kennung doppelt. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{blatt_tasten::test_app, epr, epr_write, eprfile::{Block, EprFile}, model::*};
use std::collections::HashMap;
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn u64_at(d: &[u8], at: usize) -> u64 {
    d.get(at..at + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}

/// Eigene Kennungen der Objekte: OBJ Feld 0/1, VBG Feld 10/11, VSY Byte 96 (Unterblöcke wie OWR/STB/IDL sind
/// Verweise auf andere Objekte und zählen nicht).
fn kennungen(b: &Block, out: &mut HashMap<u64, usize>) {
    let k = if b.is("OBJ") {
        u64_at(&b.data, 0)
    } else if b.is("VBG") {
        u64_at(&b.data, 40)
    } else if b.is("VSY") {
        u64_at(&b.data, 96)
    } else {
        0
    };
    if k != 0 {
        *out.entry(k).or_default() += 1;
    }
    for c in &b.kids {
        kennungen(c, out);
    }
}

fn doppelte(bytes: &[u8]) -> usize {
    let f = EprFile::parse(bytes).unwrap();
    let mut m = HashMap::new();
    for r in &f.roots {
        kennungen(r, &mut m);
    }
    m.values().filter(|&&n| n > 1).count()
}

#[test]
fn eingefuegte_objekte_mit_neuen_kennungen() {
    let Some(path) = sample() else { return };
    let project = epr::import(&path).unwrap();
    let vorher = doppelte(&std::fs::read(&path).unwrap());
    // Blatt mit Symbolen, Leitungen und OBJ-Blöcken (Klemmstelle, Verweis oder Sammelschiene)
    let (r, ids) = project
        .anlagen
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| a.sheets.iter().enumerate().map(move |(si, s)| (SheetRef { anlage: ai, sheet: si }, s)))
        .find_map(|(r, s)| {
            let hat = |f: &dyn Fn(&Obj) -> bool| s.objects.iter().any(|o| o.raw.is_some() && f(o));
            let ok = hat(&|o| matches!(o.kind, Kind::Symbol { .. }))
                && hat(&|o| matches!(o.kind, Kind::Wire { .. }))
                && hat(&|o| matches!(o.kind, Kind::Terminal { .. } | Kind::Verweis { .. } | Kind::Busbar { .. }));
            ok.then(|| (r, s.objects.iter().filter(|o| !matches!(o.kind, Kind::Spiegel { .. })).map(|o| o.id).collect::<Vec<_>>()))
        })
        .expect("Blatt mit Symbol, Leitung und OBJ");
    let mut app = test_app();
    app.project = project;
    app.tabs = vec![r];
    app.active = 0;
    let n0 = app.sheet().unwrap().objects.len();
    app.selection = ids.clone();
    app.copy();
    app.paste();
    app.paste();
    let s = app.sheet().unwrap();
    assert_eq!(s.objects.len(), n0 + 2 * ids.len());
    let bytes = epr_write::to_bytes(&app.project).unwrap();
    assert_eq!(doppelte(&bytes), vorher, "keine Kennung doppelt nach Einfügen");
    assert!(app.sheet().unwrap().objects[n0..].iter().all(|o| o.raw.is_none()), "eingefügte Objekte ohne Originalblock");
    // Rundreise: alle Objekte wieder da
    let tmp = std::env::temp_dir().join(format!("ep_einfuegen_{}.epr", std::process::id()));
    std::fs::write(&tmp, &bytes).unwrap();
    let q = epr::import(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(q.sheet(r).unwrap().objects.len(), n0 + 2 * ids.len());
}
