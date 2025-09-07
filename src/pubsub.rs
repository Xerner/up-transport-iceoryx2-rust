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

use std::{collections::HashMap, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::Duration};
use iceoryx2::{prelude::ServiceName, service::ipc};
use tokio::{sync::{oneshot, Mutex}, task::JoinHandle};
use up_rust::{UCode, UListener, UMessage, UStatus, UTransport, UUri};
use async_trait::async_trait;
use crate::{service_name_mapping::{ServiceNameMapper, ToServiceName}, transport::UTransportIceoryx2, umessage::UMessageZeroCopy};

enum TransportCommand {
    Send {
        message: UMessage,
        result: oneshot::Sender<Result<(), UStatus>>,
    },
    RegisterListener {
        service_name: ServiceName,
        listener: Arc<dyn UListener>,
        result: oneshot::Sender<Result<(), UStatus>>,
    },
    UnregisterListener {
        service_name: ServiceName,
        listener: Arc<dyn UListener>,
        result: oneshot::Sender<Result<(), UStatus>>,
    },
}

pub struct PubSubThreadWorker
{
    transport: UTransportIceoryx2<ipc::Service>,
    command_sender: std::sync::mpsc::Sender<TransportCommand>,
    data_receiver: Mutex<std::sync::mpsc::Receiver<TransportCommand>>,
    runtime: tokio::runtime::Runtime,
    keep_alive: Arc<AtomicBool>,
    cycle_time: std::time::Duration,
}

impl PubSubThreadWorker
{
    pub fn new(transport: UTransportIceoryx2<ipc::Service>, runtime: tokio::runtime::Runtime, cycle_time: Duration) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            transport,
            runtime,
            command_sender: tx,
            data_receiver: Mutex::new(rx),
            keep_alive: Arc::new(AtomicBool::new(true)),
            cycle_time,
        }
    }

    /// single threaded version of register_listener until iceoryx2 supports thread safe stuff inside their Publisher/Subscriber services
    fn register_listener_single_thread(
        &self,
        service_name: &ServiceName,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let subscriber_service = self.transport.node
            .service_builder(service_name)
            .publish_subscribe::<UMessageZeroCopy>()
            .open_or_create()
            .map_err(|e| UStatus::fail_with_code(UCode::INTERNAL, e.to_string()))?;
        let subscriber = subscriber_service
            .subscriber_builder()
            .create()
            .map_err(|e| UStatus::fail_with_code(UCode::INTERNAL, e.to_string()))?;
        while self.transport.node.wait(self.transport.cycle_time).is_ok() && self.transport.keep_alive.load(Ordering::Relaxed) {
            match subscriber.receive() {
                Ok(Some(sample)) => {
                    let umessage_zero_copy_wrapper = sample.payload();
                    // How would this be done without cloning the payload? 
                    // Pretty sure the whole point of using iceoryx2 was to prevent cloning samples
                    let umessage = umessage_zero_copy_wrapper.0.to_owned();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(listener.on_receive(umessage));
                    });
                }
                Ok(None) => {
                    println!("[main] no sample received");
                }
                Err(e) => {
                    eprintln!("Error receiving sample: {e}");
                }
            }
        }
        Ok(())
    }
}

pub struct PubSubThreadRunner {
    handle: JoinHandle<Result<(), UStatus>>,
    worker: PubSubThreadWorker,
}


impl PubSubThreadRunner {
    pub fn create(transport: UTransportIceoryx2<ipc::Service>, runtime: tokio::runtime::Runtime, cycle_time: Duration) -> (PubSubThreadWorker, JoinHandle<Result<(), UStatus>>) {
        let worker = PubSubThreadWorker::new(transport, runtime, cycle_time);
        let handle = tokio::spawn(PubSubThreadRunner::run(&worker));
        (worker, handle)
    }
    
