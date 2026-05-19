//! End-to-end tests for the claim flow against a real in-memory
//! SQLite database.

#![cfg(feature = "sqlite")]

use starter_auth_token::{
    claim::claim_pending, regenerate_claim_pending, store::SqliteClaimStore, ClaimError,
    TokenAuthenticator,
};
use starter_spi::auth::{Authenticator, Role};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static TOKEN_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/starter_auth_token");

async fn fresh_store() -> SqliteClaimStore {
    let pool: Pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_token",
            migrator: &TOKEN_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    SqliteClaimStore::new(pool)
}

#[tokio::test]
async fn first_boot_flow_yields_owner_token() {
    let store = fresh_store().await;
    let pending = regenerate_claim_pending(&store).await.expect("seed");
    assert_eq!(pending.plaintext.len(), 43); // 32 bytes b64url no pad

    let claimed = claim_pending(&store, &pending.plaintext)
        .await
        .expect("claim");
    assert_eq!(claimed.claim_id, pending.id);
    assert_eq!(claimed.plaintext.len(), 43);
}

#[tokio::test]
async fn authenticator_accepts_owner_token() {
    let store = fresh_store().await;
    let pending = regenerate_claim_pending(&store).await.unwrap();
    let claimed = claim_pending(&store, &pending.plaintext).await.unwrap();

    let auth = TokenAuthenticator::new(store);
    let principal = auth.verify(&claimed.plaintext).await.expect("verify ok");
    assert_eq!(principal.subject, claimed.claim_id);
    assert_eq!(principal.role, Role::Admin);
    assert!(principal.scopes.is_empty());
}

#[tokio::test]
async fn authenticator_rejects_wrong_token() {
    let store = fresh_store().await;
    let pending = regenerate_claim_pending(&store).await.unwrap();
    let _ = claim_pending(&store, &pending.plaintext).await.unwrap();

    let auth = TokenAuthenticator::new(store);
    let err = auth.verify("not the owner token").await.unwrap_err();
    assert!(matches!(err, starter_spi::Error::Unauthenticated));
}

#[tokio::test]
async fn authenticator_rejects_when_unclaimed() {
    let store = fresh_store().await;
    let _ = regenerate_claim_pending(&store).await.unwrap();
    // Not claimed yet — no digest row, so verify must reject.
    let auth = TokenAuthenticator::new(store);
    let err = auth.verify("anything").await.unwrap_err();
    assert!(matches!(err, starter_spi::Error::Unauthenticated));
}

#[tokio::test]
async fn replay_rejects_after_first_claim() {
    let store = fresh_store().await;
    let pending = regenerate_claim_pending(&store).await.unwrap();
    let _ = claim_pending(&store, &pending.plaintext).await.unwrap();

    // Second attempt with the same pending plaintext is rejected.
    let err = claim_pending(&store, &pending.plaintext).await.unwrap_err();
    assert!(matches!(err, ClaimError::AlreadyClaimed), "{err:?}");
}

#[tokio::test]
async fn wrong_pending_token_rejected_with_invalid() {
    let store = fresh_store().await;
    let _ = regenerate_claim_pending(&store).await.unwrap();
    let err = claim_pending(&store, "definitely-not-it")
        .await
        .unwrap_err();
    assert!(matches!(err, ClaimError::InvalidToken), "{err:?}");
}

#[tokio::test]
async fn claim_without_seed_is_no_pending() {
    let store = fresh_store().await;
    let err = claim_pending(&store, "anything").await.unwrap_err();
    assert!(matches!(err, ClaimError::NoPending), "{err:?}");
}

#[tokio::test]
async fn factory_reset_invalidates_prior_owner_token() {
    let store = fresh_store().await;
    let pending = regenerate_claim_pending(&store).await.unwrap();
    let original = claim_pending(&store, &pending.plaintext).await.unwrap();

    // Reset wipes the claimed row; without it, the prior owner
    // token can no longer authenticate.
    let _new_pending = regenerate_claim_pending(&store).await.unwrap();
    let auth_after = TokenAuthenticator::new(store);
    let err = auth_after.verify(&original.plaintext).await.unwrap_err();
    assert!(matches!(err, starter_spi::Error::Unauthenticated));
}

#[tokio::test]
async fn regenerate_with_secrets_writes_plaintext_to_store() {
    use starter_auth_token::{regenerate_claim_pending_with_secrets, PENDING_SECRET_KEY};
    use starter_spi::secrets::{Secret, SecretError, SecretStore};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemSecrets(Mutex<std::collections::HashMap<String, String>>);
    impl SecretStore for MemSecrets {
        fn ready(&self) -> bool {
            true
        }
        fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
            Ok(self.0.lock().unwrap().get(name).cloned().map(Secret::new))
        }
        fn put(&self, name: &str, value: Secret) -> Result<(), SecretError> {
            self.0
                .lock()
                .unwrap()
                .insert(name.to_string(), value.into_inner());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), SecretError> {
            self.0.lock().unwrap().remove(name);
            Ok(())
        }
    }

    let store = fresh_store().await;
    let secrets = MemSecrets::default();
    let pending = regenerate_claim_pending_with_secrets(&store, &secrets)
        .await
        .expect("seed + secret");

    let stored = secrets.get(PENDING_SECRET_KEY).unwrap().expect("present");
    assert_eq!(stored.expose(), pending.plaintext);
}
