use async_trait::async_trait;
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
                },
            );
        }

        Ok(serde_json::to_value(Status { disks: result })?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    #[serde(default)]
    pub disks: HashMap<String, DiskStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiskStatus {
    pub total: u64,
    pub free: u64,
}
