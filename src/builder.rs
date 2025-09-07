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

use iceoryx2::node::NodeBuilder;
use up_rust::UStatus;
use crate::{listener_registry::ListenerRegistry, pubsub::PubSubThreadWorker, transport::UTransportIceoryx2};

/// [`UTransportIceoryx2`] uses a builder pattern that integrates with iceoryx2's already existing builders 
/// and configuration to create the desired [`UTransport`](up_rust::UTransport) wrapper that uses the desired 
/// messaging pattern. The uProtocol spec defines that [only the publish/subscribe pattern](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l1/iceoryx2.adoc#3-iceoryx2-messagingpattern) 
/// shall be implemented for this [`UTransport`] wrapper.
/// 
/// ---
/// 
/// Iceoryx2 has/is planned to implement several messaging patterns to send zero-copy data 
/// between processes. [Iceoryx2 talks about what messaging protocols it supports in its README introduction](https://github.com/eclipse-iceoryx/iceoryx2/tree/main?tab=readme-ov-file#introduction)
pub struct UTransportIceoryx2Builder {}

const DEFAULT_CYCLE_TIME: Duration = Duration::from_millis(100);

impl UTransportIceoryx2Builder {
    pub(crate) fn default<Service>() -> Result<UTransportIceoryx2<Service>, UStatus> 
    where
        Service: iceoryx2::service::Service
    {
        let configure_fn: Option<fn(&NodeBuilder)> = Some(|_| {});
        UTransportIceoryx2Builder::create::<Service>(configure_fn)
    }

    pub(crate) fn create<Service: iceoryx2::service::Service>(configure: Option<impl FnOnce(&NodeBuilder)>) -> Result<UTransportIceoryx2<Service>, UStatus>
    {
        let node_builder = NodeBuilder::new();
        if let Some(configure) = configure {
            configure(&node_builder);
        }
        let node = node_builder
            .create::<Service>()
            .expect("Failed to create Iceoryx2 Node");
        Ok(UTransportIceoryx2 {
            node,
            listeners: ListenerRegistry::default(),
            cycle_time: DEFAULT_CYCLE_TIME,
            keep_alive: AtomicBool::new(true),
        })
    }
    
    pub(crate) fn publish_subscribe<Service>(transport: UTransportIceoryx2<Service>) -> Result<PubSubThreadWorker<Service>, UStatus>
    where
        Service: iceoryx2::service::Service
    {
        PubSubThreadWorker::new(transport)
    }
}
