//! Persistent state in the secure element's NVM. One AtomicStorage holding
//! the whole app state: updates are all-or-nothing across power loss, which
//! is what makes "a burned number is never a duplicated number" true.

use crate::certs::{ALBUM_CERT_LEN, PRESSING_CERT_LEN, TITLE_MAX};
use crate::crypto::{self, PUBKEY_LEN};
use crate::AppSW;
use ledger_device_sdk::nvm::{AtomicStorage, SingleStorage};
use ledger_device_sdk::NVMData;

#[derive(Clone, Copy)]
pub struct PresseNvm {
    pub initialized: u8,
    pub dev_priv: [u8; 32],
    pub dev_pub: [u8; PUBKEY_LEN],

    pub has_master: u8,
    pub alb_priv: [u8; 32],
    pub alb_pub: [u8; PUBKEY_LEN],
    pub title: [u8; TITLE_MAX],
    pub title_len: u8,
    pub edition: u16,
    pub counter: u16,
    pub album_cert: [u8; ALBUM_CERT_LEN],

    pub has_pressing: u8,
    pub pressing_cert: [u8; PRESSING_CERT_LEN],
    pub pressing_album_cert: [u8; ALBUM_CERT_LEN],
}

const EMPTY: PresseNvm = PresseNvm {
    initialized: 0,
    dev_priv: [0; 32],
    dev_pub: [0; PUBKEY_LEN],
    has_master: 0,
    alb_priv: [0; 32],
    alb_pub: [0; PUBKEY_LEN],
    title: [0; TITLE_MAX],
    title_len: 0,
    edition: 0,
    counter: 0,
    album_cert: [0; ALBUM_CERT_LEN],
    has_pressing: 0,
    pressing_cert: [0; PRESSING_CERT_LEN],
    pressing_album_cert: [0; ALBUM_CERT_LEN],
};

#[link_section = ".nvm_data"]
static mut DATA: NVMData<AtomicStorage<PresseNvm>> = NVMData::new(AtomicStorage::new(&EMPTY));

pub struct Store;

impl Store {
    /// Read-only snapshot of NVM. Lazily generates the device identity key on
    /// first access so every device has an identity before any ceremony.
    pub fn get() -> Result<PresseNvm, AppSW> {
        let data = &raw mut DATA;
        let storage = unsafe { (*data).get_mut() };
        let mut current = *storage.get_ref();
        if current.initialized == 0 {
            let (sk, pk) = crypto::gen_keypair()?;
            current.dev_priv = sk;
            current.dev_pub = pk;
            current.initialized = 1;
            unsafe {
                storage.update(&current);
            }
        }
        Ok(current)
    }

    /// Atomically persist a full new state.
    pub fn put(new_state: &PresseNvm) {
        let data = &raw mut DATA;
        let storage = unsafe { (*data).get_mut() };
        unsafe {
            storage.update(new_state);
        }
    }
}
