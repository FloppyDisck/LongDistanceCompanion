#![no_std]
#![no_main]

use alloc::format;
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::tcp::TcpSocket;
use embassy_net::{Stack, StackResources};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::prelude::*;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::EspWifiController;
use heapless::{String, Vec};
use log::{debug, error, info};
use reqwless::client::HttpClient;
use reqwless::request::Method;

extern crate alloc;

const SSID: &str = "DemoNetwork";
const PASSWORD: &str = "nomeacuerdo";

const MESSAGE_QUERY: &str = "http://24.144.124.202:3000/message";
const TICK_HISTORY_QUERY: &str = "http://24.144.124.202:3000/compressed_tick_history";

// make a static variable
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024);

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);
    info!("Embassy initialized!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = &*mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(
            timg0.timer0,
            esp_hal::rng::Rng::new(peripherals.RNG),
            peripherals.RADIO_CLK
        )
        .unwrap()
    );

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let config = embassy_net::Config::dhcpv4(Default::default());

    let seed = 1234;

    // Network stack
    let stack: &'static Stack<WifiDevice<'static, WifiStaDevice>> = &*mk_static!(
        Stack<WifiDevice<'_, WifiStaDevice>>,
        Stack::new(
            wifi_interface,
            config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed
        )
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(&stack)).ok();

    // Check for link
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let mut response_buffer = [0; 4096];
    loop {
        Timer::after(Duration::from_millis(1_000)).await;
        let tcp_client_state: TcpClientState<1, 1024, 1024> = TcpClientState::new();
        let mut tcp = TcpClient::new(&stack, &tcp_client_state);
        // let mut tcp = TcpSocket::new(&stack, &mut rx_buffer, &mut tx_buffer);
        let mut dns = DnsSocket::new(&stack);

        let mut client = HttpClient::new(&tcp, &dns);

        let mut query = client
            .request(Method::GET, TICK_HISTORY_QUERY)
            .await
            .unwrap();

        let response = query.send(&mut response_buffer).await.unwrap();

        let mut body_buffer = [0; 1024];
        response
            .body()
            .reader()
            .read_to_end(&mut body_buffer)
            .await
            .unwrap();

        let response_str = core::str::from_utf8(&body_buffer).unwrap();

        info!("{}", response_str);
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("start connection task");
    info!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started!");
        }
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(e) => {
                info!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
