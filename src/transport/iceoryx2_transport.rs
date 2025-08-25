use iceoryx2::{node::NodeBuilder, service::ipc};
use iceoryx2::node::Node;
use up_rust::{UCode, UMessageType, UStatus, UUri};
use std::error::Error;
use std::fmt::Debug;
use iceoryx2::{port::{publisher::Publisher, subscriber::Subscriber}, prelude::{ServiceName, ZeroCopySend}};


pub struct Iceoryx2Transport<Service: iceoryx2::service::Service = ipc::Service> {
    service_name: &str,
    node: Node<Service>,
}

#[allow(dead_code)]
impl<Service: iceoryx2::service::Service> Iceoryx2Transport<Service> {
    pub async fn new<IntoString: Into<String>>(
        service_name: IntoString,
        node: Node<Service>
    ) -> Result<Self, UStatus> {
        let transport = Self {
            node
        };
        Ok(transport)
    }

    fn publisher<PayloadType: Debug + Sized + ZeroCopySend>(&self, uuri: &UUri) -> Result<Publisher<Service, PayloadType, ()>, Box<dyn Error>> {
        let service_name: ServiceName = uuri.authority_name().as_str().try_into()?;
        let service = self.node.service_builder(&service_name)
            .publish_subscribe::<PayloadType>()
            .open_or_create()?;  
        let publisher = service.publisher_builder().create()?;
        Ok(publisher)
    }

    fn request_and_response<PayloadType: Debug + Sized + ZeroCopySend>(&self, uuri: &UUri) -> Result<Subscriber<Service, PayloadType, ()>, Box<dyn Error>> {
        let service_name: ServiceName = uuri.authority_name().as_str().try_into()?;
        let service = self.node.service_builder(&service_name)
            .request_response::<PayloadType>()
            .open_or_create()?;
        let subscriber = service.subscriber_builder().create()?;
        Ok(subscriber)
    }

    fn encode_uuri_segments(uuri: &UUri) -> Vec<String> {
        vec![
            uuri.authority_name.clone(),
            Self::encode_hex(uuri.uentity_type_id() as u32),
            Self::encode_hex(uuri.uentity_instance_id() as u32),
            Self::encode_hex(uuri.uentity_major_version() as u32),
            Self::encode_hex(uuri.resource_id() as u32),
        ]
    }

    fn encode_hex(value: u32) -> String {
        format!("{:X}", value)
    }

    /// Assumption: valid source and sink URIs provided:
    /// send() makes use of UAttributesValidator
    /// register_listener() and unregister_listener() use verify_filter_criteria()
    /// Criteria for identification of message types can be found here: https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/uattributes.adoc
    fn determine_message_type(source: &UUri, sink: Option<&UUri>) -> Result<UMessageType, UStatus> {
        let src_id = source.resource_id;
        let sink_id = sink.map(|s| s.resource_id);

        if src_id == 0 {
            if let Some(id) = sink_id {
                if id >= 1 && id <= 0x7FFF {
                    return Ok(UMessageType::UMESSAGE_TYPE_REQUEST);
                }
            }
        } else if sink_id == Some(0) && src_id >= 1 && src_id <= 0xFFFE {
            return Ok(UMessageType::UMESSAGE_TYPE_RESPONSE);
        } else if src_id >= 1 && src_id <= 0x7FFF {
            return Ok(UMessageType::UMESSAGE_TYPE_PUBLISH);
        }

        Err(UStatus::fail_with_code(
            UCode::INVALID_ARGUMENT,
            "Unsupported UMessageType",
        ))
    }

    fn compute_service_name(source: &UUri, sink: Option<&UUri>) -> Result<String, UStatus> {
        let join_segments = |segments: Vec<String>| segments.join("/");

        match Self::determine_message_type(source, sink)? {
            UMessageType::UMESSAGE_TYPE_REQUEST => {
                let Some(sink_uri) = sink else {
                    return Err(UStatus::fail_with_code(
                        UCode::INVALID_ARGUMENT,
                        "sink required for RpcRequest",
                    ));
                };
                let segments = Self::encode_uuri_segments(sink_uri);
                Ok(format!("up/{}", join_segments(segments)))
            }
            UMessageType::UMESSAGE_TYPE_RESPONSE => {
                let Some(sink_uri) = sink else {
                    return Err(UStatus::fail_with_code(
                        UCode::INVALID_ARGUMENT,
                        "sink required for ResponseOrNotification",
                    ));
                };
                let source_segments = Self::encode_uuri_segments(source);
                let sink_segments = Self::encode_uuri_segments(sink_uri);
                Ok(format!(
                    "up/{}/{}",
                    join_segments(source_segments),
                    join_segments(sink_segments)
                ))
            }
            UMessageType::UMESSAGE_TYPE_PUBLISH => {
                let segments = Self::encode_uuri_segments(source);
                Ok(format!("up/{}", join_segments(segments)))
            }
        }
    }
}

