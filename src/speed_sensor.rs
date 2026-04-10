pub struct SpeedSensor;
 
impl SpeedSensor {
    pub fn new() -> Self {
        Self
    }
 
    
    pub fn parse_packet(&self, packet: &str) -> Option<f32> {
        let rest = packet.strip_prefix("S,")?;
        rest.trim().parse::<f32>().ok()
    }
}