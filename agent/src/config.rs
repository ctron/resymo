//! The configuration model
//!
//! ## Schema
//!
//! Whenever you make changes to the configuration model, run:
//!
//! ```bash
//! run --package resymo-agent --example gen_schema
//! ```

use crate::collector::exec;
use crate::{uplink, utils::is_default};

/// Agent configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct Config {
    #[serde(default)]
    pub uplinks: Uplinks,

    #[serde(default)]
    pub collectors: Collectors,
}

/// Uplink configuration
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Uplinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_server: Option<uplink::http_server::Options>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<uplink::homeassistant::Options>,
}

/// Common collector settings
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommonCollector {
    #[serde(default, skip_serializing_if = "is_default")]
    pub disabled: bool,
}

/// Collector configurations
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Collectors {
    /// Load average
    #[serde(default)]
    pub load_avg: CommonCollector,

    /// Swap
    #[serde(default)]
    pub swap: CommonCollector,

    /// Memory
    #[serde(default)]
    pub memory: CommonCollector,

    /// Disk
    #[serde(default)]
    pub disk_free: CommonCollector,

    /// Exec
    #[serde(default)]
    pub exec: exec::Configuration,
}
