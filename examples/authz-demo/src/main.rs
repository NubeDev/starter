//! `authz-demo` — show user creation + per-user authz over a
//! built-in endpoint and an extension-contributed endpoint.
//!
//! Subcommands:
//!
//!   migrate                              apply auth-users + authz + reports migrations
//!   serve                                run the HTTP server
//!   user create <email> <password>       create a user (role: --role reader|writer|admin)
//!   user token <user-id>                 issue an API token for a user (prints once)
//!   grant  <user-id> <resource> <action> insert an Allow rule scoped to that user
//!   revoke <user-id> <resource> <action> insert a Deny rule scoped to that user
//!
//! `user`, `grant`, and `revoke` mutate the database directly — they
//! never go through HTTP. This keeps the bootstrap loop short and
//! avoids the chicken-and-egg of "you need an admin token to create
//! the first admin".

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Arg, ArgMatches, Command};
use starter_auth_users::role::Role;
use starter_auth_users::store::{SqliteTokenStore, SqliteUserStore};
use starter_auth_users::store::{TokenStore, UserStore};
use starter_authz::store::SqlitePolicyStore;
use starter_authz::{DbPolicyEngine, StaticRegistry};
use starter_authz_demo::{admin, migrations, server as demo_server};
use starter_observability::{metrics::StandardMetrics, tracing::Format};
use starter_spi::authz::{Ownership, ResourceRegistry, ResourceSpec};
use starter_store_sqlite::{migrate, pool, Pool};

const DEFAULT_DATABASE_URL: &str = "sqlite:./authz-demo.db?mode=rwc";
const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = starter_observability::tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    let app = Command::new("authz-demo")
        .about("User + per-user authz demo.")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("migrate")
                .about("Apply auth-users + authz + reports migrations")
                .arg(database_url_arg()),
        )
        .subcommand(
            Command::new("serve")
                .about("Run the HTTP server")
                .arg(database_url_arg())
                .arg(
                    Arg::new("http-bind")
                        .long("http-bind")
                        .default_value(DEFAULT_HTTP_BIND),
                ),
        )
        .subcommand(
            Command::new("user")
                .about("User management")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .about("Create a user")
                        .arg(database_url_arg())
                        .arg(Arg::new("email").required(true))
                        .arg(Arg::new("password").required(true))
                        .arg(
                            Arg::new("role")
                                .long("role")
                                .default_value("reader")
                                .help("reader | writer | admin"),
                        ),
                )
                .subcommand(
                    Command::new("token")
                        .about("Issue an API token for the given user id")
                        .arg(database_url_arg())
                        .arg(Arg::new("user-id").required(true)),
                ),
        )
        .subcommand(
            Command::new("grant")
                .about("Allow <user-id> to <action> <resource>")
                .arg(database_url_arg())
                .arg(Arg::new("user-id").required(true))
                .arg(Arg::new("resource").required(true))
                .arg(Arg::new("action").required(true))
                .arg(
                    Arg::new("as-admin")
                        .long("as-admin")
                        .default_value("system")
                        .help("Subject id recorded as `created_by`"),
                ),
        )
        .subcommand(
            Command::new("revoke")
                .about("Deny <user-id> from <action> <resource> (deny-overrides)")
                .arg(database_url_arg())
                .arg(Arg::new("user-id").required(true))
                .arg(Arg::new("resource").required(true))
                .arg(Arg::new("action").required(true))
                .arg(
                    Arg::new("as-admin")
                        .long("as-admin")
                        .default_value("system"),
                ),
        );

    let matches = app.get_matches();
    match matches.subcommand() {
        Some(("migrate", sub)) => run_migrate(sub).await,
        Some(("serve", sub)) => run_serve(sub).await,
        Some(("user", sub)) => match sub.subcommand() {
            Some(("create", s)) => run_user_create(s).await,
            Some(("token", s)) => run_user_token(s).await,
            _ => unreachable!(),
        },
        Some(("grant", sub)) => run_grant_or_revoke(sub, true).await,
        Some(("revoke", sub)) => run_grant_or_revoke(sub, false).await,
        _ => unreachable!(),
    }
}

