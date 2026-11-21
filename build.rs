//! Unter Windows bekommt die Exe Programmsymbol und Versionsangaben (aus Cargo.toml).
fn main() {
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
