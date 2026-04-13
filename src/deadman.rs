use crate::touch::TouchScreen;
use stm32f4xx_hal::pac;

// --------------- state machine ---------------------------------------------------

/// Dead man's switch state -- four-colour model.
///
///   Green  -- touch held, driver interaction present, monitoring paused (low power)
///   Yellow -- touch temporarily lost, warning / grace period before monitoring
///   Orange -- grace period expired, automatic high-speed monitoring activated
///   Red    -- manual override via USER button (suspected touchscreen malfunction)
#[derive(Clone, Copy, PartialEq)]
pub enum DeadmanState {
    Green,
    Yellow,
    Orange,
    Red,
}

/// Clock / power level derived from the current state.
#[derive(Clone, Copy, PartialEq)]
pub enum PowerMode {
    Low,
    Full,
}

/// Dead man's switch built on the resistive touchscreen + USER button.
///
/// State transitions (all interrupt-driven, zero polling):
///
///   EXTI15 -> handle_touch (touch edge)
///     Green  + release -> Yellow  (start TIM2 grace timer, stay low power)
///     Yellow + touch   -> Green   (cancel timer, stay low power)
///     Orange + touch   -> Green   (switch to low power)
///     Red    + any     -> Red     (ignore -- touchscreen suspected faulty)
///
///   TIM2 (grace timeout)
///     Yellow -> Orange  (switch to full power)
///
///   EXTI0 (USER button)
///     non-Red -> Red    (force full power, ignore touchscreen)
///     Red     -> Orange (exit override, resume normal monitoring)
pub struct DeadmanSwitch {
    state: DeadmanState,
}

pub const SAMPLING_MS_FULL: u32 = 100;
pub const SAMPLING_MS_LOW: u32 = 500;

const GRACE_PERIOD_MS: u32 = 3_000;

impl DeadmanSwitch {
    /// Start in Orange -- no touch at boot, full-speed monitoring.
    pub fn new() -> Self {
        Self {
            state: DeadmanState::Orange,
        }
    }

    // ---- queries ----------------------------------------------------------------

    pub fn state(&self) -> DeadmanState {
        self.state
    }

    pub fn power_mode(&self) -> PowerMode {
        match self.state {
            DeadmanState::Green | DeadmanState::Yellow => PowerMode::Low,
            DeadmanState::Orange | DeadmanState::Red => PowerMode::Full,
        }
    }

    pub fn sampling_ms(&self) -> u32 {
        match self.power_mode() {
            PowerMode::Low => SAMPLING_MS_LOW,
            PowerMode::Full => SAMPLING_MS_FULL,
        }
    }

    pub fn clock_mhz(&self) -> u32 {
        match self.power_mode() {
            PowerMode::Full => 168,
            PowerMode::Low => 48,
        }
    }

