use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use anyhow::Context;

use esp_idf_svc::eth::{BlockingEth, EspEth, EthDriver, RmiiEth};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::gpio::AnyOutputPin;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config, SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::hal::spi::*;
use esp_idf_svc::hal::units::*;
use sacn::packet::ACN_SDT_MULTICAST_PORT;
use sacn::receive::SacnReceiver;
use smart_leds_trait::{RGB8, SmartLedsWrite};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    let sys_loop = EspSystemEventLoop::take()?;

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
    let mut eth = BlockingEth::wrap(eth, sys_loop.clone())?;

    let led_pin = pins.gpio14;
    let channel = peripherals.rmt.channel0;
    let mut ws2815 = Ws2812Esp32Rmt::new(channel, led_pin).unwrap();
    let mut dmx_recv = setup_and_start(&mut eth)?;

    loop {
        let packet = dmx_recv.recv(Some(Duration::from_millis(16))).expect("Failed to receive packet");
        for dmx in packet {
            let pixels = dmx.values.chunks(3)
                .take(115)
                .map(|chunk| {
                    let rgb = RGB8 {
                        r: chunk[0],
                        g: chunk[1],
                        b: chunk[2],
                    };
                    rgb
                });
            ws2815.write(pixels)?;
        }
    }

    Ok(())
}


fn setup_and_start(eth: &mut BlockingEth<EspEth<RmiiEth>>) -> anyhow::Result<SacnReceiver> {
    eth.start()?;
    log::info!("Waiting for DHCP lease...");
    eth.wait_netif_up()?;

    let ip_info = eth.eth().netif().get_ip_info()?;

    log::info!("Eth DHCP info: {:?}", ip_info);

    let mut dmx_recv = SacnReceiver::with_ip(SocketAddr::new(ip_info.ip.into(), ACN_SDT_MULTICAST_PORT), None).map_err(|e| anyhow::anyhow!("failed to create receiver"))?;
    dmx_recv.listen_universes(&[1]).map_err(|e| anyhow::anyhow!("failed to listen to universes"))?;

    Ok(dmx_recv)
}