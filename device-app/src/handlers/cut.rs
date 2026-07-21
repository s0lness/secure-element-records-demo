use crate::certs::{build_album_cert, TITLE_MAX};
use crate::crypto;
use crate::state::Store;
use crate::AppSW;
use alloc::format;
use ledger_device_sdk::io::{Command, CommandResponse};

/// CUT: data = edition(u16 LE) || title(1..=32 bytes utf-8).
/// UI-gated. One master per device; re-cutting requires RESET_MASTER first.
pub fn handler_cut(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    let data = command.get_data();
    if data.len() < 3 || data.len() > 2 + TITLE_MAX {
        return Err(AppSW::WrongApduLength);
    }
    let edition = u16::from_le_bytes([data[0], data[1]]);
    if edition == 0 {
        return Err(AppSW::BadCert);
    }
    let title_len = data.len() - 2;
    let mut title_buf = [0u8; TITLE_MAX];
    title_buf[..title_len].copy_from_slice(&data[2..]);
    let title_bytes = &title_buf[..title_len];
    let title = core::str::from_utf8(title_bytes).map_err(|_| AppSW::BadCert)?;

    let mut nvm = Store::get()?;
    if nvm.has_master == 1 {
        return Err(AppSW::HasMaster);
    }

    let message = format!("Cut master of\n{}?", title);
    let submessage = format!(
        "Edition of {}, fixed forever.\nLosing this device destroys the plates.",
        edition
    );
    let comm = command.into_comm();
    let approved = crate::app_ui::menu::ceremony_choice().show(comm, &message, &submessage, "Cut the master", "Cancel");
    if !approved {
        return Err(AppSW::Deny);
    }

    let (alb_priv, alb_pub) = crypto::gen_keypair()?;
    let cert = build_album_cert(&alb_priv, &alb_pub, &title_buf[..title_len], edition)?;

    nvm.has_master = 1;
    nvm.alb_priv = alb_priv;
    nvm.alb_pub = alb_pub;
    nvm.title = title_buf;
    nvm.title_len = title_len as u8;
    nvm.edition = edition;
    nvm.counter = edition;
    nvm.album_cert = cert;
    Store::put(&nvm);

    let mut response = comm.begin_response();
    response.append(&cert)?;
    Ok(response)
}

/// RESET_MASTER: destroy the plates. UI-gated, deliberately scary.
pub fn handler_reset_master(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    let mut nvm = Store::get()?;
    if nvm.has_master != 1 {
        return Err(AppSW::NoMaster);
    }
    let comm = command.into_comm();
    let approved = crate::app_ui::menu::ceremony_choice().show(
        comm,
        "Destroy the plates?",
        "The album key is erased forever.\nNo further pressing will ever exist.",
        "Destroy forever",
        "Cancel",
    );
    if !approved {
        return Err(AppSW::Deny);
    }
    nvm.has_master = 0;
    nvm.alb_priv = [0; 32];
    nvm.alb_pub = [0; crate::crypto::PUBKEY_LEN];
    nvm.title = [0; TITLE_MAX];
    nvm.title_len = 0;
    nvm.edition = 0;
    nvm.counter = 0;
    nvm.album_cert = [0; crate::certs::ALBUM_CERT_LEN];
    nvm.pressed_log = [crate::state::PressedEntry {
        number: 0,
        recipient_fp: [0; 4],
    }; crate::state::PRESSED_LOG_LEN];
    Store::put(&nvm);
    let response = comm.begin_response();
    Ok(response)
}
