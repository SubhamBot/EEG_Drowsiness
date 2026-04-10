#![no_main]
#![no_std]

mod eeg_sensor;
mod speed_sensor;

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::eeg_sensor::EegSensor;
    use crate::speed_sensor::SpeedSensor;
    use core::fmt::Write;
    use heapless::spsc::{Consumer, Producer, Queue};
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal::{
        pac,
        prelude::*,
        serial::{config::Config, Event, Rx, Serial, Tx},
    };

    
    const MAILBOX_CAPACITY: usize = 512;
    const DROWSY_RATIO_THRESHOLD: f32 = 1.2;
    const T_MAX_SECONDS: f32 = 10.0;
    const V0: f32 = 60.0;
    const FRAME_PERIOD_MS: f32 = 100.0;

    
    struct MailboxWriter<'a> {
        producer: &'a mut Producer<'static, u8, MAILBOX_CAPACITY>,
    }

    impl<'a> core::fmt::Write for MailboxWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                let _ = self.producer.enqueue(byte);
            }
            Ok(())
        }
    }

    #[shared]
    struct Shared {
        current_speed: f32,
        drowsy_counter: u32,
        is_alert_active: bool,
    }

    #[local]
    struct Local {
        rx: Rx<pac::USART1>,
        tx: Tx<pac::USART1>,
        speed_iface: SpeedSensor,
        eeg_iface: EegSensor,
        buffer: [u8; 64],
        pos: usize,
        tx_producer: Producer<'static, u8, MAILBOX_CAPACITY>,
        tx_consumer: Consumer<'static, u8, MAILBOX_CAPACITY>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        static mut Q: Queue<u8, MAILBOX_CAPACITY> = Queue::new();
        let (tx_producer, tx_consumer) = unsafe { Q.split() };

        let dp = ctx.device;
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(168.MHz()).freeze();

        let gpioa = dp.GPIOA.split();
        let mut serial = Serial::new(
            dp.USART1,
            (gpioa.pa9.into_alternate(), gpioa.pa10.into_alternate()),
            Config::default().baudrate(115200.bps()),
            &clocks,
        ).unwrap();

        serial.listen(Event::RxNotEmpty);
        let (tx, rx) = serial.split();

        rprintln!("Init complete. UART Mailbox ready ({} bytes).", MAILBOX_CAPACITY);

        (
            Shared {
                current_speed: 60.0,
                drowsy_counter: 0,
                is_alert_active: false,
            },
            Local {
                rx,
                tx,
                speed_iface: SpeedSensor::new(),
                eeg_iface: EegSensor::new(),
                buffer: [0u8; 64],
                pos: 0,
                tx_producer,
                tx_consumer,
            },
            init::Monotonics(),
        )
    }

    
    #[task(binds = USART1, local = [rx, buffer, pos])]
    fn uart_handler(ctx: uart_handler::Context) {
        let rx = ctx.local.rx;
        let buf = ctx.local.buffer;
        let pos = ctx.local.pos;

        while let Ok(byte) = rx.read() {
            if byte == b'\n' || byte == b'\r' {
                if *pos > 0 {
                    let mut packet = [0u8; 64];
                    packet[..*pos].copy_from_slice(&buf[..*pos]);
                    process_logic::spawn(packet, *pos).ok();
                    *pos = 0;
                }
            } else if *pos < buf.len() {
                buf[*pos] = byte;
                *pos += 1;
            } else {
                *pos = 0; 
            }
        }
    }

    
    #[task(
        shared = [current_speed, drowsy_counter, is_alert_active],
        local  = [tx_producer, speed_iface, eeg_iface],
        capacity = 16
    )]
    fn process_logic(mut ctx: process_logic::Context, packet: [u8; 64], len: usize) {
        let s = match core::str::from_utf8(&packet[..len]) {
            Ok(s) => s.trim(),
            Err(_) => return,
        };
        if s.is_empty() { return; }

        rprintln!("RX: {}", s);

        
        let mut mb_tx = MailboxWriter { producer: ctx.local.tx_producer };

        
        if let Some(new_v) = ctx.local.speed_iface.parse_packet(s) {
            ctx.shared.current_speed.lock(|v| *v = new_v);

            
            let _ = writeln!(mb_tx, "[SPEED] {:.1} km/h", new_v);
            rprintln!("  [SPEED] -> {:.1} km/h", new_v);
            
            flush_mailbox::spawn().ok();
            return;
        }

        
        if let Some(eeg) = ctx.local.eeg_iface.parse_packet(s) {
            let ratio = eeg.alpha / eeg.beta;
            let v = ctx.shared.current_speed.lock(|v| *v);

            // Hill-function persistence math
            let v0_sq = V0 * V0;
            let t_v_seconds = T_MAX_SECONDS * v0_sq / ((v * v) + v0_sq);
            let frame_limit_f = t_v_seconds * (1000.0 / FRAME_PERIOD_MS);
            let frame_limit: u32 = if frame_limit_f < 1.0 { 1 } else { frame_limit_f as u32 };

            
            let (c_now, a_now) = ctx.shared.drowsy_counter.lock(|c| {
                if ratio > DROWSY_RATIO_THRESHOLD {
                    *c = c.saturating_add(1);
                } else {
                    *c = 0;
                }
                
                let alert = ctx.shared.is_alert_active.lock(|a| {
                    *a = *c >= frame_limit;
                    *a
                });
                (*c, alert)
            });

            let _ = writeln!(mb_tx, "[EEG] ratio={:.2} T={:.1}s lim={} cnt={} alert={}", 
                             ratio, t_v_seconds, frame_limit, c_now, a_now);

            rprintln!("  [EEG] a={:.1} b={:.1} ratio={:.2} T={:.1}s cnt={}/{} alert={}",
                      eeg.alpha, eeg.beta, ratio, t_v_seconds, c_now, frame_limit, a_now);

            flush_mailbox::spawn().ok();
        }
    }

   
    #[task(local = [tx, tx_consumer], priority = 1)]
    fn flush_mailbox(ctx: flush_mailbox::Context) {
        let tx = ctx.local.tx;
        let consumer = ctx.local.tx_consumer;

        while let Some(byte) = consumer.dequeue() {

            let _ = nb::block!(tx.write(byte));
        }
    }
}