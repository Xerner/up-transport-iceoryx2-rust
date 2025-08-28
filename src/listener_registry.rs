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

use std::error::Error;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};
use iceoryx2::node::Node;
use iceoryx2::prelude::ZeroCopySend;
use iceoryx2::service::{ipc, Service};
use iceoryx2::{port::subscriber::Subscriber};
use std::fmt::Debug;
use up_rust::{ComparableListener, UCode, UListener, UStatus};

pub struct ListenerRegistry<ServiceImpl = ipc::Service, Payload = ()>
where
    ServiceImpl: Service,
    Payload: Debug + ZeroCopySend,
{
    node: Node<ServiceImpl>,
    subscribers: Mutex<HashMap<(String, ComparableListener), Subscriber<ServiceImpl, Payload, ()>>>,
    max_subscribers: usize,
}

impl<ServiceImpl: Service> ListenerRegistry<ServiceImpl> {
    /// Creates a new registry with a given capacity.
    ///
    /// # Arguments
    ///
    /// * `node` - The Iceoryx2 node to spawn services from.
    /// * `subscribers` - The list of Iceoryx2 subscriber services that are registered and listening.
    /// * `max_subscribers` - The maximum number of subscribers that can be registered.
    pub fn new(node: Node<ServiceImpl>, max_subscribers: usize) -> Self {
        Self {
            node,
            subscribers: Mutex::new(HashMap::new()),
            max_subscribers,
        }
    }

    pub async fn register_subscriber<Payload: Debug + ZeroCopySend + Sized> (
        &self,
        service_name: &str,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let subscriber: Subscriber<ServiceImpl, Payload, ()>;
        match self.create_subscriber::<Payload>(service_name) {
            Ok(subscriber_) => subscriber = subscriber_,
            Err(error) => return Err(UStatus::fail_with_code(UCode::INTERNAL, format!("Failed to create subscriber: {:?}", error))),
        }
        let comparable_listener = ComparableListener::new(listener);
        self.subscribers.lock().unwrap().insert((service_name.to_string(), comparable_listener), subscriber);
        Ok(())
    }

    fn create_subscriber<Payload: Debug + ZeroCopySend + Sized>(&self, service_name: &str) -> Result<Subscriber<ServiceImpl, Payload, ()>, Box<dyn Error>> {
        let service = self.node
            .service_builder(&service_name.try_into()?)
            .publish_subscribe::<Payload>()
            .open_or_create()?;

        let subscriber = service.subscriber_builder().create()?;
        Ok(subscriber)
    }
}
