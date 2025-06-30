#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Delay;
use lora_phy::{sx127x, LoRa};
use lora_phy::iv::GenericSx127xInterfaceVariant;
use lora_phy::mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, RadioError, SpreadingFactor};
use lora_phy::sx127x::{Sx1276, Sx127x};

const LORA_FREQUENCY: u32 = 915_000_000;
const PREAMBLE_LENGTH: u16 = 4;
const IMPLICIT_HEADER: bool = false;
const CRC_ON: bool = true;
const IQ_INVERTED: bool = false;
const OUTPUT_POWER: i32 = 20;
const SPREADING_FACTOR: SpreadingFactor = SpreadingFactor::_10;
const BANDWIDTH: Bandwidth = Bandwidth::_250KHz;
const CODING_RATE: CodingRate = CodingRate::_4_8;

pub struct LoraRadio {
    lora: LoRa<Sx127x<SpiDevice<'static, NoopRawMutex, Spi<'static, SPI1, Async>, Output<'static>>, GenericSx127xInterfaceVariant<Output<'static>, Input<'static>>, Sx1276>, Delay>,
    mod_params: ModulationParams,
    packet_params: PacketParams
}

impl LoraRadio {
    pub async fn new(
        spi_bus: &'static Mutex<NoopRawMutex, Spi<'static, SPI1, Async>>,
        chip_select: Output<'static>,
        reset: Output<'static>,
        dio0: Input<'static>
    ) -> Self {
        let spi_device = SpiDevice::new(spi_bus, chip_select);
        let config = sx127x::Config {
            chip: Sx1276,
            tcxo_used: false,
            tx_boost: false,
            rx_boost: false,
        };

        let iv = GenericSx127xInterfaceVariant::new(reset, dio0, None, None).unwrap();
        let mut lora = LoRa::new(Sx127x::new(spi_device, iv, config), true, Delay).await.unwrap();
        let mod_params = lora.create_modulation_params(SPREADING_FACTOR, BANDWIDTH, CODING_RATE, LORA_FREQUENCY).unwrap();
        let packet_params = lora.create_tx_packet_params(PREAMBLE_LENGTH, IMPLICIT_HEADER, CRC_ON, IQ_INVERTED, &mod_params).unwrap();

        LoraRadio { lora, mod_params, packet_params }
    }

    async fn tx(&mut self, buffer: &[u8]) -> Result<(), RadioError> {
        self.lora.prepare_for_tx(&self.mod_params, &mut self.packet_params, OUTPUT_POWER, buffer).await?;
        self.lora.tx().await
    }
}

pub async fn radio_tx(radio: &'static Mutex<NoopRawMutex, LoraRadio>, data: &[u8]) -> Result<(), RadioError> {
    let mut radio = radio.lock().await;
    radio.tx(data).await
}