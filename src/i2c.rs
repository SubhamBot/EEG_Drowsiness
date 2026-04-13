use stm32f4xx_hal::pac;

/// Minimal blocking I2C3 driver with timeouts and bus recovery.
///
/// I2C3 on STM32F429 Discovery:
///   SCL = PA8   (AF4)
///   SDA = PC9   (AF4)
///
/// Every wait loop has a hard timeout — if the I2C bus locks up
/// (STMPE811 holding SDA low), the operation aborts with a STOP
/// condition and returns `false`. The caller can retry or skip.
/// This guarantees the system NEVER hangs on I2C.
pub struct I2c3;

/// Timeout iterations (~600 µs at 168 MHz, ~1.2 ms at 48 MHz).
const I2C_TIMEOUT: u32 = 100_000;

impl I2c3 {
    /// Bit-bang 16 SCL pulses + STOP condition to recover the I2C bus.
    ///
    /// If the STMPE811 was mid-transfer when the MCU reset (or when
    /// the clock was switched), SDA can be held LOW indefinitely.
    /// Clocking SCL lets the slave release SDA, then we send a clean
    /// STOP so both sides agree the bus is idle.
    ///
    /// Must be called BEFORE `init()`, with SCL/SDA pins configured
    /// as open-drain GPIO outputs (not I2C AF yet).
    pub fn bus_recovery() {
        unsafe {
            let gpioa = &*pac::GPIOA::ptr();
            let gpioc = &*pac::GPIOC::ptr();

            // SCL = PA8, SDA = PC9 — set as open-drain outputs
            gpioa.moder.modify(|_, w| w.moder8().output());
            gpioa.otyper.modify(|_, w| w.ot8().open_drain());
            gpioc.moder.modify(|_, w| w.moder9().output());
            gpioc.otyper.modify(|_, w| w.ot9().open_drain());

            // Release SDA high
            gpioc.bsrr.write(|w| w.bs9().set_bit());

            // Clock 16 SCL pulses to free any stuck slave
            for _ in 0..16 {
                gpioa.bsrr.write(|w| w.br8().set_bit()); // SCL low
                cortex_m::asm::delay(84_000);
                gpioa.bsrr.write(|w| w.bs8().set_bit()); // SCL high
                cortex_m::asm::delay(84_000);
            }

            // Generate a clean STOP: SCL low → SDA low → SCL high → SDA high
            gpioa.bsrr.write(|w| w.br8().set_bit());
            cortex_m::asm::delay(40_000);
            gpioc.bsrr.write(|w| w.br9().set_bit());
            cortex_m::asm::delay(40_000);
            gpioa.bsrr.write(|w| w.bs8().set_bit());
            cortex_m::asm::delay(40_000);
            gpioc.bsrr.write(|w| w.bs9().set_bit());
            cortex_m::asm::delay(80_000);
        }
    }

    /// Enable I2C3 peripheral clocks and configure for 100 kHz (SM).
    /// GPIO alternate-function setup must be done by the caller (main.rs)
    /// before calling this.
    pub fn init() {
        let rcc = unsafe { &*pac::RCC::ptr() };
        let i2c3 = unsafe { &*pac::I2C3::ptr() };

        // Enable I2C3 clock on APB1
        rcc.apb1enr.modify(|_, w| w.i2c3en().set_bit());

        // Reset I2C3
        i2c3.cr1.modify(|_, w| w.swrst().set_bit());
        i2c3.cr1.modify(|_, w| w.swrst().clear_bit());

        // APB1 clock = 42 MHz (SYSCLK 168 MHz / 4)
        i2c3.cr2.modify(|_, w| unsafe { w.freq().bits(42) });

        // CCR for 100 kHz standard mode: 42 MHz / (2 * 100 kHz) = 210
        i2c3.ccr.write(|w| unsafe { w.ccr().bits(210) });

        // TRISE: max rise time for SM = 1000 ns → (42 MHz * 1µs) + 1 = 43
        i2c3.trise.write(|w| w.trise().bits(43));

        // Enable — no I2C interrupts (all operations are blocking with timeout)
        i2c3.cr2.modify(|_, w| {
            w.itevten().clear_bit();
            w.itbufen().clear_bit();
            w.iterren().clear_bit()
        });
        i2c3.cr1.modify(|_, w| w.pe().set_bit());

        // Wait for bus to become idle after enable (with timeout).
        // If the STMPE811 was holding SDA low, bus_recovery() should have
        // freed it, but this check ensures we don't start transactions
        // on a busy bus.
        let mut t = I2C_TIMEOUT;
        while i2c3.sr2.read().busy().bit_is_set() && t > 0 {
            t -= 1;
        }
    }

    /// Reconfigure I2C3 for a new APB1 clock frequency (in MHz).
    /// Called after PLL clock switch to keep 100 kHz bus speed.
    pub fn reconfigure(apb1_freq: u8) {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };

        // Wait for any pending STOP to complete (with timeout)
        let mut t = I2C_TIMEOUT;
        while i2c3.cr1.read().stop().bit_is_set() && t > 0 {
            t -= 1;
        }
        // Wait for bus idle
        let mut t2 = I2C_TIMEOUT;
        while i2c3.sr2.read().busy().bit_is_set() && t2 > 0 {
            t2 -= 1;
        }

        // Disable I2C peripheral to safely modify timing registers
        i2c3.cr1.modify(|_, w| w.pe().clear_bit());

        i2c3.cr2.modify(|_, w| unsafe { w.freq().bits(apb1_freq) });

