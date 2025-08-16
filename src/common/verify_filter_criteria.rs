use up_rust::{UCode, UStatus, UUri};

pub(crate) fn verify_filter_criteria(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> Result<(), UStatus> {
    if let Some(sink_filter_uuri) = sink_filter {
        if sink_filter_uuri.is_notification_destination()
            && source_filter.is_notification_destination()
        {
            return Err(UStatus::fail_with_code(
                UCode::INVALID_ARGUMENT,
                "source and sink filters must not both have resource ID 0",
            ));
        }
        if sink_filter_uuri.is_rpc_method()
            && !source_filter.has_wildcard_resource_id()
            && !source_filter.is_notification_destination()
        {
            return Err(UStatus::fail_with_code(UCode::INVALID_ARGUMENT, "source filter must either have the wildcard resource ID or resource ID 0, if sink filter matches RPC method resource ID"));
        }
    } else if !source_filter.has_wildcard_resource_id() && !source_filter.is_event() {
        return Err(UStatus::fail_with_code(UCode::INVALID_ARGUMENT, "source filter must either have the wildcard resource ID or a resource ID from topic range, if sink filter is empty"));
    }
    // everything else might match valid messages
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

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
}