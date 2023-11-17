use crate::collector::{disk_free, Manager};
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web_extras::middleware::Condition;
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::middleware::HttpAuthentication;
use clap::Parser;
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

    HttpServer::new(move || {
        App::new()
            .app_data(manager.clone())
            .wrap(Condition::from_option(auth.clone()))
            .wrap(Logger::default())
            .service(index)
            .service(collect)
            .service(collect_all)
    })
    .workers(1)
    .bind(cli.bind_addr)?
    .run()
    .await?;

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
