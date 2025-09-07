// ################################################################################
// Copyright (c) 2025 Contributors to the Eclipse Foundation
// 
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
// 
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https: //www.apache.org/licenses/LICENSE-2.0
// 
// SPDX-License-Identifier: Apache-2.0
// ################################################################################

use iceoryx2::prelude::ServiceName;
use up_rust::{UCode, UMessageType, UStatus, UUri};

pub struct ServiceNameMapper;

/// uProtocol [`UUri`] to Iceoryx2 [`ServiceName`] mapping(s)
impl ServiceNameMapper {
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
                "Could not determine a valid UMessageType from the provided UUri(s)",
            ))
        }
    
        pub fn compute_service_name(
            source: &UUri,
            sink: Option<&UUri>,
        ) -> Result<ServiceName, UStatus> {
            let join_segments = |segments: Vec<String>| segments.join("/");
            
            let service_name_str: String;
            match Self::determine_message_type(source, sink)? {
                UMessageType::UMESSAGE_TYPE_REQUEST => {
                    let Some(sink_uri) = sink else {
                        return Err(UStatus::fail_with_code(
                            UCode::INVALID_ARGUMENT,
                            format!("sink required for UMessageType {:?}", UMessageType::UMESSAGE_TYPE_REQUEST),
                        ));
                    };
                    let segments = Self::encode_uuri_segments(sink_uri);
                    service_name_str = format!("up/{}", join_segments(segments));
                }
                UMessageType::UMESSAGE_TYPE_RESPONSE | UMessageType::UMESSAGE_TYPE_NOTIFICATION => {
                    let Some(sink_uri) = sink else {
                        return Err(UStatus::fail_with_code(
                            UCode::INVALID_ARGUMENT,
                            format!("sink required for UMessageType {:?} or {:?}", UMessageType::UMESSAGE_TYPE_RESPONSE, UMessageType::UMESSAGE_TYPE_NOTIFICATION),
                        ));
                    };
                    let source_segments = Self::encode_uuri_segments(source);
                    let sink_segments = Self::encode_uuri_segments(sink_uri);
                    service_name_str = format!(
                        "up/{}/{}",
                        join_segments(source_segments),
                        join_segments(sink_segments)
                    );
                }
                UMessageType::UMESSAGE_TYPE_PUBLISH => {
                    let segments = Self::encode_uuri_segments(source);
                    service_name_str = format!("up/{}", join_segments(segments));
                }
                _ => return Err(UStatus::fail_with_code(
                    UCode::INVALID_ARGUMENT,
                    "Unsupported UMessageType for service name computation",
                )),
            }
    
            Ok(ServiceName::new(service_name_str.as_str()).expect("Failed to create service name"))
        }
}

pub trait ToServiceName {
    /// Converts a [`UUri`] to a [`ServiceName`] by combining it with a sink [`UUri`]. The calling [`UUri`] is considered the source
    ///
    /// <b>TODO: As of 2025-08-31. This does not follow the up-spec. It needs to be updated to return proper up-spec formatting. 
    ///          See existing work to convert [`UUri`] instances to [`ServiceName`] instances at [`compute_service_name`](crate::transport::compute_service_name)</b>
    /// 
    /// [Link to up-spec on Iceoryx2 service name structure](https://github.com/eclipse-uprotocol/up-spec/blob/3130bc5bfe661f60509b9d5e1d03995d608a18b2/up-l1/iceoryx2.adoc#5-iceoryx2-service-name-structure)
    fn to_service_name(&self, sink_uuri: Option<&UUri>) -> Result<ServiceName, UStatus>;
}

impl ToServiceName for UUri {
    fn to_service_name(&self, sink_uuri: Option<&UUri>) -> Result<ServiceName, UStatus> {
        let service_name_str = ServiceNameMapper::compute_service_name(self, sink_uuri)?;
        Ok(ServiceName::new(service_name_str.as_str()).expect("Failed to create service name"))
    }
}

#[cfg(test)]
mod tests {
    use crate::service_name_mapping::ServiceNameMapper;
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

        let name = ServiceNameMapper::compute_service_name(&source, None).unwrap();
        assert_eq!(name, "up/device1/10AB/0/3/7FFF");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_notification_service_name() {
        let source = test_uri("device1", 0x0000, 0x10AB, 0x03, 0x80CD);
        let sink = test_uri("device1", 0x0000, 0x30EF, 0x04, 0x0000);
        let name = ServiceNameMapper::compute_service_name(&source, Some(&sink)).unwrap();
        assert_eq!(name, "up/device1/10AB/0/3/80CD/device1/30EF/0/4/0");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_rpc_request_service_name() {
        let sink = test_uri("device1", 0x0004, 0x03AB, 0x03, 0x0000);
        let reply_to = test_uri("device1", 0x0000, 0x00CD, 0x04, 0xB);

        let name = ServiceNameMapper::compute_service_name(&sink, Some(&reply_to)).unwrap();
        assert_eq!(name, "up/device1/CD/0/4/B");
    }

    #[test]
    // [specitem,oft-sid="dsn~up-transport-iceoryx2-service-name~1",oft-needs="utest"]
    fn test_rpc_response_service_name() {
        let source = test_uri("device1", 0x0000, 0x00CD, 0x04, 0xB);
        let sink = test_uri("device1", 0x0004, 0x3AB, 0x3, 0x0000);

        let name = ServiceNameMapper::compute_service_name(&source, Some(&sink)).unwrap();
        assert_eq!(name, "up/device1/CD/0/4/B/device1/3AB/4/3/0");
    }

    // performing failing tests for service name computation

    #[test]
    // .specitem[dsn~up-attributes-request-source~1]
    // .specitem[dsn~up-attributes-response-source~1]
    // .specitem[dsn~up-attributes-notification-source~1]
    fn test_missing_uri_error() {
        let uuri = UUri::new();
        let result = ServiceNameMapper::compute_service_name(&uuri, None);

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
        let result = ServiceNameMapper::compute_service_name(&source, Some(&sink));
        assert!(result.is_err_and(|err| err.get_code() == UCode::INVALID_ARGUMENT));
    }

    #[test]
    //source has resource id=0 but missing sink
    // .specitem[dsn~up-attributes-request-sink~1]
    // .specitem[dsn~up-attributes-request-source~1]
    fn test_fail_missing_sink_error() {
        let source = test_uri("device1", 0x0000, 0x00CD, 0x04, 0x000);
        let result = ServiceNameMapper::compute_service_name(&source, None);
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
        let result = ServiceNameMapper::compute_service_name(&uuri, Some(&sink));
        assert!(result.is_err_and(|err| err.get_code() == UCode::INVALID_ARGUMENT));
    }
}
