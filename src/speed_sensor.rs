use crate::hc05::Hc05;

/// Vehicle speed sensor. Consumes `"S,<value>"` packets that arrive over
/// the HC-05 Bluetooth transport (USART2). The sensor owns its protocol
/// (the `"S,"` prefix and the float format); the transport — USART2 +
/// DMA — is set up once via `init_dma`.
///
/// Layering:
///
///   USART2 RX  ->  UartDma (line extraction)
///        |
///        +--> SpeedSensor::on_line   ("S,..." matches -> callback with f32)
///        +--> EegSensor::on_line     ("E,..." matches -> callback with EegData)
///
/// The sensor is stateless between packets; the struct exists so the
/// dispatcher pattern in `main.rs` keeps a clean `&mut` resource per
/// sensor (future extensions — moving averages, dropout tracking, etc.
/// — can add state without changing the call site).
pub struct SpeedSensor;

impl SpeedSensor {
    pub fn new() -> Self {
        Self
    }

    /// One-shot transport setup: configures USART2 + DMA1 Stream 5 via
    /// the HC-05 Bluetooth adapter.
    pub fn init_dma(rx_buf: &[u8]) {
        Hc05::init_dma(rx_buf);
    }

    /// Offer a line extracted from the shared Bluetooth stream. If it
    /// matches the `"S,..."` speed-packet format, invokes `on_speed`
    /// with the parsed value and returns `true`. Otherwise returns
    /// `false` so the dispatcher can try the next sensor.
    pub fn on_line(
        &mut self,
        line: &[u8],
        mut on_speed: impl FnMut(f32),
    ) -> bool {
        match Self::parse(line) {
            Some(v) => {
                on_speed(v);
                true
            }
            None => false,
        }
    }

    /// Parse `"S,{value}"` from raw bytes. Pure function — testable on
    /// host without hardware.
    pub fn parse(line: &[u8]) -> Option<f32> {
        let s = core::str::from_utf8(line).ok()?;
        let rest = s.trim().strip_prefix("S,")?;
        rest.trim().parse::<f32>().ok()
    }
}
