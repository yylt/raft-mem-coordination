use actix_web::post;
use actix_web::web;
use actix_web::web::Data;
use actix_web::Responder;
use openraft::error::CheckIsLeaderError;
use openraft::error::Infallible;
use openraft::error::RaftError;
use openraft::BasicNode;
use web::Json;

use crate::app::App;
use crate::store::Request;
use crate::NodeId;

/**
 * Application API
 */
#[post("/write")]
pub async fn write(app: Data<App>, req: Json<Request>) -> actix_web::Result<impl Responder> {
    let response = app.raft.client_write(req.0).await;
    Ok(Json(response))
}

#[post("/read")]
pub async fn read(app: Data<App>, req: Json<String>) -> actix_web::Result<impl Responder> {
    let ret = app.raft.ensure_linearizable().await;

    match ret {
        Ok(_) => {
            let state_machine = app.state_machine_store.state_machine.read().await;
            let key = req.0;
            let value = state_machine.data.get(&key).cloned();

            let res: Result<String, RaftError<NodeId, CheckIsLeaderError<NodeId, BasicNode>>> =
                Ok(value.unwrap_or_default());
            Ok(Json(res))
        }
        Err(e) => Ok(Json(Err(e))),
    }
}
