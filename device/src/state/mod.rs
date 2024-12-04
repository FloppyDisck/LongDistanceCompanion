mod server_state;
mod tick_history;
mod time;

use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::TcpClient;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use reqwless::client::HttpClient;
use reqwless::request::Method;
pub use server_state::*;
pub use tick_history::*;
pub use time::*;

// Message queries
pub const MESSAGE_QUERY: &str = "http://24.144.124.202:3000/message";
pub const TICK_QUERY: &str = "http://24.144.124.202:3000/ticks";
pub const TICK_HISTORY_QUERY: &str = "http://24.144.124.202:3000/compressed_tick_history";

// Message size
pub const MESSAGE_SIZE: usize = 1024;

// Tick info
pub const TICK_RX_ALLOC: usize = 1024;
pub const TICK_SIZE: usize = 25;
pub const TICK_ALLOC: usize = 10;

// Tick history
pub const TICK_HISTORY_RX_ALLOC: usize = 2048;
// We calculate size by getting tick history alloc substracting 2 (returned ticks) and dividing by 3 (tick size)
pub const TICK_HISTORY_SIZE: usize = (TICK_HISTORY_RX_ALLOC - 2) / 3;

type Client<'a, 'b, 'c, 'd, 'e, const WIFIRX: usize> = HttpClient<
    'a,
    TcpClient<'b, WifiDevice<'c, WifiStaDevice>, 1, WIFIRX>,
    DnsSocket<'d, WifiDevice<'e, WifiStaDevice>>,
>;

pub async fn query<const RX: usize, const WIFIRX: usize>(
    client: &mut Client<'_, '_, '_, '_, '_, WIFIRX>,
    response_buffer: &mut [u8],
    query: &str,
) -> [u8; RX] {
    let mut query = client.request(Method::GET, query).await.unwrap();

    let response = query.send(response_buffer).await.unwrap();

    let mut body_buffer = [0; RX];
    response
        .body()
        .reader()
        .read_to_end(&mut body_buffer)
        .await
        .unwrap();

    body_buffer
}
