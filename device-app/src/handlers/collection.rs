use crate::certs::parse_album_cert;
use crate::handlers::press::fingerprint_str;
use crate::state::{Store, PRESSED_LOG_LEN};
use crate::AppSW;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use ledger_device_sdk::io::{Command, CommandResponse};
use ledger_device_sdk::nbgl::{Field, NbglGenericReview, NbglPageContent, TagValueList};

fn title_str(title: &[u8], title_len: u8) -> Result<&str, AppSW> {
    core::str::from_utf8(&title[..title_len as usize]).map_err(|_| AppSW::BadCert)
}

fn collection_fields() -> Result<(Vec<String>, Vec<String>), AppSW> {
    let nvm = Store::get()?;
    let mut names: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if nvm.has_master == 1 {
        let title = title_str(&nvm.title, nvm.title_len)?;
        names.push(String::from("Master"));
        values.push(format!("{}, edition of {}", title, nvm.edition));
        names.push(String::from("Still to press"));
        values.push(format!("{}", nvm.counter));
        let pressed = (nvm.edition - nvm.counter) as usize;
        for entry in nvm.pressed_log.iter().take(pressed.min(PRESSED_LOG_LEN)) {
            if entry.number == 0 {
                continue;
            }
            names.push(format!("Pressed {} of {}", entry.number, nvm.edition));
            values.push(format!("for device {}", fingerprint_str(&entry.recipient_fp)));
        }
        if pressed > PRESSED_LOG_LEN {
            names.push(String::from("Earlier pressings"));
            values.push(format!("{} more, not listed", pressed - PRESSED_LOG_LEN));
        }
    }

    if nvm.has_pressing == 1 {
        let album = parse_album_cert(&nvm.pressing_album_cert)?;
        let title = title_str(&album.title, album.title_len)?;
        let pressing = crate::certs::parse_pressing_cert(&nvm.pressing_cert, &album.albpub)?;
        names.push(String::from("In my collection"));
        values.push(format!(
            "{}, {} of {}",
            title, pressing.number, pressing.edition
        ));
    }

    if names.is_empty() {
        names.push(String::from("Collection"));
        values.push(String::from("Empty. Cut a master or receive a pressing."));
    }
    Ok((names, values))
}

/// Draws the collection screen and blocks until "Back". Callable both from
/// the APDU handler and from the home action button's NBGL callback.
pub fn show_collection_screen() -> Result<(), AppSW> {
    let (names, values) = collection_fields()?;
    let fields: Vec<Field> = names
        .iter()
        .zip(values.iter())
        .map(|(n, v)| Field {
            name: n.as_str(),
            value: v.as_str(),
        })
        .collect();

    NbglGenericReview::new()
        .add_content(NbglPageContent::TagValueList(TagValueList::new(
            &fields, 0, false, true,
        )))
        .show_from_callback("Back");
    Ok(())
}

/// COLLECTION over APDU: same screen, host-triggered (used by tests and the
/// relay demos).
pub fn handler_collection(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    show_collection_screen()?;
    let response = command.into_response();
    Ok(response)
}
