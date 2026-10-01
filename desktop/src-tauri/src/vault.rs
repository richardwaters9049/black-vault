//! Local vault core. Secret material remains behind the Rust/Tauri IPC boundary.
//! A separate random master key is wrapped for each vault with a password-derived KEK.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};
use zeroize::{Zeroize, Zeroizing};

mod crypto;
mod error;
mod files;
mod models;
mod nested;
mod session;

use crypto::{derive, open, seal, valid_id};
pub use error::VaultError;
use models::{
    KdfRecord, NestedVaultPayload, NestedVaultRecord, OpenVault, VaultRecord, WrappedSecret,
};
pub use models::{NestedVaultSummary, VaultSummary};

const KDF_MEMORY_KIB: u32 = 19_456;
const KDF_ITERATIONS: u32 = 2;
const KDF_PARALLELISM: u32 = 1;
const MASTER_KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const RECORD_MAX_BYTES: u64 = 8_192;

// Test-only, thread-local deterministic failure points for encrypted transfer and restore
// commit sequence. Never enabled in production; no secret material is logged.
#[cfg(test)]
thread_local! {
    static TRANSFER_FAILURE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
fn transfer_checkpoint(stage: &'static str) -> std::io::Result<()> {
    // Opt-in integration child exits immediately at a real persistence boundary.
    // Compiled only for tests; the parent verifies the exit and reopens storage.
    if std::env::var("BV_TEST_EXIT_STAGE").ok().as_deref() == Some(stage) {
        if let Ok(marker) = std::env::var("BV_TEST_EXIT_MARKER") {
            let mut marker_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)?;
            marker_file.write_all(stage.as_bytes())?;
            marker_file.sync_all()?;
        }
        std::process::exit(86);
    }
    if TRANSFER_FAILURE.with(|slot| slot.get() == Some(stage)) {
        return Err(std::io::Error::other("injected transfer interruption"));
    }
    Ok(())
}
#[cfg(not(test))]
fn transfer_checkpoint(_stage: &'static str) -> std::io::Result<()> {
    Ok(())
}

pub struct VaultStore {
    root: PathBuf,
    active: Mutex<Option<OpenVault>>,
    nested_attempts: Mutex<std::collections::HashMap<String, (u8, std::time::Instant)>>,
}

#[cfg(test)]
mod tests;
