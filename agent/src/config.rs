use crate::uplink;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct Config {
    #[serde(default)]
    pub uplinks: Uplinks,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Uplinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_server: Option<uplink::http_server::Options>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<uplink::homeassistant::Options>,
}
