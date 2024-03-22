use crate::config::CommonCollector;
use crate::utils::is_default;
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    #[serde(flatten)]
    pub common: CommonCollector,

    /// execution tasks
    #[serde(default)]
    pub items: HashMap<String, Task>,
}

impl Deref for Configuration {
    type Target = CommonCollector;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(with = "humantime_serde", default = "default::period")]
    #[schemars(schema_with = "crate::utils::humantime_duration")]
    pub period: Duration,

    /// The binary to call
    pub command: String,

    /// The arguments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// The environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub envs: HashMap<String, String>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub clean_env: bool,

    /// The Home Assistant discovery section
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovery: Vec<Discovery>,
}

mod default {
    use super::*;

    pub const fn period() -> Duration {
        Duration::from_secs(60)
    }
}

#[derive(Clone, Debug)]
pub struct Error(String);

impl From<Error> for anyhow::Error {
    fn from(value: Error) -> Self {
        anyhow!("{}", value.0)
    }
}

#[derive(Debug)]
struct Inner {
    config: Task,
    last_run: Option<Instant>,
    state: Result<Value, Error>,
}

impl Inner {
    fn new(config: Task) -> Self {
        Self {
            config,
            last_run: None,
            state: Err(Error("Not yet initialized".into())),
        }
    }

    async fn run_once(&mut self) -> anyhow::Result<Value> {
        let mut cmd = Command::new(&self.config.command);

        if self.config.clean_env {
            cmd.env_clear();
        }

        cmd.kill_on_drop(true)
            .args(self.config.args.clone())
            .envs(self.config.envs.clone());

        let output = cmd.output().await?;
        if !output.status.success() {
            bail!("Command failed: rc == {}", output.status);
        }

        self.mark_run();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let status = output.status.code();

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "status": status,
        }))
    }

    async fn run(&mut self) -> anyhow::Result<Value> {
        if self.need_run() {
            self.state = self.run_once().await.map_err(|err| Error(err.to_string()));
        }

        Ok(self.state.clone()?)
    }

    fn need_run(&self) -> bool {
        match self.last_run {
            Some(last_run) => Instant::now() - last_run > self.config.period,
            None => true,
        }
    }

    fn mark_run(&mut self) {
        // TODO: we should do better and provide a constant delay
        self.last_run = Some(Instant::now());
    }
}

#[derive(Debug)]
pub struct Collector {
    inner: Arc<Mutex<Inner>>,
    descriptor: Vec<Discovery>,
}

impl Collector {
    pub fn new(config: Configuration) -> HashMap<String, Self> {
        config
            .items
            .into_iter()
            .map(|(name, mut task)| {
                let mut discovery = task.discovery.drain(..).collect::<Vec<_>>();

                let mut auto = 0;
                for discovery in &mut discovery {
                    if discovery.unique_id.is_none() {
                        let name = if auto == 0 {
                            name.to_string()
                        } else {
                            // start suffixing items with a counter, better provide a unique_id
                            format!("{name}_{auto}")
                        };
                        discovery.unique_id = Some(name);
                        auto += 1;
                    }

                    if discovery.value_template.is_none() {
                        // default to stdout
                        discovery.value_template = Some("{{ value_json.stdout }}".into());
                    }
                }

                let collector = Self {
                    descriptor: discovery.into_iter().collect(),
                    inner: Arc::new(Mutex::new(Inner::new(task))),
                };

                (name, collector)
            })
            .collect()
    }
}

#[async_trait]
impl super::Collector for Collector {
    async fn collect(&self) -> anyhow::Result<Value> {
        self.inner.lock().await.run().await
    }

    fn describe_ha(&self) -> Vec<Discovery> {
        self.descriptor.clone()
    }
}
