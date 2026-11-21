//! Eingebettete TrueType-Schriften im PDF (Texte mit eigener Schrift, `TextStyle::font`).
//!
//! Jede verwendete Schrift (Datei und Schnitt wie beim Zeichnen, `schriften::finden`) wird als Teilmenge
//! eingebettet: Type0-Schrift mit Identity-H, CIDFontType2 mit CIDToGIDMap Identity, also Glyphennummern direkt
//! im Text. Die Teilmenge behält die Glyphennummern (nicht benutzte Glyphen werden leer, zusammengesetzte Glyphen
//! bringen ihre Teile mit) und nur die Tabellen, die das PDF für TrueType braucht (head, hhea, hmtx, maxp, loca,
//! glyf, cvt, fpgm, prep). Dazu eine ToUnicode-Tabelle, damit der Text kopier- und durchsuchbar bleibt.
//!
//! Rückfall auf die Standardschriften (Times/Helvetica/Courier, `schriften::pdf_art`) wie bisher: Times bzw. leerer
//! Name, nicht installierte Schriften, Schriften ohne glyf-Umrisse (CFF/OpenType-PS), Schriften, deren
//! Lizenzbits (OS/2 fsType) das Einbetten verbieten, und einzelne Zeichen, die der Schrift fehlen.

use skrifa::instance::{LocationRef, Size};
use skrifa::{MetadataProvider, Tag};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

/// Eine eingebettete Schrift
struct Schrift {
    daten: Arc<Vec<u8>>,
    index: u32,
    /// PostScript-Name (ohne Leer- und Sonderzeichen)
    name: String,
    upem: f32,
    /// benutzte Glyphen: Nummer → (Vorschub in Schrifteinheiten, Zeichen)
    glyphen: BTreeMap<u16, (f32, char)>,
}

/// Die Schriften eines PDF
#[derive(Default)]
pub struct Schriften {
    liste: Vec<Schrift>,
    /// (Name klein, fett, kursiv) → Index in `liste` (`None` = Standardschrift)
    nach_name: HashMap<(String, bool, bool), Option<usize>>,
    nach_datei: HashMap<(PathBuf, u32), usize>,
}

/// Darf und kann die Schrift eingebettet werden? (TrueType-Umrisse, fsType nicht „eingeschränkt“ bzw. „nur Bitmap“)
fn einbettbar(f: &skrifa::FontRef) -> bool {
    if f.table_data(Tag::new(b"glyf")).is_none() || f.table_data(Tag::new(b"loca")).is_none() || f.table_data(Tag::new(b"head")).is_none() {
        return false;
    }
    let fs_type = f.table_data(Tag::new(b"OS/2")).and_then(|d| be16(d.as_bytes(), 8)).unwrap_or(0);
    fs_type & 0x000f != 0x0002 && fs_type & 0x0200 == 0
}

impl Schriften {
    /// Eingebettete Schrift für Name und Schnitt (`None` = Standardschrift).
    pub fn schrift(&mut self, name: &str, bold: bool, italic: bool) -> Option<usize> {
        if crate::schriften::ist_times(name) {
            return None;
        }
        self.laden(name, bold, italic)
    }

    /// Eingebettete Ersatzschrift für ein Zeichen, das die Standardschriften (WinAnsi) nicht kennen (z. B. Ω, →, ✓):
    /// Times New Roman, sonst Schriften mit vielen Symbolen.
    pub fn ersatz(&mut self, c: char, bold: bool, italic: bool) -> Option<usize> {
        ["Times New Roman", "Segoe UI Symbol", "DejaVu Sans", "Arial Unicode MS"]
            .iter()
            .find_map(|n| self.laden(n, bold, italic).filter(|&k| self.glyphe(k, c).is_some()))
    }

    fn laden(&mut self, name: &str, bold: bool, italic: bool) -> Option<usize> {
        let key = (name.trim().to_ascii_lowercase(), bold, italic);
        if let Some(k) = self.nach_name.get(&key) {
            return *k;
        }
        let k = (|| {
            let s = crate::schriften::finden(name, bold, italic)?;
            if let Some(k) = self.nach_datei.get(&(s.pfad.clone(), s.index)) {
                return Some(*k);
            }
            let daten = crate::schriften::datei_inhalt(&s.pfad)?;
            let k = self.hinzu(daten, s.index)?;
            self.nach_datei.insert((s.pfad, s.index), k);
            Some(k)
        })();
        self.nach_name.insert(key, k);
        k
    }

