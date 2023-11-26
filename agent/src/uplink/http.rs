use crate::collector::Manager;
use actix_tls::accept::openssl::reexports::SslAcceptor;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web_extras::middleware::Condition;
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::middleware::HttpAuthentication;
use anyhow::bail;
use openssl::ssl::{SslFiletype, SslMethod};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

const DEFAULT_BIND_PORT: u16 = 4242;
const DEFAULT_BIND_HOST: &str = "::1";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// Bind host
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bind_host: Option<String>,

    /// Bind port
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bind_port: Option<u16>,

    /// Remote access token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,

    /// Allow disabling the authentication
    #[serde(default)]
    disable_authentication: bool,

    /// A TLS certificate
    #[cfg(feature = "openssl")]
    tls_certificate: Option<PathBuf>,

    /// A TLS key
    #[cfg(feature = "openssl")]
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

pub async fn run(options: Options, manager: Arc<Manager>) -> anyhow::Result<()> {
    let manager = web::Data::from(manager);

    let bind_addr = SocketAddr::new(
        IpAddr::from_str(options.bind_host.as_deref().unwrap_or(DEFAULT_BIND_HOST))?,
        options.bind_port.unwrap_or(DEFAULT_BIND_PORT),
    );

    log::info!("  Binding on: {}", bind_addr);
    log::info!(
        "  TLS - key: {}",
        options
            .tls_key
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );
    log::info!(
        "  TLS - certificate: {}",
        options
            .tls_certificate
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );

    let auth = options.token.map(|token| {
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
        if options.disable_authentication {
            log::warn!("Running without access token. This is discouraged as it may compromise your system.");
        } else {
            bail!("Running without access token. This is discouraged as it may compromise your system. If you really want to do it, use --disable-authentication");
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

    let server = match (options.tls_key, options.tls_certificate) {
        (Some(key), Some(cert)) => {
            #[cfg(feature = "openssl")]
            {
                let mut acceptor = SslAcceptor::mozilla_modern_v5(SslMethod::tls_server())?;
                acceptor.set_certificate_chain_file(cert)?;
                acceptor.set_private_key_file(key, SslFiletype::PEM)?;
                server.bind_openssl(bind_addr, acceptor)?.run()
            }
        }
        (None, None) => server.bind(bind_addr)?.run(),
        _ => {
            bail!("Enabling TLS requires both --tls-key and --tls-certificate");
        }
    };

    server.await?;

    Ok(())
}
