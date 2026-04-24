use std::sync::Arc;

use actix_web::middleware;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::HttpServer;
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use raft_mem_coordination::config::BasicAuth;
use raft_mem_coordination::config::Config;
use raft_mem_coordination::create_app;
use raft_mem_coordination::load_tls_config;
use raft_mem_coordination::network::api;
use raft_mem_coordination::network::coordinate;
use raft_mem_coordination::network::management;
use raft_mem_coordination::network::raft;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.yaml".to_string());

    let app_config = Config::load(&config_path).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;

    let http_addr = app_config.addr.clone();
    let auth_user = app_config.basic_auth.as_ref().map(|a| a.user.clone());
    let auth_pass = app_config.basic_auth.as_ref().map(|a| a.password.clone());
    let tls_cert = app_config.tls_cert_file.clone();
    let tls_key = app_config.tls_key_file.clone();
    
    let app_data = Data::new(create_app(app_config).await);

    let server = HttpServer::new(move || {
        let user = auth_user.clone();
        let pass = auth_pass.clone();
        
        let auth_middleware = HttpAuthentication::basic(move |req, credentials| {
            let user = user.clone();
            let pass = pass.clone();
            async move {
                match (&user, &pass) {
                    (Some(u), Some(p)) => {
                        let req_user = credentials.user_id();
                        let req_pass = credentials.password().unwrap_or("");
                        if req_user == u && req_pass == p {
                            Ok(req)
                        } else {
                            Err((actix_web::error::ErrorUnauthorized("Unauthorized"), req))
                        }
                    }
                    _ => Ok(req),
                }
            }
        });

        let app = actix_web::App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(middleware::Compress::default())
            .app_data(app_data.clone());

        // Routes without auth
        let app = app
            .service(raft::append)
            .service(raft::snapshot)
            .service(raft::vote)
            .service(coordinate::list_coordinates)
            .service(coordinate::get_coordinate)
            .service(coordinate::create_coordinate)
            .service(coordinate::delete_coordinate);

        // API routes with auth if configured
        let app = if auth_user.is_some() {
            app.service(
                actix_web::web::scope("/api")
                    .wrap(auth_middleware)
                    .service(api::write)
                    .service(api::read)
                    .service(api::consistent_read)
                    .service(management::init)
                    .service(management::add_learner)
                    .service(management::change_membership)
                    .service(management::metrics),
            )
        } else {
            app.service(
                actix_web::web::scope("/api")
                    .service(api::write)
                    .service(api::read)
                    .service(api::consistent_read)
                    .service(management::init)
                    .service(management::add_learner)
                    .service(management::change_membership)
                    .service(management::metrics),
            )
        };
        
        app
    });

    let x = match (&tls_cert, &tls_key) {
        (Some(cert), Some(key)) => server.bind_openssl(http_addr, load_tls_config(cert, key)?)?,
        _ => server.bind(http_addr)?,
    };

    x.run().await
}