    async fn run(worker: &PubSubThreadWorker) -> Result<(), UStatus> {
        let node = UTransportIceoryx2::<ipc::Service>::create_node(None)?;

        let mut publishers: HashMap<
            String,
            iceoryx2::port::publisher::Publisher<ipc::Service, RawBytes, CustomHeader>,
        > = HashMap::new();

        let mut subscribers: HashMap<
            String,
            iceoryx2::port::subscriber::Subscriber<ipc::Service, RawBytes, CustomHeader>,
        > = HashMap::new();

        let mut listeners: HashMap<String, Vec<Arc<dyn UListener>>> = HashMap::new();

        while worker.keep_alive.load(Ordering::Relaxed) && worker.transport.node.wait(worker.cycle_time).is_ok() {
            while let Ok(command) = worker.data_receiver.try_recv() {
                match command {
                    TransportCommand::Send { message, result: response } => {
                        let service_name = match UTransportIceoryx2::<ipc::Service>::compute_service_name(&message) {
                            Ok(name) => name,
                            Err(e) => {
                                let _ = response.send(Err(e));
                                continue;
                            }
                        };

                        let publisher =
                            publishers.entry(service_name.clone()).or_insert_with(|| {
                                let service_name_res: Result<ServiceName, _> =
                                    service_name.as_str().try_into();
                                let service = node
                                    .service_builder(&service_name_res.unwrap())
                                    .publish_subscribe::<RawBytes>()
                                    .user_header::<CustomHeader>()
                                    .open_or_create()
                                    .expect("Failed to create service");

                                service
                                    .publisher_builder()
                                    .create()
                                    .expect("Failed to create publisher")
                            });

                        let result = Self::handle_send(publisher, message);
                        let _ = response.send(result);
                    }
                    TransportCommand::RegisterListener {
                        source_filter,
                        sink_filter,
                        listener,
                        result: response,
                    } => {
                        let res = Self::handle_register_listener(
                            &node,
                            &mut subscribers,
                            &mut listeners,
                            source_filter,
                            sink_filter.as_ref(),
                            listener,
                        );
                        let _ = response.send(res);
                    }
                    TransportCommand::UnregisterListener {
                        source_filter,
                        sink_filter,
                        listener,
                        result: response,
                    } => {
                        let res = Self::handle_unregister_listener(
                            &mut subscribers,
                            &mut listeners,
                            source_filter,
                            sink_filter.as_ref(),
                            &listener,
                        );
                        let _ = response.send(res);
                    }
                }
            }

            // Integrate dispatch: In polling/receive, extract attributes and reconstruct UMessage
            // Only process subscribers that have active listeners
            let active_services: Vec<(String, Vec<Arc<dyn UListener>>)> = listeners
                .iter()
                .filter(|(service_name, listeners_vec)| {
                    !listeners_vec.is_empty() && subscribers.contains_key(*service_name)
                })
                .map(|(service_name, listeners_vec)| (service_name.clone(), listeners_vec.clone()))
                .collect();
            
            for (service_name, listeners_to_notify) in active_services {
                if let Some(subscriber) = subscribers.get(&service_name) {
                    while let Some(sample) = subscriber.receive().ok().flatten() {
                        for listener in &listeners_to_notify {
                            // Extract payload bytes
                            let payload_bytes = sample.payload().to_bytes();

                            // Reconstruct UMessage with deserialized header to UAttributes
                            let mut new_umessage = UMessage::new();
                            
                            // Extract attributes (UAttributes::from(custom_header) - full impl: parse version, deserialize Protobuf)
                            new_umessage.attributes = MessageField::some(UAttributes::from(sample.user_header()));
                            
                            // Attach payload bytes
                            new_umessage.payload = Some(payload_bytes.into());

                            // Invoke listener.on_message with reconstructed UMessage
                            let listener_clone = listener.clone();
                            tokio::spawn(async move {
                                listener_clone.on_receive(new_umessage).await;
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn kill(worker: &PubSubThreadWorker) {
        worker.keep_alive.store(false, Ordering::Relaxed);
    }
}

#[async_trait]
impl UTransport for PubSubThreadRunner
{
    async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
        todo!();
    }

    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        up_rust::verify_filter_criteria(source_filter, sink_filter)?;
        let (tx, rx) = oneshot::channel::<Result<(), UStatus>>();
        let service_name = ServiceNameMapper::compute_service_name(source_filter, sink_filter)?;
        // self.transport.listeners.add_listener(&service_name, listener).await?;

        let command = TransportCommand::RegisterListener {
            service_name,
            listener,
            result: tx,
        };

        self.worker.command_sender
            .send(command)
            .map_err(|_| UStatus::fail_with_code(UCode::INTERNAL, "Background task has died"))?;

        rx.blocking_recv().map_err(|_| {
            UStatus::fail_with_code(UCode::INTERNAL, "Background task response failed")
        })?
    }

    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        up_rust::verify_filter_criteria(source_filter, sink_filter)?;
        let service_name = source_filter.to_service_name(sink_filter)?;
        self.transport.listeners.remove_listener(service_name, listener).await?;
        todo!()
    }
   
}
