use crate::state::Store;
use crate::AppSW;
use ledger_device_sdk::io::{Command, CommandResponse};

const FLAG_HAS_MASTER: u8 = 1 << 0;
const FLAG_HAS_PRESSING: u8 = 1 << 1;

/// GET_INFO: [flags][devpub 65][edition u16][counter u16][title_len][title 32]
pub fn handler_get_info(command: Command<'_>) -> Result<CommandResponse<'_>, AppSW> {
    let nvm = Store::get()?;
    let mut flags = 0u8;
    if nvm.has_master == 1 {
        flags |= FLAG_HAS_MASTER;
    }
    if nvm.has_pressing == 1 {
        flags |= FLAG_HAS_PRESSING;
    }
    let mut response = command.into_response();
    response.append(&[flags])?;
    response.append(&nvm.dev_pub)?;
    response.append(&nvm.edition.to_le_bytes())?;
    response.append(&nvm.counter.to_le_bytes())?;
    response.append(&[nvm.title_len])?;
    response.append(&nvm.title)?;
    Ok(response)
}