    /// Schriftdatei aufnehmen (auch für Tests mit eigenen Daten); `None`, wenn sie nicht einbettbar ist.
    pub fn hinzu(&mut self, daten: Arc<Vec<u8>>, index: u32) -> Option<usize> {
        let f = skrifa::FontRef::from_index(&daten, index).ok()?;
        if !einbettbar(&f) {
            return None;
        }
        let upem = f.metrics(Size::unscaled(), LocationRef::default()).units_per_em as f32;
        let ps = f
            .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
            .english_or_first()
            .or_else(|| f.localized_strings(skrifa::string::StringId::FAMILY_NAME).english_or_first())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let mut name: String = ps.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        if name.is_empty() {
            name = format!("Schrift{}", self.liste.len() + 1);
        }
        self.liste.push(Schrift { daten, index, name, upem: upem.max(1.0), glyphen: BTreeMap::new() });
        Some(self.liste.len() - 1)
    }

    /// Glyphe eines Zeichens in Schrift `k` (merkt sie für die Teilmenge vor); `None`, wenn sie fehlt.
    /// Rückgabe: Glyphennummer und Vorschub in 1/1000 Geviert.
    pub fn glyphe(&mut self, k: usize, c: char) -> Option<(u16, f32)> {
        let s = self.liste.get_mut(k)?;
        let f = skrifa::FontRef::from_index(&s.daten, s.index).ok()?;
        let g = f.charmap().map(c)?;
        let gid = u16::try_from(g.to_u32()).ok().filter(|g| *g != 0)?;
        let w = f.glyph_metrics(Size::unscaled(), LocationRef::default()).advance_width(g).unwrap_or(0.0);
        s.glyphen.entry(gid).or_insert((w, c));
        Some((gid, w * 1000.0 / s.upem))
    }

    /// Name der Schrift im Ressourcenverzeichnis der Seiten
    pub fn res(k: usize) -> String {
        format!("E{}", k + 1)
    }

    /// Anzahl der eingebetteten Schriften
    pub fn anzahl(&self) -> usize {
        self.liste.len()
    }

    /// PDF-Objekte aller Schriften, durchnummeriert ab `erste`: je Schrift Type0, CIDFont, FontDescriptor,
    /// FontFile2, ToUnicode. Rückgabe: Objekte und Einträge für das Schriftverzeichnis (`/E1 12 0 R` …).
    pub fn objekte(&self, erste: usize) -> (Vec<Vec<u8>>, Vec<String>) {
        let mut objs = vec![];
        let mut eintraege = vec![];
        for (k, s) in self.liste.iter().enumerate() {
            let n = erste + objs.len();
            let gids: BTreeSet<u16> = s.glyphen.keys().copied().collect();
            let f = skrifa::FontRef::from_index(&s.daten, s.index).ok();
            let Some((f, datei)) = f.and_then(|f| teilmenge(&f, &gids).map(|d| (f, d))) else {
                // sollte nach `einbettbar` nicht vorkommen; das PDF bleibt gültig
                eintraege.push(format!("/{} << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>", Self::res(k)));
                continue;
            };
            let basis = format!("{}+{}", kennung(&s.name, &gids), s.name);
            let e = |v: f32| (v * 1000.0 / s.upem).round() as i32;
            // Breiten: je Glyphe [w]
            let w: String = s.glyphen.iter().map(|(g, (adv, _))| format!("{g} [{}]", e(*adv))).collect::<Vec<_>>().join(" ");
            objs.push(format!("<< /Type /Font /Subtype /Type0 /BaseFont /{basis} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>", n + 1, n + 4).into_bytes());
            objs.push(
                format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{basis} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW 1000 /W [{w}] /CIDToGIDMap /Identity >>",
                    n + 2
                )
                .into_bytes(),
            );
            let m = f.metrics(Size::unscaled(), LocationRef::default());
            let head = f.table_data(Tag::new(b"head")).map(|d| d.as_bytes().to_vec()).unwrap_or_default();
            let bb: Vec<i32> = [36, 38, 40, 42].iter().map(|&o| e(be16(&head, o).unwrap_or(0) as i16 as f32)).collect();
            let flags = 32 | if m.is_monospace { 1 } else { 0 } | if m.italic_angle != 0.0 { 64 } else { 0 };
            objs.push(
                format!(
                    "<< /Type /FontDescriptor /FontName /{basis} /Flags {flags} /FontBBox [{} {} {} {}] /ItalicAngle {:.1} /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
                    bb[0],
                    bb[1],
                    bb[2],
                    bb[3],
                    m.italic_angle,
                    e(m.ascent),
                    e(m.descent),
                    e(m.cap_height.unwrap_or(m.ascent)),
                    n + 3
                )
                .into_bytes(),
            );
            objs.push(strom(&format!("/Length1 {}", datei.len()), &datei));
            objs.push(strom("", to_unicode(&s.glyphen).as_bytes()));
            eintraege.push(format!("/{} {n} 0 R", Self::res(k)));
        }
        (objs, eintraege)
    }
}

