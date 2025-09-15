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

use async_trait::async_trait;
use up_rust::UStatus;

#[async_trait]
pub trait TransportRelay: Send + Sync + 'static {
    /// Relays any and all available messages on the transport to all registered listeners
    async fn relay(&self) -> Result<(), UStatus>;
}
