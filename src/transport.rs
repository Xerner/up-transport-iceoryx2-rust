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

use std::{sync::atomic::AtomicBool, time::Duration};

use iceoryx2::{node::{Node, NodeBuilder}, prelude::ServiceName};
use up_rust::{UCode, UMessageType, UStatus, UUri};
use crate::{builder::UTransportIceoryx2Builder, listener_registry::ListenerRegistry, pubsub::PubSubSingleThreadWrapper};

pub struct UTransportIceoryx2<Service: iceoryx2::service::Service> {
    pub(crate) node: Node<Service>,
    pub(crate) listeners: ListenerRegistry,
    pub(crate) cycle_time: Duration,
    pub(crate) keep_alive: AtomicBool,
}

/// Acts as a uProtocol-specific interface for the Iceoryx2 transport system
impl<Service: iceoryx2::service::Service> UTransportIceoryx2<Service> {
    pub fn publish_subscribe() -> Result<PubSubSingleThreadWrapper, UStatus> 
    {
        let transport = UTransportIceoryx2Builder::default::<Service>()?;
        let transport_pubsub = UTransportIceoryx2Builder::publish_subscribe(transport)?;
        Ok(transport_pubsub)
    }

    pub(crate) fn create_node(configure: Option<impl FnOnce(&NodeBuilder)>) -> Result<Node<Service>, UStatus>
    {
        let node_builder = NodeBuilder::new();
        if let Some(configure) = configure {
            configure(&node_builder);
        }
        let node = node_builder
            .create::<Service>()
            .expect("Failed to create Iceoryx2 Node");
        Ok(node)
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
