//! `starter admin create --email … --role admin` — bootstrap the
//! first admin user. The password is read from stdin (interactive
//! prompt), never accepted as a flag.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};

use crate::registry::{Command, CommandError};

/// `admin create` subcommand.
pub struct AdminCreate;

#[async_trait]
impl Command for AdminCreate {
    fn name(&self) -> &'static str {
        "admin"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Manage admin users (programmatic-only; no HTTP path)")
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
        // TODO(ap): prompt for password via crate::prompt::password,
        // call starter_auth::admin::create_admin.
        Ok(())
    }
}
