//! Envelope encryption for datasource connection secrets.
//!
//! A datasource password never sits in the database as plaintext or under a
//! single shared key. Each secret gets its own random **data key** that
//! encrypts it (AES-256-GCM); that data key is itself encrypted ("wrapped") by a
//! **master key** held outside the database (env-injected for v1, a KMS later).
//! Only the wrapped data key and the secret ciphertext — never the master key,
//! never plaintext — are persisted.
//!
//! Two consequences this buys: rotating the master key re-wraps the data keys
//! without touching every secret ciphertext, and a database leak yields only
//! ciphertext. Decryption happens solely at stream-build time inside the
//! runners, and every decrypt is audited by the caller.

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key};
use rand::RngCore;

/// What persists for one secret: the secret encrypted under its data key, and
/// that data key encrypted under the master key. Nonces are stored alongside;
/// they are not secret, only unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedSecret {
    pub secret_cipher: Vec<u8>,
    pub secret_nonce: Vec<u8>,
    pub wrapped_data_key: Vec<u8>,
    pub data_key_nonce: Vec<u8>,
    pub key_version: i32,
}

/// Failure modes of sealing/opening. All decrypt failures collapse to
/// `Decrypt` so a caller can't probe which step failed.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("invalid master key (must be 32 bytes)")]
    BadMasterKey,
    #[error("secret decryption failed")]
    Decrypt,
}

/// Holds the master key and the version it represents. Constructed once from
/// config; rotating keys means holding several by version.
#[derive(Clone)]
pub struct Envelope {
    master: Key<Aes256Gcm>,
    key_version: i32,
}

impl Envelope {
    /// Build from a 32-byte master key and its version. Rejects any other length
    /// rather than silently truncating/padding.
    pub fn new(master_key: &[u8], key_version: i32) -> Result<Self, SecretError> {
        if master_key.len() != 32 {
            return Err(SecretError::BadMasterKey);
        }
        Ok(Self {
            master: *Key::<Aes256Gcm>::from_slice(master_key),
            key_version,
        })
    }

    /// Encrypt `plaintext` under a fresh data key wrapped by the master key.
    pub fn seal(&self, plaintext: &[u8]) -> Result<SealedSecret, SecretError> {
        let mut data_key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut data_key_bytes);
        let data_key = Key::<Aes256Gcm>::from_slice(&data_key_bytes);

        let (secret_cipher, secret_nonce) = encrypt(data_key, plaintext)?;
        let (wrapped_data_key, data_key_nonce) = encrypt(&self.master, &data_key_bytes)?;

        Ok(SealedSecret {
            secret_cipher,
            secret_nonce,
            wrapped_data_key,
            data_key_nonce,
            key_version: self.key_version,
        })
    }

    /// Recover the plaintext from a sealed secret. The caller audits the call.
    pub fn open(&self, sealed: &SealedSecret) -> Result<Vec<u8>, SecretError> {
        let data_key_bytes = decrypt(
            &self.master,
            &sealed.wrapped_data_key,
            &sealed.data_key_nonce,
        )?;
        let data_key = Key::<Aes256Gcm>::from_slice(&data_key_bytes);
        decrypt(data_key, &sealed.secret_cipher, &sealed.secret_nonce)
    }
}

fn encrypt(key: &Key<Aes256Gcm>, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), SecretError> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| SecretError::Decrypt)?;
    Ok((ct, nonce.to_vec()))
}

fn decrypt(
    key: &Key<Aes256Gcm>,
    cipher_bytes: &[u8],
    nonce: &[u8],
) -> Result<Vec<u8>, SecretError> {
    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from_slice(nonce);
    cipher
        .decrypt(nonce, cipher_bytes)
        .map_err(|_| SecretError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn master() -> [u8; 32] {
        *b"0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn round_trips_a_secret() {
        let env = Envelope::new(&master(), 1).unwrap();
        let sealed = env.seal(b"hunter2").unwrap();
        // The plaintext is nowhere in the sealed form.
        assert!(!sealed.secret_cipher.windows(7).any(|w| w == b"hunter2"));
        assert_eq!(env.open(&sealed).unwrap(), b"hunter2");
    }

    #[test]
    fn a_wrong_master_key_cannot_open() {
        let env = Envelope::new(&master(), 1).unwrap();
        let sealed = env.seal(b"hunter2").unwrap();
        let other = Envelope::new(b"ffffffffffffffffffffffffffffffff", 1).unwrap();
        assert!(matches!(other.open(&sealed), Err(SecretError::Decrypt)));
    }

    #[test]
    fn each_seal_uses_a_fresh_data_key_and_nonce() {
        let env = Envelope::new(&master(), 1).unwrap();
        let a = env.seal(b"same").unwrap();
        let b = env.seal(b"same").unwrap();
        // Same plaintext, different ciphertext — no deterministic encryption.
        assert_ne!(a.secret_cipher, b.secret_cipher);
        assert_ne!(a.wrapped_data_key, b.wrapped_data_key);
    }

    #[test]
    fn rejects_a_short_master_key() {
        assert!(matches!(
            Envelope::new(b"too-short", 1),
            Err(SecretError::BadMasterKey)
        ));
    }
}
