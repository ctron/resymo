//! Load average collector

use async_trait::async_trait;
use homeassistant_agent::model::{Discovery, StateClass};
use serde_json::Value;
use sysinfo::{LoadAvg, System};

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
        let LoadAvg { one, five, fifteen } = System::load_average();
        Ok(serde_json::to_value(Status { one, five, fifteen })?)
    }

    fn describe_ha(&self) -> Vec<Discovery> {
        vec![
            Discovery {
                unique_id: Some("loadavg_1".to_string()),
                name: Some("Load Average 1m".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.one }}".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("loadavg_5".to_string()),
                name: Some("Load Average 5m".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.five }}".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("loadavg_15".to_string()),
                name: Some("Load Average 15m".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.fifteen }}".to_string()),
                ..Default::default()
            },
        ]
    }
}