fn initialize_node<Service: iceoryx2::service::Service>() -> Result<iceoryx2::node::Node<Service>, UStatus> {
    match NodeBuilder::new().create::<Service>() {
        Ok(node) => Ok(node),
        Err(err) => Err(UStatus::fail_with_code(
            UCode::INTERNAL,
            format!("Failed to create Iceoryx2 node: {}", err),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use up_rust::{UCode, UUri};

    fn test_uri(authority: &str, instance: u16, typ: u16, version: u8, resource: u16) -> UUri {
        let entity_id = ((instance as u32) << 16) | (typ as u32);
        UUri::try_from_parts(authority, entity_id, version, resource).unwrap()
    }

    // performing successful tests for service name computation

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_publish_service_name() {
        let source = test_uri("device1", 0x0000, 0x10AB, 0x03, 0x7FFF);

        let name = Iceoryx2Transport::compute_service_name(&source, None).unwrap();
        assert_eq!(name, "up/device1/10AB/0/3/7FFF");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_notification_service_name() {
        let source = test_uri("device1", 0x0000, 0x10AB, 0x03, 0x80CD);
        let sink = test_uri("device1", 0x0000, 0x30EF, 0x04, 0x0000);
        let name = Iceoryx2Transport::compute_service_name(&source, Some(&sink)).unwrap();
        assert_eq!(name, "up/device1/10AB/0/3/80CD/device1/30EF/0/4/0");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_rpc_request_service_name() {
        let sink = test_uri("device1", 0x0004, 0x03AB, 0x03, 0x0000);
        let reply_to = test_uri("device1", 0x0000, 0x00CD, 0x04, 0xB);

        let name = Iceoryx2Transport::compute_service_name(&sink, Some(&reply_to)).unwrap();
        assert_eq!(name, "up/device1/CD/0/4/B");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_rpc_response_service_name() {
        let source = test_uri("device1", 0x0000, 0x00CD, 0x04, 0xB);
        let sink = test_uri("device1", 0x0004, 0x3AB, 0x3, 0x0000);

        let name = Iceoryx2Transport::compute_service_name(&source, Some(&sink)).unwrap();
        assert_eq!(name, "up/device1/CD/0/4/B/device1/3AB/4/3/0");
    }

    // performing failing tests for service name computation

    #[test]
    // .specitem[dsn~up-attributes-request-source~1]
    // .specitem[dsn~up-attributes-response-source~1]
    // .specitem[dsn~up-attributes-notification-source~1]
    fn test_missing_uri_error() {
        let uuri = UUri::new();
        let result = Iceoryx2Transport::compute_service_name(&uuri, None);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().get_code(), UCode::INVALID_ARGUMENT);
    }

    #[test]
    //both source and sink have resource ID equal to 0
    // .specitem[dsn~up-attributes-request-source~1]
    // .specitem[dsn~up-attributes-request-sink~1]
    // .specitem[dsn~up-attributes-response-source~1]
    // .specitem[dsn~up-attributes-response-sink~1]
    fn test_fail_resource_id_error() {
        let source = test_uri("device1", 0x0000, 0x00CD, 0x04, 0x000);
        let sink = test_uri("device1", 0x0004, 0x3AB, 0x3, 0x0000);
        let result = Iceoryx2Transport::compute_service_name(&source, Some(&sink));
        assert!(result.is_err_and(|err| err.get_code() == UCode::INVALID_ARGUMENT));
    }

    #[test]
    //source has resource id=0 but missing sink
    // .specitem[dsn~up-attributes-request-sink~1]
    // .specitem[dsn~up-attributes-request-source~1]
    fn test_fail_missing_sink_error() {
        let source = test_uri("device1", 0x0000, 0x00CD, 0x04, 0x000);
        let result = Iceoryx2Transport::compute_service_name(&source, None);
        assert!(result.is_err_and(|err| err.get_code() == UCode::INVALID_ARGUMENT));
    }

    #[test]
    //missing source URI
    // .specitem[dsn~up-attributes-request-source~1]
    // .specitem[dsn~up-attributes-response-source~1]
    // .specitem[dsn~up-attributes-notification-source~1]
    fn test_fail_missing_source_error() {
        let uuri = UUri::new();
        let sink = test_uri("device1", 0x0004, 0x3AB, 0x3, 0x000);
        let result = Iceoryx2Transport::compute_service_name(&uuri, Some(&sink));
        assert!(result.is_err_and(|err| err.get_code() == UCode::INVALID_ARGUMENT));
    }
}
