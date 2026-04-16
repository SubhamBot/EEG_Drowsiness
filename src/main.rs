// Conditional no_std: when running `cargo test` on the host, we need std + test harness.
// When building for the ARM target, full no_std/no_main embedded binary.
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

// Always available (pure logic, no hardware dependencies)
mod alert;

// Hardware-dependent modules — only compiled for the embedded target
#[cfg(not(test))]
mod deadman;
#[cfg(not(test))]
mod eeg_sensor;
#[cfg(not(test))]
mod hc05;
#[cfg(not(test))]
mod i2c;
#[cfg(not(test))]
mod led;
#[cfg(not(test))]
mod logger;
#[cfg(not(test))]
mod speed_sensor;
#[cfg(not(test))]
mod touch;
#[cfg(not(test))]
mod uart;

#[cfg(not(test))]
use panic_halt as _;

// EXTI0 is reserved for USER button (PA0), so use SPI4 as a dispatcher instead.
#[cfg(not(test))]
#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SPI4, EXTI1, EXTI2, EXTI3])]
mod app {
    use crate::alert::AlertState;
    use crate::deadman::{DeadmanState, DeadmanSwitch, PowerMode};
    use crate::eeg_sensor::{EegData, EegSensor};
    use crate::i2c::I2c3;
    use crate::led::StatusLed;
    use crate::logger::LogWriter;
    use crate::speed_sensor::SpeedSensor;
    use crate::touch::{TouchScreen, UserButton};
    use crate::uart::{UartDma, UartPort};
    use core::fmt::Write;
    use stm32f4xx_hal::{
        pac,
        prelude::*,
        serial::{config::Config, Serial},
    };

    // --------------- buffer sizes ------------------------------------------------
    const EEG_RX_BUF_SIZE: usize = 128;
    const SPEED_RX_BUF_SIZE: usize = 128;

    // --------------- drowsiness algorithm constants ------------------------------
    const DROWSY_RATIO_THRESHOLD: f32 = 1.2;

    // --------------- RTIC resources ----------------------------------------------

    #[shared]
    struct Shared {
        current_speed: f32,
        alert: AlertState,
        deadman: DeadmanSwitch,
    }

    #[local]
    struct Local {
        eeg: EegSensor,
        eeg_rx_buf: &'static mut [u8; EEG_RX_BUF_SIZE],
        uart2: UartDma,
        speed_rx_buf: &'static mut [u8; SPEED_RX_BUF_SIZE],
    }

    // --------------- init --------------------------------------------------------

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        // RTT in NoBlockSkip mode -- if the host is slow, drop log lines
        // rather than stalling the MCU. Prevents RTT from ever blocking.
        use rtt_target::ChannelMode::NoBlockSkip;
        rtt_target::rtt_init_print!(NoBlockSkip);
        rtt_target::rprintln!("EEG drowsiness detection -- RTT active (NoBlockSkip)");

        // ---- static DMA buffers ----
        static mut EEG_DMA_BUF: [u8; EEG_RX_BUF_SIZE] = [0; EEG_RX_BUF_SIZE];
        static mut SPEED_DMA_BUF: [u8; SPEED_RX_BUF_SIZE] = [0; SPEED_RX_BUF_SIZE];

        let eeg_rx_buf: &'static mut [u8; EEG_RX_BUF_SIZE] =
            unsafe { &mut *core::ptr::addr_of_mut!(EEG_DMA_BUF) };
        let speed_rx_buf: &'static mut [u8; SPEED_RX_BUF_SIZE] =
            unsafe { &mut *core::ptr::addr_of_mut!(SPEED_DMA_BUF) };

        // ---- clocks & GPIO ----
        let dp = ctx.device;
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(168.MHz()).freeze();
        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();

        // ---- DMA clocks ----
        unsafe {
            let rcc_reg = &*pac::RCC::ptr();
            rcc_reg
                .ahb1enr
                .modify(|_, w| w.dma1en().set_bit().dma2en().set_bit());
        }

        // ---- USART1 for EEG sensor (PA9 TX, PA10 RX) ----
        let eeg_serial: Serial<_, u8> = Serial::new(
            dp.USART1,
            (gpioa.pa9.into_alternate(), gpioa.pa10.into_alternate()),
            Config::default().baudrate(115200.bps()),
            &clocks,
        )
        .unwrap();
        core::mem::forget(eeg_serial);

