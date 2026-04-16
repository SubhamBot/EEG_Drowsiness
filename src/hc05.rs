use crate::uart::UartDma;
use stm32f4xx_hal::pac;

/// HC-05 Bluetooth Classic (SPP) transport adapter.
///
/// The HC-05 is a transparent UART bridge — it handles all Bluetooth
/// protocol internally. From the STM32's perspective it is just a
/// UART peripheral. This module owns the transport layer for the
/// Bluetooth link:
///
///   packets over the air  →  HC-05  →  USART2 RX  →  DMA1 Stream 5
///                                           │
///                                           └→ UartDma line extraction
///                                                  │
///                                                  ├→ SpeedSensor::on_line
///                                                  └→ EegSensor::on_line
///
/// Both speed and EEG packets arrive interleaved on this single link;
/// `main.rs` dispatches each extracted line to the sensors in turn.
///
/// Data-mode baud rate is fixed at 9600 (the HC-05 default). If the
/// module is reconfigured via AT commands (AT+UART=…) this constant
/// must be kept in sync with `main.rs` Serial::new and the
/// `set_baud_usart2*` helpers in `deadman.rs`.
pub struct Hc05;

impl Hc05 {
    /// HC-05 default data-mode baud rate.
    pub const BAUD: u32 = 9600;

    /// One-shot transport setup: configures USART2 + DMA1 Stream 5 for
    /// the HC-05 link. Called once during `main.rs::init`.
    pub fn init_dma(rx_buf: &[u8]) {
        UartDma::init_usart2(rx_buf);
    }

    /// Blocking write of `data` to the HC-05 via USART2 TX (PA2).
    ///
    /// Used for handshakes, debug echoes, or AT-command probes during
    /// bring-up. This is a polled, byte-by-byte send — fine for the
    /// occasional short message; not suitable for high-throughput data
    /// (use DMA TX if that becomes a need).
    pub fn tx(data: &[u8]) {
        let usart2 = unsafe { &*pac::USART2::ptr() };
        for &byte in data {
            while usart2.sr.read().txe().bit_is_clear() {}
            usart2.dr.write(|w| unsafe { w.bits(byte as u32) });
        }
        while usart2.sr.read().tc().bit_is_clear() {}
    }

    /// Emit a one-shot RTT snapshot of USART2 + DMA1 Stream 5 registers.
    /// Handy when the Bluetooth link looks dead: confirms clocks, baud,
    /// DMA enable, and how many bytes the DMA has received.
    pub fn diagnostic_dump() {
        let usart2 = unsafe { &*pac::USART2::ptr() };
        let dma1 = unsafe { &*pac::DMA1::ptr() };
        rtt_target::rprintln!(
            "[HC05] USART2 CR1={:#06x} CR3={:#06x} BRR={:#06x} SR={:#06x}",
            usart2.cr1.read().bits(),
            usart2.cr3.read().bits(),
            usart2.brr.read().bits(),
            usart2.sr.read().bits(),
        );
        rtt_target::rprintln!(
            "[HC05] DMA1_S5 CR={:#010x} NDTR={} M0AR={:#010x} PAR={:#010x}",
            dma1.st[5].cr.read().bits(),
            dma1.st[5].ndtr.read().bits(),
            dma1.st[5].m0ar.read().bits(),
            dma1.st[5].par.read().bits(),
        );
    }
}
