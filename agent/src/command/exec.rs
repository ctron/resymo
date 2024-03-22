use crate::config::CommonCommand;
use crate::utils::is_default;
use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    #[serde(flatten)]
    pub common: CommonCommand,

    /// execution tasks
    #[serde(default)]
    pub items: HashMap<String, Run>,
}

impl Deref for Configuration {
    type Target = CommonCommand;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Run {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery: Option<Discovery>,
}

pub struct Command {
    config: Run,
    discovery: Option<Discovery>,
}

impl Command {
    pub fn new(config: Configuration) -> HashMap<String, Command> {
        config
            .items
            .into_iter()
            .map(|(name, config)| {
                let command = Self::new_run(&name, config);
                (name, command)
            })
            .collect()
    }

    fn new_run(name: &str, config: Run) -> Self {
        let discovery = if let Some(mut discovery) = config.discovery.clone() {
            if discovery.unique_id.is_none() {
                discovery.unique_id = Some(name.into());
            }

            if discovery.value_template.is_none() {
                // default to stdout
                discovery.value_template = Some("{{ value_json.stdout }}".into());
            }
            Some(discovery)
        } else {
            None
        };

        Self { config, discovery }
    }
}

#[async_trait(?Send)]
impl super::Command for Command {
    async fn start(&self, payload: Cow<'_, str>, callback: Box<dyn Fn(Result<(), ()>) + Send>) {
        log::info!("running command: {payload}");

        let mut cmd = tokio::process::Command::new(&self.config.command);

        if self.config.clean_env {
            cmd.env_clear();
        }

        cmd.args(self.config.args.clone())
            .envs(self.config.envs.clone());

        tokio::spawn(async move {
            let result = match cmd.output().await {
                Ok(output) if output.status.success() => Ok(()),
                Ok(_) => Err(()),
                Err(err) => {
                    log::warn!("Failed to launch command: {err}");
                    Err(())
                }
            };

            (callback)(result);
        });
    }

    fn describe_ha(&self) -> Option<Discovery> {
        self.discovery.clone()
    }
}