        // ---- USART2 for speed sensor via HC-05 (PA2 TX, PA3 RX) ----
        let speed_serial: Serial<_, u8> = Serial::new(
            dp.USART2,
            (gpioa.pa2.into_alternate(), gpioa.pa3.into_alternate()),
            Config::default().baudrate(9600.bps()),
            &clocks,
        )
        .unwrap();
        core::mem::forget(speed_serial);

        // ---- I2C3 bus recovery (bit-bang to free stuck STMPE811) ----
        I2c3::bus_recovery();
        // Let the STMPE811 settle after bus recovery before switching
        // SCL/SDA from GPIO to I2C alternate function.
        cortex_m::asm::delay(168_000 * 20); // ~20 ms

        // ---- I2C3 for touchscreen (PA8 SCL, PC9 SDA) ----
        let _scl = gpioa.pa8.into_alternate_open_drain::<4>();
        let _sda = gpioc.pc9.into_alternate_open_drain::<4>();
        I2c3::init();

        // ---- SYSCFG clock (needed for EXTI mapping) ----
        unsafe {
            let rcc_reg = &*pac::RCC::ptr();
            rcc_reg.apb2enr.modify(|_, w| w.syscfgen().set_bit());
        }

        // ---- PA15 as input for STMPE811 INT (active-low, board has 4.7k pull-up) ----
        let _touch_int = gpioa.pa15.into_floating_input();

        // ---- PA0 as input for USER button (active-high, external pull-down) ----
        let _user_btn = gpioa.pa0.into_floating_input();

        // ---- Sensor + touch + button init ----
        EegSensor::init_dma(eeg_rx_buf);
        SpeedSensor::init_dma(speed_rx_buf);

        // ---- Status LEDs (PG13 green, PG14 red) + blink timer (TIM3) ----
        StatusLed::init();
        StatusLed::init_blink_timer();
        StatusLed::set(DeadmanState::Orange); // Boot state = Orange

        // Initialize STMPE811 with verification (retries up to 3 times).
        // If I2C fails, the system stays in Orange (full monitoring) —
        // safe default, touch just won't work.
        let touch_ok = TouchScreen::init_verified();
        TouchScreen::init_exti();
        UserButton::init_exti();
        DeadmanSwitch::init_grace_timer();

        // ---- USART1 diagnostic (for debugging USB-serial cable) ----
        {
            let usart1 = unsafe { &*pac::USART1::ptr() };
            let dma2 = unsafe { &*pac::DMA2::ptr() };
            rtt_target::rprintln!(
                "USART1: CR1={:#06x} CR3={:#06x} BRR={:#06x} SR={:#06x}",
                usart1.cr1.read().bits(),
                usart1.cr3.read().bits(),
                usart1.brr.read().bits(),
                usart1.sr.read().bits(),
            );
            rtt_target::rprintln!(
                "DMA2_S2: CR={:#010x} NDTR={} M0AR={:#010x}",
                dma2.st[2].cr.read().bits(),
                dma2.st[2].ndtr.read().bits(),
                dma2.st[2].m0ar.read().bits(),
            );
            // Send test message on USART1 TX (PA9) — if the USB-serial
            // cable is connected, reader.py will print this.
            let test_msg = b"STM32_USART1_OK\r\n";
            for &byte in test_msg {
                while usart1.sr.read().txe().bit_is_clear() {}
                usart1.dr.write(|w| unsafe { w.bits(byte as u32) });
            }
            while usart1.sr.read().tc().bit_is_clear() {}
        }

        rtt_target::rprintln!(
            "Init complete: touch={} EXTI15 EXTI0 TIM2 | EEG now via BT (USART2)",
            if touch_ok { "OK(0x0811)" } else { "FAIL" }
        );

