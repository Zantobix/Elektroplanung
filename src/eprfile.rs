//! Verlustfreies Lesen und Schreiben der Original-Projektdateien (*.epr, Kennung "EP16").
//!
//! Aufbau, ermittelt aus Elektroplanung.exe (Kennungstabelle bei 0x5b9134, "EP16" bei 0x5b92e1)
//! und geprüft an W2.epr:
//! - Dateikopf: 4 Byte "EP16".
//! - Block: 0x99, 3 Zeichen Kennung, u16 Länge, i16 Prüfwert (= !Länge), Daten, u32 Prüfsumme.
//!   Die Prüfsumme ist die Summe aller Datenbytes als *vorzeichenbehaftete* Bytes (i8), als u32.
//! - Unterblöcke folgen direkt auf den Block; "0x99 END" (ohne Länge) schließt ihn.
//! - Bilder (BLD › JPG) über 65535 Byte werden geteilt: Länge 0xFFFF, danach 4 Byte Teilkopf
//!   (Nr., Anzahl, u16 Länge des letzten Teils); die Prüfsumme umfasst nur die Nutzdaten ohne Teilkopf.
//!   Die Folgeteile sind eigene JPG-Köpfe mit Teilkopf; erst nach dem letzten Teil folgt das END.

/// Alle Blockkennungen, die das Original kennt (Tabelle im Datensegment des Originals).
pub const TAGS: &[&str] = &[
    "END", "MAK", "PRJ", "SYM", "LTG", "SIT", "POT", "FUS", "BOX", "BLD", "ERS", "TLI", "PRA", "OPT", "SEI", "BTL", "JPG",
    "ANL", "BLT", "ZCN", "VSY", "ALT", "GRD", "GRS", "STR", "TIT", "WRT", "SMS", "KLM", "KFO", "VBG", "VBP", "OWR", "STL",
    "OBJ", "BZD", "SPA", "GRP", "PFB", "STB", "FKT", "IDL", "DAT",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub tag: [u8; 3],
    pub data: Vec<u8>,
    /// Teilkopf geteilter JPG-Blöcke
    pub chunk: Option<[u8; 4]>,
    /// Folgeteile geteilter JPG-Blöcke (Teilkopf, Nutzdaten)
    pub parts: Vec<([u8; 4], Vec<u8>)>,
    pub kids: Vec<Block>,
    /// Wurde der Block in der Datei mit END geschlossen?
    pub closed: bool,
}

impl Block {
    pub fn new(tag: &str, data: Vec<u8>) -> Self {
        let t = tag.as_bytes();
        Block { tag: [t[0], t[1], t[2]], data, chunk: None, parts: vec![], kids: vec![], closed: true }
    }
    pub fn tag_str(&self) -> &str {
        std::str::from_utf8(&self.tag).unwrap_or("???")
    }
    pub fn is(&self, t: &str) -> bool {
        &self.tag == t.as_bytes()
    }
    pub fn kid(&self, t: &str) -> Option<&Block> {
        self.kids.iter().find(|k| k.is(t))
    }
    pub fn kids_of<'a>(&'a self, t: &'a str) -> impl Iterator<Item = &'a Block> + 'a {
        self.kids.iter().filter(move |k| k.is(t))
    }
    pub fn i32(&self, idx: usize) -> i32 {
        i32_at(&self.data, idx * 4)
    }
    pub fn set_i32(&mut self, idx: usize, v: i32) {
        let p = idx * 4;
        if self.data.len() < p + 4 {
            self.data.resize(p + 4, 0);
        }
        self.data[p..p + 4].copy_from_slice(&v.to_le_bytes());
    }
    /// Anzahl aller Blöcke im Teilbaum (inklusive dieses).
    pub fn count(&self) -> usize {
        1 + self.kids.iter().map(Block::count).sum::<usize>()
    }
}

