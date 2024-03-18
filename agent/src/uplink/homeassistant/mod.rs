use crate::collector::Manager;
use actix_web::web::Bytes;
use gethostname::gethostname;
use homeassistant_agent::{
    connector::{Client, Connector, ConnectorHandler, ConnectorOptions},
    model::{Component, Device, DeviceId, Discovery},
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::oneshot, time::MissedTickBehavior};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UplinkOptions {
    /// The device ID. Will default to the value of the `HOSTNAME` environment variable.
    pub device_id: Option<String>,
    /// Base topic
    #[serde(default = "default_base")]
    pub base: String,
}

fn default_base() -> String {
    "resymo".to_string()
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct Options {
    #[serde(flatten)]
    pub options: UplinkOptions,

    /// Uplink connector options
    pub connector: ConnectorOptions,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] homeassistant_agent::connector::ClientError),
}

pub struct ResymoUplink {
    client: Client,
    manager: Arc<Manager>,
    options: RunnerOptions,
    _tx: oneshot::Sender<()>,
}

impl ResymoUplink {
    fn new(client: Client, manager: Arc<Manager>, options: RunnerOptions) -> Self {
        let (tx, rx) = oneshot::channel::<()>();

        let runner = Runner {
            shutdown: rx,
            client: client.clone(),
            manager: manager.clone(),
            options: options.clone(),
        };

        tokio::spawn({
            async move {
                runner.run().await;
            }
        });

        Self {
            client,
            manager,
            options,
            _tx: tx,
        }
    }
}

impl ConnectorHandler for ResymoUplink {
    type Error = Error;

    async fn connected(&mut self, state: bool) -> Result<(), Self::Error> {
        log::info!("Connected: {state}");
        if state {
            self.subscribe().await?;
            self.announce().await?;
        }
        Ok(())
    }

    async fn restarted(&mut self) -> Result<(), Self::Error> {
        log::info!("Restarted");
        self.announce().await?;
        Ok(())
    }

    async fn message(&mut self, _topic: String, _payload: Bytes) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ResymoUplink {
    async fn subscribe(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn announce(&self) -> Result<(), Error> {
        let device = Device {
            identifiers: vec![self.options.device_id.clone()],
            name: Some(format!("ReSyMo: {}", self.options.device_id)),
            base_topic: None,
            sw_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            support_url: None,
        };

        for (name, collector) in &self.manager.collectors {
            let state_topic = format!(
                "{}/{}/{name}/state",
                self.options.base, self.options.device_id
            );

            let entities = collector.describe_ha();

            for entity in entities {
                let Some(unique_id) = entity
                    .unique_id
                    .as_ref()
                    .map(|id| format!("{}_{name}_{id}", self.options.device_id,))
                else {
                    continue;
                };

                let entity = Discovery {
                    state_topic: Some(state_topic.clone()),
                    device: Some(device.clone()),
                    unique_id: Some(unique_id.clone()),
                    ..(entity.clone())
                };

                let id = DeviceId::new(unique_id.clone(), Component::Sensor);
                self.client.announce(&id, &entity).await?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct RunnerOptions {
    device_id: String,
    base: String,
}

struct Runner {
    pub shutdown: oneshot::Receiver<()>,
    pub client: Client,
    pub manager: Arc<Manager>,
    pub options: RunnerOptions,
}

impl Runner {
    async fn run(mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    log::info!("Update state");
                    if let Err(err) = self.collect().await {
                        log::warn!("Failed to collect state: {err}");
                    }
                }
                _ = &mut self.shutdown => {
                    log::info!("received shutdown signal");
                    break;
                }
            }
        }
    }

    async fn collect(&self) -> anyhow::Result<()> {
        for (collector, state) in self.manager.collect_all().await? {
            let topic = format!(
                "{}/{}/{collector}/state",
                self.options.base, self.options.device_id
            );

            self.client
                .update_state(topic, serde_json::to_vec(&state)?)
                .await?;
        }

        Ok(())
    }
}

pub async fn run(options: Options, manager: Arc<Manager>) -> anyhow::Result<()> {
    let Options { options, connector } = options;

    let device_id = match options.device_id {
        Some(device_id) => device_id,
        None => gethostname().to_string_lossy().to_string(),
    };

    let options = RunnerOptions {
        device_id,
        base: options.base,
    };

    let connector = Connector::new(connector, |client| {
        ResymoUplink::new(client, manager, options)
    });
    connector.run().await?;

    Ok(())
}
