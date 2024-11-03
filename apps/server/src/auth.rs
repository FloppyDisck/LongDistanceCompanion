use crate::config::Config;
use crate::settings::{save_sequence, sequence};
use axum::http::{HeaderValue, StatusCode};
use secp256k1::ecdsa::Signature;
use secp256k1::hashes::{sha256, Hash};
use secp256k1::{Message, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
pub struct Authentication<T> {
    pub sequence: u64,
    pub message: T,
}

pub fn hash<T: Serialize>(msg: Authentication<T>) -> Message {
    let digest = sha256::Hash::hash(serde_json::to_vec(&msg).unwrap().as_slice());
    Message::from_digest(digest.to_byte_array())
}

pub fn sign<T: Serialize>(secret_key: &SecretKey, msg: T, sequence: u64) -> Signature {
    let secp = Secp256k1::signing_only();
    let msg = hash(Authentication {
        sequence,
        message: msg,
    });
    secp.sign_ecdsa(&msg, secret_key)
}

pub async fn evaulate<T: Serialize>(
    config: &Config,
    cert: &HeaderValue,
    expected: T,
) -> Option<StatusCode> {
    let sequence = sequence(&config.db).await;
    let expected_message = hash(Authentication {
        sequence,
        message: expected,
    });

    let signature = Signature::from_str(cert.to_str().unwrap()).unwrap();

    let secp = Secp256k1::verification_only();

    if secp
        .verify_ecdsa(&expected_message, &signature, &config.pubkey)
        .is_ok()
    {
        save_sequence(&config.db, sequence + 1).await;
        None
    } else {
        Some(StatusCode::UNAUTHORIZED)
    }
}
