pub struct EegSensor;
 
pub struct EegData {
    pub alpha: f32,
    pub beta: f32,
}
 
impl EegSensor {
    pub fn new() -> Self {
        Self
    }
 
    /// Try to parse an EEG packet. Returns `Some(EegData)` on success.
    /// Beta is clamped to a small non-zero minimum so downstream ratio
    /// computations never divide by zero.
    pub fn parse_packet(&self, packet: &str) -> Option<EegData> {
        let rest = packet.strip_prefix("E,")?;
        let mut parts = rest.split(',');
        let a_str = parts.next()?;
        let b_str = parts.next()?;
        let alpha = a_str.trim().parse::<f32>().ok()?;
        let beta_raw = b_str.trim().parse::<f32>().ok()?;
        let beta = if beta_raw.abs() < 1e-3 { 1e-3 } else { beta_raw };
        Some(EegData { alpha, beta })
    }
}