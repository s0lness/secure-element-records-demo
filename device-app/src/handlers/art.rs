use crate::crypto;
use crate::state::{Art, Store, ART_LEN};
use crate::AppSW;
use ledger_device_sdk::io::{Command, CommandResponse};

/// SET_ART: data = offset(u16 LE) || chunk. Cover art is public data (it
/// travels with the album), so no UI gate; what makes it trustworthy is the
/// hash stored in the album state at cut time.
pub fn handler_set_art(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    let data = command.get_data();
    if data.len() < 3 {
        return Err(AppSW::WrongApduLength);
    }
    let offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    Art::write_chunk(offset, &data[2..])?;
    let response = command.into_response();
    Ok(response)
}

/// SEAL_ART: bind the uploaded art by storing its hash. From then on the
/// device only renders art whose hash matches, so a tampered upload shows
/// the fallback label rather than a lie. Sealed by the master after upload,
/// and by a receiver once the art has been carried across at press time.
pub fn handler_seal_art(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    let mut nvm = Store::get()?;
    if nvm.has_master != 1 && nvm.has_pressing != 1 {
        return Err(AppSW::NoMaster);
    }
    let hash = crypto::sha256(&[Art::get()])?;
    nvm.art_hash = hash;
    nvm.has_art = 1;
    Store::put(&nvm);
    let mut response = command.into_response();
    response.append(&hash)?;
    Ok(response)
}

/// GET_ART: read back a chunk (p1 = chunk index, 224 bytes each) so the relay
/// can carry the art to the receiving device during a press.
pub fn handler_get_art(command: Command<'_>, chunk: u8) -> Result<CommandResponse<'_>, AppSW> {
    const CHUNK: usize = crate::state::ART_CHUNK;
    let offset = chunk as usize * CHUNK;
    if offset >= ART_LEN {
        return Err(AppSW::WrongP1P2);
    }
    let end = (offset + CHUNK).min(ART_LEN);
    let art = Art::get();
    let mut response = command.into_response();
    response.append(&art[offset..end])?;
    Ok(response)
}
