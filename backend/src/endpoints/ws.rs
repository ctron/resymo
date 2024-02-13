use actix::{Actor, ActorContext, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};

pub struct Runner {}

impl Actor for Runner {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Runner {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(payload)) => ctx.pong(&payload),
            Ok(ws::Message::Text(payload)) => {
                if let Err(err) = self.command(payload.as_bytes()) {
                    log::warn!("Failed to handle text command: {err}");
                    ctx.close(Some(CloseReason::from(CloseCode::Error)));
                    ctx.stop();
                }
            }
            Ok(ws::Message::Binary(payload)) => {
                if let Err(err) = self.command(&payload) {
                    log::warn!("Failed to handle binary command: {err}");
                    ctx.close(Some(CloseReason::from(CloseCode::Error)));
                    ctx.stop();
                }
            }
            Ok(ws::Message::Close(reason)) => {
                log::info!("Received close: {reason:?}");
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Command {}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Message {}

impl Runner {
    fn command(&self, payload: &[u8]) -> anyhow::Result<()> {
        let command: Command = serde_json::from_slice(payload)?;

        Ok(())
    }
}
