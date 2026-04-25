#![allow(clippy::uninlined_format_args)]
#![deny(unused_qualifications)]

use std::fs::{self, File};
use std::io::BufReader;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use openraft::Config;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use tokio_rustls::rustls::pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer,
};
use tokio_rustls::rustls::ServerConfig;

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
    pub type RPCError<E = openraft::error::Infallible> =
        openraft::error::RPCError<NodeId, BasicNode, RaftError<E>>;

    pub type ClientWriteError = openraft::error::ClientWriteError<NodeId, BasicNode>;
    pub type CheckIsLeaderError = openraft::error::CheckIsLeaderError<NodeId, BasicNode>;
    pub type ForwardToLeader = openraft::error::ForwardToLeader<NodeId, BasicNode>;
    pub type InitializeError = openraft::error::InitializeError<NodeId, BasicNode>;

    pub type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;
}

use crate::app::App;
use crate::config::Config as AppConfig;
use crate::config::TlsConfig;
use crate::network::Network;

const HEARTBEAT_INTERVAL_MS: u64 = 500;
const ELECTION_TIMEOUT_MS: u64 = 3000;
const GENERATED_CERT_FILE: &str = "cert.pem";
const GENERATED_KEY_FILE: &str = "key.pem";

fn raft_config(app_config: &AppConfig) -> Arc<Config> {
    Arc::new(
        Config {
            heartbeat_interval: app_config
                .heartbeat_interval
                .unwrap_or(HEARTBEAT_INTERVAL_MS),
            election_timeout_min: app_config.election_timeout.unwrap_or(ELECTION_TIMEOUT_MS),
            election_timeout_max: app_config.election_timeout.unwrap_or(ELECTION_TIMEOUT_MS),
            ..Default::default()
        }
        .validate()
        .unwrap(),
    )
}

pub async fn create_app(app_config: AppConfig) -> Arc<App> {
    let node_id = app_config.id;
    let http_addr = app_config.addr.clone();
    let config = raft_config(&app_config);

    let log_store = LogStore::default();
    let state_machine_store = Arc::new(StateMachineStore::default());

    let network = Network {
        use_https: app_config.tls.is_some(),
    };

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

fn load_certs(path: &Path) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

fn load_private_key(path: &Path) -> std::io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if let Some(key) = pkcs8_private_keys(&mut reader)
        .next()
        .transpose()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?
    {
        return Ok(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)));
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    if let Some(key) = rsa_private_keys(&mut reader)
        .next()
        .transpose()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?
    {
        return Ok(PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key)));
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "no private key found",
    ))
}

fn load_server_config(cert_file: &Path, key_file: &Path) -> std::io::Result<ServerConfig> {
    let certs = load_certs(cert_file)?;
    let key = load_private_key(key_file)?;

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

fn generate_self_signed_tls(cert_dir: &Path) -> std::io::Result<ServerConfig> {
    fs::create_dir_all(cert_dir)?;

    let cert_path = cert_dir.join(GENERATED_CERT_FILE);
    let key_path = cert_dir.join(GENERATED_KEY_FILE);

    if !cert_path.exists() || !key_path.exists() {
        let mut params = CertificateParams::new(vec!["localhost".to_string()])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "raft-mem-coordination");

        let key_pair = KeyPair::generate()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        fs::write(&cert_path, cert.pem())?;
        fs::write(&key_path, key_pair.serialize_pem())?;
    }

    load_server_config(&cert_path, &key_path)
}

pub fn build_tls_config(app_config: &AppConfig) -> std::io::Result<Option<ServerConfig>> {
    match app_config.tls() {
        Some(TlsConfig::Files {
            cert_file,
            key_file,
        }) => load_server_config(Path::new(cert_file), Path::new(key_file)).map(Some),
        Some(TlsConfig::SelfSigned { cert_dir }) => {
            generate_self_signed_tls(Path::new(cert_dir)).map(Some)
        }
        None => Ok(None),
    }
}
