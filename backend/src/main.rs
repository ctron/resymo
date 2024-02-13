mod endpoints;

use actix_web::middleware::Logger;
use actix_web::App;
use clap::Parser;
use futures::FutureExt;
use resymo_common::http;
use std::net::{IpAddr, Ipv6Addr};

#[derive(Clone, Debug, clap::Parser)]
pub struct Cli {
    #[command(flatten)]
    http: http::Options,
}

const DEFAULT_HOST: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 8080;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    env_logger::init();

    let http = async {
        http::run_server(
            cli.http,
            http::Defaults {
                port: DEFAULT_PORT,
                host: DEFAULT_HOST,
            },
            || {
                App::new()
                    .wrap(Logger::default())
                    .configure(endpoints::configure)
            },
        )
        .await
    };

    let (result, _, _) = futures::future::select_all(vec![http.boxed()]).await;

    result?;

    Ok(())
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
