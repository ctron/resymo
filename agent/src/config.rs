use crate::uplink;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub uplinks: Uplinks,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Uplinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<uplink::http::Options>,
}
