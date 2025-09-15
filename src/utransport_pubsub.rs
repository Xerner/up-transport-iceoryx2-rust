// // ################################################################################
// // Copyright (c) 2025 Contributors to the Eclipse Foundation
// //
// // See the NOTICE file(s) distributed with this work for additional
// // information regarding copyright ownership.
// //
// // This program and the accompanying materials are made available under the
// // terms of the Apache License Version 2.0 which is available at
// // https: //www.apache.org/licenses/LICENSE-2.0
// //
// // SPDX-License-Identifier: Apache-2.0
// // ################################################################################

use std::sync::Arc;

use async_trait::async_trait;
use iceoryx2::prelude::MessagingPattern;
use up_rust::{ComparableListener, UCode, UListener, UMessage, UStatus, UTransport, UUri};

use crate::{
    pubsub::Iceoryx2PubSub, service_name_mapping::ServiceNameMapper, umessage::UMessageZeroCopy,
};

#[async_trait]
impl UTransport for Iceoryx2PubSub {
    async fn send(&self, message: UMessage) -> Result<(), UStatus> {
        let service_name = {
            let source_filter = &message.attributes.source;
            let sink_filter = message.attributes.sink.as_ref();
            ServiceNameMapper::compute_service_name(
                source_filter,
                sink_filter,
                MessagingPattern::PublishSubscribe,
            )?
        };
        let publisher = self
            .get_or_create_publisher(service_name)
            .await
            .map_err(|e| {
                UStatus::fail_with_code(UCode::INTERNAL, format!("Failed to get publisher: {e}"))
            })?;

        let sample = publisher.loan_uninit().map_err(|e| {
            UStatus::fail_with_code(UCode::INTERNAL, format!("Failed to loan sample: {e}"))
        })?;
        let sample_final = sample.write_payload(UMessageZeroCopy(message));
        sample_final.send().map_err(|e| {
            UStatus::fail_with_code(UCode::INTERNAL, format!("Failed to send: {e}"))
        })?;
        Ok(())
    }

    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        up_rust::verify_filter_criteria(source_filter, sink_filter)?;
        let service_name = ServiceNameMapper::compute_service_name(
            &source_filter,
            sink_filter,
            MessagingPattern::PublishSubscribe,
        )?;
        let subscribers = self.subscribers.read().await;
        // insert subscriber for service name if it does not already exist
        if !subscribers.contains_key(&service_name) {
            let subscriber = self.create_subscriber()?;
            let mut subscribers = self.subscribers.write().await;
            subscribers.insert(service_name.clone(), Arc::new(subscriber));
        }
        // insert listener for service name if it does not already exist
        if !self.listeners.read().await.contains_key(&service_name) {
            let mut listeners = self.listeners.write().await;
            listeners
                .entry(service_name)
                .or_default()
                .insert(ComparableListener::new(listener));
        }
        Ok(())
    }

    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        up_rust::verify_filter_criteria(source_filter, sink_filter)?;
        let service_name = ServiceNameMapper::compute_service_name(
            &source_filter,
            sink_filter,
            MessagingPattern::PublishSubscribe,
        )?;
        let comparable_listener = ComparableListener::new(listener.clone());
        let mut listeners = self.listeners.write().await;
        if let Some(existing_listeners) = listeners.get_mut(&service_name) {
            existing_listeners.retain(|l| !l.eq(&comparable_listener));
            if existing_listeners.is_empty() {
                let mut subscribers = self.subscribers.write().await;
                subscribers.remove(&service_name);
            }
        }
        Ok(())
    }
}
