#![allow(clippy::uninlined_format_args)]
#![deny(unused_qualifications)]

use std::io::Cursor;
use std::sync::Arc;

use openraft::Config;

pub mod app;
pub mod client;
pub mod config;
pub mod network;
pub mod store;
#[cfg(test)]
mod test;

pub type NodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig:
        D = store::Request,
        R = store::Response,
);

pub type LogStore = store::LogStore;
pub type StateMachineStore = store::StateMachineStore;
pub type Raft = openraft::Raft<TypeConfig>;

pub mod typ {
    use openraft::BasicNode;

    use crate::NodeId;
    use crate::TypeConfig;

    pub type RaftError<E = openraft::error::Infallible> = openraft::error::RaftError<NodeId, E>;
    pub type RPCError<E = openraft::error::Infallible> = openraft::error::RPCError<NodeId, BasicNode, RaftError<E>>;

    pub type ClientWriteError = openraft::error::ClientWriteError<NodeId, BasicNode>;
    pub type CheckIsLeaderError = openraft::error::CheckIsLeaderError<NodeId, BasicNode>;
    pub type ForwardToLeader = openraft::error::ForwardToLeader<NodeId, BasicNode>;
    pub type InitializeError = openraft::error::InitializeError<NodeId, BasicNode>;

    pub type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;
}

use crate::app::App;
use crate::config::Config as AppConfig;
use crate::network::Network;

/// Create App instance from config
pub async fn create_app(app_config: AppConfig) -> Arc<App> {
    let node_id = app_config.id;
    let http_addr = app_config.addr.clone();

    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };

    let config = Arc::new(config.validate().unwrap());

    let log_store = LogStore::default();
    let state_machine_store = Arc::new(StateMachineStore::default());

    let network = Network {};

    let raft = openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store.clone(),
        state_machine_store.clone(),
    )
    .await
    .unwrap();

    Arc::new(App {
        id: node_id,
        addr: http_addr,
        raft,
        log_store,
        state_machine_store,
        config,
        app_config,
    })
}

/// Load TLS config from cert and key files
pub fn load_tls_config(cert_file: &str, key_file: &str) -> std::io::Result<openssl::ssl::SslAcceptorBuilder> {
    let mut builder = openssl::ssl::SslAcceptor::mozilla_intermediate_v5(openssl::ssl::SslMethod::tls())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    builder.set_certificate_chain_file(cert_file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    builder.set_private_key_file(key_file, openssl::ssl::SslFiletype::PEM)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(builder)
}
