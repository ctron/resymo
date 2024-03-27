mod discovery;

use crate::manager::Manager;
use crate::uplink::homeassistant::discovery::MixinAvailability;
use actix_web::web::Bytes;
use gethostname::gethostname;
use homeassistant_agent::{
    connector::{AvailabilityOptions, Client, Connector, ConnectorHandler, ConnectorOptions},
    model::{Component, Device, DeviceId, Discovery},
};
use rumqttc::QoS;
use std::{borrow::Cow, sync::Arc, time::Duration};
use tokio::{sync::oneshot, time::MissedTickBehavior};

pub const PAYLOAD_RUNNING: &str = "ON";
pub const PAYLOAD_STOPPED: &str = "OFF";

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

    async fn message(&mut self, topic: String, payload: Bytes) -> Result<(), Self::Error> {
        match topic.split('/').collect::<Vec<_>>().as_slice() {
            [base, device_id, name, "command"]
                if format!("{base}/{device_id}") == self.options.base =>
            {
                let payload = String::from_utf8_lossy(&payload);
                self.handle_command(name, payload).await;
            }
            _ => {
                log::warn!("received message for unknown topic: {topic}");
            }
        }

        Ok(())
    }
}

impl ResymoUplink {
    fn state_topic(&self, name: &str) -> String {
        format!("{base}/{name}/state", base = self.options.base)
    }

    fn command_topic(&self, name: &str) -> String {
        format!("{base}/{name}/command", base = self.options.base)
    }

    async fn subscribe(&self) -> Result<(), Error> {
        for (name, command) in &self.manager.commands {
            if command.describe_ha().is_none() {
                continue;
            }

            let command_topic = self.command_topic(name);

            self.client
                .subscribe(command_topic, QoS::AtMostOnce)
                .await?;
        }

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
            let state_topic = self.state_topic(name);
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

                let base = format!("{base}/{name}", base = self.options.base);
                let entity = entity.mixin_availability(&base, &self.options.availability_topic);

                let id = DeviceId::new(unique_id.clone(), Component::Sensor);
                self.client.announce(&id, &entity).await?;
            }
        }

        for (name, command) in &self.manager.commands {
            let entity = command.describe_ha();

            if let Some(entity) = entity {
                let command_topic = self.command_topic(name);
                let state_topic = self.state_topic(name);

                let Some(unique_id) = entity
                    .unique_id
                    .as_ref()
                    .map(|id| format!("{}_{id}", self.options.device_id))
                else {
                    continue;
                };

                let entity = Discovery {
                    command_topic: Some(command_topic.clone()),
                    device: Some(device.clone()),
                    unique_id: Some(unique_id.clone()),
                    ..(entity.clone())
                };

                let entity = entity.mixin_availability(
                    &format!("{base}/{name}", base = self.options.base),
                    &self.options.availability_topic,
                );

                let id = DeviceId::new(unique_id.clone(), Component::Button);
                self.client.announce(&id, &entity).await?;

                // state entity

                let unique_id = format!("{unique_id}_running");

                let entity = Discovery {
                    state_topic: Some(state_topic.clone()),
                    device: Some(device.clone()),
                    unique_id: Some(unique_id.clone()),
                    device_class: None,
                    value_template: None,
                    command_topic: None,
                    availability: vec![],
                    ..(entity.clone())
                };

                let entity =
                    entity.mixin_availability(&self.options.base, &self.options.availability_topic);

                let id = DeviceId::new(unique_id, Component::BinarySensor);
                self.client.announce(&id, &entity).await?;

                // update initial state

                // FIXME: we might need to check the actual state

                self.client
                    .update_state(self.state_topic(name), PAYLOAD_STOPPED)
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, name: &str, payload: Cow<'_, str>) {
        let Some(command) = self.manager.commands.get(name) else {
            log::warn!("Received trigger for unknown command: {name}");
            return;
        };

        let state_topic = self.state_topic(name);
        let _ = self
            .client
            .update_state(state_topic.clone(), PAYLOAD_RUNNING)
            .await;

        let client = self.client.clone();

        command
            .start(
                payload,
                Box::new(move |result| {
                    Box::pin(async move {
                        let _ = client.update_state(state_topic, PAYLOAD_STOPPED).await;

                        if result.is_ok() {
                            log::info!("completed: ok");
                        } else {
                            log::info!("completed: failed");
                        }
                    })
                }),
            )
            .await;
    }
}

#[derive(Clone, Debug)]
struct RunnerOptions {
    device_id: String,
    base: String,
    availability_topic: String,
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
            let topic = format!("{base}/{collector}/state", base = self.options.base);

            self.client
                .update_state(topic, serde_json::to_vec(&state)?)
                .await?;
        }

        Ok(())
    }
}

pub async fn run(options: Options, manager: Arc<Manager>) -> anyhow::Result<()> {
    let Options { options, connector } = options;

    let device_id = options
        .device_id
        .unwrap_or_else(|| gethostname().to_string_lossy().to_string());

    let availability_topic = format!("{base}/{device_id}/availability", base = options.base);
    let availability = AvailabilityOptions::new(availability_topic.clone());

    let base = format!("{base}/{device_id}", base = options.base);
    let options = RunnerOptions {
        device_id,
        base,
        availability_topic,
    };

    let connector = Connector::new(connector, |client| {
        ResymoUplink::new(client, manager, options)
    })
    .availability(availability);
    connector.run().await?;

    Ok(())
}
