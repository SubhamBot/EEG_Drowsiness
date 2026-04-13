import pandas as pd
import serial
import time
import threading
from pynput import keyboard

# --- Configuration ---
# Both EEG and speed data go through HC-05 Bluetooth on USART2.
# The USB-serial cable (COM6) had persistent write timeouts due to
# hardware flow control at the adapter chip level. Bluetooth works
# reliably. At 9600 baud with both streams (~280 bytes/sec), we're
# at ~29% utilization — plenty of headroom.
COM_PORT = 'COM7'
BAUD = 9600
FILE_NAME = 'acquiredDataset.csv'
SAMPLING_INTERVAL = 0.1

# --- Simulation State ---
state = {
    "speed": 60.0,
    "running": True
}

# --- Serial Initialization ---
try:
    ser = serial.Serial(
        COM_PORT, BAUD, timeout=0.1, write_timeout=2.0,
        rtscts=False, dsrdtr=False, xonxoff=False,
    )
    ser.reset_input_buffer()
    ser.reset_output_buffer()
    print(f"Connected → {COM_PORT} @ {BAUD} (HC-05 Bluetooth, EEG + Speed on USART2)")
except Exception as e:
    print(f"Serial Error: {e}")
    exit()

def on_press(key):
    try:
        if key == keyboard.Key.up:
            state["speed"] += 5.0
        elif key == keyboard.Key.down:
            state["speed"] = max(0.0, state["speed"] - 5.0)

        payload = f"S,{state['speed']:.1f}\n"
        try:
            ser.write(payload.encode())
        except serial.SerialTimeoutException:
            pass
    except Exception:
        pass

# Background Keyboard Listener
listener = keyboard.Listener(on_press=on_press)
listener.start()

def stream_data():
    df = pd.read_csv(FILE_NAME)
    print("Streaming started. Use UP/DOWN arrows to simulate speed.")

    # Send initial speed so the STM32 has a value from the start
    ser.write(f"S,{state['speed']:.1f}\n".encode())

    for _, row in df.iterrows():
        if not state["running"]:
            break

        alpha = row['lowAlpha'] + row['highAlpha']
        beta = row['lowBeta'] + row['highBeta']

        # EEG data now goes through Bluetooth (same port as speed)
        payload = f"E,{alpha:.1f},{beta:.1f}\n"
        try:
            ser.write(payload.encode())
        except serial.SerialTimeoutException:
            print(f"\n[Warning] Write timeout: packet dropped.", end='')

        print(f"Speed: {state['speed']:.1f} km/h | EEG Tx: {payload.strip()}    ", end='\r')
        time.sleep(SAMPLING_INTERVAL)

try:
    stream_data()
except KeyboardInterrupt:
    state["running"] = False
    print("\nSimulation Stopped.")
finally:
    ser.close()