fn database_url_arg() -> Arg {
    Arg::new("database-url")
        .long("database-url")
        .env("DATABASE_URL")
        .default_value(DEFAULT_DATABASE_URL)
}

async fn open_pool(matches: &ArgMatches) -> Result<Pool> {
    let url = matches
        .get_one::<String>("database-url")
        .map(String::as_str)
        .unwrap_or(DEFAULT_DATABASE_URL);
    pool::connect(url)
        .await
        .with_context(|| format!("connect to {url}"))
}

async fn run_migrate(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;
    let mut chain = migrate(&pool);
    for source in migrations::sources() {
        chain = chain.with_source(source);
    }
    chain.run().await.context("apply migrations")?;
    println!("migrations applied");
    Ok(())
}

async fn run_serve(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;

    let registry = Arc::new(prometheus::Registry::new());
    let metrics = Arc::new(
        StandardMetrics::register(&registry).context("register prometheus metrics")?,
    );

    let built = demo_server::build(pool, registry, metrics).await?;

    let http_bind: SocketAddr = matches
        .get_one::<String>("http-bind")
        .map(String::as_str)
        .unwrap_or(DEFAULT_HTTP_BIND)
        .parse()
        .context("parse --http-bind")?;

    tracing::info!(%http_bind, "authz-demo serving");
    starter_server::builder::bind(built.router, http_bind)
        .await
        .context("http serve")
}

async fn run_user_create(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;
    let email = matches.get_one::<String>("email").unwrap();
    let password = matches.get_one::<String>("password").unwrap();
    let role = parse_role(matches.get_one::<String>("role").unwrap())?;

    let users: Arc<dyn UserStore> = Arc::new(SqliteUserStore::new(pool));
    let id = admin::create_user(&users, email, password, role).await?;
    println!("{id}");
    Ok(())
}

async fn run_user_token(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;
    let user_id = matches.get_one::<String>("user-id").unwrap();
    let tokens: Arc<dyn TokenStore> = Arc::new(SqliteTokenStore::new(pool));
    let issued = admin::issue_token(&tokens, user_id).await?;
    println!("{}", issued.plaintext);
    Ok(())
}

async fn run_grant_or_revoke(matches: &ArgMatches, allow: bool) -> Result<()> {
    let pool = open_pool(matches).await?;
    let user_id = matches.get_one::<String>("user-id").unwrap();
    let resource = matches.get_one::<String>("resource").unwrap();
    let action = matches.get_one::<String>("action").unwrap();
    let as_admin = matches.get_one::<String>("as-admin").unwrap();

    // Build the engine just like `server.rs` does so the rule lands in
    // the same `starter_authz_rules` table the running server reads.
    let store = Arc::new(SqlitePolicyStore::new(pool));
    let registry = Arc::new(StaticRegistry::new());
    registry.register_spec(ResourceSpec::from_static(
        "reports",
        &["read", "create"],
        Ownership::Subject,
        "Reports",
        "",
    ));
    registry.register_spec(ResourceSpec::from_static(
        "weather",
        &["read", "refresh"],
        Ownership::None,
        "Weather",
        "",
    ));
    let engine = Arc::new(
        DbPolicyEngine::new(store, registry as Arc<dyn ResourceRegistry>, true)
            .await
            .context("build engine")?,
    );

    let id = if allow {
        admin::grant(&engine, as_admin, user_id, resource, action).await?
    } else {
        admin::revoke(&engine, as_admin, user_id, resource, action).await?
    };
    println!("{id}");
    Ok(())
}

fn parse_role(s: &str) -> Result<Role> {
    match s.to_ascii_lowercase().as_str() {
        "reader" => Ok(Role::Reader),
        "writer" => Ok(Role::Writer),
        "admin" => Ok(Role::Admin),
        other => Err(anyhow::anyhow!(
            "unknown role `{other}` (reader | writer | admin)"
        )),
    }
}

