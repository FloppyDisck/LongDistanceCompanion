mod auth;
mod config;
mod settings;
mod tick;

use crate::config::{initialize_db, Config};
use crate::settings::*;
use crate::tick::{get_embedded_tick_history, get_tick_history, get_ticks, trigger_tick};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use dotenv::dotenv;
use dotenv_codegen::dotenv;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use std::fs::{exists, remove_file};
use std::path::PathBuf;
use std::str::FromStr;
use tokio_rusqlite::Connection;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let db_path = PathBuf::from(dotenv!("DB_PATH"));
    let public_key = PublicKey::from_str(dotenv!("PUBLIC_KEY")).unwrap();

    #[cfg(debug_assertions)]
    remove_file(db_path.clone()).ok();

    let init = !exists(db_path.clone()).unwrap();

    let conn = Connection::open(db_path).await.unwrap();

    if init {
        initialize_db(&conn).await;
    }

    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        .route("/", get(health_check))
        .route("/message", get(get_message).post(set_message))
        .route("/active", get(get_active).post(set_active))
        .route("/sequence", get(get_sequence))
        .route("/tick", post(trigger_tick))
        .route("/ticks", get(get_ticks))
        .route("/tick_history", get(get_tick_history))
        .route("/compressed_tick_history", get(get_embedded_tick_history))
        .with_state(Config {
            db: conn,
            pubkey: public_key,
        });

    // run our app with hyper
    let listener = tokio::net::TcpListener::bind(dotenv!("SERVER_URL"))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> impl IntoResponse {
    "healthy".to_string()
}