        (
            Shared {
                current_speed: 60.0,
                alert: AlertState::new(),
                deadman: DeadmanSwitch::new(),
            },
            Local {
                eeg: EegSensor::new(),
                eeg_rx_buf,
                uart2: UartDma::new(),
                speed_rx_buf,
            },
            init::Monotonics(),
        )
    }

    // --------------- sensor ISRs (thin shims) ------------------------------------

    /// USART1 IDLE -> EEG sensor extracts data -> spawns drowsiness check
    #[task(binds = USART1, local = [eeg, eeg_rx_buf])]
    fn usart1_idle(ctx: usart1_idle::Context) {
        ctx.local.eeg.on_idle(*ctx.local.eeg_rx_buf, |data| {
            drowsiness_check::spawn(data).ok();
        });
    }

    /// USART2 IDLE -> HC-05 Bluetooth carries BOTH speed and EEG packets.
    /// Each line is tried against both parsers (prefix-based: "S,..." or "E,...").
    #[task(binds = USART2, local = [uart2, speed_rx_buf, idle_count: u32 = 0])]
    fn usart2_idle(ctx: usart2_idle::Context) {
        *ctx.local.idle_count += 1;
        let cnt = *ctx.local.idle_count;
        // Log first few IDLE events to verify interrupt fires
        if cnt <= 3 {
            let dma1 = unsafe { &*pac::DMA1::ptr() };
            let ndtr = dma1.st[5].ndtr.read().bits();
            rtt_target::rprintln!(
                "[U2] IDLE #{} NDTR={}",
                cnt,
                ndtr
            );
        }
        ctx.local
            .uart2
            .on_idle(&UartPort::Usart2, *ctx.local.speed_rx_buf, |line| {
                if let Some(speed) = SpeedSensor::parse(line) {
                    update_speed::spawn(speed).ok();
                } else if let Some(eeg) = EegSensor::parse(line) {
                    drowsiness_check::spawn(eeg).ok();
                }
            });
    }

    // --------------- dead man's switch -------------------------------------------
    //
    // Architecture (no async I2C, no polling, no deadlocks):
    //
    //   EXTI15 ISR (priority 3, ~100 ns)
    //     |-- clears EXTI pending bit
    //     |-- spawns handle_touch software task
    //
    //   handle_touch (priority 1, ~800 us)
    //     |-- blocking I2C read: is_touched()     [with timeout]
    //     |-- blocking I2C writes: clear_interrupt [with timeout]
    //     |-- state machine transition
    //     |-- clock scaling if needed
    //     |-- log
    //
    // Why this can't deadlock:
    //   - RTIC uses Priority Ceiling Protocol (mathematically deadlock-free)
    //   - Every I2C wait loop has a 100K-iteration hard timeout
    //   - Bus recovery runs at boot (bit-bang SCL to free stuck slaves)
    //   - I2C failures return false/None, never hang
    //   - No I2C interrupts are enabled (ITEVTEN/ITBUFEN/ITERREN all off)
    //   - DMA buffers UART data during the ~800 us I2C window

    /// EXTI15_10 -- STMPE811 INT falling edge (touch event).
    /// Fast ISR: just clears EXTI and spawns deferred handler.
    #[task(binds = EXTI15_10, priority = 3)]
    fn exti15_touch(_ctx: exti15_touch::Context) {
        TouchScreen::clear_exti_pending();
        rtt_target::rprintln!("[EXTI15] touch IRQ");
        handle_touch::spawn().ok();
    }

    /// Deferred touch handler -- blocking I2C at priority 1.
    /// DMA buffers all UART data during the ~800 us I2C window.
    #[task(shared = [deadman], capacity = 2)]
    fn handle_touch(mut ctx: handle_touch::Context) {
        rtt_target::rprintln!("[TOUCH] handle start");
        let (state, clk, samp) = ctx.shared.deadman.lock(|d| {
            let s = d.on_touch_interrupt();
            rtt_target::rprintln!("[TOUCH] i2c+state done");
            (s, d.clock_mhz(), d.sampling_ms())
        });
        rtt_target::rprintln!("[TOUCH] clock done");

        StatusLed::set(state);

        let mut w = LogWriter::new();
        match state {
            DeadmanState::Green => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] GREEN -- touch held, low power ({} MHz / {} ms)",
                    clk, samp
                );
            }
            DeadmanState::Yellow => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] YELLOW -- touch lost, grace period ({} MHz / {} ms)",
                    clk, samp
                );
            }
            DeadmanState::Orange => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] ORANGE -- monitoring active ({} MHz / {} ms)",
                    clk, samp
                );
            }
            DeadmanState::Red => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] RED -- manual override ({} MHz / {} ms)",
                    clk, samp
                );
            }
        }
    }

    /// TIM2 -- grace period expired (Yellow -> Orange).
    #[task(binds = TIM2, shared = [deadman])]
    fn tim2_grace(mut ctx: tim2_grace::Context) {
        let tim2 = unsafe { &*pac::TIM2::ptr() };
        tim2.sr.modify(|_, w| w.uif().clear_bit());

        let (state, clk, samp) = ctx.shared.deadman.lock(|d| {
            let s = d.on_grace_timeout();
            (s, d.clock_mhz(), d.sampling_ms())
        });

        if state == DeadmanState::Orange {
            StatusLed::set(DeadmanState::Orange);
            let mut w = LogWriter::new();
            let _ = writeln!(
                w,
                "[DEADMAN] YELLOW -> ORANGE -- grace expired, full monitoring ({} MHz / {} ms)",
                clk, samp
            );
        }
    }

    /// TIM3 -- Red-state LED blink (~4 Hz toggle of both LEDs).
    #[task(binds = TIM3)]
    fn tim3_blink(_ctx: tim3_blink::Context) {
        let tim3 = unsafe { &*pac::TIM3::ptr() };
        tim3.sr.modify(|_, w| w.uif().clear_bit());
        StatusLed::toggle();
    }

    /// EXTI0 -- USER button press (toggle Red override).
    #[task(binds = EXTI0, shared = [deadman])]
    fn exti0_user_button(mut ctx: exti0_user_button::Context) {
        UserButton::clear_exti_pending();

        let (state, clk, samp) = ctx.shared.deadman.lock(|d| {
            let s = d.on_user_button();
            (s, d.clock_mhz(), d.sampling_ms())
        });

        StatusLed::set(state);

        let mut w = LogWriter::new();
        match state {
            DeadmanState::Red => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] RED -- USER button override ({} MHz / {} ms)",
                    clk, samp
                );
            }
            DeadmanState::Orange => {
                let _ = writeln!(
                    w,
                    "[DEADMAN] RED -> ORANGE -- override released ({} MHz / {} ms)",
                    clk, samp
                );
            }
            _ => {
                let _ = writeln!(w, "[DEADMAN] unexpected state after USER btn");
            }
        }
    }

    // --------------- application logic -------------------------------------------

    /// Update the shared speed value and log it.
    #[task(shared = [current_speed, deadman], capacity = 8)]
    fn update_speed(mut ctx: update_speed::Context, speed: f32) {
        ctx.shared.current_speed.lock(|v| *v = speed);
        let (clk, samp, label) = ctx
            .shared
            .deadman
            .lock(|d| (d.clock_mhz(), d.sampling_ms(), d.state_label()));
        let mut w = LogWriter::new();
        let _ = writeln!(
            w,
            "[SPEED] {:.1} km/h | {} {}MHz {}ms",
            speed, label, clk, samp
        );
    }

    /// Drowsiness detection with multi-level alert escalation.
    ///
    ///   5 consecutive drowsy frames  -> Alert1
    ///   10 consecutive drowsy frames -> Alert1 + Alert2
    ///   5 consecutive normal frames  -> de-escalate one level
    ///
    /// In Low power mode (Green/Yellow), only every 5th EEG packet is
    /// processed — effective sampling drops from 100 ms to 500 ms.
    #[task(shared = [current_speed, alert, deadman], local = [skip_count: u32 = 0], capacity = 8)]
    fn drowsiness_check(mut ctx: drowsiness_check::Context, eeg: EegData) {
        // ---- Low-power packet skipping ----
        // In Low mode: process 1 out of every 5 packets (500 ms effective rate).
        // In Full mode: process every packet (100 ms).
        let power_low = ctx
            .shared
            .deadman
            .lock(|d| d.power_mode() == PowerMode::Low);

        if power_low {
            *ctx.local.skip_count += 1;
            if *ctx.local.skip_count < 5 {
                return; // silently skip — save CPU cycles
            }
            *ctx.local.skip_count = 0;
        } else {
            *ctx.local.skip_count = 0;
        }

        // ---- drowsiness ratio ----
        let ratio = eeg.alpha / eeg.beta;
        let is_drowsy = ratio > DROWSY_RATIO_THRESHOLD;

        // ---- speed-scaled alert thresholds ----
        // Faster car -> shorter persistence window (fewer drowsy frames
        // needed before Alert1/Alert2). Targets ~5 m distance-to-Alert2.
        let v = ctx.shared.current_speed.lock(|v| *v);
        let (t1, t2) = crate::alert::thresholds_for_speed(v);

        // ---- feed into alert state machine ----
        let (level_label, d_cnt, n_cnt) = ctx.shared.alert.lock(|a| {
            a.update_with_thresholds(is_drowsy, t1, t2);
            (a.level_label(), a.drowsy_count(), a.normal_count())
        });

        let (clk, samp, label) = ctx
            .shared
            .deadman
            .lock(|d| (d.clock_mhz(), d.sampling_ms(), d.state_label()));

        let mut w = LogWriter::new();
        let _ = writeln!(
            w,
            "[EEG] ratio={:.2} drowsy={} dcnt={} ncnt={} {} | v={:.1} t1={} t2={} {} {}MHz {}ms",
            ratio, is_drowsy, d_cnt, n_cnt, level_label, v, t1, t2, label, clk, samp
        );
    }
}
