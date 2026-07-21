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

/// COLLECTION: browsable on-device view of what this device holds: the master
/// and its issued pressings, and the pressing bound to this device. Pages
/// paginate under NBGL; the footer button leaves the screen.
pub fn handler_collection(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
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
        let pressing =
            crate::certs::parse_pressing_cert(&nvm.pressing_cert, &album.albpub)?;
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

    let fields: Vec<Field> = names
        .iter()
        .zip(values.iter())
        .map(|(n, v)| Field {
            name: n.as_str(),
            value: v.as_str(),
        })
        .collect();

    let comm = command.into_comm();
    NbglGenericReview::new()
        .add_content(NbglPageContent::TagValueList(TagValueList::new(
            &fields, 0, false, true,
        )))
        .show(comm, "Back");

    let response = comm.begin_response();
    Ok(response)
}
