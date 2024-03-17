use anyhow::Context;
use clap::Parser;
use resymo_agent::collector::{disk_free, load_avg, Manager};
use resymo_agent::config::Config;
use resymo_agent::uplink;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::signal;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

const CONFIG_FILE: &str = "resymo/agent.yaml";

#[derive(Clone, Debug, clap::Parser)]
pub struct Cli {
    /// Be quiet
    #[arg(short, long, env)]
    quiet: bool,

    /// Be more verbose
    #[arg(short, long, env, conflicts_with = "quiet", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to the configuration file
    #[arg(short, long, env, default_value = config_file())]
    config: PathBuf,
}

fn config_file() -> String {
    PathBuf::from("/etc")
        .join(CONFIG_FILE)
        .display()
        .to_string()
}

fn init_logger(cli: &Cli) {
    if std::env::var("RUST_LOG").is_ok() {
        // if we have an external configuration, use it
        env_logger::init();
        return;
    }

    let mut logger = env_logger::builder();

    let filters = match (cli.verbose, cli.quiet) {
        // quiet overrides verbose
        (_, true) => "error,resymo_agent=warn",
        // increase verbosity
        (0, false) => "warn,resymo_agent=info",
        (1, false) => "info,resymo_agent=debug",
        (2, false) => "debug,resymo_agent=trace",
        (_, false) => "trace",
    };

    logger.parse_filters(filters).init();
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();

    init_logger(&cli);

    let config: Config = serde_yaml::from_reader(
        std::fs::File::open(&cli.config)
            .with_context(|| format!("Reading configuration file: '{}'", cli.config.display()))?,
    )?;

    let manager = Arc::new(
        Manager::new()
            .register("disk_free", disk_free::Collector)
            .register("load_avg", load_avg::Collector),
    );

    log::info!("Starting agent");

    let mut uplinks = Vec::<Pin<Box<dyn Future<Output = Result<(), anyhow::Error>>>>>::new();
    if let Some(options) = config.uplinks.http_server {
        log::info!("Starting HTTP server uplink");
        uplinks.push(Box::pin(async {
            uplink::http_server::run(options, manager.clone()).await
        }));
    }
    if let Some(options) = config.uplinks.homeassistant {
        log::info!("Starting Homeassistant MQTT uplink");
        uplinks.push(Box::pin(async {
            uplink::homeassistant::run(options, manager.clone()).await
        }));
    }

    if uplinks.is_empty() {
        log::warn!("No uplink configured");
    }

    let mut tasks = uplinks;
    tasks.push(Box::pin(async {
        signal::ctrl_c().await.context("termination failed")?;
        Ok(())
    }));

    #[cfg(unix)]
    tasks.push(Box::pin(async {
        signal(SignalKind::terminate())?.recv().await;
        Ok(())
    }));

    let (result, _index, _others) = futures::future::select_all(tasks).await;
    result?;

    log::info!("Exiting agent");

    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod test {
    use super::Cli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
