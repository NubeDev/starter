//! Sample shape for the `admin create` bootstrap subcommand.
//!
//! **Not registered by `register_starter_defaults` on purpose.** The
//! command needs the consumer's database pool to call
//! `starter_auth_users::admin::create_admin`, and `starter-cli` is
//! deliberately store-agnostic (SCOPE: it talks to a server via
//! `starter-client-rs`, never directly to the DB). This file lives
//! here as a copy-paste template for consumers wiring their own
//! `admin` subcommand inside their binary: instantiate a pool +
//! `SqliteUserStore`, prompt for the password via
//! [`crate::prompt::password`], and call `create_admin`.
//!
//! Re-introduce as a generic-over-store `AdminCreate<U: UserStore>`
//! when a real consumer needs a copy-paste-free integration.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};

use crate::registry::{Command, CommandError};

/// `admin` subcommand surface. Run path returns a UserFacing error
/// pointing to the module doc — consumers wire their own.
#[allow(dead_code)]
pub(crate) struct AdminCreate;

#[async_trait]
impl Command for AdminCreate {
    fn name(&self) -> &'static str {
        "admin"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Manage admin users (programmatic; consumer wires the store)")
            .subcommand(
                ClapCommand::new("create")
                    .about("Create the first-run admin user")
                    .arg(Arg::new("email").long("email").required(true))
                    .arg(
                        Arg::new("role")
                            .long("role")
                            .default_value("admin")
                            .value_parser(["reader", "writer", "admin"]),
                    ),
            )
    }

    async fn run(&self, _matches: &ArgMatches) -> Result<(), CommandError> {
        Err(CommandError::UserFacing(
            "starter-cli does not ship a stock `admin create` — wire one in your binary using starter_auth_users::admin::create_admin against your own pool".into(),
        ))
    }
}
