use crate::collector::{disk_free, Manager};
use actix_tls::accept::openssl::reexports::SslAcceptor;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web_extras::middleware::Condition;
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::middleware::HttpAuthentication;
use clap::Parser;
use openssl::ssl::{SslFiletype, SslMethod};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

mod collector;

#[derive(Clone, Debug, clap::Parser)]
pub struct Cli {
    /// Bind address
    #[arg(short, long, env, default_value = "[::1]:4242")]
    bind_addr: String,

    /// Remote access token
    #[arg(short, long, env)]
    token: Option<String>,

    /// Allow disabling the authentication
    #[arg(long, env)]
    disable_authentication: bool,

    /// Be quiet
    #[arg(short, long, env)]
    quiet: bool,

    /// Be more verbose
    #[arg(short, long, env, conflicts_with = "quiet", action = clap::ArgAction::Count)]
    verbose: u8,

    /// A TLS certificate
    #[cfg(feature = "openssl")]
    #[arg(long, env)]
    tls_certificate: Option<PathBuf>,

    /// A TLS key
    #[cfg(feature = "openssl")]
    #[arg(long, env)]
    tls_key: Option<PathBuf>,
}

#[get("/")]
async fn index() -> impl Responder {
    ""
}

#[get("/api/v1/collect")]
async fn collect_all(manager: web::Data<Manager>) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(manager.collect_all().await?))
}

#[get("/api/v1/collect/{collector}")]
async fn collect(
    path: web::Path<String>,
    manager: web::Data<Manager>,
) -> actix_web::Result<HttpResponse> {
    let collector = path.into_inner();

    log::info!("Collecting: {collector}");

    Ok(match manager.collect(&collector).await? {
        Some(result) => HttpResponse::Ok().json(result),
        None => HttpResponse::NotFound().finish(),
    })
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

    let manager = Manager::new().register("disk_free", disk_free::Collector::new());
    let manager = web::Data::new(manager);

    log::info!("Starting agent");
    log::info!("  Binding on: {}", cli.bind_addr);
    log::info!(
        "  TLS - key: {}",
        cli.tls_key
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );
    log::info!(
        "  TLS - certificate: {}",
        cli.tls_certificate
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );

    let auth = cli.token.map(|token| {
        let token = Arc::new(token);
        HttpAuthentication::bearer(move |req, credentials| {
            let token = token.clone();
            async move {
                if credentials.token() == *token {
                    Ok(req)
                } else {
                    let config = req
                        .app_data::<bearer::Config>()
                        .cloned()
                        .unwrap_or_default()
                        .scope("api");

                    Err((AuthenticationError::from(config).into(), req))
                }
            }
        })
    });

    if auth.is_none() {
        if cli.disable_authentication {
            log::warn!("Running without access token. This is discouraged as it may compromise your system.");
        } else {
            log::error!("Running without access token. This is discouraged as it may compromise your system. If you really want to do it, use --disable-authentication");
            return Ok(ExitCode::FAILURE);
        }
    }

    let server = HttpServer::new(move || {
        App::new()
            .app_data(manager.clone())
            .wrap(Condition::from_option(auth.clone()))
            .wrap(Logger::default())
            .service(index)
            .service(collect)
            .service(collect_all)
    })
    .workers(1);

    let server = match (cli.tls_key, cli.tls_certificate) {
        (Some(key), Some(cert)) => {
            #[cfg(feature = "openssl")]
            {
                let mut acceptor = SslAcceptor::mozilla_modern_v5(SslMethod::tls_server())?;
                acceptor.set_certificate_chain_file(cert)?;
                acceptor.set_private_key_file(key, SslFiletype::PEM)?;
                // let acceptor = actix_tls::accept::openssl::Acceptor::new()
                server.bind_openssl(cli.bind_addr, acceptor)?.run()
            }
        }
        (None, None) => server.bind(cli.bind_addr)?.run(),
        _ => {
            log::error!("Enabling TLS requires both --tls-key and --tls-certificate");
            return Ok(ExitCode::FAILURE);
        }
    };

    server.await?;

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
