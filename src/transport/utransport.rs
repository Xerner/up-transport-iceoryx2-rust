use async_trait::async_trait;
use std::sync::Arc;
use up_rust::{UMessage, UStatus, UUri, UTransport, UListener};
use crate::transport::iceoryx2_transport::Iceoryx2Transport;

#[async_trait]
impl UTransport for Iceoryx2Transport {
    async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
        let message_type = _message.type_();
        todo!();
    }

    async fn register_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        todo!()
    }

    async fn unregister_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        todo!()
    }
}