/// Datenstrom (mit Flate gepackt) als PDF-Objekt
pub(crate) fn strom(extra: &str, daten: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
    let _ = z.write_all(daten);
    let gepackt = z.finish().unwrap_or_default();
    let mut o = format!("<< /Length {} /Filter /FlateDecode {extra} >>\nstream\n", gepackt.len()).into_bytes();
    o.extend_from_slice(&gepackt);
    o.extend_from_slice(b"\nendstream");
    o
}

/// Kennung der Teilmenge (sechs Großbuchstaben, aus Name und Glyphen gebildet)
fn kennung(name: &str, gids: &BTreeSet<u16>) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in name.bytes().chain(gids.iter().flat_map(|g| g.to_be_bytes())) {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    (0..6).map(|i| (b'A' + ((h >> (i * 5)) % 26) as u8) as char).collect()
}

/// ToUnicode-Tabelle (Glyphe → Zeichen)
fn to_unicode(glyphen: &BTreeMap<u16, (f32, char)>) -> String {
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let v: Vec<_> = glyphen.iter().collect();
    for block in v.chunks(100) {
        s.push_str(&format!("{} beginbfchar\n", block.len()));
        for (g, (_, c)) in block {
            let u: String = c.encode_utf16(&mut [0; 2]).iter().map(|x| format!("{x:04X}")).collect();
            s.push_str(&format!("<{g:04X}> <{u}>\n"));
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(o)?, *b.get(o + 1)?]))
}
fn be32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*b.get(o)?, *b.get(o + 1)?, *b.get(o + 2)?, *b.get(o + 3)?]))
}

/// Teilmenge einer TrueType-Schrift mit den Glyphen `gids` (und Glyphe 0), Glyphennummern bleiben erhalten.
pub fn teilmenge(f: &skrifa::FontRef, gids: &BTreeSet<u16>) -> Option<Vec<u8>> {
    let tab = |t: &[u8; 4]| f.table_data(Tag::new(t)).map(|d| d.as_bytes());
    let (head, loca, glyf, maxp) = (tab(b"head")?, tab(b"loca")?, tab(b"glyf")?, tab(b"maxp")?);
    let n = be16(maxp, 4)? as usize;
    let lang = be16(head, 50)? != 0;
    let lage = |g: usize| -> Option<(usize, usize)> {
        let (a, b) = if lang { (be32(loca, g * 4)? as usize, be32(loca, g * 4 + 4)? as usize) } else { (be16(loca, g * 2)? as usize * 2, be16(loca, g * 2 + 2)? as usize * 2) };
        (a <= b && b <= glyf.len()).then_some((a, b))
    };
    // zusammengesetzte Glyphen bringen ihre Teile mit
    let mut behalten: BTreeSet<u16> = gids.iter().copied().filter(|g| (*g as usize) < n).collect();
    behalten.insert(0);
    let mut offen: Vec<u16> = behalten.iter().copied().collect();
    while let Some(g) = offen.pop() {
        let Some((a, b)) = lage(g as usize) else { continue };
        let d = &glyf[a..b];
        if d.len() < 10 || (be16(d, 0)? as i16) >= 0 {
            continue;
        }
        let mut p = 10;
        loop {
            let (Some(flags), Some(teil)) = (be16(d, p), be16(d, p + 2)) else { break };
            if (teil as usize) < n && behalten.insert(teil) {
                offen.push(teil);
            }
            p += 4 + if flags & 0x0001 != 0 { 4 } else { 2 };
            p += if flags & 0x0008 != 0 {
                2
            } else if flags & 0x0040 != 0 {
                4
            } else if flags & 0x0080 != 0 {
                8
            } else {
                0
            };
            if flags & 0x0020 == 0 {
                break;
            }
        }
    }
    // neue glyf- und loca-Tabelle (lang)
    let mut nglyf = vec![];
    let mut nloca = Vec::with_capacity((n + 1) * 4);
    for g in 0..n {
        nloca.extend_from_slice(&(nglyf.len() as u32).to_be_bytes());
        if behalten.contains(&(g as u16)) {
            if let Some((a, b)) = lage(g) {
                nglyf.extend_from_slice(&glyf[a..b]);
                while nglyf.len() % 4 != 0 {
                    nglyf.push(0);
                }
            }
        }
    }
    nloca.extend_from_slice(&(nglyf.len() as u32).to_be_bytes());
    let mut nhead = head.to_vec();
    if nhead.len() < 54 {
        return None;
    }
    nhead[8..12].copy_from_slice(&[0; 4]); // checkSumAdjustment
    nhead[50..52].copy_from_slice(&1u16.to_be_bytes());
    let mut tabellen: Vec<([u8; 4], Vec<u8>)> = vec![(*b"glyf", nglyf), (*b"head", nhead), (*b"loca", nloca), (*b"maxp", maxp.to_vec())];
    for t in [b"cvt ", b"fpgm", b"hhea", b"hmtx", b"prep"] {
        if let Some(d) = tab(t) {
            tabellen.push((*t, d.to_vec()));
        }
    }
    tabellen.sort_by(|a, b| a.0.cmp(&b.0));
    Some(sfnt(&tabellen))
}

