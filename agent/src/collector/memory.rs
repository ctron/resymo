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
    pub available: u64,
}

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        let mut system = System::new();
        system.refresh_memory();

        let status = Status {
            free: system.free_memory(),
            total: system.total_memory(),
            used: system.used_memory(),
            available: system.available_memory(),
        };

        Ok(serde_json::to_value(status)?)
    }

    fn describe_ha(&self) -> Vec<Discovery> {
        vec![
            Discovery {
                unique_id: Some("free".to_string()),
                name: Some("Free memory".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.free }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("total".to_string()),
                name: Some("Total memory".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.total }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("used".to_string()),
                name: Some("Used memory".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.used }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
            Discovery {
                unique_id: Some("available".to_string()),
                name: Some("Available memory".to_string()),
                state_class: Some(StateClass::Measurement),
                value_template: Some("{{ value_json.available }}".to_string()),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            },
        ]
    }
}
