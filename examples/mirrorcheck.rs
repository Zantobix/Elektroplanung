//! Kreuzprobe: welche Deutung der Spiegelbits passt zum vom Original gespeicherten Umriss?
use elektroplanung::{epr, model::*, symbols};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = epr::import(std::path::Path::new(&a[1])).unwrap();
    let (mut ok_cur, mut ok_swap) = (0, 0);
    for s in p.anlagen.iter().flat_map(|a| &a.sheets) {
        for o in &s.objects {
            if let Kind::Symbol { key, pos, rot, mirror_h, mirror_v, scale, option, ext, bmk, .. } = &o.kind {
                if !(*mirror_h || *mirror_v) { continue; }
                let Some(stored) = ext.bounds else { continue };
                let Some(d) = symbols::find(&p.symdefs, key) else { continue };
                let (mn, mx) = symbols::bounds_opt(d, *option);
                let bb = |mh: bool, mv: bool| {
                    let c = [P::new(mn.x, mn.y), P::new(mx.x, mn.y), P::new(mx.x, mx.y), P::new(mn.x, mx.y)];
                    let pts: Vec<P> = c.iter().map(|q| sym_xf(*q, P::default(), *rot, mh, mv, *scale)).collect();
                    let x0 = pts.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                    let x1 = pts.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                    let y0 = pts.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                    let y1 = pts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
                    (x0, y0, x1, y1)
                };
                let err = |b: (f32, f32, f32, f32)| (b.0 - stored.0.x).abs() + (b.1 - stored.0.y).abs() + (b.2 - stored.1.x).abs() + (b.3 - stored.1.y).abs();
                let (ec, es) = (err(bb(*mirror_h, *mirror_v)), err(bb(*mirror_v, *mirror_h)));
                println!("{bmk} rot={rot} mh={mirror_h} mv={mirror_v} pos={pos:?} stored={stored:?} Fehler jetzt={ec:.1} getauscht={es:.1}");
                if ec < es { ok_cur += 1 } else if es < ec { ok_swap += 1 }
            }
        }
    }
    println!("passt: jetzige Deutung {ok_cur}, getauscht {ok_swap}");
}
