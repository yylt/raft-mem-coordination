use std::sync::Arc;

use actix_web::web::ServiceConfig;

use crate::config::Config;
use crate::network::health;
use crate::network::management;
use crate::network::raft;
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

pub fn configure_public(cfg: &mut ServiceConfig) {
    cfg.service(health::health).service(management::metrics);
}

// todo
pub fn configure_coordination(cfg: &mut ServiceConfig) {
    todo!()
}

pub fn configure_raft(cfg: &mut ServiceConfig) {
    cfg.service(management::init)
        .service(management::add_learner)
        .service(management::change_membership)
        .service(raft::append)
        .service(raft::snapshot)
        .service(raft::vote);
}
