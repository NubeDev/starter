//! `rubix-admin secrets <verb>` — manage the age-encrypted secret
//! store the agent reads at boot.
//!
//! The canonical use is stashing the database password once so the
//! agent's `agent.toml` can carry a password-less DSN plus
//! `database_password_secret = "db:password"`:
//!
//! ```text
//! rubix-admin secrets set db:password <pwd>
//! ```
//!
//! The store is the same age-encrypted
//! [`FileSecretStore`](starter_secrets_file::FileSecretStore) the
//! agent opens via `boot::secrets::build_secrets_store`. Its root
//! directory comes from `--path`, else the agent config's
//! `secrets_path`. The matching identity key is created (on first
//! run) and read at `<path>/identity.age-key` — back that file up.

use anyhow::{anyhow, Result};
use clap::{Args as ClapArgs, Subcommand};
use starter_secrets_file::FileSecretStoreBuilder;
use starter_spi::secrets::{Secret, SecretStore};
use tracing::info;

use rubix_agent::boot::AgentConfig;

/// Keyring service / file-store binary name. Matches
/// `boot::secrets::SECRETS_BINARY` so the agent reads the same store.
const SECRETS_BINARY: &str = "rubix-agent";

#[derive(Debug, Subcommand)]
pub enum SecretsCommand {
    /// Store (or overwrite) a secret value under `name`.
    Set(SetArgs),
}

/// Flags for `secrets set`.
#[derive(Debug, ClapArgs)]
pub struct SetArgs {
    /// Logical secret name, e.g. `db:password`.
    name: String,

    /// Secret value. Never logged.
    #[arg(hide = true)]
    value: String,

    /// Override the secrets directory. Defaults to the agent config's
    /// `secrets_path`. Required when that is unset.
    #[arg(long)]
    path: Option<std::path::PathBuf>,
}

pub async fn run(cmd: SecretsCommand) -> Result<()> {
    match cmd {
        SecretsCommand::Set(args) => set(args).await,
    }
}

async fn set(args: SetArgs) -> Result<()> {
    // Resolve the store directory: explicit `--path` wins, else the
    // agent config's `secrets_path`. Without either there is no file
    // store to write to (the env-var fallback's `put` is a no-op).
    let dir = match args.path {
        Some(p) => p,
        None => AgentConfig::load()
            .map_err(|e| anyhow!("load agent config: {e}"))?
            .secrets_path
            .ok_or_else(|| {
                anyhow!(
                    "no secrets directory: set `secrets_path` in agent.toml \
                     or pass `--path <dir>`"
                )
            })?,
    };

    let store = FileSecretStoreBuilder::new(SECRETS_BINARY)
        .data_dir(dir.clone())
        .build()
        .map_err(|e| anyhow!("open file secret store at {}: {e}", dir.display()))?;

    store
        .put(&args.name, Secret::new(args.value))
        .map_err(|e| anyhow!("store secret {:?}: {e}", args.name))?;

    info!(
        name = %args.name,
        dir = %dir.display(),
        "secret stored; identity key lives at <dir>/identity.age-key — back it up"
    );
    Ok(())
}
