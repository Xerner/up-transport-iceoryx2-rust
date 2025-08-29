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

use std::collections::HashSet;
use std::sync::Arc;

use up_rust::{ComparableListener, UListener, UStatus};
type ListenerSet = HashSet<ComparableListener>;

pub struct ListenerRegistry
{
    listeners: ListenerSet
}

impl ListenerRegistry {
    /// Creates a new registry with a given capacity.
    ///
    /// # Arguments
    ///
    /// * `node` - The Iceoryx2 node to spawn services from.
    /// * `subscribers` - The list of Iceoryx2 subscriber services that are registered and listening.
    /// * `max_subscribers` - The maximum number of subscribers that can be registered.
    pub fn new(max_subscribers: usize) -> Self {
        Self {
            listeners: HashSet::new()
        }
    }

    pub async fn add_listener (
        &self,
        service_name: &str,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let comparable_listener = ComparableListener::new(listener);
        self.listeners.lock().unwrap().insert((service_name.to_string(), comparable_listener), comparable_listener);
        Ok(())
    }
}
