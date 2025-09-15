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

use iceoryx2::prelude::MessagingPattern;
use std::sync::atomic::Ordering;
use tokio::task::JoinHandle;
use up_rust::UStatus;

use crate::workers::{relay::TransportRelay, worker::Iceoryx2RelayWorker};

pub struct Iceoryx2WorkerDispatcher {
    pub messaging_pattern: MessagingPattern,
    pub handle: JoinHandle<Result<(), UStatus>>,
}

impl Iceoryx2WorkerDispatcher {
    pub fn create_listener_worker<Relay: TransportRelay>(
        relay: Relay,
    ) -> JoinHandle<Result<(), UStatus>> {
        let worker = Iceoryx2RelayWorker::new(relay);
        let future = Iceoryx2WorkerDispatcher::run(worker);
        tokio::spawn(future)
    }

    async fn run<Relay: TransportRelay>(worker: Iceoryx2RelayWorker<Relay>) -> Result<(), UStatus> {
        while worker.keep_alive.load(Ordering::Relaxed) {
            worker.relay.relay().await?;
        }
        Ok(())
    }
}
