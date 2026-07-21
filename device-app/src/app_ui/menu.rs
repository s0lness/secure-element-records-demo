use ledger_device_sdk::include_gif;
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::nbgl::{NbglGlyph, NbglHomeAndSettings};

pub fn ui_menu_main(_: &mut Comm) -> NbglHomeAndSettings {
    #[cfg(target_os = "apex_p")]
    const ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/crab_48x48.png", NBGL));
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/home_nano_nbgl.png", NBGL));

    NbglHomeAndSettings::new()
        .glyph(&ICON)
        .tagline("Finite editions,\npressed in silicon.")
        .infos("Presse", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"))
}
