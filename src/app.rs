use std::sync::Arc;

use crate::config::Config;
use crate::LogStore;
use crate::NodeId;
use crate::Raft;
use crate::StateMachineStore;

pub struct App {
    pub id: NodeId,
    pub addr: String,
    pub raft: Raft,
    pub log_store: LogStore,
    pub state_machine_store: Arc<StateMachineStore>,
    pub config: Arc<openraft::Config>,
    pub app_config: Config,
}
