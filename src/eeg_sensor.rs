use crate::hc05::Hc05;

pub struct EegData {
    pub alpha: f32,
    pub beta: f32,
}

/// EEG sensor. Consumes `"E,alpha,beta"` packets that arrive over the
/// HC-05 Bluetooth transport (USART2), interleaved with `"S,..."`
/// speed packets.
///
/// Layering mirrors `SpeedSensor`:
///
///   USART2 RX  ->  UartDma (line extraction)
///        |
///        +--> SpeedSensor::on_line   ("S,..." matches -> callback with f32)
///        +--> EegSensor::on_line     ("E,..." matches -> callback with EegData)
///
/// The sensor is stateless between packets; the struct exists so the
/// dispatcher pattern in `main.rs` keeps a clean `&mut` resource per
/// sensor (future extensions — dropout counters, rolling averages,
/// etc. — can add state without changing the call site).
pub struct EegSensor;

impl EegSensor {
    pub fn new() -> Self {
        Self
    }

    /// One-shot transport setup: configures USART2 + DMA1 Stream 5 via
    /// the HC-05 Bluetooth adapter. Shared with `SpeedSensor::init_dma`
    /// — whichever sensor's `init_dma` is called first wins; the other
    /// is a no-op on the same transport.
    pub fn init_dma(rx_buf: &[u8]) {
        Hc05::init_dma(rx_buf);
    }

    /// Offer a line extracted from the shared Bluetooth stream. If it
    /// matches the `"E,..."` EEG-packet format, invokes `on_packet`
    /// with the parsed `EegData` and returns `true`. Otherwise returns
    /// `false` so the dispatcher can try the next sensor.
    pub fn on_line(
        &mut self,
        line: &[u8],
        mut on_packet: impl FnMut(EegData),
    ) -> bool {
        match Self::parse(line) {
            Some(data) => {
                on_packet(data);
                true
            }
            None => false,
        }
    }

    /// Parse `"E,alpha,beta"` from raw bytes.
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
