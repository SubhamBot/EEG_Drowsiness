use crate::hc05::Hc05;

pub struct SpeedSensor {
    hc05: Hc05,
}

impl SpeedSensor {
    pub fn new() -> Self {
        Self {
            hc05: Hc05::new(),
        }
    }

    /// Set up USART2 + DMA for receiving speed data via HC-05 Bluetooth.
    pub fn init_dma(rx_buf: &[u8]) {
        Hc05::init_dma(rx_buf);
    }

    /// Handle USART2 IDLE interrupt. Parses speed packets received
    /// over Bluetooth and delivers speed (km/h) to the callback.
    pub fn on_idle(&mut self, rx_buf: &[u8], mut on_packet: impl FnMut(f32)) {
        self.hc05.on_idle(rx_buf, |line| {
            if let Some(speed) = Self::parse(line) {
                on_packet(speed);
            }
        });
    }

    /// Parse "S,{value}" from raw bytes.
    pub fn parse(line: &[u8]) -> Option<f32> {
        let s = core::str::from_utf8(line).ok()?;
        let rest = s.trim().strip_prefix("S,")?;
        rest.trim().parse::<f32>().ok()
    }
}
