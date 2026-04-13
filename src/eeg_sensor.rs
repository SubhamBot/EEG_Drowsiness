use crate::uart::{UartDma, UartPort};

pub struct EegData {
    pub alpha: f32,
    pub beta: f32,
}

pub struct EegSensor {
    uart: UartDma,
}

impl EegSensor {
    pub fn new() -> Self {
        Self {
            uart: UartDma::new(),
        }
    }

    /// Set up USART1 + DMA for receiving EEG data.
    pub fn init_dma(rx_buf: &[u8]) {
        UartDma::init_dma(&UartPort::Usart1, rx_buf);
    }

    /// Handle USART1 IDLE interrupt. Parses EEG packets and delivers
    /// `EegData` to the callback.
    pub fn on_idle(&mut self, rx_buf: &[u8], mut on_packet: impl FnMut(EegData)) {
        self.uart.on_idle(&UartPort::Usart1, rx_buf, |line| {
            if let Some(data) = Self::parse(line) {
                on_packet(data);
            }
        });
    }

    /// Parse "E,alpha,beta" from raw bytes.
    /// Beta is clamped to a small non-zero minimum to prevent division by zero.
    pub fn parse(line: &[u8]) -> Option<EegData> {
        let s = core::str::from_utf8(line).ok()?;
        let rest = s.trim().strip_prefix("E,")?;
        let mut parts = rest.split(',');
        let alpha = parts.next()?.trim().parse::<f32>().ok()?;
        let beta_raw = parts.next()?.trim().parse::<f32>().ok()?;
        let beta = if beta_raw.abs() < 1e-3 { 1e-3 } else { beta_raw };
        Some(EegData { alpha, beta })
    }
}
