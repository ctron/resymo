pub mod disk_free;
pub mod exec;
pub mod load_avg;
pub mod memory;
pub mod swap;

use actix_web::{body::BoxBody, HttpResponse, ResponseError};
use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct ValueDescriptor {
    pub name: &'static str,
    pub unit_of_measurement: Option<&'static str>,
    pub value_template: &'static str,
}

#[async_trait]
pub trait Collector: Send + Sync {
    async fn collect(&self) -> anyhow::Result<serde_json::Value>;

    /// Describe payload for Home Assistant
    fn describe_ha(&self) -> Vec<Discovery> {
        vec![]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Collector error: {0}")]
    Collector(String),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Self::Collector(err) => HttpResponse::InternalServerError().json(json!({
                "type": "CollectorError",
                "message": err
            })),
        }
    }
}
