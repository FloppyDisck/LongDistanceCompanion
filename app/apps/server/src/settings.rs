use crate::auth::evaulate;
use crate::config::Config;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio_rusqlite::{params, Connection};

pub const ACTIVE_SETTING: &'static str = "active";
pub const MESSAGE_SETTING: &'static str = "message";
pub const SEQUENCE_SETTING: &'static str = "sequence";

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub message: String,
}

pub async fn set_message(
    State(config): State<Config>,
    header_map: HeaderMap,
    Json(payload): Json<Message>,
) -> impl IntoResponse {
    let val = header_map.get("auth").unwrap();

    if let Some(res) = evaulate(&config, val, &payload).await {
        return (res, "".to_string());
    }

    let message = payload.message;
    set_setting(&config.db, MESSAGE_SETTING, message.clone()).await;

    (StatusCode::CREATED, message)
}

pub async fn get_message(State(config): State<Config>) -> impl IntoResponse {
    query_setting(&config.db, MESSAGE_SETTING).await
}

#[derive(Serialize, Deserialize)]
pub struct Active {
    pub active: bool,
}

pub async fn set_active(
    State(config): State<Config>,
    header_map: HeaderMap,
    Json(payload): Json<Active>,
) -> impl IntoResponse {
    let val = header_map.get("auth").unwrap();

    if let Some(res) = evaulate(&config, val, &payload).await {
        return (res, "".to_string());
    }

    let active = payload.active.to_string();
    set_setting(&config.db, ACTIVE_SETTING, active.clone()).await;

    (StatusCode::CREATED, active)
}

pub async fn get_active(State(config): State<Config>) -> impl IntoResponse {
    query_setting(&config.db, ACTIVE_SETTING).await
}

pub async fn get_sequence(State(config): State<Config>) -> impl IntoResponse {
    query_setting(&config.db, SEQUENCE_SETTING).await
}

pub async fn sequence(conn: &Connection) -> u64 {
    query_setting(conn, SEQUENCE_SETTING)
        .await
        .parse::<u64>()
        .unwrap()
}

pub async fn save_sequence(conn: &Connection, sequence: u64) {
    set_setting(conn, SEQUENCE_SETTING, sequence.to_string()).await;
}

async fn query_setting(connection: &Connection, key: &str) -> String {
    let key = key.to_string();
    connection
        .call(move |conn| {
            let res = conn
                .prepare("SELECT value FROM settings WHERE key = ?1")
                .unwrap()
                .query_row(params![key], |r| r.get(0))?;
            Ok(res)
        })
        .await
        .unwrap()
}

async fn set_setting(connection: &Connection, key: &str, value: String) {
    let key = key.to_string();
    connection
        .call(move |conn| {
            conn.execute(
                "UPDATE settings SET value = ?1 WHERE key = ?2",
                params![value, key],
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::initialize_db;
    use std::fs::remove_file;
    use std::path::PathBuf;

    #[tokio::test]
    async fn active_setting() {
        let db_path = PathBuf::from("./active_setting_db");
        let conn = Connection::open(db_path.clone()).await.unwrap();
        initialize_db(&conn).await;

        set_setting(&conn, ACTIVE_SETTING, false.to_string()).await;
        assert_eq!(query_setting(&conn, ACTIVE_SETTING).await, "false");

        set_setting(&conn, ACTIVE_SETTING, true.to_string()).await;
        assert_eq!(query_setting(&conn, ACTIVE_SETTING).await, "true");

        remove_file(db_path.clone()).unwrap();
    }

    #[tokio::test]
    async fn sequence_setting() {
        let db_path = PathBuf::from("./sequence_setting_db");
        let conn = Connection::open(db_path.clone()).await.unwrap();
        initialize_db(&conn).await;

        set_setting(&conn, SEQUENCE_SETTING, 5.to_string()).await;
        assert_eq!(query_setting(&conn, SEQUENCE_SETTING).await, "5");

        set_setting(&conn, SEQUENCE_SETTING, 10.to_string()).await;
        assert_eq!(query_setting(&conn, SEQUENCE_SETTING).await, "10");

        remove_file(db_path.clone()).unwrap();
    }

    #[tokio::test]
    async fn message_setting() {
        let db_path = PathBuf::from("./message_setting_db");
        let conn = Connection::open(db_path.clone()).await.unwrap();
        initialize_db(&conn).await;

        set_setting(&conn, MESSAGE_SETTING, "SOMETHING".to_string()).await;
        assert_eq!(query_setting(&conn, MESSAGE_SETTING).await, "SOMETHING");

        set_setting(&conn, MESSAGE_SETTING, "SOMETHING_ELSE".to_string()).await;
        assert_eq!(
            query_setting(&conn, MESSAGE_SETTING).await,
            "SOMETHING_ELSE"
        );

        remove_file(db_path.clone()).unwrap();
    }
}
