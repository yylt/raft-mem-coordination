use std::collections::HashMap;

use serde::Deserialize;

use crate::NodeId;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub id: NodeId,
    pub addr: String,

    #[serde(default)]
    pub tls: Option<TlsConfig>,
    #[serde(default)]
    pub peer_addr: HashMap<NodeId, Vec<String>>,
    #[serde(default)]
    pub basic_auth: Option<BasicAuth>,
    #[serde(default)]
    pub heartbeat_interval: Option<u64>,
    #[serde(default)]
    pub election_timeout: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TlsConfig {
    Files { cert_file: String, key_file: String },
    SelfSigned { cert_dir: String },
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

    pub fn basic_auth(&self) -> Option<&BasicAuth> {
        self.basic_auth.as_ref()
    }

    pub fn tls(&self) -> Option<&TlsConfig> {
        self.tls.as_ref()
    }

    pub fn from_params(id: NodeId, addr: String) -> Self {
        Self {
            id,
            addr,
            tls: None,
            peer_addr: HashMap::new(),
            heartbeat_interval: None,
            election_timeout: None,
            basic_auth: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_tls_files_config() {
        let cfg: Config = serde_yaml::from_str(
            r#"
            id: 1
            addr: 127.0.0.1:8000
            tls:
              cert_file: cert.pem
              key_file: key.pem
            "#,
        )
        .unwrap();

        assert!(matches!(cfg.tls, Some(TlsConfig::Files { .. })));
    }

    #[test]
    fn deserializes_tls_self_signed_config() {
        let cfg: Config = serde_yaml::from_str(
            r#"
            id: 1
            addr: 127.0.0.1:8000
            tls:
              cert_dir: /tmp/certs
            "#,
        )
        .unwrap();

        assert!(matches!(cfg.tls, Some(TlsConfig::SelfSigned { .. })));
    }
}
