use crate::collector::Manager;
use actix_web::web::Bytes;
use anyhow::Context;
use homeassistant_agent::connector::{Client, Connector, ConnectorHandler, ConnectorOptions};
use homeassistant_agent::model::{Component, Device, DeviceId, Discovery};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct UplinkOptions {
    /// The device ID. Will default to the value of the `HOSTNAME` environment variable.
    pub device_id: Option<String>,
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
    device_id: String,
    _tx: oneshot::Sender<()>,
}

impl ResymoUplink {
    pub fn new(client: Client, manager: Arc<Manager>, device_id: String) -> Self {
        let (tx, rx) = oneshot::channel::<()>();

        let runner = Runner {
            shutdown: rx,
            client: client.clone(),
            manager: manager.clone(),
            device_id: device_id.clone(),
        };

        tokio::spawn({
            async move {
                runner.run().await;
            }
        });

        Self {
            client,
            manager,
            device_id,
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
            identifiers: vec![self.device_id.clone()],
            name: Some(format!("ReSyMo: {}", self.device_id)),
            base_topic: None,
            sw_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            support_url: None,
        };

        for (name, collector) in &self.manager.collectors {
            let state_topic = format!("{}/{name}/state", self.device_id);

            let entities = collector.describe_ha();

            for entity in entities {
                let Some(unique_id) = entity
                    .unique_id
                    .as_ref()
                    .map(|id| format!("{}_{name}_{id}", self.device_id,))
                else {
                    continue;
                };

                let entity = Discovery {
                    state_topic: Some(state_topic.clone()),
                    device: Some(device.clone()),
                    ..(entity.clone())
                };

                let id = DeviceId::new(unique_id.clone(), Component::Sensor);
                self.client.announce(&id, &entity).await?;
            }
        }

        Ok(())
    }
}

struct Runner {
    pub shutdown: oneshot::Receiver<()>,
    pub client: Client,
    pub manager: Arc<Manager>,
    pub device_id: String,
}

impl Runner {
    async fn run(mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
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
            let topic = format!("{}/{collector}/state", self.device_id);

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
        None => std::env::var("HOSTNAME")
            .context("Unable to evaluate hostname from the 'HOSTNAME' environment variable")?,
    };

    let connector = Connector::new(connector, |client| {
        ResymoUplink::new(client, manager, device_id)
    });
    connector.run().await?;

    Ok(())
}
