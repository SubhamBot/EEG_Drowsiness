# EEG Drowsiness Detection

Real-time drowsiness detection for STM32F429 using EEG alpha/beta ratio and vehicle speed, with a dead man's switch for adaptive power management. Built with RTIC.

## Module Graph

```mermaid
graph TD
    uart["uart.rs\n(DMA + IDLE)"]
    hc05["hc05.rs\n(Bluetooth SPP)"]
    i2c["i2c.rs\n(I2C3 bus)"]
    touch["touch.rs\n(STMPE811)"]
    deadman["deadman.rs\n(power policy)"]

    eeg["eeg_sensor.rs"] --> uart
    speed["speed_sensor.rs"] --> hc05
    hc05 --> uart
    touch --> i2c

    eeg -->|EegData| main["main.rs\n(drowsiness logic)"]
    speed -->|speed f32| main
    deadman -->|PowerMode| main
    deadman --> touch
    main --> logger["logger.rs\nLogWriter → RTT"]
```

## Data Flow

```mermaid
sequenceDiagram
    participant EEG_HW as EEG Sensor
    participant UART as uart.rs
    participant EEG as eeg_sensor.rs
    participant SPD_HW as Speed Sensor
    participant HC05 as hc05.rs
    participant SPD as speed_sensor.rs
    participant TOUCH as touch.rs / i2c.rs
    participant DM as deadman.rs
    participant MAIN as main.rs
    participant LOG as LogWriter (RTT)

    EEG_HW->>UART: USART1 RX (wire, 115200)
    UART->>EEG: line bytes
    EEG->>MAIN: EegData { alpha, beta }

    SPD_HW->>HC05: Bluetooth Classic
    HC05->>UART: USART2 RX (HC-05, 9600)
    UART->>SPD: line bytes
    SPD->>MAIN: speed f32

    TOUCH->>DM: is_touched()
    alt Touch held
        DM->>DM: 48 MHz, 500ms sampling
    else Touch released
        DM->>DM: 168 MHz, 100ms sampling
    end
    DM->>MAIN: PowerMode + sampling_ms

    MAIN->>MAIN: T(v) = T_max * v0^2 / (v^2 + v0^2)
    MAIN->>MAIN: frame_limit adapts to sampling rate
    MAIN->>LOG: [EEG] ratio=.. alert=.. pwr=..
```

## Architecture

- **uart.rs** — shared UART+DMA module. Circular DMA, IDLE interrupt, line assembly. Used by both sensors.
- **hc05.rs** — HC-05 Bluetooth Classic (SPP) wrapper over `uart.rs`.
- **i2c.rs** — shared I2C3 driver (PA8 SCL, PC9 SDA). Used by the touch controller.
- **touch.rs** — STMPE811 touch controller driver. Reports touch held/released.
- **deadman.rs** — dead man's switch policy. Touch held = low power (48 MHz, 500ms sampling). Released = full power (168 MHz, 100ms). Handles clock scaling and UART baud recalculation.
- **eeg_sensor.rs** — USART1, parses `E,alpha,beta`, delivers `EegData`.
- **speed_sensor.rs** — USART2 via HC-05, parses `S,speed`, delivers speed.
- **main.rs** — orchestrator. Drowsiness detection with adaptive persistence window `T(v) = T_max * v0^2 / (v^2 + v0^2)`. Frame limit adapts to the current sampling rate from the dead man's switch.
- **logger.rs** — RTT output to `cargo run` terminal.

## Dead Man's Switch

| State | Clock | Sampling | Rationale |
|---|---|---|---|
| Touch held | 48 MHz | 500 ms | Driver is attentive — save power |
| Touch released | 168 MHz | 100 ms | Driver may be drowsy — monitor aggressively |

## Running

Flash and open RTT log:
```sh
cargo run --release
```

Stream simulated sensor data from PC:
```sh
python reader.py
```

Configure `COM_PORT_EEG` and `COM_PORT_SPEED` in `reader.py` to match your adapters.
