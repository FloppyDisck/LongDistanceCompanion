use dotenv_codegen::dotenv;
use reqwest::{Client, Response, Url};
use secp256k1::SecretKey;
use serde::Serialize;
use server::{sign, Active, Message, Tick, TickType, TriggerTick};
use std::str::FromStr;

// TODO: import all functions from the server and implement a few functions to submit txs
#[tokio::main]
async fn main() {
    let url = Url::parse(dotenv!("CLIENT_URL")).unwrap();
    let priv_key = SecretKey::from_str(dotenv!("SECRET_KEY")).unwrap();

    dbg!(get_message(&url).await);
    dbg!(get_sequence(&url).await);
    dbg!(get_active(&url).await);
    dbg!(get_ticks(&url).await);

    set_message(
        &url,
        &priv_key,
        format!("message {}", get_sequence(&url).await),
    )
    .await;
    set_active(&url, &priv_key, !get_active(&url).await).await;
    tick(&url, &priv_key, 1).await;
    tick(&url, &priv_key, 2).await;
    tick(&url, &priv_key, 3).await;

    dbg!(get_tick_history(&url).await);
    dbg!(get_message(&url).await);
    dbg!(get_sequence(&url).await);
    dbg!(get_active(&url).await);
}

async fn get_sequence(url: &Url) -> u64 {
    reqwest::get(url.join("/sequence").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
        .parse()
        .unwrap()
}

async fn get_message(url: &Url) -> String {
    reqwest::get(url.join("/message").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

async fn get_active(url: &Url) -> bool {
    reqwest::get(url.join("/active").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
        .parse()
        .unwrap()
}

async fn post<T: Serialize>(url: &Url, path: &str, privkey: &SecretKey, message: T) -> Response {
    let sequence = get_sequence(&url).await;
    Client::builder()
        .build()
        .unwrap()
        .post(url.join(path).unwrap())
        .json(&message)
        .header("auth", sign(privkey, message, sequence).to_string())
        .send()
        .await
        .unwrap()
}

async fn set_message(url: &Url, privkey: &SecretKey, message: String) {
    post(url, "/message", privkey, Message { message }).await;
}

async fn set_active(url: &Url, privkey: &SecretKey, active: bool) {
    post(url, "/active", privkey, Active { active }).await;
}

async fn get_ticks(url: &Url) -> Vec<TickType> {
    reqwest::get(url.join("/ticks").unwrap())
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn get_tick_history(url: &Url) -> Vec<Tick> {
    reqwest::get(url.join("/tick_history").unwrap())
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn tick(url: &Url, privkey: &SecretKey, tick: u8) {
    dbg!(post(url, "/tick", privkey, TriggerTick { ty: tick }).await);
}
