use core::fmt;

/// Log writer that sends output over RTT (Real-Time Transfer).
/// Visible in the `cargo run` / probe-rs terminal via the SWD debug link.
pub struct LogWriter;

impl LogWriter {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Write for LogWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        rtt_target::rprint!("{}", s);
        Ok(())
    }
}
