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

use crate::{pubsub::Iceoryx2PubSub, workers::dispatcher::Iceoryx2WorkerDispatcher};
use tokio::task::JoinHandle;
use up_rust::{UStatus, UTransport};

pub struct UTransportIceoryx2<Transport: UTransport> {
    pub(crate) relay_worker_handle: JoinHandle<Result<(), UStatus>>,
    pub transport: Transport,
}

/// Acts as a uProtocol-specific interface for the Iceoryx2 transport system
impl<Transport: UTransport> UTransportIceoryx2<Transport> {
    pub fn publish_subscribe() -> UTransportIceoryx2<Iceoryx2PubSub> {
        let transport = Iceoryx2PubSub::new();
        let relay_worker_handle =
            Iceoryx2WorkerDispatcher::create_listener_worker(transport.clone());
        UTransportIceoryx2 {
            relay_worker_handle,
            transport,
        }
    }
}
