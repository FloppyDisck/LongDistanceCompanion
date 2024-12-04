#![no_std]
#![no_main]

use device::ServerState;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Stack, StackResources,
};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::TextStyle;
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    pixelcolor::BinaryColor,
    prelude::*,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use embedded_hal::spi::{ErrorType, Operation, SpiBus};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, Level, Output, Pull},
    prelude::*,
    spi::{
        master::{Config as SpiConfig, Spi},
        SpiMode,
    },
    timer::timg::TimerGroup,
    Blocking,
};
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::EspWifiController;
use log::{debug, error, info};
use profont::PROFONT_9_POINT;
use reqwless::client::HttpClient;
use weact_studio_epd::graphics::Display213BlackWhite;
use weact_studio_epd::{Color, WeActStudio213BlackWhiteDriver};

extern crate alloc;

pub const QUERY_BUFFER_SIZE: usize = 1024 * 4;

// make a static variable
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

// Wifi network credentials
const SSID: &str = "DemoNetwork";
const PASSWORD: &str = "nomeacuerdo";

struct SpiWrapper<'a> {
    spi: Spi<'a, Blocking>,
}

impl<'a> SpiWrapper<'a> {
    fn new(spi: Spi<'a, Blocking>) -> Self {
        Self { spi }
    }
}

impl<'a> ErrorType for SpiWrapper<'a> {
    type Error = esp_hal::spi::Error;
}

impl<'a> embedded_hal::spi::SpiDevice for SpiWrapper<'a> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        for operation in operations {
            match operation {
                Operation::Read(buf) => self.spi.read(buf)?,
                Operation::Write(buf) => self.spi.write(buf)?,
                Operation::Transfer(_read, write) => self.spi.write(write)?,
                Operation::TransferInPlace(buf) => self.spi.transfer_in_place(buf)?,
                Operation::DelayNs(dur) => {
                    embassy_time::block_for(embassy_time::Duration::from_nanos(*dur as u64))
                }
            }
        }

        Ok(())
    }
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
        esp_wifi::wifi::new_with_mode(init, wifi, WifiStaDevice).unwrap();

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
    spawner.spawn(net_task(stack)).ok();

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

    info!("Creating Http Client");
    let mut response_buffer = [0; QUERY_BUFFER_SIZE];
    let tcp_client_state: TcpClientState<1, 300, 1024> = TcpClientState::new();
    let tcp = TcpClient::new(stack, &tcp_client_state);
    let dns = DnsSocket::new(stack);
    let mut client = HttpClient::new(&tcp, &dns);

    info!("Initializing SPI");
    let spi = Spi::new_with_config(
        peripherals.SPI2,
        SpiConfig {
            frequency: 100.kHz(),
            mode: SpiMode::Mode0,
            ..Default::default()
        },
    )
    .with_sck(peripherals.GPIO5)
    .with_miso(peripherals.GPIO21)
    .with_mosi(peripherals.GPIO19);

    let cs_pin = Output::new(peripherals.GPIO15, Level::High);
    let edc = Output::new(peripherals.GPIO33, Level::Low);
    // The library asks for these but we're not using them
    let reset = Output::new(peripherals.GPIO13, Level::High);
    let busy = Input::new(peripherals.GPIO12, Pull::Down);

    let spi_device = ExclusiveDevice::new(spi, cs_pin, Delay).unwrap();
    let spi_interface = SPIInterface::new(spi_device, edc);

    info!("Setting Up Display Controller");
    let mut driver = WeActStudio213BlackWhiteDriver::new(spi_interface, busy, reset, Delay);
    let mut display = Display213BlackWhite::new();
    info!("Initializing Display Controller");
    display.set_rotation(weact_studio_epd::graphics::DisplayRotation::Rotate90);
    driver.init().unwrap();

    info!("Clearing Display");
    display.clear(Color::White);

    info!("Demo write");
    // display.set_rotation(DisplayRotation::Rotate0);
    let style = MonoTextStyle::new(&PROFONT_9_POINT, Color::Black);
    let _ = Text::with_text_style(
        "Hello World!",
        Point::new(0, 15),
        style,
        TextStyle::default(),
    )
    .draw(&mut display);
    // Rectangle::new(Point::new(0, 0), Size::new(40, 40))
    //     .into_styled(PrimitiveStyle::with_fill(Color::Black))
    //     .draw(&mut display)
    //     .unwrap();

    info!("Creating State");
    let mut state = ServerState::new(&mut client, &mut response_buffer).await;

    loop {
        state.update(&mut client, &mut response_buffer).await;

        debug!("Displaying");
        // TODO: display message in the top left

        // TODO: Display the graph
        // TODO: maybe do a graph where each tick is a different thinking state
        // TODO: maybe have bar charts, one on each side or all going up
        // TODO: maybe simply have a count of each one and say whats the latest one

        driver.full_update(&display).unwrap();

        // Might need to increase even more
        Timer::after(Duration::from_secs(600)).await;
    }
}

#[embassy_executor::task]
async fn update_server_state() {}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    debug!("start connection task");
    debug!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            debug!("Starting wifi");
            controller.start_async().await.unwrap();
            debug!("Wifi started!");
        }
        debug!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => debug!("Wifi connected!"),
            Err(e) => {
                error!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
