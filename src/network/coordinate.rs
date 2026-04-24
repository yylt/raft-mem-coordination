use actix_web::post;
use actix_web::web;
use actix_web::web::Data;
use actix_web::Responder;

use crate::app::App;

/// K8s coordinate resource API
/// These endpoints handle coordinate CRUD operations without auth
#[post("/coordinates")]
pub async fn list_coordinates(app: Data<App>) -> actix_web::Result<impl Responder> {
    let state_machine = app.state_machine_store.state_machine.read().await;
    let coordinates: Vec<String> = state_machine.data.keys().cloned().collect();
    Ok(web::Json(coordinates))
}

#[post("/coordinate/{key}")]
pub async fn get_coordinate(
    app: Data<App>,
    key: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    let state_machine = app.state_machine_store.state_machine.read().await;
    let value = state_machine.data.get(key.as_str()).cloned();
    Ok(web::Json(value))
}

#[post("/coordinate")]
pub async fn create_coordinate(
    app: Data<App>,
    req: web::Json<(String, String)>,
) -> actix_web::Result<impl Responder> {
    let (key, value) = req.0;
    let request = crate::store::Request::Set { key, value };
    let response = app.raft.client_write(request).await;
    Ok(web::Json(response))
}

#[post("/coordinate/{key}/delete")]
pub async fn delete_coordinate(
    app: Data<App>,
    key: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    let request = crate::store::Request::Set {
        key: key.to_string(),
        value: String::new(),
    };
    let response = app.raft.client_write(request).await;
    Ok(web::Json(response))
}
