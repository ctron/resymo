pub mod disk_free;
pub mod load_avg;
pub mod memory;
pub mod swap;

use actix_web::{body::BoxBody, HttpResponse, ResponseError};
use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

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

#[derive(Default)]
pub struct Manager {
    pub collectors: HashMap<String, Box<dyn Collector>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            collectors: Default::default(),
        }
    }

    pub fn register<N: Into<String>, C: Collector + 'static>(
        mut self,
        name: N,
        collector: C,
    ) -> Self {
        self.collectors.insert(name.into(), Box::new(collector));
        self
    }

    pub async fn collect_one(&self, name: &str) -> Result<Option<serde_json::Value>, Error> {
        Ok(match self.collectors.get(name) {
            Some(collector) => Some(
                collector
                    .collect()
                    .await
                    .map_err(|err| Error::Collector(err.to_string()))?,
            ),
            None => None,
        })
    }

    pub async fn collect_all(&self) -> Result<BTreeMap<String, serde_json::Value>, Error> {
        let mut result = BTreeMap::new();
        for (name, collector) in &self.collectors {
            result.insert(
                name.to_string(),
                collector
                    .collect()
                    .await
                    .map_err(|err| Error::Collector(err.to_string()))?,
            );
        }
        Ok(result)
    }
}
