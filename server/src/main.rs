use crate::config::database::DatabaseTrait;
use crate::config::{database, parameter};
use std::sync::Arc;

mod config;
mod dto;
mod entity;
mod error;
mod handler;
mod repository;
mod response;
mod routes;
mod service;
mod state;

#[tokio::main]
async fn main() {
    parameter::init();
    tracing_subscriber::fmt::init();

    let connection = database::Database::init()
        .await
        .unwrap_or_else(|e| panic!("Database error: {}", e));

    let port = parameter::get_or("PORT", "8000");
    let host = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .unwrap_or_else(|e| panic!("Listener error: {}", e));

    tracing::info!("Server started on {}", host);

    axum::serve(listener, routes::root::routes(Arc::new(connection)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| panic!("Server error: {}", e));
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown requested");
}
