use crate::collector::Manager;
use anyhow::bail;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// The MQTT client id, defaults to a random ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// The device name, defaults to the hostname
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,

    /// The MQTT's servers/brokers hostname
    pub host: String,

    /// The MQTT's server/brokers port, defaults to 8883
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// TLS is used by default, you can disable it here.
    #[serde(default, skip_serializing_if = "is_default")]
    pub disable_tls: bool,

    #[serde(default = "default_keep_alive", skip_serializing_if = "is_default")]
    #[serde(with = "humantime_serde")]
    pub keep_alive: Duration,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

fn default_keep_alive() -> Duration {
    Duration::from_secs(5)
}

fn is_default<D: Default + Eq>(value: &D) -> bool {
    value == &D::default()
}

pub async fn run(options: Options, manager: Arc<Manager>) -> anyhow::Result<()> {
    let client_id = options.client_id.unwrap_or_else(|| "resymo".to_string());

    let port = options
        .port
        .unwrap_or(if options.disable_tls { 1883 } else { 8883 });

    let device = options.device.or_else(|| std::env::var("HOSTNAME").ok());
    let device = match device {
        Some(device) => device,
        None => {
            log::error!("No device name provided, and failed to detect one");
            bail!("No device name provided, and failed to detect one");
        }
    };
    log::info!("Publishing as: {device}");
    let device = urlencoding::encode(&device).to_string();

    let mut mqttoptions = MqttOptions::new(client_id, options.host, port);
    mqttoptions.set_keep_alive(options.keep_alive);

    if !options.disable_tls {
        mqttoptions.set_transport(Transport::Tls(TlsConfiguration::Native));
    }

    log::debug!("Options: {mqttoptions:#?}");

    if let Some(username) = options.username {
        mqttoptions.set_credentials(username, options.password.unwrap_or_default());
    }

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let runner = async {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            log::info!("Publishing state");
            match manager.collect_all().await {
                Ok(state) => {
                    for (name, state) in state {
                        match serde_json::to_vec(&state) {
                            Ok(payload) => {
                                if let Err(err) = client
                                    .publish(
                                        format!("{device}/state/{name}"),
                                        QoS::AtMostOnce,
                                        false,
                                        payload,
                                    )
                                    .await
                                {
                                    log::warn!("Failed to publish payload: {err}");
                                }
                            }
                            Err(err) => {
                                log::warn!("Failed to serialize payload: {err}");
                            }
                        };
                    }
                }
                Err(err) => {
                    log::warn!("Failed collecting state: {err}");
                    let _ = client
                        .publish(
                            format!("{device}/log/error"),
                            QoS::AtMostOnce,
                            false,
                            format!("Failed collecting state: {err}"),
                        )
                        .await;
                }
            }
        }
    };

    let connection = async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    log::info!("Connected");
                }
                Ok(Event::Incoming(Incoming::Disconnect)) => {
                    log::info!("Disconnected");
                }
                Ok(_) => {}
                Err(err) => {
                    log::warn!("Connection failed: {err}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    };

    tokio::select! {
        _ = runner => {},
        _ = connection => {},
    }

    log::info!("MQTT runner exited");

    Ok(())
}
