pub mod exec;

use async_trait::async_trait;
use homeassistant_agent::model::Discovery;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;

pub type CallbackFn = dyn FnOnce(Result<(), ()>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send;

#[async_trait(?Send)]
pub trait Command: Send + Sync {
    async fn start(&self, payload: Cow<'_, str>, callback: Box<CallbackFn>);

    fn describe_ha(&self) -> Option<Discovery> {
        None
    }
}