    pub fn state_label(&self) -> &'static str {
        match self.state {
            DeadmanState::Green => "GREEN",
            DeadmanState::Yellow => "YELLOW",
            DeadmanState::Orange => "ORANGE",
            DeadmanState::Red => "RED",
        }
    }

    // ---- interrupt handlers -----------------------------------------------------

    /// Called from the `handle_touch` software task (priority 1).
    ///
    /// Performs blocking I2C reads/writes (~800 us total) to read the
    /// STMPE811 touch state and clear its interrupt. Every I2C call has
    /// a hard timeout -- if the bus is stuck, we abort and treat touch
    /// as "not detected" (fail-safe: system stays in monitoring mode).
    pub fn on_touch_interrupt(&mut self) -> DeadmanState {
        let touched = TouchScreen::is_touched();
        TouchScreen::clear_interrupt();
        self.on_touch_event(touched)
    }

    /// Pure state-machine logic (no I/O).
    fn on_touch_event(&mut self, touched: bool) -> DeadmanState {
        match (self.state, touched) {
            (DeadmanState::Red, _) => {}

            (DeadmanState::Green, true) => {}
            (DeadmanState::Yellow, true) => {
                Self::stop_grace_timer();
                self.state = DeadmanState::Green;
            }
            (DeadmanState::Orange, true) => {
                self.transition(DeadmanState::Green);
            }

            (DeadmanState::Green, false) => {
                Self::start_grace_timer();
                self.state = DeadmanState::Yellow;
            }
            (DeadmanState::Yellow, false) => {}
            (DeadmanState::Orange, false) => {}
        }

        self.state
    }

    /// Called from TIM2 ISR when the grace period expires.
    pub fn on_grace_timeout(&mut self) -> DeadmanState {
        if self.state == DeadmanState::Yellow {
            self.transition(DeadmanState::Orange);
        }
        self.state
    }

    /// Called from EXTI0 ISR on USER button press.
    pub fn on_user_button(&mut self) -> DeadmanState {
        match self.state {
            DeadmanState::Red => {
                self.state = DeadmanState::Orange;
            }
            _ => {
                if self.state == DeadmanState::Yellow {
                    Self::stop_grace_timer();
                }
                self.transition(DeadmanState::Red);
            }
        }
        self.state
    }

    // ---- internal ---------------------------------------------------------------

    fn transition(&mut self, new_state: DeadmanState) {
        let old_power = self.power_mode();
        self.state = new_state;
        let new_power = self.power_mode();
        if old_power != new_power {
            Self::apply_clock(new_power);
        }
    }

    // ---- grace period timer (TIM2, one-pulse) -----------------------------------

    pub fn init_grace_timer() {
        let rcc = unsafe { &*pac::RCC::ptr() };
        rcc.apb1enr.modify(|_, w| w.tim2en().set_bit());

        let tim2 = unsafe { &*pac::TIM2::ptr() };
        tim2.cr1.modify(|_, w| w.cen().clear_bit());
        tim2.cr1.modify(|_, w| w.opm().set_bit());
        tim2.dier.modify(|_, w| w.uie().set_bit());
        tim2.sr.modify(|_, w| w.uif().clear_bit());
    }

    fn start_grace_timer() {
        let tim2 = unsafe { &*pac::TIM2::ptr() };
        tim2.cr1.modify(|_, w| w.cen().clear_bit());

        // At low power: timer clock = 48 MHz (APB1=24 MHz, x2)
        tim2.psc.write(|w| unsafe { w.bits(47_999) }); // -> 1 kHz
        tim2.arr.write(|w| w.bits(GRACE_PERIOD_MS - 1));

        tim2.cnt.write(|w| w.bits(0));
        tim2.egr.write(|w| w.ug().set_bit());
        tim2.sr.modify(|_, w| w.uif().clear_bit());
        tim2.cr1.modify(|_, w| w.cen().set_bit());
    }

    fn stop_grace_timer() {
        let tim2 = unsafe { &*pac::TIM2::ptr() };
        tim2.cr1.modify(|_, w| w.cen().clear_bit());
        tim2.sr.modify(|_, w| w.uif().clear_bit());
    }

    // ---- clock scaling (PLL reconfiguration) ------------------------------------

    /// Hard timeout for clock switch wait loops (~10 ms at 168 MHz, ~35 ms at 48 MHz).
    const CLK_TIMEOUT: u32 = 500_000;

    // Raw PLLCFGR values (bypasses PAC abstraction to avoid potential .write() issues):
    //   Bits 30:28: PLLR    Bits 27:24: PLLQ    Bit 22: PLLSRC = HSE (1)
    //   Bits 17:16: PLLP    Bits 14:6: PLLN     Bits 5:0: PLLM
    //
    // CRITICAL: PLLR (bits 30:28) MUST be ≥ 2 on STM32F429.
    // Writing 0 is invalid and prevents PLL from locking.
    //
    // Full: HSE/8 * 336 / 2 = 168 MHz, Q=7 for USB, R=2 (minimum valid)
    const PLLCFGR_FULL: u32 = (2 << 28) | (7 << 24) | (1 << 22) | (0b00 << 16) | (336 << 6) | 8;
    // Low:  HSE/8 * 192 / 4 =  48 MHz, Q=4 for USB, R=2 (minimum valid)
    const PLLCFGR_LOW: u32 = (2 << 28) | (4 << 24) | (1 << 22) | (0b01 << 16) | (192 << 6) | 8;

    fn apply_clock(mode: PowerMode) {
        let rcc = unsafe { &*pac::RCC::ptr() };
        let flash = unsafe { &*pac::FLASH::ptr() };

        // Step 1: Switch to HSI while we reconfigure PLL
        rcc.cr.modify(|_, w| w.hsion().set_bit());
        let mut t = Self::CLK_TIMEOUT;
        while rcc.cr.read().hsirdy().bit_is_clear() && t > 0 {
            t -= 1;
        }
        rcc.cfgr.modify(|_, w| w.sw().hsi());
        t = Self::CLK_TIMEOUT;
        while !rcc.cfgr.read().sws().is_hsi() && t > 0 {
            t -= 1;
        }

        // Step 2: Disable PLL
        rcc.cr.modify(|_, w| w.pllon().clear_bit());
        t = Self::CLK_TIMEOUT;
        while rcc.cr.read().pllrdy().bit_is_set() && t > 0 {
            t -= 1;
        }

        // Step 2b: Verify HSE is still running (PLL source).
        // If HSE stopped for any reason, re-enable it.
        if rcc.cr.read().hserdy().bit_is_clear() {
            rcc.cr.modify(|_, w| w.hseon().set_bit());
            t = Self::CLK_TIMEOUT;
            while rcc.cr.read().hserdy().bit_is_clear() && t > 0 {
                t -= 1;
            }
        }

        // Log state before PLL reconfig
        rtt_target::rprintln!(
            "[CLK] pre: CR={:#010x} PLLCFGR={:#010x}",
            rcc.cr.read().bits(),
            rcc.pllcfgr.read().bits(),
        );

        // Step 3: Flash latency + PLL config + bus prescalers
        let pllcfgr_val = match mode {
            PowerMode::Full => {
                // Going UP: increase flash wait states FIRST
                flash.acr.modify(|_, w| w.latency().bits(5));
                rcc.cfgr
                    .modify(|_, w| w.hpre().div1().ppre1().div4().ppre2().div2());
                Self::PLLCFGR_FULL
            }
            PowerMode::Low => {
                // Going DOWN: keep high flash WS until after switch
                rcc.cfgr
                    .modify(|_, w| w.hpre().div1().ppre1().div2().ppre2().div1());
                Self::PLLCFGR_LOW
            }
        };
        // Write PLLCFGR using raw bits (not PAC field methods)
        rcc.pllcfgr.write(|w| unsafe { w.bits(pllcfgr_val) });

        // Brief delay for PLL analog to latch new divider config
        cortex_m::asm::delay(200);

        rtt_target::rprintln!(
            "[CLK] wrote PLLCFGR={:#010x} (readback={:#010x})",
            pllcfgr_val,
            rcc.pllcfgr.read().bits(),
        );

        // Step 4: Re-enable PLL and wait for lock
        rcc.cr.modify(|_, w| w.pllon().set_bit());
        t = Self::CLK_TIMEOUT;
        while !rcc.cr.read().pllrdy().bit_is_set() && t > 0 {
            t -= 1;
        }

        rtt_target::rprintln!(
            "[CLK] PLL: CR={:#010x} t_left={}",
            rcc.cr.read().bits(),
            t,
        );

        if t == 0 {
            // PLL failed to lock — stay on HSI.
            // Prescalers were already changed; with HSI=16MHz:
            //   Full prescalers (div1/div4/div2): APB1=4MHz, APB2=8MHz
            //   Low  prescalers (div1/div2/div1): APB1=8MHz, APB2=16MHz
            flash.acr.modify(|_, w| w.latency().bits(0));
            match mode {
                PowerMode::Full => {
                    // APB1=4MHz, APB2=8MHz
                    Self::set_baud_usart1_raw(69);  // 8M / 115200
                    Self::set_baud_usart2_raw(417); // 4M / 9600
                }
                PowerMode::Low => {
                    // APB1=8MHz, APB2=16MHz
                    Self::set_baud_usart1_raw(139);  // 16M / 115200
                    Self::set_baud_usart2_raw(833);  // 8M / 9600
                }
            }
            rtt_target::rprintln!("[CLK] PLL FAILED - running on HSI 16 MHz");
            return;
        }

        // Step 5: Switch system clock to PLL
        rcc.cfgr.modify(|_, w| w.sw().pll());
        t = Self::CLK_TIMEOUT;
        while !rcc.cfgr.read().sws().is_pll() && t > 0 {
            t -= 1;
        }

        // Step 6: Reduce flash latency for Low mode (safe after switch)
        if matches!(mode, PowerMode::Low) {
            flash.acr.modify(|_, w| w.latency().bits(1));
        }

        // Step 7: Reconfigure UART baud rates for new PLL clock
        Self::set_baud_usart1(mode);
        Self::set_baud_usart2(mode);

        // Step 8: Reconfigure I2C3 timings
        match mode {
            PowerMode::Full => crate::i2c::I2c3::reconfigure(42),
            PowerMode::Low => crate::i2c::I2c3::reconfigure(24),
        }
    }

    fn set_baud_usart1(mode: PowerMode) {
        let brr: u16 = match mode {
            PowerMode::Full => 0x02D9, // 84 MHz APB2 / 115200
            PowerMode::Low => 0x01A1,  // 48 MHz APB2 / 115200
        };
        Self::set_baud_usart1_raw(brr);
    }

    fn set_baud_usart1_raw(brr: u16) {
        let usart1 = unsafe { &*pac::USART1::ptr() };
        usart1.brr.write(|w| unsafe { w.bits(brr as u32) });
    }

    fn set_baud_usart2(mode: PowerMode) {
        let brr: u16 = match mode {
            PowerMode::Full => 0x1117, // 42 MHz APB1 / 9600
            PowerMode::Low => 0x09C4,  // 24 MHz APB1 / 9600
        };
        Self::set_baud_usart2_raw(brr);
    }

    fn set_baud_usart2_raw(brr: u16) {
        let usart2 = unsafe { &*pac::USART2::ptr() };
        usart2.brr.write(|w| unsafe { w.bits(brr as u32) });
    }
}
