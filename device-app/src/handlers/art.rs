use crate::state::{Art, ART_LEN};
use crate::AppSW;
use ledger_device_sdk::io::{Command, CommandResponse};

/// SET_ART: data = offset(u16 LE) || chunk. Cover art is public data (it
/// travels with the album), so no UI gate. What makes it trustworthy is the
/// sleeve hash signed into the album certificate at cut time: the device
/// renders the uploaded art only when its hash matches that certificate.
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
