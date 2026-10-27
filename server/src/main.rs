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
        .unwrap_or_else(|e| panic!("Database error: {}", e.to_string()));

    let port = parameter::get_or("PORT", "8000");
    let host = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .unwrap_or_else(|e| panic!("Listener error: {}", e.to_string()));

    tracing::info!("Server started on {}", host);

    axum::serve(listener, routes::root::routes(Arc::new(connection)))
        .await
        .unwrap_or_else(|e| panic!("Server error: {}", e.to_string()));
}
