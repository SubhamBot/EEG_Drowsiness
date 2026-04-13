use crate::i2c::I2c3;
use stm32f4xx_hal::pac;

/// STMPE811 I2C address on STM32F429 Discovery.
const STMPE811_ADDR: u8 = 0x41;

// ---- STMPE811 registers ----
const SYS_CTRL1: u8 = 0x03;
const SYS_CTRL2: u8 = 0x04;
const INT_CTRL: u8 = 0x09;
const INT_EN: u8 = 0x0A;
const INT_STA: u8 = 0x0B;
const ADC_CTRL1: u8 = 0x20;
const ADC_CTRL2: u8 = 0x21;
const TSC_CTRL: u8 = 0x40;
const TSC_CFG: u8 = 0x41;
const FIFO_TH: u8 = 0x4A;
const FIFO_STA: u8 = 0x4B;
const TSC_FRACTION_Z: u8 = 0x56;
const TSC_I_DRIVE: u8 = 0x58;
const GPIO_AF: u8 = 0x17;

/// Touch controller driver for the STMPE811 on the STM32F429 Discovery.
pub struct TouchScreen;

impl TouchScreen {
    /// Initialise the STMPE811 for touch detection.
    /// I2C3 must be initialised before calling this.
    pub fn init() {
        // Software reset
        I2c3::write_reg(STMPE811_ADDR, SYS_CTRL1, 0x02);
        cortex_m::asm::delay(168_000 * 10);
        I2c3::write_reg(STMPE811_ADDR, SYS_CTRL1, 0x00);

        // Enable TSC and ADC clocks, disable GPIO clock
        I2c3::write_reg(STMPE811_ADDR, SYS_CTRL2, 0x04);

        // ADC: 12-bit, internal reference, 80-cycle sample time
        I2c3::write_reg(STMPE811_ADDR, ADC_CTRL1, 0x49);
        cortex_m::asm::delay(168_000 * 2);
        I2c3::write_reg(STMPE811_ADDR, ADC_CTRL2, 0x01);

        // Route PA4-7 to touchscreen function (not GPIO)
        I2c3::write_reg(STMPE811_ADDR, GPIO_AF, 0x00);

        // TSC config: averaging 4, touch detect delay 500us, settling 500us
        I2c3::write_reg(STMPE811_ADDR, TSC_CFG, 0x9A);

        // FIFO threshold = 1
        I2c3::write_reg(STMPE811_ADDR, FIFO_TH, 0x01);

        // Clear FIFO
        I2c3::write_reg(STMPE811_ADDR, FIFO_STA, 0x01);
        I2c3::write_reg(STMPE811_ADDR, FIFO_STA, 0x00);

        // Z fraction = 7/1
        I2c3::write_reg(STMPE811_ADDR, TSC_FRACTION_Z, 0x07);

        // Touchscreen drive current: 50 mA
        I2c3::write_reg(STMPE811_ADDR, TSC_I_DRIVE, 0x01);

        // Enable TSC: X, Y acquisition
        I2c3::write_reg(STMPE811_ADDR, TSC_CTRL, 0x01);

        // Enable touch-detect interrupt
        I2c3::write_reg(STMPE811_ADDR, INT_EN, 0x01);

        // Interrupt: active-low EDGE, global enable
        //   Bit 0: GLOBAL_INT = 1
        //   Bit 1: INT_TYPE   = 1 (edge, not level)
        //   Bit 2: INT_POLARITY = 0 (active low)
        I2c3::write_reg(STMPE811_ADDR, INT_CTRL, 0x03);

        // Clear any pending interrupts
        I2c3::write_reg(STMPE811_ADDR, INT_STA, 0xFF);
    }

    /// Returns `true` if a finger is currently touching the screen.
    /// Blocking I2C read (~200 us). Returns `false` on I2C failure.
    pub fn is_touched() -> bool {
        match I2c3::read_reg(STMPE811_ADDR, TSC_CTRL) {
            Some(ctrl) => (ctrl & 0x80) != 0,
            None => false, // I2C failed — treat as "not touched"
        }
    }

    /// Clear the STMPE811 interrupt flag and flush the FIFO.
    /// Blocking I2C writes (~600 us total). Failures are silently ignored
    /// (the next touch event will retry).
    pub fn clear_interrupt() {
        I2c3::write_reg(STMPE811_ADDR, INT_STA, 0xFF);
        I2c3::write_reg(STMPE811_ADDR, FIFO_STA, 0x01);
        I2c3::write_reg(STMPE811_ADDR, FIFO_STA, 0x00);
    }

    /// Configure PA15 as EXTI15 input for the STMPE811 INT pin.
    /// Falling edge only (active-low edge interrupt from STMPE811).
    pub fn init_exti() {
        let syscfg = unsafe { &*pac::SYSCFG::ptr() };
        let exti = unsafe { &*pac::EXTI::ptr() };

        // Map EXTI15 to PA15
        syscfg
            .exticr4
            .modify(|r, w| unsafe { w.bits((r.bits() & !0xF000) | 0x0000) });

        // Unmask EXTI line 15
        exti.imr
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 15)) });

        // Falling edge only
        exti.ftsr
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 15)) });
        exti.rtsr
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << 15)) });

        // Clear any pending
        exti.pr.write(|w| unsafe { w.bits(1 << 15) });
    }

    /// Clear the EXTI15 pending bit.
    pub fn clear_exti_pending() {
        let exti = unsafe { &*pac::EXTI::ptr() };
        exti.pr.write(|w| unsafe { w.bits(1 << 15) });
    }

    /// Read the STMPE811 chip ID (registers 0x00–0x01).
    /// Returns `Some(0x0811)` if the device is responsive.
    pub fn chip_id() -> Option<u16> {
        let hi = I2c3::read_reg(STMPE811_ADDR, 0x00)?;
        let lo = I2c3::read_reg(STMPE811_ADDR, 0x01)?;
        Some(((hi as u16) << 8) | (lo as u16))
    }

    /// Initialize with verification and retry (up to 3 attempts).
    /// Returns `true` if the STMPE811 responds with the correct chip ID
    /// after configuration, meaning I2C is healthy and interrupts will work.
    pub fn init_verified() -> bool {
        for attempt in 0..3u8 {
            if attempt > 0 {
                // Extra settle time between retries
                cortex_m::asm::delay(168_000 * 50); // ~50 ms
            }
            Self::init();
            // Let the STMPE811 settle after init
            cortex_m::asm::delay(168_000 * 5); // ~5 ms

            if let Some(0x0811) = Self::chip_id() {
                return true;
            }
        }
        false
    }
}

// ---- USER button (PA0 / EXTI0) -------------------------------------------------

/// Blue USER button (B1) on the STM32F429 Discovery.
/// PA0 active HIGH, external 10k pull-down on the board.
pub struct UserButton;

impl UserButton {
    /// Configure EXTI0 on PA0 for rising-edge (button press) interrupts.
    pub fn init_exti() {
        let syscfg = unsafe { &*pac::SYSCFG::ptr() };
        let exti = unsafe { &*pac::EXTI::ptr() };

        syscfg
            .exticr1
            .modify(|r, w| unsafe { w.bits((r.bits() & !0x000F) | 0x0000) });

        exti.imr
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 0)) });
        exti.rtsr
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 0)) });

        exti.pr.write(|w| unsafe { w.bits(1 << 0) });
    }

    /// Clear the EXTI0 pending bit.
    pub fn clear_exti_pending() {
        let exti = unsafe { &*pac::EXTI::ptr() };
        exti.pr.write(|w| unsafe { w.bits(1 << 0) });
    }
}