        let ccr = (apb1_freq as u32 * 1_000_000) / (2 * 100_000);
        i2c3.ccr.write(|w| unsafe { w.ccr().bits(ccr as u16) });

        let trise = apb1_freq + 1;
        i2c3.trise.write(|w| w.trise().bits(trise));

        i2c3.cr1.modify(|_, w| w.pe().set_bit());
    }

    /// Write a single byte to `register` at `addr`.
    /// Returns `true` on success, `false` on timeout/NACK (bus is recovered).
    pub fn write_reg(addr: u8, register: u8, value: u8) -> bool {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };

        // Wait for any pending STOP from a previous transaction to complete.
        // Starting a new transaction while STOP is still set causes the
        // I2C peripheral to enter an undefined state (RM: "Do not set
        // START while STOP is set").
        let mut t = I2C_TIMEOUT;
        while i2c3.cr1.read().stop().bit_is_set() && t > 0 {
            t -= 1;
        }
        t = I2C_TIMEOUT;
        while i2c3.sr2.read().busy().bit_is_set() && t > 0 {
            t -= 1;
        }

        i2c3.cr1.modify(|_, w| w.start().set_bit());
        if !Self::wait_flag(|sr1| sr1.sb().bit_is_set(), false) {
            return Self::abort();
        }

        i2c3.dr.write(|w| unsafe { w.bits((addr << 1) as u32) });
        if !Self::wait_addr() {
            return Self::abort();
        }

        i2c3.dr.write(|w| unsafe { w.bits(register as u32) });
        if !Self::wait_flag(|sr1| sr1.tx_e().bit_is_set(), true) {
            return Self::abort();
        }

        i2c3.dr.write(|w| unsafe { w.bits(value as u32) });
        if !Self::wait_flag(|sr1| sr1.btf().bit_is_set(), true) {
            return Self::abort();
        }

        i2c3.cr1.modify(|_, w| w.stop().set_bit());
        true
    }

    /// Read a single byte from `register` at `addr`.
    /// Returns `Some(value)` on success, `None` on timeout/NACK.
    pub fn read_reg(addr: u8, register: u8) -> Option<u8> {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };

        // Wait for any pending STOP to complete before starting.
        let mut t = I2C_TIMEOUT;
        while i2c3.cr1.read().stop().bit_is_set() && t > 0 {
            t -= 1;
        }
        t = I2C_TIMEOUT;
        while i2c3.sr2.read().busy().bit_is_set() && t > 0 {
            t -= 1;
        }

        // START
        i2c3.cr1.modify(|_, w| w.start().set_bit());
        if !Self::wait_flag(|sr1| sr1.sb().bit_is_set(), false) {
            Self::abort();
            return None;
        }

        // Address + Write
        i2c3.dr.write(|w| unsafe { w.bits((addr << 1) as u32) });
        if !Self::wait_addr() {
            Self::abort();
            return None;
        }

        // Register address
        i2c3.dr.write(|w| unsafe { w.bits(register as u32) });
        if !Self::wait_flag(|sr1| sr1.tx_e().bit_is_set(), true) {
            Self::abort();
            return None;
        }

        // Repeated START
        i2c3.cr1.modify(|_, w| w.start().set_bit());
        if !Self::wait_flag(|sr1| sr1.sb().bit_is_set(), false) {
            Self::abort();
            return None;
        }

        // Address + Read
        i2c3.dr
            .write(|w| unsafe { w.bits(((addr << 1) | 1) as u32) });

        // Single-byte read: clear ACK before clearing ADDR
        i2c3.cr1.modify(|_, w| w.ack().clear_bit());
        if !Self::wait_addr() {
            Self::abort();
            return None;
        }

        // Set STOP after clearing ADDR
        i2c3.cr1.modify(|_, w| w.stop().set_bit());

        // Wait for data
        if !Self::wait_flag(|sr1| sr1.rx_ne().bit_is_set(), false) {
            Self::abort();
            return None;
        }

        Some(i2c3.dr.read().bits() as u8)
    }

    // ---- internal helpers (all with timeouts) ----

    /// Wait for ADDR flag, checking for NACK. Clears ADDR on success.
    fn wait_addr() -> bool {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };
        let mut t = I2C_TIMEOUT;
        loop {
            let sr1 = i2c3.sr1.read();
            if sr1.addr().bit_is_set() {
                // Clear ADDR by reading SR1 then SR2
                let _ = i2c3.sr1.read();
                let _ = i2c3.sr2.read();
                return true;
            }
            if sr1.af().bit_is_set() || t == 0 {
                return false;
            }
            t -= 1;
        }
    }

    /// Wait for a status flag with timeout. Optionally checks NACK (AF).
    fn wait_flag(check: fn(&pac::i2c3::sr1::R) -> bool, check_af: bool) -> bool {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };
        let mut t = I2C_TIMEOUT;
        loop {
            let sr1 = i2c3.sr1.read();
            if check(&sr1) {
                return true;
            }
            if check_af && sr1.af().bit_is_set() {
                return false;
            }
            if t == 0 {
                return false;
            }
            t -= 1;
        }
    }

    /// Abort a failed I2C transaction: send STOP, clear error flags.
    fn abort() -> bool {
        let i2c3 = unsafe { &*pac::I2C3::ptr() };
        i2c3.cr1.modify(|_, w| w.stop().set_bit());
        i2c3.sr1.modify(|_, w| {
            w.af()
                .clear_bit()
                .berr()
                .clear_bit()
                .arlo()
                .clear_bit()
                .ovr()
                .clear_bit()
        });
        false
    }
}
