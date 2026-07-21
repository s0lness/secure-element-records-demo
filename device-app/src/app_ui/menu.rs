use ledger_device_sdk::include_gif;
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::nbgl::{NbglChoice, NbglGlyph, NbglHomeAndSettings};

#[cfg(target_os = "apex_p")]
pub const RECORD: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/crab_48x48.png", NBGL));
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub const RECORD: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/vinyl_64x64.gif", NBGL));
#[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
pub const RECORD: NbglGlyph =
    NbglGlyph::from_include(include_gif!("glyphs/home_nano_nbgl.png", NBGL));

pub fn ui_menu_main(_: &mut Comm) -> NbglHomeAndSettings {
    NbglHomeAndSettings::new()
        .glyph(&RECORD)
        .tagline("Finite editions,\npressed in silicon.")
        .infos("Presse", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"))
}

/// The confirmation page used by every ceremony, vinyl front and center.
pub fn ceremony_choice() -> NbglChoice<'static> {
    NbglChoice::new().glyph(&RECORD)
}
