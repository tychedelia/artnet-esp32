use std::net::{ToSocketAddrs, UdpSocket};

use artnet_protocol::{ArtCommand, Poll};
use esp_idf_svc::eth::{BlockingEth, EspEth, EthDriver};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::gpio::AnyOutputPin;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config, SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::hal::spi::*;
use esp_idf_svc::hal::units::*;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;

    let pins = peripherals.pins;

    let sys_loop = EspSystemEventLoop::take()?;

    // Make sure to configure ethernet in sdkconfig and adjust the parameters below for your hardware
    let eth_driver = EthDriver::new(
        peripherals.mac,
        pins.gpio25,
        pins.gpio26,
        pins.gpio27,
        pins.gpio16,
        pins.gpio22,
        pins.gpio21,
        pins.gpio19,
        pins.gpio17,
        esp_idf_svc::eth::RmiiClockConfig::<gpio::Gpio0, gpio::Gpio16, gpio::Gpio17>::Input(pins.gpio0),
        None::<AnyOutputPin>,
        esp_idf_svc::eth::RmiiEthChipset::RTL8201,
        Some(0),
        sys_loop.clone(),
    )?;
    let eth = EspEth::wrap(eth_driver)?;

    let spi = peripherals.spi2; // HSPI
    let sclk = pins.gpio14;
    let serial_in = pins.gpio12; // SDI
    let serial_out = pins.gpio13; // SDO
    let cs = pins.gpio15;

    log::info!("Starting SPI");

    let driver = SpiDriver::new::<SPI2>(
        spi,
        sclk,
        serial_out,
        Some(serial_in),
        &SpiDriverConfig::new(),
    )?;
    let config = config::Config::new().baudrate(KiloHertz::try_from(800)?.into());
    let mut led_strip = SpiDeviceDriver::new(&driver, Some(cs), &config)?;

    log::info!("Eth created");

    let mut eth = BlockingEth::wrap(eth, sys_loop.clone())?;

    log::info!("Starting eth...");

    eth.start()?;

    log::info!("Waiting for DHCP lease...");

    eth.wait_netif_up()?;

    let ip_info = eth.eth().netif().get_ip_info()?;

    log::info!("Eth DHCP info: {:?}", ip_info);

    let socket = UdpSocket::bind((ip_info.ip, 6454)).unwrap();
    socket.set_broadcast(true).unwrap();

    log::info!("Bound port");

    loop {
        log::info!("Listen!");
        let mut buffer = [0u8; 1024];
        let (length, addr) = socket.recv_from(&mut buffer).unwrap();
        let command = ArtCommand::from_buffer(&buffer[..length]).unwrap();

        log::info!("Received {:?}", command);
        match command {
            ArtCommand::Poll(poll) => {

            },
            ArtCommand::PollReply(reply) => {

            },
            ArtCommand::Output(output) => {
                led_strip.write(output.data.as_ref())?;
            }
            _ => {}
        }
    }

    Ok(())
}
