use crate::crypto::PUBKEY_LEN;
use crate::session::{PairState, Role, Session};
use crate::AppSW;
use alloc::format;
use ledger_device_sdk::io::{Command, CommandResponse};

/// PAIR_COMMIT (master): begin, return SHA256 commitment to our ephemeral.
pub fn handler_commit<'a>(
    command: Command<'a>,
    session: &mut Session,
) -> Result<CommandResponse<'a>, AppSW> {
    if !command.get_data().is_empty() {
        return Err(AppSW::WrongApduLength);
    }
    session.begin(Role::Master)?;
    let commitment = session.commitment()?;
    session.state = PairState::Committed;
    let mut response = command.into_response();
    response.append(&commitment)?;
    Ok(response)
}

/// PAIR_RESPOND (receiver): data = commitment(32). Store it, reveal our
/// ephemeral. We reveal only after the peer is bound: that asymmetry is the
/// grind resistance.
pub fn handler_respond<'a>(
    command: Command<'a>,
    session: &mut Session,
) -> Result<CommandResponse<'a>, AppSW> {
    let data = command.get_data();
    if data.len() != 32 {
        return Err(AppSW::WrongApduLength);
    }
    session.begin(Role::Receiver)?;
    session.peer_commit.copy_from_slice(data);
    session.state = PairState::Responded;
    let my_pub = session.my_pub;
    let mut response = command.into_response();
    response.append(&my_pub)?;
    Ok(response)
}

/// PAIR_REVEAL (master): data = receiver ephemeral(65). Derive, reveal ours.
pub fn handler_reveal<'a>(
    command: Command<'a>,
    session: &mut Session,
) -> Result<CommandResponse<'a>, AppSW> {
    let data = command.get_data();
    if data.len() != PUBKEY_LEN {
        return Err(AppSW::WrongApduLength);
    }
    if session.state != PairState::Committed || session.role != Role::Master {
        session.reset();
        return Err(AppSW::BadState);
    }
    let mut peer = [0u8; PUBKEY_LEN];
    peer.copy_from_slice(data);
    session.derive(&peer)?;
    let my_pub = session.my_pub;
    let mut response = command.into_response();
    response.append(&my_pub)?;
    Ok(response)
}

/// PAIR_FINISH (receiver): data = master ephemeral(65). Check it against the
/// commitment, then derive. A mismatch is a hard abort.
pub fn handler_finish<'a>(
    command: Command<'a>,
    session: &mut Session,
) -> Result<CommandResponse<'a>, AppSW> {
    let data = command.get_data();
    if data.len() != PUBKEY_LEN {
        return Err(AppSW::WrongApduLength);
    }
    if session.state != PairState::Responded || session.role != Role::Receiver {
        session.reset();
        return Err(AppSW::BadState);
    }
    let mut peer = [0u8; PUBKEY_LEN];
    peer.copy_from_slice(data);
    let expected = crate::crypto::sha256(&[b"presse-commit", &peer])?;
    if !crate::crypto::mac_eq(&expected, &session.peer_commit) {
        session.reset();
        return Err(AppSW::BadMac);
    }
    session.derive(&peer)?;
    let response = command.into_response();
    Ok(response)
}

/// PAIR_SAS: display the 4 words, wait for the human. UI-gated.
pub fn handler_sas<'a>(
    command: Command<'a>,
    session: &mut Session,
) -> Result<CommandResponse<'a>, AppSW> {
    if session.state != PairState::Derived {
        session.reset();
        return Err(AppSW::BadState);
    }
    let words = session.sas_words();
    let message = format!("{}\n{}\n{}\n{}", words[0], words[1], words[2], words[3]);
    let comm = command.into_comm();
    let approved = crate::app_ui::menu::ceremony_choice().show(
        comm,
        &message,
        "Confirm only if the other device\nshows exactly these words.",
        "Words match",
        "Abort",
    );
    if !approved {
        session.reset();
        return Err(AppSW::Deny);
    }
    session.confirm_sas();
    let sas = session.sas;
    let mut response = comm.begin_response();
    response.append(&sas)?;
    Ok(response)
}
