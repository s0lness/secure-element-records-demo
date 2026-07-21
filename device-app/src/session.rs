//! RAM-only pairing session. Dies on power cycle, on any MAC failure, and on
//! SAS rejection: a session is cheap, trust is not.

use crate::certs::ALBUM_CERT_LEN;
use crate::crypto::{self, PUBKEY_LEN};
use crate::wordlist::WORDS;
use crate::AppSW;

const COMMIT_TAG: &[u8] = b"presse-commit";
const SAS_TAG: &[u8] = b"presse-sas";
const SESSION_TAG: &[u8] = b"presse-session";

/// Online SAS-grinding cap: pairing attempts allowed per power cycle.
const MAX_ATTEMPTS: u8 = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Master,
    Receiver,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PairState {
    Idle,
    /// Master generated its ephemeral and sent the commitment.
    Committed,
    /// Receiver stored the commitment and revealed its ephemeral.
    Responded,
    /// Shared secret derived; awaiting human SAS confirmation.
    Derived,
    /// SAS confirmed on this device; MACed payloads may flow.
    Ready,
}

pub struct Session {
    pub state: PairState,
    pub role: Role,
    attempts: u8,
    eph_priv: [u8; 32],
    pub my_pub: [u8; PUBKEY_LEN],
    pub peer_commit: [u8; 32],
    session_key: [u8; 32],
    pub sas: [u8; 4],
    pub send_seq: u8,
    pub recv_seq: u8,
    /// Album cert staged by the receiver between PRESS_LOAD_ALBUM and
    /// PRESS_ACCEPT.
    pub staged_album: [u8; ALBUM_CERT_LEN],
    pub staged_album_valid: bool,
}

impl Session {
    pub fn new() -> Session {
        Session {
            state: PairState::Idle,
            role: Role::Master,
            attempts: 0,
            eph_priv: [0; 32],
            my_pub: [0; PUBKEY_LEN],
            peer_commit: [0; 32],
            session_key: [0; 32],
            sas: [0; 4],
            send_seq: 0,
            recv_seq: 0,
            staged_album: [0; ALBUM_CERT_LEN],
            staged_album_valid: false,
        }
    }

    /// Reset everything but the per-boot attempt counter.
    pub fn reset(&mut self) {
        let attempts = self.attempts;
        *self = Session::new();
        self.attempts = attempts;
    }

    /// Begin a pairing attempt: fresh ephemeral, counted against the per-boot
    /// cap so a hostile relay cannot silently retry its way to a SAS match.
    pub fn begin(&mut self, role: Role) -> Result<(), AppSW> {
        if self.attempts >= MAX_ATTEMPTS {
            return Err(AppSW::TooManyAttempts);
        }
        self.reset();
        self.attempts += 1;
        self.role = role;
        let (sk, pk) = crypto::gen_keypair()?;
        self.eph_priv = sk;
        self.my_pub = pk;
        Ok(())
    }

    pub fn commitment(&self) -> Result<[u8; 32], AppSW> {
        crypto::sha256(&[COMMIT_TAG, &self.my_pub])
    }

    /// Derive session key + SAS from the ECDH secret and the transcript.
    /// Transcript order is (master ephemeral, receiver ephemeral) on both
    /// sides, so a MITM running two handshakes cannot make them collide.
    pub fn derive(&mut self, peer_pub: &[u8; PUBKEY_LEN]) -> Result<(), AppSW> {
        let secret = crypto::ecdh(&self.eph_priv, peer_pub)?;
        let (master_pub, receiver_pub) = match self.role {
            Role::Master => (&self.my_pub, peer_pub),
            Role::Receiver => (peer_pub, &self.my_pub),
        };
        let transcript = crypto::sha256(&[SAS_TAG, master_pub, receiver_pub])?;
        self.session_key = crypto::hmac_sha256(&secret, &[SESSION_TAG, &transcript])?;
        let sas_full = crypto::hmac_sha256(&secret, &[SAS_TAG, &transcript])?;
        self.sas.copy_from_slice(&sas_full[..4]);
        self.state = PairState::Derived;
        // The ephemeral has served its purpose; scrub it.
        self.eph_priv = [0u8; 32];
        Ok(())
    }

    pub fn sas_words(&self) -> [&'static str; 4] {
        [
            WORDS[self.sas[0] as usize],
            WORDS[self.sas[1] as usize],
            WORDS[self.sas[2] as usize],
            WORDS[self.sas[3] as usize],
        ]
    }

    pub fn confirm_sas(&mut self) {
        self.state = PairState::Ready;
        self.attempts = 0;
    }

    pub fn require_ready(&self) -> Result<(), AppSW> {
        if self.state == PairState::Ready {
            Ok(())
        } else {
            Err(AppSW::BadState)
        }
    }

    /// MAC an outgoing payload and bump the send counter.
    pub fn mac_send(&mut self, ins: u8, payload: &[u8]) -> Result<[u8; 32], AppSW> {
        let mac = crypto::hmac_sha256(&self.session_key, &[&[ins, self.send_seq], payload])?;
        self.send_seq += 1;
        Ok(mac)
    }

    /// Verify an incoming payload's MAC. Any failure kills the session.
    pub fn mac_verify(&mut self, ins: u8, payload: &[u8], mac: &[u8]) -> Result<(), AppSW> {
        if mac.len() != 32 {
            self.reset();
            return Err(AppSW::BadMac);
        }
        let expected = crypto::hmac_sha256(&self.session_key, &[&[ins, self.recv_seq], payload])?;
        let mut given = [0u8; 32];
        given.copy_from_slice(mac);
        if !crypto::mac_eq(&expected, &given) {
            self.reset();
            return Err(AppSW::BadMac);
        }
        self.recv_seq += 1;
        Ok(())
    }
}
