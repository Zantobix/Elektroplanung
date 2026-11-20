//! Original-Glyphen (aus der Exe des Originals ausgelesen) sind nicht Teil des Repos. Liegen sie vor
//! (Ordner aus der Umgebungsvariable EP_GLYPHEN, sonst resources/glyphs), werden sie eingebettet;
//! sonst zeigen die betroffenen Knöpfe Text.
//! Unter Windows bekommt die Exe außerdem Programmsymbol und Versionsangaben (aus Cargo.toml).
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
    windows_ressourcen();
}

#[cfg(windows)]
fn windows_ressourcen() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo::rerun-if-changed=resources/elektroplanung.ico");
    let mut r = winresource::WindowsResource::new();
    r.set_icon("resources/elektroplanung.ico");
    r.set("ProductName", "Elektroplanung");
    r.set("FileDescription", "Elektroplanung – unabhängiger Nachbau");
    r.set("LegalCopyright", "GPL-3.0");
    if let Err(e) = r.compile() {
        println!("cargo::warning=Programmsymbol nicht eingebettet: {e}");
    }
}

#[cfg(not(windows))]
fn windows_ressourcen() {}
