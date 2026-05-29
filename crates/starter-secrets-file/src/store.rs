//! `FileSecretStore` — age-encrypted single-file `SecretStore`.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use age::secrecy::ExposeSecret;
use age::x25519::{Identity, Recipient};
use parking_lot::Mutex;
use starter_spi::secrets::{Secret, SecretError, SecretStore};

/// Errors specific to building the file-backed store. Once it's built,
/// `SecretStore` methods return `SecretError`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FileSecretsError {
    /// `STARTER_SECRETS_KEY` was set but did not parse as an age
    /// identity.
    #[error("STARTER_SECRETS_KEY is not a valid age identity: {0}")]
    BadEnvIdentity(String),

    /// The configured identity path could not be read or parsed.
    #[error("identity file at {path:?} could not be loaded: {source}")]
    IdentityFile {
        /// Path that failed to load.
        path: PathBuf,
        /// Wrapped lower-level error.
        #[source]
        source: std::io::Error,
    },

    /// Identity-file contents did not parse as an age identity.
    #[error("identity file at {path:?} did not parse as an age identity: {message}")]
    IdentityParse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Diagnostic from age.
        message: String,
    },

    /// `dirs::data_dir()` was unavailable on this platform.
    #[error("XDG_DATA_HOME could not be resolved for binary {0:?}")]
    NoDataDir(String),

    /// I/O error reaching disk while building the store.
    #[error("file secret store I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Builder for [`FileSecretStore`]. Lets the consumer override the
/// binary name (used for `$XDG_DATA_HOME/<binary>/`) and the identity
/// path before opening the store.
#[derive(Debug, Clone)]
pub struct FileSecretStoreBuilder {
    binary: String,
    data_dir: Option<PathBuf>,
    identity_path: Option<PathBuf>,
}

impl FileSecretStoreBuilder {
    /// Start a builder for `binary` (consumer crate name). The data
    /// directory defaults to `$XDG_DATA_HOME/<binary>/`.
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            data_dir: None,
            identity_path: None,
        }
    }

    /// Override the data directory (mostly for tests).
    pub fn data_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.data_dir = Some(dir.into());
        self
    }

    /// Resolve the data directory from a shared [`starter_paths::Paths`]
    /// handle. The store writes into `<root>/<binary>/secrets/` so it
    /// composes cleanly with the rest of the platform — one resolved
    /// root, many subdirs, instead of every consumer calling
    /// `dirs::data_dir()` for itself.
    ///
    /// Returns an error if the requested subdir is somehow invalid
    /// (relative-only, no path traversal). On success the builder
    /// behaves as if [`Self::data_dir`] had been called with the
    /// resolved path.
    pub fn with_paths(mut self, paths: &starter_paths::Paths) -> Result<Self, FileSecretsError> {
        // `binary/secrets` so a single Paths handle can host several
        // binaries' secret blobs without their key files colliding.
        let subdir = format!("{}/secrets", self.binary);
        let dir = paths.subdir(&subdir).map_err(|e| {
            FileSecretsError::Io(std::io::Error::other(format!(
                "resolve secrets subdir {subdir:?}: {e}"
            )))
        })?;
        self.data_dir = Some(dir);
        Ok(self)
    }

    /// Override the identity path. Skipped when `STARTER_SECRETS_KEY`
    /// is set.
    pub fn identity_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.identity_path = Some(path.into());
        self
    }

    /// Open or initialise the store. Generates a fresh identity on
    /// first run when neither the env var nor `identity_path` resolve.
    pub fn build(self) -> Result<FileSecretStore, FileSecretsError> {
        let data_dir = match self.data_dir {
            Some(d) => d,
            None => dirs::data_dir()
                .map(|d| d.join(&self.binary))
                .ok_or_else(|| FileSecretsError::NoDataDir(self.binary.clone()))?,
        };
        fs::create_dir_all(&data_dir)?;

        let secrets_path = data_dir.join("secrets.age");
        let default_identity_path = data_dir.join("identity.age-key");
        let identity_path = self.identity_path.unwrap_or(default_identity_path);

        let identity = resolve_identity(&identity_path)?;
        let recipient = identity.to_public();

        Ok(FileSecretStore {
            inner: Arc::new(Inner {
                path: secrets_path,
                identity,
                recipient,
                cache: Mutex::new(None),
            }),
        })
    }
}

