use crate::settings::{ACTIVE_SETTING, MESSAGE_SETTING, SEQUENCE_SETTING};
use dotenv_codegen::dotenv;
use secp256k1::PublicKey;
use tokio_rusqlite::{params, Connection};

#[derive(Clone)]
pub struct Config {
    pub(crate) db: Connection,
    pub(crate) pubkey: PublicKey,
}

pub(crate) async fn initialize_db(conn: &Connection) {
    conn.call(|conn| {
        // Create settings
        let query = "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );";
        conn.execute(query, ())?;

        let insert = "INSERT INTO settings (key, value) VALUES (?1, ?2);";
        let mut settings_insert = conn.prepare(insert)?;
        settings_insert
            .execute(params![ACTIVE_SETTING, "true"])
            .unwrap();
        settings_insert
            .execute(params![MESSAGE_SETTING, "generic_message"])
            .unwrap();
        settings_insert
            .execute(params![SEQUENCE_SETTING, "0"])
            .unwrap();

        // Go through all the defined ticks and create them
        let query = "CREATE TABLE tick_types (
                id INTEGER PRIMARY KEY,
                value TEXT NOT NULL
            );";
        conn.execute(query, ())?;
        let insert = "INSERT INTO tick_types (value) VALUES (?1);";
        let mut tick_types_insert = conn.prepare(insert)?;
        for tick in dotenv!("TICKS").split(',') {
            tick_types_insert.execute(params![tick]).unwrap();
        }

        // Create the tick history table
        let query = "CREATE TABLE ticks (
                id INTEGER PRIMARY KEY,
                tick_type INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(tick_type) REFERENCES tick_types(id)
            );";
        conn.execute(query, ())?;

        Ok(())
    })
    .await
    .unwrap();
}
