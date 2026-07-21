use crate::certs::parse_album_cert;
use crate::handlers::press::fingerprint_str;
use crate::state::{Store, PRESSED_LOG_LEN};
use crate::AppSW;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use ledger_device_sdk::io::{Command, CommandResponse};
use ledger_device_sdk::nbgl::{Field, NbglGenericReview, NbglPageContent, TagValueList};
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{CenteredInfo, CenteredInfoStyle, NbglGlyph};

fn title_str(title: &[u8], title_len: u8) -> Result<&str, AppSW> {
    core::str::from_utf8(&title[..title_len as usize]).map_err(|_| AppSW::BadCert)
}

/// The sleeve this device should show for an album: the stored art when its
/// hash still matches what was sealed, otherwise the generated label art.
///
/// A mismatch is not an error to report but a fact to render honestly: the
/// device shows the album's own generated face rather than a bitmap it cannot
/// vouch for.
pub fn album_sleeve(album_id: &[u8; 32]) -> alloc::vec::Vec<u8> {
    if let Ok(nvm) = Store::get() {
        if nvm.has_art == 1 {
            let art = crate::state::Art::get();
            if let Ok(hash) = crate::crypto::sha256(&[art]) {
                if crate::crypto::mac_eq(&hash, &nvm.art_hash) {
                    return art.to_vec();
                }
            }
        }
    }
    crate::sleeve::fallback_sleeve(crate::state::ART_W, album_id)
}

#[cfg(any(target_os = "stax", target_os = "flex"))]
fn album_card(
    title: &str,
    line2: &str,
    line3: &str,
    glyph: &NbglGlyph,
) -> NbglPageContent {
    NbglPageContent::CenteredInfo(CenteredInfo::new(
        title,
        line2,
        line3,
        Some(glyph),
        false,
        CenteredInfoStyle::LargeCaseBoldInfo,
        0,
    ))
}

fn fields_page(names: &[String], values: &[String]) -> NbglPageContent {
    let fields: Vec<Field> = names
        .iter()
        .zip(values.iter())
        .map(|(n, v)| Field {
            name: n.as_str(),
            value: v.as_str(),
        })
        .collect();
    NbglPageContent::TagValueList(TagValueList::new(&fields, 0, false, true))
}

/// Draws the collection as swipeable record cards and blocks until "Back"
/// (or an incoming APDU). Callable from the APDU handler and from the home
/// action button's NBGL callback.
pub fn show_collection_screen() -> Result<(), AppSW> {
    let nvm = Store::get()?;

    // Sleeves are computed up front and kept alive for the whole call: NBGL
    // holds a pointer into each bitmap until the screen goes away. Thumbnails
    // rather than full size, which is all this card template has room for.
    let half = crate::state::ART_W / 2;
    let master_sleeve = if nvm.has_master == 1 {
        let album_id = crate::crypto::sha256(&[&nvm.alb_pub])?;
        Some(crate::sleeve::decimate(&album_sleeve(&album_id), crate::state::ART_W))
    } else {
        None
    };
    let pressing_sleeve = if nvm.has_pressing == 1 {
        let album = parse_album_cert(&nvm.pressing_album_cert)?;
        let pressing = crate::certs::parse_pressing_cert(&nvm.pressing_cert, &album.albpub)?;
        Some(crate::sleeve::decimate(&album_sleeve(&pressing.album_id), crate::state::ART_W))
    } else {
        None
    };

    let mut review = NbglGenericReview::new();
    let mut any = false;

    // Owned strings must outlive show(): collect them here.
    let mut names_m: Vec<String> = Vec::new();
    let mut values_m: Vec<String> = Vec::new();
    let mut names_h: Vec<String> = Vec::new();
    let mut values_h: Vec<String> = Vec::new();
    let mut card_lines: Vec<String> = Vec::new();

    if nvm.has_master == 1 {
        any = true;
        let title = title_str(&nvm.title, nvm.title_len)?;
        card_lines.push(format!("My master, edition of {}", nvm.edition));
        card_lines.push(format!("{} left to press", nvm.counter));
        #[cfg(any(target_os = "stax", target_os = "flex"))]
        {
            let bitmap = master_sleeve.as_deref().unwrap_or(&[]);
            let glyph = NbglGlyph::new(
                bitmap,
                half as u16,
                half as u16,
                crate::state::ART_BPP as u8,
                false,
            );
            review = review.add_content(album_card(
                title,
                &card_lines[card_lines.len() - 2],
                &card_lines[card_lines.len() - 1],
                &glyph,
            ));
        }
        names_m.push(String::from("Still to press"));
        values_m.push(format!("{}", nvm.counter));
        let pressed = (nvm.edition - nvm.counter) as usize;
        for entry in nvm.pressed_log.iter().take(pressed.min(PRESSED_LOG_LEN)) {
            if entry.number == 0 {
                continue;
            }
            names_m.push(format!("Pressed {} of {}", entry.number, nvm.edition));
            values_m.push(format!("for device {}", fingerprint_str(&entry.recipient_fp)));
        }
        if pressed > PRESSED_LOG_LEN {
            names_m.push(String::from("Earlier pressings"));
            values_m.push(format!("{} more, not listed", pressed - PRESSED_LOG_LEN));
        }
        review = review.add_content(fields_page(&names_m, &values_m));
    }

    if nvm.has_pressing == 1 {
        any = true;
        let album = parse_album_cert(&nvm.pressing_album_cert)?;
        let title = title_str(&album.title, album.title_len)?;
        let pressing = crate::certs::parse_pressing_cert(&nvm.pressing_cert, &album.albpub)?;
        card_lines.push(format!("Pressing {} of {}", pressing.number, pressing.edition));
        #[cfg(any(target_os = "stax", target_os = "flex"))]
        {
            let bitmap = pressing_sleeve.as_deref().unwrap_or(&[]);
            let glyph = NbglGlyph::new(
                bitmap,
                half as u16,
                half as u16,
                crate::state::ART_BPP as u8,
                false,
            );
            review = review.add_content(album_card(
                title,
                &card_lines[card_lines.len() - 1],
                "Bound to this device",
                &glyph,
            ));
        }
        names_h.push(String::from("In my collection"));
        values_h.push(format!(
            "{}, {} of {}",
            title, pressing.number, pressing.edition
        ));
        review = review.add_content(fields_page(&names_h, &values_h));
    }

    if !any {
        names_m.push(String::from("Collection"));
        values_m.push(String::from("Empty. Cut a master or receive a pressing."));
        review = review.add_content(fields_page(&names_m, &values_m));
    }

    review.show_from_callback("Back");
    Ok(())
}

