//! Swap space collector

use async_trait::async_trait;
use homeassistant_agent::model::{Discovery, SensorClass, StateClass};
use serde_json::Value;
use sysinfo::System;

pub struct Collector;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub free: u64,
    pub total: u64,
    pub used: u64,
    /// used / total
    pub percentage: f64,
}

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        let mut system = System::new();
        system.refresh_memory();

        let percentage = match system.total_swap() {
            0 => 0f64,
            n => system.used_swap() as f64 / n as f64,
        };

        let status = Status {
            free: system.free_swap(),
            total: system.total_swap(),
            used: system.used_swap(),
            percentage,
        };

        Ok(serde_json::to_value(status)?)
    }

    fn describe_ha(&self) -> Vec<Discovery> {
        vec![
            Discovery {
                unique_id: Some("free".to_string()),
                name: Some("Free swap space".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.free }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("total".to_string()),
                name: Some("Total swap space".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.total }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("used".to_string()),
                name: Some("Used swap space".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.used }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("percentage".to_string()),
                name: Some("Used swap space (%)".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some(r#"{{ value_json.percentage * 100 }}"#.to_string()),
                unit_of_measurement: Some("%".to_string()),
                ..Default::default()
            },
        ]
    }
}
