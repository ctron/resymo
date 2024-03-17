use async_trait::async_trait;
use homeassistant_agent::model::{Discovery, SensorClass, StateClass};
use serde_json::Value;
use std::collections::HashMap;
use sysinfo::Disks;

#[derive(Default)]
pub struct Collector;

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        let disks = Disks::new_with_refreshed_list();

        let mut result = HashMap::new();
        for disk in disks.list() {
            result.insert(
                disk.name().to_string_lossy().to_string(),
                DiskStatus {
                    free: disk.available_space(),
                    total: disk.total_space(),
                    usage: 1f64 - (disk.available_space() as f64) / (disk.total_space() as f64),
                },
            );
        }

        Ok(serde_json::to_value(Status { disks: result })?)
    }

    fn describe_ha(&self) -> Vec<Discovery> {
        let mut result = vec![];
        let disks = Disks::new_with_refreshed_list();

        for disk in disks.list() {
            let display_name = disk.name().to_string_lossy();
            let id_name = display_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
            let source_name = disk.name().to_string_lossy();

            result.push(Discovery {
                unique_id: Some(format!("disk_{id_name}_free")),
                name: Some(format!("Disk free {display_name}")),
                state_class: Some(StateClass::Measurement),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                value_template: Some(format!(
                    r#"{{{{ value_json.disks['{source_name}'].free }}}}"#
                )),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            });
            result.push(Discovery {
                unique_id: Some(format!("disk_{id_name}_total")),
                name: Some(format!("Disk total {display_name}")),
                state_class: Some(StateClass::Measurement),
                value_template: Some(format!(
                    r#"{{{{ value_json.disks['{source_name}'].total }}}}"#
                )),
                device_class: Some(SensorClass::DataSize.as_ref().to_string()),
                unit_of_measurement: Some("B".to_string()),
                ..Default::default()
            });
            result.push(Discovery {
                unique_id: Some(format!("disk_{id_name}_usage")),
                name: Some(format!("Disk usage {display_name}")),
                state_class: Some(StateClass::Measurement),
                value_template: Some(format!(
                    r#"{{{{ value_json.disks['{source_name}'].usage * 100 }}}}"#
                )),
                unit_of_measurement: Some("%".to_string()),
                ..Default::default()
            });
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    #[serde(default)]
    pub disks: HashMap<String, DiskStatus>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DiskStatus {
    pub total: u64,
    pub free: u64,
    pub usage: f64,
}
