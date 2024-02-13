use crate::collector::Manager;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, Responder};
use actix_web_extras::middleware::Condition;
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::middleware::HttpAuthentication;
use anyhow::bail;
use resymo_common::http;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::Arc;

const DEFAULT_BIND_PORT: u16 = 4242;
const DEFAULT_BIND_HOST: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// Remote access token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,

    /// Allow disabling the authentication
    #[serde(default)]
    disable_authentication: bool,

    #[serde(flatten)]
    http: http::Options,
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

    http::run_server(
        options.http,
        http::Defaults {
            port: DEFAULT_BIND_PORT,
            host: DEFAULT_BIND_HOST,
        },
        move || {
            App::new()
                .app_data(manager.clone())
                .wrap(Condition::from_option(auth.clone()))
                .wrap(Logger::default())
                .service(index)
                .service(collect)
                .service(collect_all)
        },
    )
    .await?;

    Ok(())
}
