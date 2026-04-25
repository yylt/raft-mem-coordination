use actix_web::get;
use actix_web::Responder;

#[get("/health")]
pub async fn health() -> actix_web::Result<impl Responder> {
    Ok("ok")
}
