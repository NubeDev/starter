mod cli;
mod config;
mod generator;
mod model;
mod mqtt;
mod run;
mod transport;
mod zenoh;

use anyhow::Result;
use clap::Parser;

use crate::cli::Args;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    run::run(Args::parse().try_into()?).await
}