/// COLLECTION over APDU: same screen, host-triggered (used by tests and the
/// relay demos).
pub fn handler_collection(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    show_collection_screen()?;
    let response = command.into_response();
    Ok(response)
}

/// ART_TEST: development probe for the raw-NBGL path. P1 = 0 draws a
/// device-generated pattern packed with the agreed 1bpp convention; P1 = 1
/// draws whatever sleeve is currently in NVM. Both put a text label *over*
/// the image, which is the mechanism the record card needs.
///
/// The generated pattern is an "F" (asymmetric under all eight flips and
/// rotations) plus a bar flush with the top edge: the F settles the geometry,
/// the bar settles the bit order inside a byte.
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub fn handler_art_test(command: Command<'_>, stage: u8) -> Result<CommandResponse<'_>, AppSW> {
    use crate::app_ui::library::{run_event_loop, Screen, ScreenArena, SCREEN_W};
    use crate::state::{Art, ART_W};

    const N: usize = ART_W;
    let mut arena = ScreenArena::new();
    let icon = if stage == 1 {
        arena.icon_static(Art::get(), N as u16, N as u16, ledger_secure_sdk_sys::NBGL_BPP_1)
    } else {
        let mut bitmap = alloc::vec![0u8; N * N / 8];
        let s = N / 16;
        for y in 0..N {
            for x in 0..N {
                let stem = x >= 3 * s && x < 4 * s && y >= 3 * s && y < 13 * s;
                let top_arm = y >= 3 * s && y < 4 * s && x >= 3 * s && x < 11 * s;
                let mid_arm = y >= 7 * s && y < 8 * s && x >= 3 * s && x < 9 * s;
                let top_bar = y < 2 && x < 4 * s;
                if stem || top_arm || mid_arm || top_bar {
                    // The convention agreed with the host-side packer: the
                    // display is the row-major decode of the buffer rotated
                    // 90 clockwise, so packing pre-rotates counter-clockwise.
                    let k = (N - 1 - x) * N + y;
                    bitmap[k >> 3] |= 0x80 >> (k & 7);
                }
            }
        }
        arena.icon(bitmap, N as u16, N as u16, ledger_secure_sdk_sys::NBGL_BPP_1)
    };
    let label = arena.text("PROBE");

    if stage != 3 {
        // The layout path, which is the one that renders.
        let mut layout = crate::app_ui::library::Layout::new();
        layout.centered_info(icon, label, core::ptr::null(), core::ptr::null(), 0);
        layout.draw();
        let _ = run_event_loop();
    } else {
        // P1 = 3 keeps the raw object path reachable for further study.
        let x0 = (SCREEN_W - N as i16) / 2;
        let y0 = 160;
        let mut screen = Screen::push(2, false);
        screen.image(icon, N as u16, N as u16, x0, y0, None);
        screen.text(
            label,
            x0,
            y0 + N as i16 - 44,
            N as u16,
            40,
            ledger_secure_sdk_sys::BAGL_FONT_INTER_SEMIBOLD_24px,
            ledger_secure_sdk_sys::BLACK,
            ledger_secure_sdk_sys::CENTER,
            None,
        );
        screen.draw();
        let _ = run_event_loop();
    }

    let response = command.into_response();
    Ok(response)
}