/// Schriftdatei aus Tabellen (nach Kennung sortiert) zusammensetzen
fn sfnt(tabellen: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let nt = tabellen.len() as u16;
    let mut pot = 1u16;
    let mut sel = 0u16;
    while pot * 2 <= nt {
        pot *= 2;
        sel += 1;
    }
    let mut out = vec![];
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    for v in [nt, pot * 16, sel, nt * 16 - pot * 16] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut off = 12 + 16 * tabellen.len();
    let mut daten = vec![];
    for (tag, d) in tabellen {
        let mut sum: u32 = 0;
        for c in d.chunks(4) {
            let mut w = [0u8; 4];
            w[..c.len()].copy_from_slice(c);
            sum = sum.wrapping_add(u32::from_be_bytes(w));
        }
        out.extend_from_slice(tag);
        out.extend_from_slice(&sum.to_be_bytes());
        out.extend_from_slice(&(off as u32).to_be_bytes());
        out.extend_from_slice(&(d.len() as u32).to_be_bytes());
        daten.extend_from_slice(d);
        while daten.len() % 4 != 0 {
            daten.push(0);
        }
        off = 12 + 16 * tabellen.len() + daten.len();
    }
    out.extend_from_slice(&daten);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use skrifa::raw::TableProvider;

    fn hack() -> Arc<Vec<u8>> {
        let defs = eframe::egui::FontDefinitions::default();
        Arc::new(defs.font_data.get("Hack").unwrap().bytes().to_vec())
    }

    /// Teilmenge der egui-Schrift Hack: wieder lesbar, gleiche Glyphenzahl und Vorschübe, unbenutzte Glyphen leer,
    /// deutlich kleiner als die ganze Schrift.
    #[test]
    fn teilmenge_bleibt_lesbar() {
        let d = hack();
        let f = skrifa::FontRef::new(&d).unwrap();
        let gid = |c| f.charmap().map(c).unwrap().to_u32() as u16;
        let gids: BTreeSet<u16> = "Aä€".chars().map(gid).collect();
        let t = teilmenge(&f, &gids).unwrap();
        assert!(t.len() < d.len() / 2, "{} von {}", t.len(), d.len());
        let g = skrifa::FontRef::new(&t).unwrap();
        let alt = f.glyph_metrics(Size::unscaled(), LocationRef::default());
        let neu = g.glyph_metrics(Size::unscaled(), LocationRef::default());
        assert_eq!(g.maxp().unwrap().num_glyphs(), f.maxp().unwrap().num_glyphs());
        for &x in &gids {
            assert_eq!(alt.advance_width(x.into()), neu.advance_width(x.into()));
            let o = g.outline_glyphs().get(x.into()).unwrap();
            let mut pen = skrifa::outline::pen::ControlBoundsPen::default();
            o.draw(Size::unscaled(), &mut pen).unwrap();
            assert!(pen.bounding_box().is_some(), "Glyphe {x} hat Umrisse");
        }
        // nicht benutzte Glyphe: leer
        let o = g.outline_glyphs().get(gid('Z').into()).unwrap();
        let mut pen = skrifa::outline::pen::ControlBoundsPen::default();
        o.draw(Size::unscaled(), &mut pen).unwrap();
        assert!(pen.bounding_box().is_none());
    }

    #[test]
    fn objekte_einer_schrift() {
        let mut s = Schriften::default();
        let k = s.hinzu(hack(), 0).unwrap();
        assert!(s.glyphe(k, 'A').is_some());
        assert!(s.glyphe(k, '\u{10FFFD}').is_none());
        let (objs, eintraege) = s.objekte(20);
        assert_eq!(objs.len(), 5);
        assert_eq!(eintraege, ["/E1 20 0 R"]);
        let t0 = String::from_utf8_lossy(&objs[0]);
        assert!(t0.contains("/Subtype /Type0") && t0.contains("+Hack") && t0.contains("/DescendantFonts [21 0 R]") && t0.contains("/ToUnicode 24 0 R"), "{t0}");
        assert!(String::from_utf8_lossy(&objs[2]).contains("/FontFile2 23 0 R"));
        assert!(String::from_utf8_lossy(&objs[3]).contains("/Length1 "));
        // Times und leere Namen werden nie eingebettet
        assert_eq!(s.schrift("", false, false), None);
        assert_eq!(s.schrift("Times New Roman", true, false), None);
        assert_eq!(s.schrift("Gibt Es Nicht 4711", false, false), None);
    }
}