fn resolve_identity(path: &Path) -> Result<Identity, FileSecretsError> {
    if let Ok(env) = std::env::var("STARTER_SECRETS_KEY") {
        return Identity::from_str(env.trim())
            .map_err(|e| FileSecretsError::BadEnvIdentity(e.to_string()));
    }

    match fs::read_to_string(path) {
        Ok(body) => {
            let line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
            Identity::from_str(line.trim()).map_err(|e| FileSecretsError::IdentityParse {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let identity = Identity::generate();
            let serialized = identity.to_string();
            let public = identity.to_public().to_string();
            fs::write(path, serialized.expose_secret().as_bytes()).map_err(|src| {
                FileSecretsError::IdentityFile {
                    path: path.to_path_buf(),
                    source: src,
                }
            })?;
            tracing::warn!(
                target: "starter_secrets_file",
                identity_path = %path.display(),
                public_key = %public,
                "generated a fresh age identity for the secret store — back up this file; losing it makes secrets.age unreadable",
            );
            Ok(identity)
        }
        Err(e) => Err(FileSecretsError::IdentityFile {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}

/// `SecretStore` backed by an age-encrypted file. Construct via
/// [`FileSecretStoreBuilder`].
#[derive(Clone)]
pub struct FileSecretStore {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for FileSecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSecretStore")
            .field("path", &self.inner.path)
            .finish_non_exhaustive()
    }
}

struct Inner {
    path: PathBuf,
    identity: Identity,
    recipient: Recipient,
    cache: Mutex<Option<HashMap<String, String>>>,
}

impl FileSecretStore {
    fn load(&self) -> Result<HashMap<String, String>, SecretError> {
        let mut cache = self.inner.cache.lock();
        if let Some(map) = cache.as_ref() {
            return Ok(map.clone());
        }

        let map = match fs::read(&self.inner.path) {
            Ok(bytes) if bytes.is_empty() => HashMap::new(),
            Ok(bytes) => decrypt(&bytes, &self.inner.identity)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(SecretError::Io(e)),
        };
        *cache = Some(map.clone());
        Ok(map)
    }

    fn persist(&self, map: HashMap<String, String>) -> Result<(), SecretError> {
        let bytes = encrypt(&map, &self.inner.recipient)?;
        let tmp = self.inner.path.with_extension("age.tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.inner.path)?;
        *self.inner.cache.lock() = Some(map);
        Ok(())
    }
}

impl SecretStore for FileSecretStore {
    fn ready(&self) -> bool {
        // Identity resolved at build time; if construction succeeded
        // we are ready. Disk failures surface on the first get/put.
        true
    }

    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
        Ok(self.load()?.get(name).cloned().map(Secret::new))
    }

    fn put(&self, name: &str, value: Secret) -> Result<(), SecretError> {
        let mut map = self.load()?;
        map.insert(name.to_string(), value.into_inner());
        self.persist(map)
    }

    fn delete(&self, name: &str) -> Result<(), SecretError> {
        let mut map = self.load()?;
        if map.remove(name).is_some() {
            self.persist(map)?;
        }
        Ok(())
    }
}

fn encrypt(map: &HashMap<String, String>, recipient: &Recipient) -> Result<Vec<u8>, SecretError> {
    let plain = serde_json::to_vec(map).map_err(|e| SecretError::Backend(e.to_string()))?;
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(recipient as &dyn age::Recipient))
            .map_err(|e| SecretError::Backend(e.to_string()))?;
    let mut out = Vec::new();
    let armor = age::armor::ArmoredWriter::wrap_output(&mut out, age::armor::Format::AsciiArmor)
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    let mut writer = encryptor
        .wrap_output(armor)
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    writer
        .write_all(&plain)
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    let armor = writer
        .finish()
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    armor
        .finish()
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    Ok(out)
}

fn decrypt(bytes: &[u8], identity: &Identity) -> Result<HashMap<String, String>, SecretError> {
    let armor = age::armor::ArmoredReader::new(bytes);
    let decryptor = age::Decryptor::new(armor).map_err(|e| SecretError::Backend(e.to_string()))?;
    if decryptor.is_scrypt() {
        return Err(SecretError::Backend(
            "secrets.age is passphrase-encrypted; expected identity".into(),
        ));
    }
    let mut reader = decryptor
        .decrypt(std::iter::once(identity as &dyn age::Identity))
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| SecretError::Backend(e.to_string()))?;
    serde_json::from_slice(&buf).map_err(|e| SecretError::Backend(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store_in(dir: &TempDir) -> FileSecretStore {
        FileSecretStoreBuilder::new("test-binary")
            .data_dir(dir.path())
            .build()
            .expect("build")
    }

    #[test]
    fn round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.put("k1", Secret::new("v1")).unwrap();
        s.put("k2", Secret::new("v2")).unwrap();
        assert_eq!(s.get("k1").unwrap().unwrap().expose(), "v1");
        assert_eq!(s.get("k2").unwrap().unwrap().expose(), "v2");
        assert!(s.get("missing").unwrap().is_none());
    }

    #[test]
    fn delete_removes() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.put("k", Secret::new("v")).unwrap();
        s.delete("k").unwrap();
        assert!(s.get("k").unwrap().is_none());
        s.delete("k").unwrap();
    }

    #[test]
    fn persists_across_instances() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.put("k", Secret::new("hello")).unwrap();

        let s2 = store_in(&dir);
        assert_eq!(s2.get("k").unwrap().unwrap().expose(), "hello");
    }

    #[test]
    fn ready_true_after_build() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        assert!(s.ready());
    }

    #[test]
    fn with_paths_resolves_under_binary_secrets_subdir() {
        let dir = TempDir::new().unwrap();
        let paths = starter_paths::Paths::from_root(dir.path().to_path_buf());
        paths.ensure().unwrap();
        let s = FileSecretStoreBuilder::new("test-binary")
            .with_paths(&paths)
            .unwrap()
            .build()
            .expect("build");
        s.put("k", Secret::new("v")).unwrap();
        assert!(dir.path().join("test-binary/secrets/secrets.age").exists());
        assert_eq!(s.get("k").unwrap().unwrap().expose(), "v");
    }
}
