use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use sysinfo::{DiskExt, System, SystemExt};

pub struct Collector {}

impl Collector {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        let mut system = System::new();
        system.refresh_disks_list();
        system.refresh_disks();

        let mut disks = HashMap::new();
        for disk in system.disks() {
            disks.insert(
                disk.name().to_string_lossy().to_string(),
                DiskStatus {
                    free: disk.available_space(),
                    total: disk.total_space(),
                },
            );
        }

        Ok(serde_json::to_value(Status { disks })?)
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
