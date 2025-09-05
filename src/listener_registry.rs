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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use iceoryx2::prelude::ServiceName;
use tokio::sync::Mutex;
use up_rust::{ComparableListener, UListener, UStatus, UCode};

pub type ListenerMap = HashMap<ServiceName, HashSet<ComparableListener>>;

/// Registry for active [`UListener`] instance(s) on a [`UTransportIceoryx2`](crate::transport::UTransportIceoryx2) 
/// for a specific Iceoryx2 [`ServiceName`]
#[derive(Default)]
pub struct ListenerRegistry
{
    listeners: Mutex<ListenerMap>
}

impl ListenerRegistry {
    /// Wraps the [`UListener`] with a [`ComparableListener`], and then adds the [`ComparableListener`] to the [`ListenerRegistry`].
    /// One [`ServiceName`] may register multiple [`UListener`] instances.
    ///
    /// Returns [`ListenerRegistryError::AlreadyExists`] if the [`UListener`] is already registered under the given [`ServiceName`]
    pub async fn add_listener (
        &self,
        service_name: &ServiceName,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let mut listeners = self.listeners.lock().await;
        let comparable_listener = ComparableListener::new(listener);
        let listener_already_exists = self.has_listener(&listeners, service_name, &comparable_listener);
        if listener_already_exists {
            return Err(ListenerRegistryError::AlreadyExists.into());
        }
        let existing_listeners = listeners.entry(service_name.clone()).or_default();
        existing_listeners.insert(comparable_listener.clone());
        Ok(())
    }

    /// Removes the [`UListener`] from the [`ListenerRegistry`]
    ///
    /// Returns [`ListenerRegistryError::NotFound`] if the [`UListener`] is not registered under the given [`ServiceName`]
    pub async fn remove_listener (
        &self,
        service_name: ServiceName,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let mut listeners = self.listeners.lock().await;
        let comparable_listener = ComparableListener::new(listener);
        let listener_exists = self.has_listener(&listeners, &service_name, &comparable_listener);
        if !listener_exists {
            return Err(ListenerRegistryError::NotFound.into());
        }
        let existing_listeners = listeners.get_mut(&service_name);
        match existing_listeners {
            Some(listeners) => {
                listeners.remove(&comparable_listener);
                Ok(())
            }
            None => {
                Err(ListenerRegistryError::NotFound.into())
            }
        }
    }

    /// Checks if the [`UListener`] is registered under the given [`ServiceName`]
    pub fn has_listener(&self, listener_map: &ListenerMap, service_name: &ServiceName, comparable_listener: &ComparableListener) -> bool {
        let existing_listeners = listener_map.get(service_name);
        existing_listeners
            .map(|listeners| listeners.contains(comparable_listener))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub enum ListenerRegistryError {
    AlreadyExists,
    NotFound,
}

impl ListenerRegistryError {
    pub fn description(&self) -> &str {
        match self {
            ListenerRegistryError::AlreadyExists => "Listener already exists",
            ListenerRegistryError::NotFound => "Listener not found",
        }
    }
}

impl std::error::Error for ListenerRegistryError {}

impl std::fmt::Display for ListenerRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<ListenerRegistryError> for UStatus {
    fn from(error: ListenerRegistryError) -> Self {
        match error {
            ListenerRegistryError::AlreadyExists => UStatus::fail_with_code(
                UCode::ALREADY_EXISTS,
                error.description(),
            ),
            ListenerRegistryError::NotFound => UStatus::fail_with_code(
                UCode::NOT_FOUND,
                error.description(),
            ),
        }
    }
}