pub fn i32_at(d: &[u8], p: usize) -> i32 {
    d.get(p..p + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
}

/// Prüfsumme des Originals: Summe der Bytes als i8.
pub fn checksum(d: &[u8]) -> u32 {
    d.iter().fold(0i32, |s, &b| s.wrapping_add(b as i8 as i32)) as u32
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EprFile {
    pub magic: [u8; 4],
    pub roots: Vec<Block>,
    /// Bytes nach dem letzten erkennbaren Block (normalerweise leer)
    pub trailer: Vec<u8>,
    /// Blöcke, deren gespeicherte Prüfsumme nicht stimmte (nur Diagnose)
    pub bad_checksums: usize,
}

fn tag_known(t: &[u8]) -> bool {
    TAGS.iter().any(|x| x.as_bytes() == t)
}

impl EprFile {
    pub fn parse(d: &[u8]) -> Result<EprFile, String> {
        if d.len() < 4 || &d[0..2] != b"EP" {
            return Err("Keine Elektroplanung-Projektdatei (Kennung EP fehlt)".into());
        }
        let magic = [d[0], d[1], d[2], d[3]];
        // Stapel offener Blöcke; Index 0 ist die Wurzelebene
        let mut stack: Vec<Block> = vec![Block::new("PRJ", vec![])];
        let mut p = 4;
        let mut bad = 0;
        while p < d.len() {
            if p + 4 > d.len() || d[p] != 0x99 || !tag_known(&d[p + 1..p + 4]) {
                break;
            }
            let tag = [d[p + 1], d[p + 2], d[p + 3]];
            if &tag == b"END" {
                if stack.len() < 2 {
                    break;
                }
                let mut b = stack.pop().unwrap();
                b.closed = true;
                stack.last_mut().unwrap().kids.push(b);
                p += 4;
                continue;
            }
            if p + 8 > d.len() {
                break;
            }
            let ln = u16::from_le_bytes([d[p + 4], d[p + 5]]) as usize;
            let chk = i16::from_le_bytes([d[p + 6], d[p + 7]]);
            if chk != !(ln as u16 as i16) {
                return Err(format!("Block {} bei {p}: Längenprüfwert falsch", String::from_utf8_lossy(&tag)));
            }
            let mut q = p + 8;
            let mut chunk = None;
            // Folgeteil eines geteilten Bildes? (oberster offener Block ist ein JPG mit Teil Nr. < Anzahl)
            let top = stack.last().unwrap();
            let last_part = top.parts.last().map(|(h, _)| *h).or(top.chunk);
            let continues = &tag == b"JPG" && top.is("JPG") && matches!(last_part, Some(h) if h[0] < h[1]);
            if &tag == b"JPG" && (ln == 0xffff || continues) {
                let h = d.get(q..q + 4).ok_or("Datei endet im JPG-Teilkopf")?;
                chunk = Some([h[0], h[1], h[2], h[3]]);
                q += 4;
            }
            let data = d.get(q..q + ln).ok_or_else(|| format!("Block bei {p} reicht über das Dateiende"))?.to_vec();
            let sum = u32::from_le_bytes(d.get(q + ln..q + ln + 4).ok_or("Prüfsumme fehlt")?.try_into().unwrap());
            if sum != checksum(&data) {
                bad += 1;
            }
            p = q + ln + 4;
            if continues {
                stack.last_mut().unwrap().parts.push((chunk.unwrap(), data));
            } else {
                stack.push(Block { tag, data, chunk, parts: vec![], kids: vec![], closed: false });
            }
        }
        while stack.len() > 1 {
            let b = stack.pop().unwrap();
            stack.last_mut().unwrap().kids.push(b);
        }
        Ok(EprFile { magic, roots: stack.pop().unwrap().kids, trailer: d[p.min(d.len())..].to_vec(), bad_checksums: bad })
    }

    pub fn read(path: &std::path::Path) -> Result<EprFile, String> {
        let d = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&d)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 << 20);
        out.extend_from_slice(&self.magic);
        for b in &self.roots {
            write_block(&mut out, b);
        }
        out.extend_from_slice(&self.trailer);
        out
    }

    pub fn root(&self, t: &str) -> Option<&Block> {
        self.roots.iter().find(|b| b.is(t))
    }
    pub fn root_mut(&mut self, t: &str) -> Option<&mut Block> {
        self.roots.iter_mut().find(|b| b.is(t))
    }
}

fn write_head(out: &mut Vec<u8>, tag: &[u8; 3], chunk: Option<[u8; 4]>, data: &[u8]) {
    let ln = data.len().min(0xffff) as u16;
    out.push(0x99);
    out.extend_from_slice(tag);
    out.extend_from_slice(&ln.to_le_bytes());
    out.extend_from_slice(&(!ln).to_le_bytes());
    if let Some(c) = chunk {
        out.extend_from_slice(&c);
    }
    out.extend_from_slice(&data[..ln as usize]);
    out.extend_from_slice(&checksum(&data[..ln as usize]).to_le_bytes());
}

fn write_block(out: &mut Vec<u8>, b: &Block) {
    write_head(out, &b.tag, b.chunk, &b.data);
    for (h, d) in &b.parts {
        write_head(out, &b.tag, Some(*h), d);
    }
    for k in &b.kids {
        write_block(out, k);
    }
    if b.closed {
        out.extend_from_slice(b"\x99END");
    }
}

/// Feste Textfelder (Windows-1252, mit 0 aufgefüllt) in einen Block schreiben.
pub fn put_str(data: &mut Vec<u8>, from: usize, len: usize, s: &str) {
    if data.len() < from + len {
        data.resize(from + len, 0);
    }
    let enc = encode_cp1252(s);
    for i in 0..len {
        data[from + i] = if i + 1 < len { enc.get(i).copied().unwrap_or(0) } else { 0 };
    }
}

/// Text als nullterminierten Windows-1252-Text (Inhalt eines TIT/STL/WRT-Blocks).
pub fn cstr_bytes(s: &str) -> Vec<u8> {
    let mut v = encode_cp1252(s);
    v.push(0);
    v
}

pub fn encode_cp1252(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '€' => 0x80,
            '‚' => 0x82,
            '„' => 0x84,
            '…' => 0x85,
            '–' => 0x96,
            '—' => 0x97,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            c if (c as u32) < 256 => c as u32 as u8,
            _ => b'?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_signed_byte_sum() {
        assert_eq!(checksum(&[1, 2, 3]), 6);
        assert_eq!(checksum(&[0xff]), 0xffff_ffff);
    }

    #[test]
    fn synthetic_roundtrip() {
        let mut prj = Block::new("PRJ", vec![1, 2, 0x80]);
        prj.kids.push(Block::new("POT", b"L1\0".to_vec()));
        let mut bld = Block::new("BLD", vec![0; 4]);
        let mut jpg = Block::new("JPG", vec![7; 0xffff]);
        jpg.chunk = Some([1, 2, 3, 0]);
        jpg.parts.push(([2, 2, 3, 0], vec![9, 9, 9]));
        bld.kids.push(jpg);
        prj.kids.push(bld);
        let f = EprFile { magic: *b"EP16", roots: vec![prj], trailer: vec![], bad_checksums: 0 };
        let bytes = f.to_bytes();
        assert_eq!(EprFile::parse(&bytes).unwrap(), f);
    }
}
