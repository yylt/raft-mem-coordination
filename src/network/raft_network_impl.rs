use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RemoteError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::BasicNode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

use crate::typ;
use crate::NodeId;
use crate::TypeConfig;

#[derive(Clone, Copy)]
pub struct Network {
    pub(crate) use_https: bool,
}

impl Network {
    fn target_addrs(target_node: &BasicNode) -> Vec<&str> {
        let addrs: Vec<&str> = target_node
            .addr
            .split(';')
            .filter(|addr| !addr.is_empty())
            .collect();

        if addrs.is_empty() {
            vec![target_node.addr.as_str()]
        } else {
            addrs
        }
    }

    fn scheme(&self) -> &'static str {
        if self.use_https {
            "https"
        } else {
            "http"
        }
    }

    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: NodeId,
        target_node: &BasicNode,
        uri: &str,
        req: Req,
    ) -> Result<Resp, openraft::error::RPCError<NodeId, BasicNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;

        let addrs = Self::target_addrs(target_node);
        let mut last_err = None;

        for (idx, addr) in addrs.iter().enumerate() {
            let url = format!("{}://{}/{}", self.scheme(), addr, uri);
            tracing::debug!("send_rpc to url: {}", url);

            let resp = match client.post(url.clone()).json(&req).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    let err = if e.is_connect() {
                        openraft::error::RPCError::Unreachable(Unreachable::new(&e))
                    } else {
                        openraft::error::RPCError::Network(NetworkError::new(&e))
                    };

                    last_err = Some(err);
                    if idx + 1 < addrs.len() {
                        continue;
                    }

                    return Err(last_err.take().expect("last error must exist"));
                }
            };

            tracing::debug!("client.post() is sent");

            let res: Result<Resp, Err> = resp
                .json()
                .await
                .map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;

            return res
                .map_err(|e| openraft::error::RPCError::RemoteError(RemoteError::new(target, e)));
        }

        Err(last_err.expect("send_rpc requires at least one address"))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        NetworkConnection {
            owner: *self,
            target,
            target_node: node.clone(),
        }
    }
}

pub struct NetworkConnection {
    owner: Network,
    target: NodeId,
    target_node: BasicNode,
}

impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, typ::RPCError> {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-append", req)
            .await
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<InstallSnapshotResponse<NodeId>, typ::RPCError<InstallSnapshotError>> {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-snapshot", req)
            .await
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, typ::RPCError> {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-vote", req)
            .await
    }
}
