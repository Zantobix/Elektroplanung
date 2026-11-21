//! Vektor-Icons: eigene SVG-Grafiken aus resources/icons (erzeugt mit tools/icons_erzeugen.py), gezeichnet über
//! egui_extras in jeder Größe scharf. Einheitliches System: je Objektart ein Grund-Icon (`obj_*`, hier die deutschen
//! Namen wie `Klemmleiste`), das Werkzeugleisten, Projektbaum und Dialoge gemeinsam nutzen; „Neu“-Knöpfe zeigen
//! dasselbe Grund-Icon mit grünem Plus unten rechts, „Bearbeiten“ mit Stift, „Entfernen“ mit rotem Minus.

use eframe::egui::{self, Color32, Rect};

macro_rules! icons {
    ($($v:ident => $f:literal,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Icon {
            $($v,)*
        }

        /// Alle Icons (für Tests und Übersichten).
        pub const ALLE: &[Icon] = &[$(Icon::$v,)*];

        /// SVG-Quelle eines Icons (im Programm eingebettet).
        pub fn source(icon: Icon) -> egui::ImageSource<'static> {
            match icon {
                $(Icon::$v => egui::include_image!(concat!("../resources/icons/", $f, ".svg")),)*
            }
        }

        /// Dateiname (ohne .svg) eines Icons.
        pub fn datei(icon: Icon) -> &'static str {
            match icon {
                $(Icon::$v => $f,)*
            }
        }
    };
}

icons! {
    NewProject => "new_projekt",
    Open => "open",
    Save => "save",
    SaveProject => "save_project",
    Print => "print",
    Cut => "cut",
    Copy => "copy",
    Paste => "paste",
    Undo => "undo",
    Redo => "redo",
    Delete => "delete",
    Zoom => "zoom",
    Grid => "grid",
    PathAlign => "path_align",
    AutoConn => "auto_conn",
    SizeFrame => "size_frame",
    Fix => "fix",
    Rotate => "rotate",
    MirrorV => "mirror_v",
    MirrorH => "mirror_h",
    Group => "group",
    Ungroup => "ungroup",
    Select => "select",
    Line => "line",
    Curve => "curve",
    Rect => "rect",
    Triangle => "triangle",
    Circle => "circle",
    CaseSensitive => "case_sensitive",
    Find => "find",
    Apply => "apply",
    Cancel => "cancel",
    OptionToggle => "option_toggle",
    AlignHor => "align_hor",
    AlignVer => "align_ver",
    SpaceHor => "space_hor",
    SpaceVer => "space_ver",
    Projekt => "obj_projekt",
    Blatt => "obj_blatt",
    Anlage => "obj_anlage",
    Potential => "obj_potential",
    Sammelschiene => "obj_sammelschiene",
    Leitung => "obj_leitung",
    Klemmleiste => "obj_klemmleiste",
    Klemme => "obj_klemme",
    Stift => "obj_stift",
    Symbol => "obj_symbol",
    Bauteil => "obj_bauteil",
    Platzhalter => "obj_platzhalter",
    Bild => "obj_bild",
    Tabelle => "obj_tabelle",
    Text => "obj_text",
    TextBox => "obj_textbox",
    Seite => "obj_seite",
    Option => "obj_option",
    Buch => "obj_buch",
    Buecher => "obj_buecher",
    NewSheet => "new_blatt",
    NewPotential => "new_potential",
    NewBusbar => "new_sammelschiene",
    NewLeitung => "new_leitung",
    NewKlemmleiste => "new_klemmleiste",
    NewPen => "new_stift",
    NewPage => "new_seite",
    NewOption => "new_option",
    EditSammelschiene => "edit_sammelschiene",
    EditBauteil => "edit_bauteil",
    EditSymbol => "edit_symbol",
    EditDrucker => "edit_drucker",
    DelZeile => "del_zeile",
}

/// Icon in `r` zeichnen; gesperrte Knöpfe blass.
pub fn draw(ui: &egui::Ui, r: Rect, icon: Icon, enabled: bool) {
    let tint = if enabled { Color32::WHITE } else { Color32::from_rgba_unmultiplied(150, 150, 150, 110) };
    egui::Image::new(source(icon)).tint(tint).fit_to_exact_size(r.size()).paint_at(ui, r);
}

/// Icon als Bild für Knöpfe (`egui::Button::image`).
pub fn image(icon: Icon, size: f32) -> egui::Image<'static> {
    egui::Image::new(source(icon)).fit_to_exact_size(egui::Vec2::splat(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(i: Icon) -> String {
        match source(i) {
            egui::ImageSource::Bytes { bytes, .. } => String::from_utf8_lossy(bytes.as_ref()).to_string(),
            _ => panic!("{i:?} nicht eingebettet"),
        }
    }

    #[test]
    fn alle_eingebettet_im_einheitlichen_raster() {
        for &i in ALLE {
            let t = text(i);
            assert!(t.starts_with("<svg") && t.contains(r#"viewBox="0 0 24 24""#), "{i:?}");
        }
    }

    /// „Neu“-Knöpfe = Grund-Icon + Abzeichen: die Datei beginnt mit dem Inhalt des Grund-Icons.
    #[test]
    fn neu_knoepfe_zeigen_das_grund_icon() {
        let inhalt = |i: Icon| {
            let t = text(i);
            let a = t.find('>').unwrap() + 1;
            t[a..t.rfind("</svg>").unwrap()].to_string()
        };
        for (neu, grund) in [
            (Icon::NewProject, Icon::Projekt),
            (Icon::NewSheet, Icon::Blatt),
            (Icon::NewPotential, Icon::Potential),
            (Icon::NewBusbar, Icon::Sammelschiene),
            (Icon::NewLeitung, Icon::Leitung),
            (Icon::NewKlemmleiste, Icon::Klemmleiste),
            (Icon::NewPen, Icon::Stift),
            (Icon::NewPage, Icon::Seite),
            (Icon::NewOption, Icon::Option),
            (Icon::EditSammelschiene, Icon::Sammelschiene),
            (Icon::EditBauteil, Icon::Bauteil),
            (Icon::EditSymbol, Icon::Symbol),
            (Icon::DelZeile, Icon::Tabelle),
        ] {
            let (n, g) = (inhalt(neu), inhalt(grund));
            assert!(n.starts_with(&g) && n.len() > g.len(), "{neu:?} enthält nicht {grund:?}");
        }
    }

    #[test]
    fn dateinamen_eindeutig() {
        let mut n: Vec<&str> = ALLE.iter().map(|&i| datei(i)).collect();
        n.sort();
        n.dedup();
        assert_eq!(n.len(), ALLE.len());
    }
}
