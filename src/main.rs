use actix_web::middleware;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::HttpServer;
use raft_mem_coordination::build_tls_config;
use raft_mem_coordination::config::Config;
use raft_mem_coordination::{app, create_app};
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

    let app_config = Config::load(&config_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let http_addr = app_config.addr.clone();
    let tls_config = build_tls_config(&app_config)?;

    let app_data = Data::new(create_app(app_config).await);

    let server = HttpServer::new(move || {
        actix_web::App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(middleware::Compress::default())
            .app_data(app_data.clone())
            .configure(app::configure_raft)
            .configure(app::configure_public)
            .configure(app::configure_coordination)
    });

    let server = match tls_config {
        Some(tls_config) => server.bind_rustls_0_23(http_addr, tls_config)?,
        None => server.bind(http_addr)?,
    };

    server.run().await
}
