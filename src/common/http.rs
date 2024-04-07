use actix_http::Request;
use actix_service::IntoServiceFactory;
use actix_web::body::MessageBody;
use actix_web::dev::{AppConfig, Response, Service, ServiceFactory};
use actix_web::*;
use anyhow::bail;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, clap::Args, schemars::JsonSchema)]
pub struct Options {
    /// Bind host
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[arg(long, env)]
    bind_host: Option<String>,

    /// Bind port
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[arg(long, env)]
    bind_port: Option<u16>,

    /// A TLS certificate
    #[cfg(feature = "openssl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[arg(long, env)]
    tls_certificate: Option<PathBuf>,

    /// A TLS key
    #[cfg(feature = "openssl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[arg(long, env)]
    tls_key: Option<PathBuf>,
}

pub struct Defaults {
    pub port: u16,
    pub host: IpAddr,
}

pub async fn run_server<F, I, S, B>(
    options: Options,
    defaults: Defaults,
    factory: F,
) -> anyhow::Result<()>
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoServiceFactory<S, Request>,

    S: ServiceFactory<Request, Config = AppConfig> + 'static,
    S::Error: Into<Error> + 'static,
    S::InitError: fmt::Debug,
    S::Response: Into<Response<B>> + 'static,
    <S::Service as Service<Request>>::Future: 'static,
    S::Service: 'static,

    B: MessageBody + 'static,
{
    let bind_addr = SocketAddr::new(
        options
            .bind_host
            .as_deref()
            .map(IpAddr::from_str)
            .transpose()?
            .unwrap_or(defaults.host),
        options.bind_port.unwrap_or(defaults.port),
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

    let server = HttpServer::new(factory);

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
