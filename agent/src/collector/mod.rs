use actix_web::body::BoxBody;
use actix_web::{HttpResponse, ResponseError};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

pub mod disk_free;
pub mod load_avg;

#[async_trait]
pub trait Collector: Send + Sync {
    async fn collect(&self) -> anyhow::Result<serde_json::Value>;
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

pub struct Manager {
    collectors: HashMap<String, Box<dyn Collector>>,
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

    pub async fn collect(&self, name: &str) -> Result<Option<serde_json::Value>, Error> {
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
