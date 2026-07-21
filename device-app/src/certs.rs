//! Fixed-layout certificates. Hand-packed: no serde on-device, and the
//! independent TypeScript verifier re-implements this layout from
//! docs/protocol.md (deliberately no shared code).

use crate::crypto::{self, PUBKEY_LEN, SIG_MAX_LEN};
use crate::AppSW;

pub const TITLE_MAX: usize = 32;

pub const ALBUM_MAGIC: &[u8; 4] = b"PRA1";
pub const ALBUM_PAYLOAD_LEN: usize = 4 + PUBKEY_LEN + 1 + TITLE_MAX + 2;
pub const ALBUM_CERT_LEN: usize = ALBUM_PAYLOAD_LEN + 1 + SIG_MAX_LEN;

pub const PRESSING_MAGIC: &[u8; 4] = b"PRP1";
pub const PRESSING_PAYLOAD_LEN: usize = 4 + 32 + 2 + 2 + PUBKEY_LEN;
pub const PRESSING_CERT_LEN: usize = PRESSING_PAYLOAD_LEN + 1 + SIG_MAX_LEN;

/// AlbumCert layout:
/// magic(4) | albpub(65) | title_len(1) | title(32, zero-padded) | edition(2 LE)
/// | sig_len(1) | sig(72, zero-padded). Signature covers the payload prefix.
pub fn build_album_cert(
    alb_priv: &[u8; 32],
    albpub: &[u8; PUBKEY_LEN],
    title: &[u8],
    edition: u16,
) -> Result<[u8; ALBUM_CERT_LEN], AppSW> {
    if title.is_empty() || title.len() > TITLE_MAX {
        return Err(AppSW::BadCert);
    }
    let mut cert = [0u8; ALBUM_CERT_LEN];
    cert[..4].copy_from_slice(ALBUM_MAGIC);
    cert[4..4 + PUBKEY_LEN].copy_from_slice(albpub);
    cert[69] = title.len() as u8;
    cert[70..70 + title.len()].copy_from_slice(title);
    cert[102..104].copy_from_slice(&edition.to_le_bytes());
    let (sig, sig_len) = crypto::sign_payload(alb_priv, &cert[..ALBUM_PAYLOAD_LEN])?;
    cert[ALBUM_PAYLOAD_LEN] = sig_len;
    cert[ALBUM_PAYLOAD_LEN + 1..].copy_from_slice(&sig);
    Ok(cert)
}

pub struct AlbumInfo {
    pub albpub: [u8; PUBKEY_LEN],
    pub title: [u8; TITLE_MAX],
    pub title_len: u8,
    pub edition: u16,
}

/// Parse and cryptographically verify an AlbumCert.
pub fn parse_album_cert(cert: &[u8]) -> Result<AlbumInfo, AppSW> {
    if cert.len() != ALBUM_CERT_LEN || &cert[..4] != ALBUM_MAGIC {
        return Err(AppSW::BadCert);
    }
    let title_len = cert[69] as usize;
    if title_len == 0 || title_len > TITLE_MAX {
        return Err(AppSW::BadCert);
    }
    let sig_len = cert[ALBUM_PAYLOAD_LEN] as usize;
    if sig_len == 0 || sig_len > SIG_MAX_LEN {
        return Err(AppSW::BadCert);
    }
    let mut albpub = [0u8; PUBKEY_LEN];
    albpub.copy_from_slice(&cert[4..4 + PUBKEY_LEN]);
    let sig = &cert[ALBUM_PAYLOAD_LEN + 1..ALBUM_PAYLOAD_LEN + 1 + sig_len];
    if !crypto::verify_payload(&albpub, &cert[..ALBUM_PAYLOAD_LEN], sig)? {
        return Err(AppSW::BadCert);
    }
    let mut title = [0u8; TITLE_MAX];
    title.copy_from_slice(&cert[70..70 + TITLE_MAX]);
    let edition = u16::from_le_bytes([cert[102], cert[103]]);
    if edition == 0 {
        return Err(AppSW::BadCert);
    }
    Ok(AlbumInfo {
        albpub,
        title,
        title_len: title_len as u8,
        edition,
    })
}

/// PressingCert layout:
/// magic(4) | album_id(32) | number(2 LE) | edition(2 LE) | recvpub(65)
/// | sig_len(1) | sig(72, zero-padded). album_id = SHA256(albpub).
pub fn build_pressing_cert(
    alb_priv: &[u8; 32],
    album_id: &[u8; 32],
    number: u16,
    edition: u16,
    recvpub: &[u8; PUBKEY_LEN],
) -> Result<[u8; PRESSING_CERT_LEN], AppSW> {
    let mut cert = [0u8; PRESSING_CERT_LEN];
    cert[..4].copy_from_slice(PRESSING_MAGIC);
    cert[4..36].copy_from_slice(album_id);
    cert[36..38].copy_from_slice(&number.to_le_bytes());
    cert[38..40].copy_from_slice(&edition.to_le_bytes());
    cert[40..40 + PUBKEY_LEN].copy_from_slice(recvpub);
    let (sig, sig_len) = crypto::sign_payload(alb_priv, &cert[..PRESSING_PAYLOAD_LEN])?;
    cert[PRESSING_PAYLOAD_LEN] = sig_len;
    cert[PRESSING_PAYLOAD_LEN + 1..].copy_from_slice(&sig);
    Ok(cert)
}

pub struct PressingInfo {
    pub album_id: [u8; 32],
    pub number: u16,
    pub edition: u16,
    pub recvpub: [u8; PUBKEY_LEN],
}

/// Parse a PressingCert and verify its signature under the given album key.
pub fn parse_pressing_cert(cert: &[u8], albpub: &[u8; PUBKEY_LEN]) -> Result<PressingInfo, AppSW> {
    if cert.len() != PRESSING_CERT_LEN || &cert[..4] != PRESSING_MAGIC {
        return Err(AppSW::BadCert);
    }
    let sig_len = cert[PRESSING_PAYLOAD_LEN] as usize;
    if sig_len == 0 || sig_len > SIG_MAX_LEN {
        return Err(AppSW::BadCert);
    }
    let sig = &cert[PRESSING_PAYLOAD_LEN + 1..PRESSING_PAYLOAD_LEN + 1 + sig_len];
    if !crypto::verify_payload(albpub, &cert[..PRESSING_PAYLOAD_LEN], sig)? {
        return Err(AppSW::BadCert);
    }
    let mut album_id = [0u8; 32];
    album_id.copy_from_slice(&cert[4..36]);
    let number = u16::from_le_bytes([cert[36], cert[37]]);
    let edition = u16::from_le_bytes([cert[38], cert[39]]);
    let mut recvpub = [0u8; PUBKEY_LEN];
    recvpub.copy_from_slice(&cert[40..40 + PUBKEY_LEN]);
    if number == 0 || edition == 0 || number > edition {
        return Err(AppSW::BadCert);
    }
    Ok(PressingInfo {
        album_id,
        number,
        edition,
        recvpub,
    })
}
