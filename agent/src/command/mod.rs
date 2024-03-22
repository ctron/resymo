pub mod exec;

use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use std::borrow::Cow;

#[async_trait(?Send)]
pub trait Command: Send + Sync {
    async fn start(&self, payload: Cow<'_, str>, callback: Box<dyn Fn(Result<(), ()>) + Send>);

    fn describe_ha(&self) -> Option<Discovery> {
        None
    }
}
