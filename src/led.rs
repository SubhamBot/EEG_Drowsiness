use stm32f4xx_hal::pac;

use crate::deadman::DeadmanState;

/// LED state indicator using the two user LEDs on STM32F429 Discovery.
///
///   LD3 = Green LED on PG13
///   LD4 = Red   LED on PG14
///
///   Green  state → solid green  (PG13 ON,  PG14 OFF)
///   Yellow state → both on      (PG13 ON,  PG14 ON)
///   Orange state → solid red    (PG13 OFF, PG14 ON)
///   Red    state → both blink   (PG13 TOGGLE, PG14 TOGGLE)
///
/// All functions are pure GPIO writes — no timers, no interrupts,
/// no shared state. Safe to call from any context.
pub struct StatusLed;

impl StatusLed {
    /// Enable GPIOG clock and configure PG13 + PG14 as push-pull outputs.
    /// Call once during init, AFTER `dp.GPIOA.split()` (which enables AHB1).
    pub fn init() {
        let rcc = unsafe { &*pac::RCC::ptr() };
        let gpiog = unsafe { &*pac::GPIOG::ptr() };

        // Enable GPIOG clock on AHB1
        rcc.ahb1enr.modify(|_, w| w.gpiogen().set_bit());

        // PG13, PG14 → general-purpose output, push-pull, low speed
        // MODER: 01 = output
        gpiog
            .moder
            .modify(|r, w| unsafe { w.bits((r.bits() & !(0b11 << 26) & !(0b11 << 28)) | (0b01 << 26) | (0b01 << 28)) });

        // OTYPER: 0 = push-pull (default, but be explicit)
        gpiog
            .otyper
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << 13) & !(1 << 14)) });

        // Start with both OFF
        gpiog
            .bsrr
            .write(|w| w.br13().set_bit().br14().set_bit());
    }

    /// Initialise TIM3 as blink timer for Red state (~4 Hz toggle).
    /// TIM3 on APB1: at 168 MHz SYSCLK, APB1=42 MHz, timer clock=84 MHz.
    /// Red state is always Full power, so timer clock is always 84 MHz.
    /// Call once during init, after `StatusLed::init()`.
    pub fn init_blink_timer() {
        let rcc = unsafe { &*pac::RCC::ptr() };
        rcc.apb1enr.modify(|_, w| w.tim3en().set_bit());

        let tim3 = unsafe { &*pac::TIM3::ptr() };
        tim3.cr1.modify(|_, w| w.cen().clear_bit());
        // 84 MHz / 8400 = 10 kHz tick, / 2500 = 4 Hz (250 ms period)
        tim3.psc.write(|w| unsafe { w.bits(8_399) });
        tim3.arr.write(|w| unsafe { w.bits(2_499) });
        tim3.dier.modify(|_, w| w.uie().set_bit());
        tim3.sr.modify(|_, w| w.uif().clear_bit());
    }

    /// Set the LEDs to reflect the current deadman state.
    /// For Green/Yellow/Orange: static pattern, blink timer stopped.
    /// For Red: starts TIM3 blink timer (both LEDs toggle at ~4 Hz).
    pub fn set(state: DeadmanState) {
        let gpiog = unsafe { &*pac::GPIOG::ptr() };

        // Always stop blink timer first (no-op if already stopped)
        Self::stop_blink();

        match state {
            DeadmanState::Green => {
                // Solid green: PG13 ON, PG14 OFF
                gpiog
                    .bsrr
                    .write(|w| w.bs13().set_bit().br14().set_bit());
            }
            DeadmanState::Yellow => {
                // Both on (amber): PG13 ON, PG14 ON
                gpiog
                    .bsrr
                    .write(|w| w.bs13().set_bit().bs14().set_bit());
            }
            DeadmanState::Orange => {
                // Solid red: PG13 OFF, PG14 ON
                gpiog
                    .bsrr
                    .write(|w| w.br13().set_bit().bs14().set_bit());
            }
            DeadmanState::Red => {
                // Both blink via TIM3: start with both ON, timer toggles
                gpiog
                    .bsrr
                    .write(|w| w.bs13().set_bit().bs14().set_bit());
                Self::start_blink();
            }
        }
    }

    /// Called from TIM3 ISR to toggle both LEDs (Red state blink).
    pub fn toggle() {
        let gpiog = unsafe { &*pac::GPIOG::ptr() };
        let odr = gpiog.odr.read().bits();
        let toggled = odr ^ (1 << 13) ^ (1 << 14);
        gpiog.odr.write(|w| unsafe { w.bits(toggled) });
    }

    fn start_blink() {
        let tim3 = unsafe { &*pac::TIM3::ptr() };
        tim3.cnt.write(|w| unsafe { w.bits(0) });
        tim3.egr.write(|w| w.ug().set_bit());
        tim3.sr.modify(|_, w| w.uif().clear_bit());
        tim3.cr1.modify(|_, w| w.cen().set_bit());
    }

    fn stop_blink() {
        let tim3 = unsafe { &*pac::TIM3::ptr() };
        tim3.cr1.modify(|_, w| w.cen().clear_bit());
        tim3.sr.modify(|_, w| w.uif().clear_bit());
    }
}
