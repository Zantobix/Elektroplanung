//! Original-Glyphen (aus der Exe des Originals ausgelesen) sind nicht Teil des Repos. Liegen sie vor
//! (Ordner aus der Umgebungsvariable EP_GLYPHEN, sonst resources/glyphs), werden sie eingebettet;
//! sonst zeigen die betroffenen Knöpfe Text.
use std::path::PathBuf;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(original_glyphen)");
    println!("cargo::rerun-if-env-changed=EP_GLYPHEN");
    let dir = std::env::var("EP_GLYPHEN").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("resources/glyphs"));
    let dir = if dir.is_absolute() { dir } else { PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(dir) };
    println!("cargo::rerun-if-changed={}", dir.display());
    if dir.join("TFORM1_Button1.png").is_file() {
        println!("cargo::rustc-cfg=original_glyphen");
        println!("cargo::rustc-env=EP_GLYPHEN_DIR={}", dir.display());
    }
}
