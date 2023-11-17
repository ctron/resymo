use async_trait::async_trait;
use serde_json::Value;
use sysinfo::{LoadAvg, System, SystemExt};

pub struct Collector;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        let system = System::new();
        let LoadAvg { one, five, fifteen } = system.load_average();
        Ok(serde_json::to_value(Status { one, five, fifteen })?)
    }
}
