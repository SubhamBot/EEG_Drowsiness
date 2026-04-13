use crate::uart::{UartDma, UartPort};

/// HC-05 Bluetooth Classic (SPP) module.
///
/// The HC-05 is a transparent UART bridge — it handles all Bluetooth
/// protocol internally. From the STM32's perspective it is just a
/// UART peripheral. This wrapper exists to make the transport layer
/// explicit in the module graph:
///
///   speed data → [Bluetooth] → HC-05 → USART2 RX → DMA
///
/// Default data-mode baud rate: 9600 (configurable via AT+UART).
pub struct Hc05 {
    uart: UartDma,
}

impl Hc05 {
    pub fn new() -> Self {
        Self {
            uart: UartDma::new(),
        }
    }

    /// Set up USART2 + DMA for receiving data through the HC-05.
    pub fn init_dma(rx_buf: &[u8]) {
        UartDma::init_dma(&UartPort::Usart2, rx_buf);
    }

    /// Handle USART2 IDLE interrupt. Extracts complete lines received
    /// over the Bluetooth link and passes them to the callback.
    pub fn on_idle(&mut self, rx_buf: &[u8], on_line: impl FnMut(&[u8])) {
        self.uart.on_idle(&UartPort::Usart2, rx_buf, on_line);
    }
}
