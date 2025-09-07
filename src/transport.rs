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
use crate::{builder::UTransportIceoryx2Builder, listener_registry::ListenerRegistry, pubsub::PubSubThreadWorker};

pub struct UTransportIceoryx2<Service: iceoryx2::service::Service> {
    pub(crate) node: Node<Service>,
    pub(crate) listeners: ListenerRegistry,
    pub(crate) cycle_time: Duration,
    pub(crate) keep_alive: AtomicBool,
}

/// Acts as a uProtocol-specific interface for the Iceoryx2 transport system
impl<Service: iceoryx2::service::Service> UTransportIceoryx2<Service> {
    pub fn publish_subscribe() -> Result<PubSubThreadWorker, UStatus> 
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
}
