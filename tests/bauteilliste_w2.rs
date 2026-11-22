//! Bauteilliste mit mehreren Bauteilen je Symbol bzw. Klemmstelle (bauteilliste.rs): ein IDL-Unterblock je
//! Bauteil in Listenreihenfolge, Rundreise an W2.epr. Ohne EP_SAMPLE übersprungen.

use elektroplanung::{bauteilliste, epr, epr_write, eprfile::{Block, EprFile}, model::*};
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let p = std::env::var("EP_SAMPLE").map(PathBuf::from).unwrap_or_default();
    p.exists().then_some(p)
}

fn objekte(p: &Project) -> impl Iterator<Item = &Obj> {
    p.anlagen.iter().flat_map(|a| &a.sheets).flat_map(|s| &s.objects)
}

fn bloecke<'a>(b: &'a Block, tag: &str, out: &mut Vec<&'a Block>) {
    if b.is(tag) {
        out.push(b);
    }
    for k in &b.kids {
        bloecke(k, tag, out);
    }
}

#[test]
fn mehrere_bauteile_rundreise_w2() {
    let Some(path) = sample() else { return };
    let mut p = epr::import(&path).unwrap();
    for (n, t) in [("Schütz", "DILM12"), ("Hilfsschalter", "DILA-XHI11"), ("Klemme", "UK5")] {
        p.bauteile.push(Bauteil { name: n.into(), typ: t.into(), ..Default::default() });
    }
    let sym = objekte(&p).find(|o| matches!(o.kind, Kind::Symbol { .. })).unwrap().id;
    let klemme = objekte(&p).find(|o| matches!(o.kind, Kind::Terminal { .. }) && o.raw.is_some()).unwrap().id;
    bauteilliste::setzen(&mut p, sym, &[0, 1, 2]);
    bauteilliste::setzen(&mut p, klemme, &[2, 0]);

    let out = std::env::temp_dir().join(format!("ep_bauteilliste_{}.epr", std::process::id()));
    epr_write::save(&p, &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let b = epr::import(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let pos = |p: &Project, id: u64| objekte(p).position(|o| o.id == id).unwrap();
    let id_in = |q: &Project, n: usize| objekte(q).nth(n).unwrap().id;
    assert_eq!(bauteilliste::liste(&b, id_in(&b, pos(&p, sym))), vec![0, 1, 2]);
    assert_eq!(bauteilliste::liste(&b, id_in(&b, pos(&p, klemme))), vec![2, 0]);

    // Ablage: je Bauteil ein IDL mit der Kennung (BTL Byte 0..8), im VSY am Ende, in der Klemmstelle vor TIT
    let f = EprFile::parse(&bytes).unwrap();
    assert_eq!(f.bad_checksums, 0);
    let prj = f.root("PRJ").unwrap();
    let btl: Vec<Vec<u8>> = prj.kids_of("BTL").map(|k| k.data[..8].to_vec()).collect();
    let mut vsy = vec![];
    let mut obj = vec![];
    for r in &f.roots {
        bloecke(r, "VSY", &mut vsy);
        bloecke(r, "OBJ", &mut obj);
    }
    let mit = |v: &[&Block]| -> Vec<Vec<Vec<u8>>> { v.iter().map(|b| b.kids_of("IDL").map(|k| k.data.clone()).collect::<Vec<_>>()).filter(|l: &Vec<Vec<u8>>| !l.is_empty()).collect() };
    assert_eq!(mit(&vsy), vec![vec![btl[0].clone(), btl[1].clone(), btl[2].clone()]]);
    assert_eq!(mit(&obj), vec![vec![btl[2].clone(), btl[0].clone()]]);
    let k = obj.iter().find(|b| b.kid("IDL").is_some()).unwrap();
    let tags: Vec<String> = k.kids.iter().map(|k| k.tag_str().to_string()).collect();
    let erst = tags.iter().position(|t| t == "IDL").unwrap();
    assert_eq!(&tags[erst..erst + 2], ["IDL", "IDL"]);
    if let Some(tit) = tags.iter().position(|t| t == "TIT") {
        assert!(tit > erst + 1, "IDL vor TIT: {tags:?}");
    }

    // unverändert erneut gespeichert: byteidentisch
    assert_eq!(epr_write::to_bytes(&b).unwrap(), bytes);

    // Liste kürzen: nur noch ein IDL
    let mut c = b.clone();
    let s2 = id_in(&c, pos(&p, sym));
    bauteilliste::setzen(&mut c, s2, &[1]);
    let f2 = EprFile::parse(&epr_write::to_bytes(&c).unwrap()).unwrap();
    let mut vsy2 = vec![];
    for r in &f2.roots {
        bloecke(r, "VSY", &mut vsy2);
    }
    assert_eq!(mit(&vsy2), vec![vec![btl[1].clone()]]);
}
