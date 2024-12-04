use crate::auth::evaulate;
use crate::config::Config;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, NaiveTime, Timelike, Utc};
use chrono_tz::America::Puerto_Rico;
use serde::{Deserialize, Serialize};
use tokio_rusqlite::{params, Connection};

#[derive(Serialize, Deserialize)]
pub struct TriggerTick {
    pub ty: u8,
}

pub async fn trigger_tick(
    State(config): State<Config>,
    header_map: HeaderMap,
    Json(payload): Json<TriggerTick>,
) -> impl IntoResponse {
    let val = header_map.get("auth").unwrap();

    if let Some(res) = evaulate(&config, val, &payload).await {
        return (res, "".to_string());
    }

    let tick = payload.ty as u8;

    config
        .db
        .call(move |conn| {
            conn.execute("INSERT INTO ticks (tick_type) VALUES (?1);", params![tick])
                .unwrap();
            Ok(())
        })
        .await
        .unwrap();

    (StatusCode::CREATED, tick.to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TickType {
    pub id: u8,
    pub tick: String,
}
pub async fn get_ticks(State(config): State<Config>) -> Json<Vec<TickType>> {
    Json(
        config
            .db
            .call(move |conn| {
                let res: Vec<TickType> = conn
                    .prepare("SELECT id, value FROM tick_types")
                    .unwrap()
                    .query_map([], |r| {
                        Ok(TickType {
                            id: r.get(0)?,
                            tick: r.get(1)?,
                        })
                    })?
                    .map(|i| i.unwrap())
                    .collect();
                Ok(res)
            })
            .await
            .unwrap(),
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tick {
    pub id: u8,
    pub tick: u8,
    pub time: String,
}

pub async fn get_tick_history(State(config): State<Config>) -> Json<Vec<Tick>> {
    Json(query_ticks(&config.db).await)
}

pub async fn query_ticks(connection: &Connection) -> Vec<Tick> {
    let time = Utc::now()
        .with_timezone(&Puerto_Rico)
        .with_time(NaiveTime::from_hms_opt(6, 0, 0).unwrap())
        .unwrap()
        .naive_utc();

    connection
        .call(move |conn| {
            let res = conn
                .prepare(
                    "\
                SELECT id, tick_type, created_at \
                FROM ticks \
                WHERE created_at >= ?1;",
                )
                .unwrap()
                .query_map(params![time], |r| {
                    Ok(Tick {
                        id: r.get(0)?,
                        tick: r.get(1)?,
                        time: r.get(2)?,
                    })
                })?
                .map(|i| i.unwrap())
                .collect();
            Ok(res)
        })
        .await
        .unwrap()
}

/// WARNING: the returned data assumes that tick is one byte and hour and minute one byte each,
/// this reduces each tick into 3 bytes total
pub async fn get_embedded_tick_history(State(config): State<Config>) -> impl IntoResponse {
    crate::tick::query_embedded_ticks(&config.db).await
}

pub async fn query_embedded_ticks(connection: &Connection) -> Bytes {
    let time = Utc::now()
        .with_timezone(&Puerto_Rico)
        .with_time(NaiveTime::from_hms_opt(6, 0, 0).unwrap())
        .unwrap()
        .naive_utc();

    let collection: Vec<[u8; 3]> = connection
        .call(move |conn| {
            let res = conn
                .prepare(
                    "\
                SELECT id, tick_type, created_at \
                FROM ticks \
                WHERE created_at >= ?1;",
                )
                .unwrap()
                .query_map(params![time], |r| {
                    let tick: u8 = r.get(1)?;
                    let date_time: DateTime<Utc> = r.get(2)?;
                    let local_time = date_time.with_timezone(&Puerto_Rico);
                    let hour = local_time.hour() as u8;
                    let minute = local_time.minute() as u8;

                    Ok([tick, hour, minute])
                })?
                .map(|i| i.unwrap())
                .collect();
            Ok(res)
        })
        .await
        .unwrap();

    let mut res = Vec::with_capacity((collection.len() * 3) + 2);
    let collection_size: [u8; 2] = (collection.len() as u16).to_be_bytes();
    res.append(&mut collection_size.to_vec());
    for collection in collection {
        res.append(&mut collection.to_vec());
    }
    Bytes::from(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::initialize_db;
    use chrono::{Days, NaiveTime, Utc};
    use chrono_tz::America::Puerto_Rico;
    use std::fs::remove_file;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ticks() {
        let db_path = PathBuf::from("./test_ticks_db");
        let conn = Connection::open(db_path.clone()).await.unwrap();
        initialize_db(&conn).await;

        // Create the important data

        conn.call(|conn| {
            let insert = "INSERT INTO ticks (tick_type, created_at) VALUES (?1, ?2);";
            let mut ticks_insert = conn.prepare(insert)?;

            for hour in 0..24 {
                let time = Utc::now()
                    .with_timezone(&Puerto_Rico)
                    .with_time(NaiveTime::from_hms_opt(hour, 0, 0).unwrap())
                    .unwrap()
                    .naive_utc();

                // Today
                ticks_insert.execute(params![2, time]).unwrap();

                // Yesterday
                ticks_insert
                    .execute(params![1, time.checked_sub_days(Days::new(1)).unwrap()])
                    .unwrap();
            }

            Ok(())
        })
        .await
        .unwrap();

        for tick in query_ticks(&conn).await {
            assert_eq!(tick.tick, 2);
        }

        println!("{:?}", query_embedded_ticks(&conn).await);

        remove_file(db_path.clone()).unwrap();
    }
}
