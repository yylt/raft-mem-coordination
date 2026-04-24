use std::collections::BTreeMap;

use serde::Deserialize;

use crate::NodeId;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub id: NodeId,
    pub addr: String,
    #[serde(default)]
    pub tls_cert_file: Option<String>,
    #[serde(default)]
    pub tls_key_file: Option<String>,
    #[serde(default)]
    pub peer_addr: BTreeMap<NodeId, Vec<String>>,
    #[serde(default)]
    pub basic_auth: Option<BasicAuth>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BasicAuth {
    pub user: String,
    pub password: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let f = std::fs::File::open(path)?;
        let cfg: Config = serde_yaml::from_reader(f)?;
        Ok(cfg)
    }

    pub fn from_params(id: NodeId, addr: String) -> Self {
        Self {
            id,
            addr,
            tls_cert_file: None,
            tls_key_file: None,
            peer_addr: BTreeMap::new(),
            basic_auth: None,
        }
    }
}
