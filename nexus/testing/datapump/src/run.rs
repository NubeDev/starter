use anyhow::Result;
use tokio::time;

use crate::config::Config;
use crate::generator::Generator;
use crate::transport::Publisher;

pub async fn run(config: Config) -> Result<()> {
    let publisher = Publisher::connect(&config.transport).await?;
    let mut generator = Generator::new(&config.shape);
    let mut interval = time::interval(config.publish.interval);
    let mut sent = 0_u64;

    tracing::info!(
        transport = %config.transport.kind(),
        meters = generator.meter_count(),
        "starting datapump"
    );

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received Ctrl-C");
                break;
            }
            _ = interval.tick() => {
                let telemetry = generator.next();
                publisher
                    .publish(
                        &config.shape.path_prefix,
                        &config.shape.path_tenant,
                        &config.publish,
                        &telemetry,
                    )
                    .await?;
                sent += 1;
                tracing::info!(
                    sent,
                    path = %telemetry.path(&config.shape.path_prefix, &config.shape.path_tenant),
                    value = telemetry.value,
                    "published telemetry"
                );

                if config.publish.count.is_some_and(|limit| sent >= limit) {
                    break;
                }
            }
        }
    }

    publisher.disconnect().await?;
    tracing::info!(sent, "datapump stopped");
    Ok(())
}
