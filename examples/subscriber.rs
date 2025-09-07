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

use std::{str::FromStr, sync::Arc};
use async_trait::async_trait;
use up_rust::{UListener, UMessage, UStatus, UUri, UTransport};
use up_transport_iceoryx2_rust::service_name_mapping::{ToServiceName};
use up_transport_iceoryx2_rust::transport::{UTransportIceoryx2};

/// This examples [`UListener`] implementation struct. The intention behind inheriting from [`tokio::runtime::Runtime`] is so
/// that its task runs on its own thread pool. This is a much more realistic example than running everything on a single thread
///
/// See the helper functions [`create_multithreaded_runtime`] and [`create_example_ulistener`] for how the thread pool
/// is configured and used with this struct
struct SubscriberListener(tokio::runtime::Runtime);

#[async_trait]
impl UListener for SubscriberListener {
    /// Spawns a thread to process the received message. In this example, we simply print the message contents.
    async fn on_receive(&self, msg: UMessage) {
        self.0.spawn(async move {
            print_umessage(&msg);
        });
    }
}

/// This example sets up a single threaded runtime for the [`UListener`] to execute on, and loops for 
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("uProtocols UTransportIceoryx2 subscriber example");
    let (source_filter_uuri, sink_filter_uuri) = create_uuris("//*/FFFFAAAA/1/8001", "//*/FFFFBBBB/1/8001");
    let transport = UTransportIceoryx2::builder()
        .build()
        .await?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("subscriber-example")
        .worker_threads(1)
        .build()?;
    let ulistener = Arc::new(SubscriberListener(runtime));
    transport.register_listener(&source_filter_uuri, Some(&sink_filter_uuri), ulistener).await?;
    print_that_subscriber_service_has_been_registered(source_filter_uuri, sink_filter_uuri)?;
    tokio::signal::ctrl_c().await.map_err(Box::from)
}

fn create_uuris(source: &str, sink: &str) -> (UUri, UUri) {
    let source_uuri = UUri::from_str(source).expect("Failed to create source UUri");
    let sink_uuri = UUri::from_str(sink).expect("Failed to create sink UUri");
    (source_uuri, sink_uuri)
}

fn print_that_subscriber_service_has_been_registered(source_uuri: UUri, sink_filter_uuri: UUri) -> Result<(), UStatus> {
    let service_name = source_uuri.to_service_name(Some(&sink_filter_uuri))?;
    let service_name_str = service_name.as_str();
    print!("Listener added for service '{service_name_str}' with source '{source_uuri}' and sink '{sink_filter_uuri}' uri's");
    Ok(())
}

/// Simply prints the [`UMessage`] instances source uri, sink uri, and payload to STDOUT
fn print_umessage(msg: &UMessage) {
    let payload_utf8 = msg.payload.as_ref().map(|p| String::from_utf8_lossy(p));
    let source_uri = get_source_uri(msg);
    let sink_uri = get_sink_uri(msg);
    println!("Received a message!");
    println!("Source Uri: {source_uri:?}");
    println!("Sink Uri: {sink_uri:?}");
    println!("Payload: {payload_utf8:?}");
}

fn get_source_uri(msg: &UMessage) -> Option<String> {
    msg.attributes
        .as_ref()
        .and_then(|a| a.source.as_ref())
        .map(|s| s.to_uri(false))
}

fn get_sink_uri(msg: &UMessage) -> Option<String> {
    msg.attributes
        .as_ref()
        .and_then(|a| a.sink.as_ref())
        .map(|s| s.to_uri(false))
}
